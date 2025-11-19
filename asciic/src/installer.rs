// I'll have to implement my own installer, because the yt-dlp crate's LibraryInstaller does not work.
// How fun.

use std::{
    fs::{self, create_dir_all},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use crate::{
    Res,
    colours::{LIGHT_GREEN, RED, RESET},
    primitives::Input,
};
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
    #[allow(unused_mut)] // Used on any OS that's not linux or windows.
    pub fn setup(input: &Input, mut use_system_binaries: bool) -> Res<Self> {
        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        {
            if !use_system_binaries {
                eprintln!(
                    "{RED}Automatically setting the flag --use-system-binaries{RESET}"
                );
                use_system_binaries = true;
            }
        }

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

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        if system_ffmpeg.is_none() || system_ffprobe.is_none() {
            return Err(
                "Automatic dependency management is unsupported for this OS",
            );
        }
    }

    let data_dir = local_data_dir()?;
    create_dir_all(&data_dir)?;

    let ffmpeg_output = data_dir.join("ffmpeg");
    let ffprobe_output = data_dir.join("ffprobe");

    if !ffmpeg_output.exists() && system_ffmpeg.is_none() {
        println!("Downloading FFmpeg binary...");
        download_binary(URLS[0], &ffmpeg_output)?;

        #[cfg(unix)]
        fix_perms(&ffmpeg_output)?;
    }
    if !ffprobe_output.exists() && system_ffprobe.is_none() {
        println!("Downloading FFprobe...");
        download_binary(URLS[1], &ffprobe_output)?;

        #[cfg(unix)]
        fix_perms(&ffprobe_output)?;
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

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        return Err(
            "Automatic dependency management is unsupported for this OS",
        );
    }

    let data_dir = local_data_dir()?;

    let ytdlp_output = data_dir.join("yt-dlp");

    if !ytdlp_output.exists() {
        println!("Downloading yt-dlp binary...");
        download_binary(URLS[2], &ytdlp_output)?;
    }
    #[cfg(unix)]
    {
        fix_perms(&ytdlp_output)?;
    }

    println!("Checking for yt-dlp updates...");

    let output = Command::new(&ytdlp_output)
        .arg("-U")
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()?;

    if !output.status.success() {
        eprintln!("yt-dlp update check failed");
    }

    Ok(ytdlp_output)
}

fn download_binary(url: &str, output: &Path) -> Res<()> {
    let bytes = reqwest::blocking::get(url)?.error_for_status()?.bytes()?;
    fs::write(output, bytes)?;
    println!("Success! {}", output.display());
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
    println!("Set executable permissions for {}", file.display());
    Ok(())
}

#[inline]
fn find_system_binary(name: &str) -> Option<PathBuf> {
    match which(name) {
        Ok(path) => {
            println!(
                "{LIGHT_GREEN}Using system {name} binary at {}{RESET}",
                path.display()
            );
            Some(path)
        }
        Err(_) => {
            eprintln!(
                "{RED}{name} not found in PATH; falling back to bundled download.{RESET}"
            );
            None
        }
    }
}
