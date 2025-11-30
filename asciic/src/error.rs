use std::{error::Error, fmt::Display, io, num, process::ExitStatus, str};

use crate::colours::{RED, RESET, YELLOW};

const FILE_STEM_NAN: &str = "\x1b[31m\
Frame processing failed.
One of the file stems was not a valid number.
This might be an FFMPEG related issue.
If you ever see this message, please open an \
issue in https://github.com/S0raWasTaken/bad_apple \
\x1b[0m";

const ITERATION_LIMIT: &str = "\
Iteration limit reached.
This usually means that you set your uncompressed frame size too low.";

const TERMINAL_SIZE: &str = "Could not detect the terminal's window size";

const FRAMERATE: &str = "Could not detect the stream's framerate";

#[derive(Debug)]
pub enum CompilerError {
    CtrlC(ctrlc::Error),
    HomeDir(user_dirs::HomeDirError),
    Io(io::Error),
    ParseFloat(num::ParseFloatError),
    Reqwest(reqwest::Error),
    Template(indicatif::style::TemplateError),
    Utf8(str::Utf8Error),
    Ascii(libasciic::AsciiError),
    Ron(ron::Error),
    Ffmpeg(ExitStatus),
    Ytdlp(String),
    Ffprobe,
    FileStemNan,
    IterationLimit,
    TerminalSize,
    Stopped,
}

impl Error for CompilerError {}

impl Display for CompilerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let error_message = match self {
            CompilerError::FileStemNan => FILE_STEM_NAN,
            CompilerError::IterationLimit => ITERATION_LIMIT,
            CompilerError::TerminalSize => TERMINAL_SIZE,
            CompilerError::Ffprobe => FRAMERATE,
            CompilerError::Stopped => "Stopped.",
            CompilerError::Ffmpeg(status) => &format!(
                "FFMPEG failed with status: {{{}}}",
                status
                    .code()
                    .map_or("TERMINATED".to_string(), |s| s.to_string())
            ),
            CompilerError::Ytdlp(url) => &format!(
                "{RED}yt-dlp failed to grab a video from {YELLOW}'{url}'{RESET}"
            ),
            CompilerError::CtrlC(error) => &error.to_string(),
            CompilerError::HomeDir(error) => &error.to_string(),
            CompilerError::Io(error) => &error.to_string(),
            CompilerError::ParseFloat(error) => &error.to_string(),
            CompilerError::Reqwest(error) => &error.to_string(),
            CompilerError::Template(error) => &error.to_string(),
            CompilerError::Utf8(error) => &error.to_string(),
            CompilerError::Ascii(error) => &error.to_string(),
            CompilerError::Ron(error) => &error.to_string(),
        };
        write!(f, "{error_message}")
    }
}

macro_rules! map_error {
   ($($from:ty => $enum_variant:tt,)*) => {
        $(impl From<$from> for CompilerError {
            fn from(value: $from) -> Self {
                Self::$enum_variant(value)
            }
        })*
    };
}

map_error! {
    ctrlc::Error => CtrlC,
    user_dirs::HomeDirError => HomeDir,
    io::Error => Io,
    num::ParseFloatError => ParseFloat,
    reqwest::Error => Reqwest,
    indicatif::style::TemplateError => Template,
    str::Utf8Error => Utf8,
    libasciic::AsciiError => Ascii,
    ron::Error => Ron,
}
