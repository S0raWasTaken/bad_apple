use std::path::PathBuf;

use clap::{value_parser, Arg, Command};

use crate::primitives::{OutputSize, PaintStyle};

#[inline]
pub fn cli() -> Command<'static> {
    Command::new("asciic")
        .version("0.3.0")
        .about("An asciinema compiler")
        .author("by S0ra")
        .args(args())
}

#[inline]
fn args() -> [Arg<'static>; 10] {
    [
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
            .help("Skips audio generation")
            .conflicts_with("image"),
        Arg::new("style")
            .takes_value(true)
            .long("style")
            .help("Sets a style to follow when generating frames")
            .default_value("bg-only")
            .value_parser(value_parser!(PaintStyle)),
    ]
}
