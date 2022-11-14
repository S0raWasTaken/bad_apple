use std::{
    path::PathBuf,
    process::{Command, Stdio},
};

use clap::{
    builder::{TypedValueParser, ValueParserFactory},
    value_parser, Arg, Command as Clap, ErrorKind,
};

pub fn ffmpeg(args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("ffmpeg")
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()?;

    if !output.status.success() {
        return Err("FFMPEG failed to run".into());
    }

    Ok(())
}

#[inline]
pub fn max_sub(a: u8, b: u8) -> u8 {
    a.max(b) - a.min(b)
}

#[derive(Debug, Clone, Copy)]
pub struct OutputSize(pub u32, pub u32);
impl ValueParserFactory for OutputSize {
    type Parser = OutputSizeParser;

    fn value_parser() -> Self::Parser {
        OutputSizeParser
    }
}

#[derive(Debug, Clone, Copy)]
pub struct OutputSizeParser;
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

pub fn cli() -> Clap<'static> {
    Clap::new("asciic")
        .version("0.1.0")
        .about("An asciinema compiler")
        .author("by S0ra")
        .args([
            Arg::new("video")
                .required_unless_present("image")
                .conflicts_with("image")
                .index(1)
                .help("Input video to transform in asciinema")
                .takes_value(true),
            Arg::new("output-dir")
                .value_parser(value_parser!(PathBuf))
                .required_unless_present("image")
                .conflicts_with("image")
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
            Arg::new("colorize").short('c').help("Colorize output"),
            Arg::new("no-compression")
                .short('n')
                .help("disables compression on colored outputs")
                .requires("colorize"),
            Arg::new("compression-threshold")
                .short('t')
                .default_value("10")
                .requires("colorize")
                .takes_value(true)
                .value_parser(value_parser!(u8))
                .help("Manually sets the compression threshold"),
        ])
}
