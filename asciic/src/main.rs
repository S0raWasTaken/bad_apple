#![warn(clippy::pedantic)]
use std::{
    error::Error,
    fs::{self, create_dir, read_dir, remove_dir_all, File},
    io::Write,
    path::{Path, PathBuf},
    process::exit,
    str::FromStr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, RwLock,
    },
    time::{Duration, Instant},
};

use image::{imageops::FilterType, io::Reader, GenericImageView, ImageError};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use tempfile::TempDir;
use util::{cli, ffmpeg, max_sub, Options, OutputSize};

mod util;

fn main() -> Result<(), Box<dyn Error>> {
    let matches = cli().get_matches();

    let options = Options {
        redimension: *matches.get_one::<OutputSize>("frame-size").unwrap(),
        colorize: matches.contains_id("colorize"),
        skip_compression: matches.contains_id("no-compression"),
        paint_fg: matches.contains_id("paint-fg"),
        compression_threshold: *matches.get_one::<u8>("compression-threshold").unwrap(),
    };

    let skip_audio = matches.contains_id("no-audio");
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
    let output_dir = matches.get_one::<PathBuf>("output-dir").unwrap();

    if !output_dir.exists() {
        create_dir(output_dir).unwrap();
    }

    let tmp = TempDir::new_in(output_dir)?;
    let tmp_path = tmp.path();

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
    )?;

    // Extract audio
    if !skip_audio {
        ffmpeg(
            &[
                "-i",
                video_path,
                &format!("{}/audio.mp3", output_dir.to_str().unwrap()),
            ],
            &ffmpeg_flags,
        )?;
    }

    let frames = read_dir(tmp_path)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect::<Vec<PathBuf>>();

    println!("\nStarting frame generation ...");

    read_frames(frames, tmp_path, output_dir, options);

    println!(
        "\n\
        >=== Done! ===<\n\
        >> Output available at {output_dir:?}"
    );

    Ok(())
}

fn read_frames(frames: Vec<PathBuf>, tmp_path: &Path, output_dir: &Path, options: Options) {
    let processed = AtomicUsize::new(0);
    let average = AtomicUsize::new(0);
    let time = Arc::new(RwLock::new(Instant::now()));
    let eta = Arc::new(RwLock::new(Duration::from_secs(0)));

    let total = frames.len();

    frames
        // If you don't want this parallelized,
        .into_par_iter() // . . . . . Remove this two lines.
        .for_each(|path| {
            let image = match process_image(&path, options) {
                Ok(p) => p,
                Err(error) => {
                    eprintln!("Image processing failed. This is probably an ffmpeg related issue");
                    eprintln!("You should try rerunning this program.");
                    eprintln!("In any case, here's the error message: \n\n{error:?}");

                    remove_dir_all(tmp_path).unwrap(); // Prevents littering temporary directory when image processing fails
                    exit(1);
                }
            };

            let out = format!(
                "{}/{}.txt",
                output_dir.to_str().unwrap(),
                path.file_stem().unwrap().to_str().unwrap()
            );

            fs::write(out, image.as_bytes()).unwrap();

            processed.fetch_add(1, Ordering::Relaxed);
            average.fetch_add(1, Ordering::Relaxed);
            let now = processed.load(Ordering::Relaxed);

            print!(
                "\rProcessing: {}% {now}/{total} (ETA: {:?})",
                (100 * now) / total,
                eta.read().unwrap()
            );

            if time.read().unwrap().elapsed() >= Duration::from_millis(512) {
                *eta.write().unwrap() = Duration::from_secs(
                    ((total - now).saturating_sub(average.load(Ordering::Relaxed)) / 30) as _,
                );

                average.store(0, Ordering::Relaxed);
                *time.write().unwrap() = Instant::now();
            }
        });
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
        res.push_str("            ");
        for x in 0..size.0 {
            let [r, g, b, _] = resized_image.get_pixel(x, y).0;

            let mut colorize = |input: char| {
                if options.colorize
                    && (max_sub(last_pixel_rgb[0], r) > options.compression_threshold
                        || max_sub(last_pixel_rgb[1], g) > options.compression_threshold
                        || max_sub(last_pixel_rgb[2], b) > options.compression_threshold
                        || is_first_row_pixel)
                    || options.skip_compression
                {
                    res.push_str(&format!(
                        "\x1b[{}8;2;{r};{g};{b}m{input}",
                        if options.paint_fg { 3 } else { 4 }
                    ));
                } else {
                    res.push(input);
                }
            };

            match r {
                0..=20 => colorize(' '),
                21..=40 => colorize('.'),
                41..=80 => colorize(':'),
                81..=100 => colorize('-'),
                101..=130 => colorize('='),
                131..=200 => colorize('+'),
                201..=250 => colorize('#'),
                _ => colorize('@'),
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
