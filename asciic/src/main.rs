#![warn(clippy::pedantic)]

use std::{
    error::Error,
    fmt::Write as FmtWrite,
    fs::{File, read_dir},
    io::{Read, Write},
    path::{Path, PathBuf},
    str::FromStr,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

use image::{GenericImageView, ImageError, imageops::FilterType, io::Reader};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use ron::ser::PrettyConfig;
use tar::Builder;
use tempfile::TempDir;
use tokio::runtime::Runtime;
use zstd::encode_all;

use cli::cli;
use primitives::{
    Options, OutputSize,
    PaintStyle::{self, BgOnly, BgPaint, FgPaint},
};
use util::{add_file, clean, clean_abort, ffmpeg, max_sub, pause};

use crate::{
    installer::{setup_ffmpeg, setup_ytdlp},
    primitives::Metadata,
    util::{probe_fps, yt_dlp},
};

mod cli;
mod installer;
mod primitives;
mod util;

#[allow(clippy::too_many_lines)]
fn main() -> Result<(), Box<dyn Error>> {
    let matches = cli().get_matches();

    let mut options = Options {
        boost: matches.contains_id("brightness-boost"),
        redimension: *matches.get_one::<OutputSize>("frame-size").unwrap(),
        colorize: matches.contains_id("colorize"),
        skip_compression: matches.contains_id("no-compression"),
        style: *matches.get_one::<PaintStyle>("style").unwrap(),
        compression_threshold: *matches.get_one::<u8>("compression-threshold").unwrap(),
        skip_audio: matches.contains_id("no-audio"),
        should_delete_video: matches.is_present("delete"),
    };

    if options.redimension == OutputSize(0, 0) {
        let Some((width, height)) = term_size::dimensions() else {
            return Err("Could not detect terminal window size.".into());
        };

        options.redimension = OutputSize(width, height);
    }

    let ffmpeg_flags = matches
        .get_many::<String>("ffmpeg-flags")
        .unwrap_or_default()
        .collect::<Vec<_>>();

    if let Some(image) = matches.get_one::<String>("image") {
        let image_path = PathBuf::from_str(image)?;
        let processed_img = process_image(&image_path, options)?;

        File::create(format!(
            "{}.txt",
            image_path.file_stem().unwrap().to_str().unwrap()
        ))?
        .write_all(processed_img.as_bytes())?;
        return Ok(());
    }

    let mut output = matches.get_one::<PathBuf>("output").unwrap().clone();

    let rt = Runtime::new()?;

    let (ffmpeg_path, ffprobe_path) = setup_ffmpeg(&rt)?;

    let video_path = &{
        if let Some(path) = matches.get_one::<String>("video") {
            path.clone()
        } else {
            let ytdlp_path = setup_ytdlp(&rt)?;
            let mp4_output = format!("{}{}", output.to_str().unwrap(), ".mp4");
            yt_dlp(
                &ytdlp_path,
                matches.get_one::<String>("youtube").unwrap(),
                &mp4_output,
            )?;
            mp4_output
        }
    };

    let cloned_video_path = video_path.clone();

    let fps = probe_fps(video_path, &ffprobe_path)?;

    let tmp = Arc::new(TempDir::new_in(".")?);
    let tmp_path = tmp.path();

    let tmp_handler = Arc::clone(&tmp);

    let should_stop = Arc::new(AtomicBool::default());
    let stop_handle = Arc::clone(&should_stop);
    ctrlc::set_handler(move || {
        stop_handle.store(true, Ordering::Relaxed);
        clean_abort(
            tmp_handler.path(),
            options.should_delete_video,
            &cloned_video_path,
        );
    })?;

    println!(">=== Running FFMPEG ===<");

    // Split file into frames
    ffmpeg(
        &ffmpeg_path,
        &[
            "-r",
            "1",
            "-i",
            video_path,
            "-r",
            "1",
            &format!("{}/%03d.png", tmp_path.to_str().unwrap()),
        ],
        &ffmpeg_flags,
    )
    .unwrap_or_else(|_| {
        clean_abort(tmp_path, options.should_delete_video, video_path);
    });

    // Extract audio
    if !options.skip_audio {
        ffmpeg(
            &ffmpeg_path,
            &[
                "-i",
                video_path,
                &format!("{}/audio.mp3", tmp_path.to_str().unwrap()),
            ],
            &ffmpeg_flags,
        )
        .unwrap_or_else(|_| {
            clean_abort(tmp_path, options.should_delete_video, video_path);
        });
    }

    let frames = read_dir(tmp_path)?
        .filter_map(Result::ok)
        .filter(|e| e.file_name() != *"audio.mp3")
        .map(|entry| entry.path())
        .collect::<Vec<PathBuf>>();

    println!("\nStarting frame generation ...");

    read_frames(
        frames,
        tmp_path,
        &mut output,
        options,
        &should_stop,
        video_path,
        fps,
    );

    println!(
        "\n\n\
        >=== Done! ===<\n\
        >> Output available at {}",
        output.display()
    );

    clean(tmp_path, options.should_delete_video, video_path);
    Ok(())
}

fn read_frames(
    frames: Vec<PathBuf>,
    tmp_path: &Path,
    output: &mut PathBuf,
    options: Options,
    should_stop: &Arc<AtomicBool>,
    video_path: &str,
    fps: usize,
) {
    output.set_extension("bapple");
    let processed = AtomicUsize::new(0);
    let total = frames.len();

    let mut tar_archive = Builder::new(File::create(output).unwrap());

    let encoded_frames = frames
        .into_par_iter()
        .map(|path| {
            if should_stop.load(Ordering::Relaxed) {
                pause();
            }
            let image = match process_image(&path, options) {
                Ok(p) => p,
                Err(error) => {
                    eprintln!("Image processing failed. This is probably an ffmpeg related issue");
                    eprintln!("You should try rerunning this program.");
                    eprintln!("In any case, here's the error message: \n\n{error:?}");

                    clean_abort(tmp_path, options.should_delete_video, video_path); // Prevents littering temporary directory when image processing fails
                }
            };

            processed.fetch_add(1, Ordering::Relaxed);
            let now = processed.load(Ordering::Relaxed);

            print!("\rProcessing: {}% {now}/{total}", (100 * now) / total);

            // Linking

            (path, encode_all(image.as_bytes(), 1).unwrap())
        })
        .collect::<Vec<_>>();

    let mut processed = 0;

    // Handle file IO on a single thread to prevent inconsistencies
    for (path, data) in encoded_frames {
        processed += 1;
        print!(
            "\rLinking: {}% {processed}/{total}",
            (processed * 100) / total
        );

        let mut inside_path = PathBuf::from(".");
        inside_path.set_file_name(path.file_stem().unwrap());
        inside_path.set_extension("zst");

        add_file(&mut tar_archive, &inside_path, &data).unwrap();
    }

    // Finally add the audio and metadata to the archive and finish
    if !options.skip_audio {
        let mut audio = File::open(tmp_path.join("audio.mp3")).unwrap();
        let mut data = Vec::new();
        audio.read_to_end(&mut data).unwrap();

        add_file(&mut tar_archive, "audio.mp3", &data).unwrap();
    }

    let metadata = ron::Options::default()
        .to_string_pretty(&Metadata::new(fps), PrettyConfig::default())
        .unwrap();

    add_file(&mut tar_archive, "metadata.ron", metadata.as_bytes()).unwrap();

    tar_archive.finish().unwrap();
}

fn process_image(image: &PathBuf, options: Options) -> Result<String, ImageError> {
    let image = Reader::open(image)?.decode()?;

    let resized_image = image.resize_exact(
        u32::try_from(options.redimension.0).unwrap(),
        u32::try_from(options.redimension.1).unwrap(),
        FilterType::Nearest,
    );

    let size = resized_image.dimensions();

    let mut res = String::new();
    let mut last_colorized_pixel = resized_image.get_pixel(size.0 - 1, size.1 - 1);
    let mut is_first_row_pixel = true;

    for y in 0..size.1 {
        for x in 0..size.0 {
            let [r, g, b, _] = resized_image.get_pixel(x, y).0;

            let mut was_colorized = false;

            let brightness = r.max(g).max(b);

            let (thresholds, chars) = if options.boost {
                (
                    [5, 10, 15, 20, 25, 40, 200],
                    [' ', '.', ':', '-', '+', '=', '#'],
                )
            } else {
                (
                    [20, 40, 80, 100, 130, 200, 250],
                    [' ', '.', ':', '-', '+', '=', '#'],
                )
            };

            let char = chars
                .iter()
                .zip(thresholds)
                .find(|(_, th)| brightness <= *th)
                .map_or('@', |(&c, _)| c);

            if options.colorize
                && (max_sub(last_colorized_pixel[0], r) > options.compression_threshold
                    || max_sub(last_colorized_pixel[1], g) > options.compression_threshold
                    || max_sub(last_colorized_pixel[2], b) > options.compression_threshold
                    || is_first_row_pixel)
                || options.skip_compression
            {
                was_colorized = true;
                write!(
                    res,
                    "\x1b[{}8;2;{r};{g};{b}m{}",
                    match options.style {
                        BgPaint | BgOnly => 4,
                        FgPaint => 3,
                    },
                    match options.style {
                        BgPaint | FgPaint => char,
                        BgOnly => ' ',
                    }
                )
                .unwrap();
            } else {
                res.push(match options.style {
                    BgPaint | FgPaint => char,
                    BgOnly => ' ',
                });
            }

            if was_colorized {
                last_colorized_pixel.0 = [r, g, b, 255];
            }
            is_first_row_pixel = false;
        }
        if options.colorize {
            res.push_str("\x1b[0m\n");
        } else {
            res.push('\n');
        }
        is_first_row_pixel = true;
    }

    Ok(res)
}
