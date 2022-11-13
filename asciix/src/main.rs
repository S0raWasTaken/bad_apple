use std::{
    ffi::OsString,
    fs::{read_dir, read_to_string},
    path::PathBuf,
    process::Command as Shell,
    thread::spawn,
    time::{Duration, Instant},
};

use clap::{value_parser, Arg, Command};
use console::Term;

fn main() {
    let matches = cli().get_matches();

    let frames_dir = matches.get_one::<PathBuf>("path").unwrap();
    let framerate = *matches.get_one::<u32>("framerate").unwrap();

    let frames = get_frames(frames_dir);
    play(frames, framerate, frames_dir.clone());
}

fn play(frames: Vec<String>, rate: u32, frames_dir: PathBuf) {
    let delay = Duration::from_secs(1) / rate;

    spawn(move || audio(&frames_dir));

    for frame in frames {
        let time = Instant::now();
        Term::stdout().clear_screen().unwrap();
        println!("{frame}");
        std::thread::sleep(delay - time.elapsed());
    }
}

fn audio(frames_dir: &PathBuf) {
    // Literally just spawn mpv in the background ¯\_(ツ)_/¯
    let Ok(dir) = read_dir(frames_dir) else {return;};
    let Some(dir_name) = frames_dir.to_str() else {return;};

    if dir
        .filter_map(Result::ok)
        .any(|e| e.file_name() == "audio.mp3")
    {
        Shell::new("mpv")
            .args(format!("{dir_name}/audio.mp3").split_ascii_whitespace())
            .output()
            .unwrap();
    }
}

fn get_frames(dir: &PathBuf) -> Vec<String> {
    let mut frames = read_dir(dir)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.file_name().ne(&OsString::from("audio.mp3")))
        .collect::<Vec<_>>();
    frames.sort_by_key(|e| {
        e.path()
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .parse::<u32>()
            .unwrap()
    });
    frames
        .iter()
        .map(|e| read_to_string(e.path()).unwrap())
        .collect()
}

fn cli() -> Command<'static> {
    Command::new("asciix")
        .about("An asciinema player")
        .version("0.1.0")
        .author("S0raWasTaken")
        .args([
            Arg::new("path")
                .index(1)
                .required(true)
                .takes_value(true)
                .help("path to the frames directory")
                .value_parser(value_parser!(PathBuf)),
            Arg::new("framerate")
                .index(2)
                .default_value("30")
                .takes_value(true)
                .help("framerate to play the ascii. Default: 30")
                .value_parser(value_parser!(u32)),
        ])
}
