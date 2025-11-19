#![warn(clippy::pedantic)]
#![allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
use std::fs::remove_file;
use std::process::exit;
use std::sync::atomic::Ordering::SeqCst;
use std::{
    error::Error, fs::remove_dir_all, sync::Arc, thread::sleep, time::Duration,
};

use clap::Parser;

use crate::colours::RESET;
use crate::{
    children::FFMPEG_RUNNING,
    cli::Args,
    colours::{RED, YELLOW},
    primitives::AsciiCompiler,
};

type Res<T> = Result<T, Box<dyn Error + Send + Sync>>;

mod children;
mod cli;
mod colours;
mod installer;
mod primitives;
fn main() -> Res<()> {
    let ascii_compiler = Arc::new(AsciiCompiler::new(Args::parse())?);

    register_ctrl_c_handle(ascii_compiler.clone())?;

    let mut status_code = 0;
    if let Err(error) = ascii_compiler.compile() {
        eprintln!("{RED}{error:?}{RESET}");
        status_code = 1;
    }

    cleanup(&ascii_compiler)?;
    exit(status_code);
}

fn register_ctrl_c_handle(ascii_compiler: Arc<AsciiCompiler>) -> Res<()> {
    ctrlc::set_handler(move || {
        ascii_compiler.stop_handle.store(true, SeqCst);
    })?;
    Ok(())
}

fn cleanup(ascii_compiler: &AsciiCompiler) -> Res<()> {
    println!("\n{YELLOW}Cleaning up...{RESET}");
    let tmp_dir_path = ascii_compiler.temp_dir.path();

    // Wait for ffmpeg before removing the directory
    let timeout = Duration::from_secs(30);
    let start = std::time::Instant::now();

    while FFMPEG_RUNNING.load(SeqCst) {
        if start.elapsed() > timeout {
            eprintln!(
                "{RED}Warning: ffmpeg did not finish within timeout, proceeding with cleanup{RESET}"
            );
            break;
        }
        sleep(Duration::from_millis(100));
    }

    remove_dir_all(tmp_dir_path)?;

    if let primitives::Input::YoutubeLink(_) = ascii_compiler.input {
        let mut temporary_video = ascii_compiler.output.clone();
        temporary_video.set_extension("mp4");

        remove_file(temporary_video)?;
    }
    Ok(())
}
