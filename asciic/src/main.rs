#![warn(clippy::pedantic)]
use std::{
    error::Error,
    fs::{create_dir, read_dir, remove_dir_all, File},
    io::Write,
    path::PathBuf,
    process::exit,
    str::FromStr,
};

use image::{imageops::FilterType, io::Reader, GenericImageView, ImageError};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use temp_dir::TempDir;
use util::{cli, ffmpeg, max_sub, OutputSize};

mod util;

fn main() -> Result<(), Box<dyn Error>> {
    let matches = cli().get_matches();
    let redimension = matches.get_one::<OutputSize>("frame-size").unwrap();
    let colorize = matches.contains_id("colorize");
    let skip_compression = matches.contains_id("no-compression");
    let compression_threshold = matches.get_one::<u8>("compression-threshold").unwrap();

    if let Some(image) = matches.get_one::<String>("image") {
        let image_path = PathBuf::from_str(image)?;
        let processed_img = process_image(
            &image_path,
            *redimension,
            colorize,
            skip_compression,
            *compression_threshold,
        )?;

        File::create(format!(
            "{}.txt",
            image_path.file_stem().unwrap().to_str().unwrap()
        ))?
        .write_all(processed_img.as_bytes())?;
        return Ok(());
    }

    let video_path = matches.get_one::<String>("video").unwrap();
    let output_dir = matches.get_one::<PathBuf>("output-dir").unwrap();

    let tmp = TempDir::new()?;
    let tmp_path = tmp.path();

    println!(">=== Running FFMPEG ===<");

    dbg!(video_path);

    // Split file into frames
    ffmpeg(&[
        "-r",
        "1",
        "-i",
        video_path,
        "-r",
        "1",
        &format!("{}/%03d.png", tmp.path().to_str().unwrap()),
    ])?;

    // Extract audio
    ffmpeg(&[
        "-i",
        video_path,
        &format!("{}/audio.mp3", output_dir.to_str().unwrap()),
    ])?;

    let frames = read_dir(tmp_path)?;

    frames
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect::<Vec<PathBuf>>() // If you don't want this parallelized,
        .into_par_iter() // . . . . . Remove this two lines.
        .for_each(|image| {
            let processed = match process_image(
                &image,
                *redimension,
                colorize,
                skip_compression,
                *compression_threshold,
            ) {
                Ok(p) => p,
                Err(error) => {
                    eprintln!("Image processing failed. This is probably an ffmpeg related issue");
                    eprintln!("You should try rerunning this program.");
                    eprintln!("In any case, here's the error message: \n\n{error:?}");

                    remove_dir_all(tmp_path).unwrap(); // Prevents littering /tmp when image processing fails
                    exit(1);
                }
            };

            if !output_dir.exists() {
                create_dir(output_dir).unwrap();
            }

            let mut output = File::create(format!(
                "{}/{}.txt",
                output_dir.to_str().unwrap(),
                image.file_stem().unwrap().to_str().unwrap()
            ))
            .unwrap();

            output.write_all(processed.as_bytes()).unwrap();
        });

    println!(
        "\n\
        >=== Done! ===<\n\
        >> Output available at {output_dir:?}"
    );

    Ok(())
}

fn process_image(
    image: &PathBuf,
    redimension: OutputSize,
    colorize: bool,
    skip_compression: bool,
    threshold: u8,
) -> Result<String, ImageError> {
    let image = Reader::open(image)?.decode()?;

    let resized_image = image.resize_exact(redimension.0, redimension.1, FilterType::Nearest);

    let size = resized_image.dimensions();

    let mut res = String::new();

    let mut last_pixel_rgb = resized_image.get_pixel(size.0 - 1, size.1 - 1);

    for y in 0..size.1 {
        res.push_str("            ");
        for x in 0..size.0 {
            let [r, g, b, _] = resized_image.get_pixel(x, y).0;

            let mut colorize = |input: char| {
                if (colorize
                    && (max_sub(last_pixel_rgb[0], r) > threshold
                        || max_sub(last_pixel_rgb[1], g) > threshold
                        || max_sub(last_pixel_rgb[2], b) > threshold))
                    || skip_compression
                {
                    res.push_str(&format!("\x1b[38;2;{r};{g};{b}m{input}"));
                } else {
                    res.push_str(&input.to_string());
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
        }
        res.push('\n');
    }

    Ok(res)
}
