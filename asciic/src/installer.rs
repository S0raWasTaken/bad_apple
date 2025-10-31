// I'll have to implement my own installer, because the yt-dlp crate's LibraryInstaller does not work.
// How fun.

use std::{
    error::Error,
    fs::{self, create_dir_all},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use tokio::runtime::Runtime;

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

// and ffprobe too
pub fn setup_ffmpeg(rt: &Runtime) -> Result<(PathBuf, PathBuf), Box<dyn Error>> {
    let data_dir = local_data_dir()?;
    create_dir_all(&data_dir)?;

    let ffmpeg_output = data_dir.join("ffmpeg");
    let ffprobe_output = data_dir.join("ffprobe");

    if !ffmpeg_output.exists() {
        println!("Downloading FFmpeg binary...");
        rt.block_on(download_binary(URLS[0], &ffmpeg_output))?;
    }
    if !ffprobe_output.exists() {
        println!("Downloading FFprobe...");
        rt.block_on(download_binary(URLS[1], &ffprobe_output))?;
    }

    #[cfg(unix)]
    {
        fix_perms(&ffmpeg_output)?;
        fix_perms(&ffprobe_output)?;
    }

    Ok((ffmpeg_output, ffprobe_output))
}

pub fn setup_ytdlp(rt: &Runtime) -> Result<PathBuf, Box<dyn Error>> {
    let data_dir = local_data_dir()?;

    let ytdlp_output = data_dir.join("yt-dlp");

    if !ytdlp_output.exists() {
        println!("Downloading yt-dlp binary...");
        rt.block_on(download_binary(URLS[2], &ytdlp_output))?;
    }
    #[cfg(unix)]
    {
        fix_perms(&ytdlp_output)?;
    }

    println!("Checking for yt-dlp updates...");

    Command::new(&ytdlp_output)
        .arg("-U")
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()?;

    Ok(ytdlp_output)
}

async fn download_binary(url: &str, output: &Path) -> Result<(), Box<dyn Error>> {
    let bytes = reqwest::get(url).await?.bytes().await?;
    fs::write(output, bytes)?;
    println!("Success! {}", output.display());
    Ok(())
}

#[inline]
fn local_data_dir() -> Result<PathBuf, Box<dyn Error>> {
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
