#![warn(clippy::pedantic)]
use std::{
    fs::{create_dir, read_dir, File},
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

use clap::{value_parser, Arg, Command as Clap};
use image::{imageops::FilterType, io::Reader, GenericImageView, ImageError};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use temp_dir::TempDir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = cli().get_matches();
    let video_path = matches.get_one::<String>("video").unwrap();
    let output_dir = matches.get_one::<PathBuf>("output-dir").unwrap();

    let tmp = TempDir::new()?;

    // Run ffmpeg on specified file
    println!(">=== Running FFMPEG ===<");
    Command::new("ffmpeg")
        .args(
            format!(
                "-r 1 -i {video_path} -r 1 {}/%03d.png",
                tmp.path().to_str().unwrap()
            )
            .split_ascii_whitespace(),
        )
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()?;

    let frames = read_dir(tmp.path())?;

    frames
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect::<Vec<PathBuf>>() // If you don't want this parallelized,
        .into_par_iter() // . . . . . Remove this two lines.
        .for_each(|image| {
            let processed = process_image(&image).unwrap();

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

fn process_image(image: &PathBuf) -> Result<String, ImageError> {
    let image = Reader::open(image)?.decode()?;
    let size = image.dimensions();

    let smallest_side = size.0.min(size.1);

    let resized_img =
        image.resize_exact(smallest_side / 5, smallest_side / 19, FilterType::Nearest);

    let size = resized_img.dimensions();

    let mut res = String::new();

    for y in 0..size.1 {
        res.push_str("            ");
        for x in 0..size.0 {
            match resized_img.get_pixel(x, y)[0] {
                0..=20 => res.push(' '),
                21..=40 => res.push('.'),
                41..=80 => res.push(':'),
                81..=100 => res.push('-'),
                101..=130 => res.push('='),
                131..=200 => res.push('+'),
                201..=250 => res.push('#'),
                _ => res.push('@'),
            }
        }
        res.push('\n');
    }

    Ok(res)
}

fn cli() -> Clap<'static> {
    Clap::new("bad_apple")
        .version("0.1.0")
        .about("An asciinema compiler")
        .author("by S0ra")
        .args([
            Arg::new("video")
                .required(true)
                .index(1)
                .help("Input video to transform in asciinema")
                .takes_value(true),
            Arg::new("output-dir")
                .takes_value(true)
                .value_parser(value_parser!(PathBuf))
                .required(true)
                .help("Output directory\nCreates a directory if it doesn't exist")
                .index(2),
        ])
}
