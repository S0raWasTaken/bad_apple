#![warn(clippy::pedantic)]

use std::{
    error::Error,
    fs::{write, File},
    io::{self, stdout, Write},
    path::PathBuf,
    process::Command as Shell,
    thread::{sleep, spawn},
    time::{Duration, Instant},
};

use bidirectional_channel::BiChannel;
use clap::{value_parser, Arg, Command};
use reader::{manage_buffer, next_frame};
use tempfile::TempDir;

mod bidirectional_channel;
mod reader;

type BoxResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

fn main() -> BoxResult<()> {
    let matches = cli().get_matches();

    let frames_file = matches.get_one::<PathBuf>("file").unwrap();
    let framerate = *matches.get_one::<u32>("framerate").unwrap();
    let loop_stream = matches.contains_id("loop");

    loop {
        // When `do {} while bool`?
        play(frames_file.clone(), framerate)?;
        if !loop_stream {
            break;
        }
    }
    Ok(())
}

fn play(tar_file: PathBuf, rate: u32) -> io::Result<()> {
    let (signal_sender, signal_recv) = BiChannel::<bool, Vec<u8>>::new();

    spawn(move || manage_buffer(&signal_recv, File::open(tar_file)?, Vec::new()));

    if let Some(audio_file) = next_frame(&signal_sender) {
        spawn(|| audio(audio_file));
    }

    let delay = Duration::from_secs(1) / rate;
    let mut lock = stdout().lock();
    loop {
        let time = Instant::now();
        if let Some(frame) = next_frame(&signal_sender) {
            lock.write_all(b"\r\x1b[2J\r\x1b[H")?;
            lock.write_all(&frame)?;
            sleep(delay.saturating_sub(time.elapsed()));
        } else {
            break;
        }
    }

    Ok(())
}

fn audio(mp3_buf: Vec<u8>) {
    let Ok(tmp_dir) = TempDir::new() else {
        return;
    };
    let mut file_path = tmp_dir.path().to_path_buf();
    file_path.set_file_name("audio");
    file_path.set_extension("mp3");

    if write(&file_path, mp3_buf).is_err() {
        return;
    }

    Shell::new("mpv").args([file_path]).output().ok();
}

fn cli() -> Command<'static> {
    Command::new("asciix")
        .about("An asciinema player")
        .version("0.1.0")
        .author("S0raWasTaken")
        .args([
            Arg::new("file")
                .index(1)
                .required(true)
                .takes_value(true)
                .help("path to the .bapple file")
                .value_parser(value_parser!(PathBuf)),
            Arg::new("framerate")
                .index(2)
                .default_value("30")
                .takes_value(true)
                .help("framerate to play the ascii. Default: 30")
                .value_parser(value_parser!(u32)),
            Arg::new("loop").long("loop").help("loops the stream"),
        ])
}
