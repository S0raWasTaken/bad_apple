use std::{
    num::ParseFloatError,
    path::Path,
    process::Command,
    str::from_utf8,
    sync::atomic::{AtomicBool, Ordering::SeqCst},
};

use crate::{
    Res,
    colours::{BCYAN, RED, RESET, YELLOW},
};

pub static FFMPEG_RUNNING: AtomicBool = AtomicBool::new(false);

const FFMPEG_FLAGS: [&str; 3] = ["-loglevel", "error", "-stats"];
pub fn ffmpeg(ffmpeg_path: &Path, args: &[&str]) -> Res<()> {
    FFMPEG_RUNNING.store(true, SeqCst);

    let status = Command::new(ffmpeg_path)
        .args(FFMPEG_FLAGS) // Default args
        .args(args)
        .status();

    FFMPEG_RUNNING.store(false, SeqCst);

    let status = status?;

    if !status.success() {
        return Err(format!(
            "FFMPEG failed with status: {{{}}}",
            status.code().map_or("TERMINATED".to_string(), |s| s.to_string())
        )
        .into());
    }

    Ok(())
}

/// Returns: (fps, frametime)
pub fn ffprobe(ffprobe_path: &Path, video_path: &str) -> Res<(u64, u64)> {
    let output = Command::new(ffprobe_path)
        .args([
            "-v",
            "0",
            "-of",
            "csv=p=0",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=r_frame_rate",
            video_path,
        ])
        .output()?;

    let rate = from_utf8(&output.stdout)?.trim();

    let [number, denominator, ..] =
        rate.split('/')
            .map(str::parse::<f64>)
            .collect::<Result<Vec<f64>, ParseFloatError>>()?[..]
    else {
        return Err("Could not detect the stream's framerate".into());
    };

    let fps = number / denominator;

    Ok((fps.round() as u64, (1_000_000.0 / fps).round() as u64))
}

pub const YTDLP_FLAGS: [&str; 3] = ["--quiet", "--no-warnings", "--progress"];
pub fn yt_dlp(ytdlp_path: &Path, url: &str, output: &str) -> Res<()> {
    println!(
        "       {BCYAN}Downloading video to {RESET}{YELLOW}{output}{RESET}"
    );
    let status = Command::new(ytdlp_path)
        .args(YTDLP_FLAGS)
        .args(["-t", "mp4", "-o", output, url])
        .status()?;

    if !status.success() {
        return Err(format!(
            "{RED}yt-dlp failed to grab a video from {YELLOW}'{url}'{RESET}"
        )
        .into());
    }
    Ok(())
}
