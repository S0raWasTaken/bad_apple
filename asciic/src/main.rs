#![warn(clippy::pedantic)]
use std::{
    fs::{create_dir, read_dir, File},
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
    str::FromStr,
};

use clap::{
    builder::{TypedValueParser, ValueParserFactory},
    value_parser, Arg, Command as Clap, ErrorKind,
};
use image::{imageops::FilterType, io::Reader, GenericImageView, ImageError};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use temp_dir::TempDir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = cli().get_matches();
    let redimension = matches.get_one::<OutputSize>("frame-size").unwrap();

    if let Some(image) = matches.get_one::<String>("image") {
        let image_path = PathBuf::from_str(image)?;
        let processed_img = process_image(&image_path, *redimension)?;

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

    let frames = read_dir(tmp.path())?;

    frames
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect::<Vec<PathBuf>>() // If you don't want this parallelized,
        .into_par_iter() // . . . . . Remove this two lines.
        .for_each(|image| {
            let processed = process_image(&image, *redimension).unwrap();

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

fn ffmpeg(args: &[&str]) -> std::io::Result<()> {
    Command::new("ffmpeg")
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()?;

    Ok(())
}

fn process_image(image: &PathBuf, redimension: OutputSize) -> Result<String, ImageError> {
    let image = Reader::open(image)?.decode()?;

    let resized_img = image.resize_exact(redimension.0, redimension.1, FilterType::Nearest);

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

#[derive(Debug, Clone, Copy)]
struct OutputSize(pub u32, pub u32);
impl ValueParserFactory for OutputSize {
    type Parser = OutputSizeParser;

    fn value_parser() -> Self::Parser {
        OutputSizeParser
    }
}

#[derive(Debug, Clone, Copy)]
struct OutputSizeParser;
impl TypedValueParser for OutputSizeParser {
    type Value = OutputSize;

    fn parse_ref(
        &self,
        cmd: &clap::Command,
        _: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let value = value
            .to_str()
            .ok_or_else(|| {
                cmd.clone()
                    .error(ErrorKind::InvalidUtf8, "Not UTF8, try 216x56.")
            })?
            .to_ascii_lowercase();

        let vals = value.split('x').collect::<Vec<_>>();
        if vals.len() != 2 {
            return Err(cmd
                .clone()
                .error(ErrorKind::InvalidValue, "Wrong pattern, try 216x56."));
        }
        let output_size = OutputSize(
            vals.first()
                .unwrap()
                .parse::<u32>()
                .map_err(|e| cmd.clone().error(ErrorKind::InvalidValue, e.to_string()))?,
            vals.last()
                .unwrap()
                .parse::<u32>()
                .map_err(|e| cmd.clone().error(ErrorKind::InvalidValue, e.to_string()))?,
        );

        if output_size.0 > 400 || output_size.1 > 200 {
            println!("WARN: Usually going too high on frame size makes stuff a bit wonky.");
        }

        Ok(output_size)
    }
}

fn cli() -> Clap<'static> {
    Clap::new("asciic")
        .version("0.1.0")
        .about("An asciinema compiler")
        .author("by S0ra")
        .args([
            Arg::new("video")
                .required_unless_present("image")
                .index(1)
                .help("Input video to transform in asciinema")
                .takes_value(true),
            Arg::new("output-dir")
                .takes_value(true)
                .value_parser(value_parser!(PathBuf))
                .required_unless_present("image")
                .help("Output directory\nCreates a directory if it doesn't exist")
                .index(2),
            Arg::new("frame-size")
                .short('s')
                .default_value("216x56")
                .long("size")
                .takes_value(true)
                .required(false)
                .help("The ratio that each frame should be resized")
                .value_parser(value_parser!(OutputSize)),
            Arg::new("image")
                .short('i')
                .takes_value(true)
                .help("compiles a single image"),
        ])
}
