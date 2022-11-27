use std::{
    fs::{remove_dir_all, File},
    io,
    path::{Path, PathBuf},
    process::{abort, Command, Stdio},
    thread::sleep,
    time::Duration,
};

use clap::{
    builder::{TypedValueParser, ValueParserFactory},
    value_parser, Arg, Command as Clap, ErrorKind,
};
use tar::{Builder, Header};

#[derive(Clone, Copy)]
pub struct Options {
    pub compression_threshold: u8,
    pub redimension: OutputSize,
    pub skip_compression: bool,
    pub paint_fg: bool,
    pub colorize: bool,
    pub skip_audio: bool,
}

pub fn cleanup(tmp_path: &Path) -> ! {
    sleep(Duration::from_secs(2));
    eprintln!("\n\nCleaning up...");
    remove_dir_all(tmp_path).unwrap();
    eprintln!("\nAborting!");
    abort();
}

#[inline]
pub fn pause() -> ! {
    loop {
        sleep(Duration::from_millis(5));
    }
}

pub fn add_file(
    tar_archive: &mut Builder<File>,
    path: impl AsRef<Path>,
    data: &Vec<u8>,
) -> io::Result<()> {
    let mut header = Header::new_gnu();
    header.set_size(data.len() as u64);
    header.set_cksum();

    tar_archive.append_data(&mut header, path, data.as_slice())
}

pub fn ffmpeg(args: &[&str], extra_flags: &[&String]) -> Result<(), Box<dyn std::error::Error>> {
    let mut command = Command::new("ffmpeg");
    command
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    if !extra_flags.is_empty() {
        command.args(extra_flags);
    }

    let output = command.output()?;

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
        .version("0.3.0")
        .about("An asciinema compiler")
        .author("by S0ra")
        .args([
            Arg::new("video")
                .required_unless_present("image")
                .conflicts_with("image")
                .index(1)
                .help("Input video to transform in asciinema")
                .takes_value(true),
            Arg::new("output")
                .value_parser(value_parser!(PathBuf))
                .default_value("output")
                .conflicts_with("image")
                .help("Output file name")
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
                .long("image")
                .takes_value(true)
                .help("Compiles a single image"),
            Arg::new("colorize").short('c').help("Colorize output"),
            Arg::new("no-compression")
                .short('n')
                .long("skip-compression")
                .help("Disables compression on colored outputs")
                .requires("colorize"),
            Arg::new("compression-threshold")
                .short('t')
                .long("threshold")
                .default_value("10")
                .requires("colorize")
                .takes_value(true)
                .value_parser(value_parser!(u8))
                .help("Manually sets the compression threshold"),
            Arg::new("paint-fg")
                .long("paint-fg")
                .requires("colorize")
                .help("Paints the foreground instead of background"),
            Arg::new("ffmpeg-flags")
                .index(3)
                .multiple_occurrences(true)
                .allow_hyphen_values(true)
                .takes_value(true)
                .conflicts_with("image")
                .multiple_values(true)
                .value_parser(value_parser!(String))
                .help("Pass extra flags to ffmpeg")
                .last(true),
            Arg::new("no-audio")
                .long("no-audio")
                .help("skips audio generation")
                .conflicts_with("image"),
        ])
}
