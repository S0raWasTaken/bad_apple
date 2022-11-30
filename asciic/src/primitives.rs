use clap::{
    builder::{TypedValueParser, ValueParserFactory},
    ErrorKind, ValueEnum,
};

#[derive(Clone, Copy)]
pub struct Options {
    pub compression_threshold: u8,
    pub redimension: OutputSize,
    pub skip_compression: bool,
    pub style: PaintStyle,
    pub colorize: bool,
    pub skip_audio: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum PaintStyle {
    FgPaint,
    BgPaint,
    BgOnly,
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
