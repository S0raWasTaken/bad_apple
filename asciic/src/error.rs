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

#[allow(clippy::enum_glob_use)]
use CompilerError::*;

impl Error for CompilerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            CtrlC(e) => Some(e),
            HomeDir(e) => Some(e),
            Io(e) => Some(e),
            ParseFloat(e) => Some(e),
            Reqwest(e) => Some(e),
            Template(e) => Some(e),
            Utf8(e) => Some(e),
            Ascii(e) => Some(e),
            Ron(e) => Some(e),
            _ => None,
        }
    }
}

impl Display for CompilerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileStemNan => write!(f, "{FILE_STEM_NAN}"),
            IterationLimit => write!(f, "{ITERATION_LIMIT}"),
            TerminalSize => write!(f, "{TERMINAL_SIZE}"),
            Ffprobe => write!(f, "{FRAMERATE}"),
            Stopped => write!(f, "Stopped."),
            Ffmpeg(status) => write!(
                f,
                "FFMPEG failed with status: {{{}}}",
                status
                    .code()
                    .map_or_else(|| "TERMINATED".into(), |s| s.to_string())
            ),
            Ytdlp(url) => write!(
                f,
                "{RED}yt-dlp failed to grab a video from {YELLOW}'{url}'{RESET}"
            ),
            CtrlC(e) => write!(f, "{e}"),
            HomeDir(e) => write!(f, "{e}"),
            Io(e) => write!(f, "{e}"),
            ParseFloat(e) => write!(f, "{e}"),
            Reqwest(e) => write!(f, "{e}"),
            Template(e) => write!(f, "{e}"),
            Utf8(e) => write!(f, "{e}"),
            Ascii(e) => write!(f, "{e}"),
            Ron(e) => write!(f, "{e}"),
        }
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
