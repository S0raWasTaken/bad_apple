#![warn(clippy::pedantic)]
#![allow(clippy::struct_excessive_bools)] // Allowing since struct Options is not a state machine.

use std::{
    error::Error,
    fs::{read_dir, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    str::FromStr,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
};

use image::{imageops::FilterType, io::Reader, GenericImageView, ImageError};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use tar::Builder;
use tempfile::TempDir;
use util::{add_file, cli, ffmpeg, max_sub, Options, OutputSize};
use zstd::encode_all;

use crate::util::{clean, clean_abort, pause};

mod util;

fn main() -> Result<(), Box<dyn Error>> {
    let matches = cli().get_matches();

    let options = Options {
        redimension: *matches.get_one::<OutputSize>("frame-size").unwrap(),
        colorize: matches.contains_id("colorize"),
        skip_compression: matches.contains_id("no-compression"),
        paint_fg: matches.contains_id("paint-fg"),
        compression_threshold: *matches.get_one::<u8>("compression-threshold").unwrap(),
        skip_audio: matches.contains_id("no-audio"),
    };
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

    let video_path = matches.get_one::<String>("video").unwrap();
    let mut output = matches.get_one::<PathBuf>("output").unwrap().clone();

    let tmp = Arc::new(TempDir::new_in(".")?);
    let tmp_path = tmp.path();

    let tmp_handler = Arc::clone(&tmp);

    let should_stop = Arc::new(AtomicBool::default());
    let stop_handle = Arc::clone(&should_stop);
    ctrlc::set_handler(move || {
        stop_handle.store(true, Ordering::Relaxed);
        clean_abort(tmp_handler.path());
    })?;

    println!(">=== Running FFMPEG ===<");

    // Split file into frames
    ffmpeg(
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
        clean_abort(tmp_path);
    });

    // Extract audio
    if !options.skip_audio {
        ffmpeg(
            &[
                "-i",
                video_path,
                &format!("{}/audio.mp3", tmp_path.to_str().unwrap()),
            ],
            &ffmpeg_flags,
        )
        .unwrap_or_else(|_| {
            clean_abort(tmp_path);
        });
    }

    let frames = read_dir(tmp_path)?
        .filter_map(Result::ok)
        .filter(|e| e.file_name() != *"audio.mp3")
        .map(|entry| entry.path())
        .collect::<Vec<PathBuf>>();

    println!("\nStarting frame generation ...");

    read_frames(frames, tmp_path, &mut output, options, &should_stop);

    println!(
        "\n\n\
        >=== Done! ===<\n\
        >> Output available at {}",
        output.display()
    );

    clean(tmp_path);
    Ok(())
}

fn read_frames(
    frames: Vec<PathBuf>,
    tmp_path: &Path,
    output: &mut PathBuf,
    options: Options,
    should_stop: &Arc<AtomicBool>,
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

                    clean_abort(tmp_path); // Prevents littering temporary directory when image processing fails
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

    // Finally add the audio to the archive and finish
    if !options.skip_audio {
        let mut audio = File::open(tmp_path.join("audio.mp3")).unwrap();
        let mut data = Vec::new();
        audio.read_to_end(&mut data).unwrap();

        add_file(&mut tar_archive, "audio.mp3", &data).unwrap();
    }

    tar_archive.finish().unwrap();
}

fn process_image(image: &PathBuf, options: Options) -> Result<String, ImageError> {
    let image = Reader::open(image)?.decode()?;

    let resized_image = image.resize_exact(
        options.redimension.0,
        options.redimension.1,
        FilterType::Nearest,
    );

    let size = resized_image.dimensions();

    let mut res = String::new();
    let mut last_pixel_rgb = resized_image.get_pixel(size.0 - 1, size.1 - 1);
    let mut is_first_row_pixel = true;

    for y in 0..size.1 {
        for x in 0..size.0 {
            let [r, g, b, _] = resized_image.get_pixel(x, y).0;

            macro_rules! colorize {
                ($input:expr) => {
                    if options.colorize
                        && (max_sub(last_pixel_rgb[0], r) > options.compression_threshold
                            || max_sub(last_pixel_rgb[1], g) > options.compression_threshold
                            || max_sub(last_pixel_rgb[2], b) > options.compression_threshold
                            || is_first_row_pixel)
                        || options.skip_compression
                    {
                        res.push_str(&format!(
                            "\x1b[{}8;2;{r};{g};{b}m{}",
                            if options.paint_fg { 3 } else { 4 },
                            $input
                        ));
                    } else {
                        res.push($input);
                    }
                };
            }

            match r {
                0..=20 => colorize!(' '),
                21..=40 => colorize!('.'),
                41..=80 => colorize!(':'),
                81..=100 => colorize!('-'),
                101..=130 => colorize!('='),
                131..=200 => colorize!('+'),
                201..=250 => colorize!('#'),
                _ => colorize!('@'),
            }

            last_pixel_rgb.0 = [r, g, b, 255];
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
