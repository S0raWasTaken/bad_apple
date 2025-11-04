use clap::{
    ErrorKind, ValueEnum,
    builder::{TypedValueParser, ValueParserFactory},
    crate_version,
};
use serde::Serialize;

#[derive(Clone, Copy)]
#[allow(clippy::struct_excessive_bools)]
pub struct Options {
    pub boost: bool,
    pub compression_threshold: u8,
    pub redimension: OutputSize,
    pub skip_compression: bool,
    pub style: PaintStyle,
    pub colorize: bool,
    pub skip_audio: bool,
    pub should_delete_video: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum PaintStyle {
    FgPaint,
    BgPaint,
    BgOnly,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OutputSize(pub usize, pub usize);
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
                .parse::<usize>()
                .map_err(|e| cmd.clone().error(ErrorKind::InvalidValue, e.to_string()))?,
            vals.last()
                .unwrap()
                .parse::<usize>()
                .map_err(|e| cmd.clone().error(ErrorKind::InvalidValue, e.to_string()))?,
        );

        if output_size.0 > 400 || output_size.1 > 200 {
            eprintln!("WARN: Usually going too high on frame size makes stuff a bit wonky.");
        }

        // I mean, why would you want to resize the image to u32::MAX?
        assert!(output_size.0 < u32::MAX as usize || output_size.1 < u32::MAX as usize);

        Ok(output_size)
    }
}

#[non_exhaustive]
#[derive(Serialize)]
pub struct Metadata {
    frametime: usize, // Pass frametime directly, so we don't have issues dealing with fps values like: 29.33
    fps: usize,       // Old value, will still be passed for old bplay versions.
    asciic_version: &'static str,
    comment: &'static str,
}

const COMMENT: &str = "If you're reading this, you're probably using an outdated version of bplay.\n\
                       Go ahead and update it on https://github.com/S0raWasTaken/bapple_player if you want.\n\
                       It sets the FPS automatically now, so go on, have fun.";

impl Metadata {
    pub fn new(fps: usize, frametime: usize) -> Self {
        Self {
            frametime,
            fps,
            asciic_version: crate_version!(),
            comment: COMMENT,
        }
    }
}
