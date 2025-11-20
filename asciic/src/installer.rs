// Suppresses warnings on other systems
#![cfg_attr(
    not(any(target_os = "windows", target_os = "linux")),
    allow(unreachable_code, unused_variables)
)]

use std::{
    fs::{self, File, create_dir_all},
    path::{Path, PathBuf},
    process::Command,
};

use crate::{
    Res,
    children::YTDLP_FLAGS,
    colours::{BCYAN, BDIM, GREEN, RED, RESET, YELLOW},
    primitives::Input,
};
use indicatif::{ProgressBar, ProgressStyle};
use which::which;

#[cfg(target_os = "linux")]
const URLS: [&str; 3] = [
    "https://github.com/S0raWasTaken/bapple_mirror/releases/download/latest/ffmpeg",
    "https://github.com/S0raWasTaken/bapple_mirror/releases/download/latest/ffprobe",
    "https://github.com/S0raWasTaken/bapple_mirror/releases/download/latest/yt-dlp",
];

#[cfg(target_os = "windows")]
const URLS: [&str; 3] = [
    "https://github.com/S0raWasTaken/bapple_mirror/releases/download/latest/ffmpeg.exe",
    "https://github.com/S0raWasTaken/bapple_mirror/releases/download/latest/ffprobe.exe",
    "https://github.com/S0raWasTaken/bapple_mirror/releases/download/latest/yt-dlp.exe",
];

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
const URLS: [&str; 3] = [""; 3];

#[derive(Default)] // To skip dependency setup
pub struct Dependencies {
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
    pub ytdlp: PathBuf,
}

impl Dependencies {
    pub fn setup(input: &Input, use_system_binaries: bool) -> Res<Self> {
        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        let use_system_binaries = true;

        println!(
            "{BDIM}[1/5]{RESET}  {BCYAN}Resolving dependencies and input...{RESET}"
        );

        match input {
            Input::Video(_) => {
                let (ffmpeg, ffprobe) = setup_ffmpeg(use_system_binaries)?;
                Ok(Self { ffmpeg, ffprobe, ..Default::default() })
            }
            Input::Image(_) => Ok(Self::default()),
            Input::YoutubeLink(_) => {
                let (ffmpeg, ffprobe) = setup_ffmpeg(use_system_binaries)?;
                let ytdlp = setup_ytdlp(use_system_binaries)?;
                Ok(Self { ffmpeg, ffprobe, ytdlp })
            }
        }
    }
}

// and ffprobe too
fn setup_ffmpeg(use_system_binaries: bool) -> Res<(PathBuf, PathBuf)> {
    let mut system_ffmpeg = None;
    let mut system_ffprobe = None;

    if use_system_binaries {
        system_ffmpeg = find_system_binary("ffmpeg");
        system_ffprobe = find_system_binary("ffprobe");
    }

    let data_dir = local_data_dir()?;

    let ffmpeg_output = data_dir.join("ffmpeg");
    let ffprobe_output = data_dir.join("ffprobe");

    if !ffmpeg_output.exists() && system_ffmpeg.is_none() {
        create_dir_all(&data_dir)?;
        download_and_setup_binary(URLS[0], &ffmpeg_output)?;
    }
    if !ffprobe_output.exists() && system_ffprobe.is_none() {
        create_dir_all(&data_dir)?;
        download_and_setup_binary(URLS[1], &ffprobe_output)?;
    }

    Ok((
        system_ffmpeg.unwrap_or(ffmpeg_output),
        system_ffprobe.unwrap_or(ffprobe_output),
    ))
}

fn setup_ytdlp(use_system_binaries: bool) -> Res<PathBuf> {
    if use_system_binaries && let Some(ytdlp) = find_system_binary("yt-dlp") {
        return Ok(ytdlp);
    }

    let data_dir = local_data_dir()?;

    let ytdlp_output = data_dir.join("yt-dlp");

    if !ytdlp_output.exists() {
        download_and_setup_binary(URLS[2], &ytdlp_output)?;
    }

    println!("       {BCYAN}Checking for {RED}yt-dlp {BCYAN}updates...{RESET}");

    let status =
        Command::new(&ytdlp_output).arg("-U").args(YTDLP_FLAGS).status()?;

    if !status.success() {
        eprintln!("{RED}yt-dlp update check failed{RESET}");
    }

    Ok(ytdlp_output)
}

const TEMPLATE: &str = "{spinner:.green} {msg} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})";
fn download_and_setup_binary(url: &str, output: &Path) -> Res<()> {
    let response = reqwest::blocking::get(url)?.error_for_status()?;
    let total_size = response.content_length().unwrap_or(0);

    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::with_template(TEMPLATE)?.progress_chars("██░"));
    pb.set_message(format!(
        "     {BCYAN}Downloading {RESET}{YELLOW}{} {BCYAN}from {RESET}{YELLOW}{}{RESET}\n      ",
        output.file_stem().unwrap().display(),
        url
    ));

    let temp_output = output.with_extension("tmp");
    let mut file = File::create(&temp_output)?;
    match std::io::copy(&mut pb.wrap_read(response), &mut file) {
        Ok(_) => {
            drop(file);
            fs::rename(&temp_output, output)?;
        }
        Err(e) => {
            drop(file);
            let _ = fs::remove_file(&temp_output);
            return Err(e.into());
        }
    }

    pb.finish_and_clear();
    // pb.finish_with_message("Download complete");
    println!(
        "       {BCYAN}Success! {RESET}{YELLOW}{}{RESET}",
        output.display()
    );

    #[cfg(unix)]
    fix_perms(output)?;

    Ok(())
}

#[inline]
fn local_data_dir() -> Res<PathBuf> {
    Ok(user_dirs::data_dir()?.join("asciic-bin"))
}

#[cfg(unix)]
fn fix_perms(file: &Path) -> Result<(), std::io::Error> {
    use std::os::unix::fs::PermissionsExt;

    let mut perms = fs::metadata(file)?.permissions();
    perms.set_mode(perms.mode() | 0o111);
    fs::set_permissions(file, perms)?;
    Ok(())
}

#[inline]
fn find_system_binary(name: &str) -> Option<PathBuf> {
    if let Ok(path) = which(name) {
        println!(
            "       {GREEN}Using system {name} binary at {}{RESET}",
            path.display()
        );
        Some(path)
    } else {
        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        {
            eprintln!(
                "{RED}{name} not found in PATH.\n\
                Automatic dependency management is not supported for this OS.{RESET}"
            );
            std::process::exit(1);
        }
        eprintln!(
            "{RED}{name} not found in PATH; falling back to bundled download.{RESET}"
        );
        None
    }
}
