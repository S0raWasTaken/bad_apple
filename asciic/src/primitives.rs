use std::{path::PathBuf, sync::atomic::AtomicBool};

use tempfile::TempDir;

use crate::{
    Res,
    cli::{Args, Style},
};

pub enum Input {
    Video(PathBuf),
    Image(PathBuf),
}

pub struct AsciiCompiler {
    pub stop_handle: AtomicBool,
    pub temp_dir: TempDir,

    // Set by args from now on ↓
    input: Input,
    colorize: bool,
    no_audio: bool,
    style: Style,
    output: PathBuf,
}

impl AsciiCompiler {
    pub fn new(args: Args) -> Res<Self> {
        let input = args.handle_input();
        todo!();
    }
}
