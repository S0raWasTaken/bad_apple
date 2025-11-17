use std::{
    num::ParseFloatError,
    path::Path,
    process::{Command, Stdio},
    str::from_utf8,
    sync::atomic::{AtomicBool, Ordering::SeqCst},
};

use crate::Res;

pub static FFMPEG_RUNNING: AtomicBool = AtomicBool::new(false);

pub fn ffmpeg(ffmpeg_path: &Path, args: &[&str]) -> Res<()> {
    let mut command = Command::new(ffmpeg_path);
    command
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    FFMPEG_RUNNING.store(true, SeqCst);

    let output = command.output();

    FFMPEG_RUNNING.store(false, SeqCst);

    if !output?.status.success() {
        return Err("FFMPEG failed to run".into());
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

    let [number, denominator, ..] = rate
        .split('/')
        .map(|n_str| n_str.parse::<f64>())
        .collect::<Result<Vec<f64>, ParseFloatError>>()?[..]
    else {
        return Err("Could detect the stream's framerate".into());
    };

    let fps = number / denominator;

    Ok((fps.round() as u64, (1_000_000.0 / fps).round() as u64))
}

pub fn yt_dlp(ytdlp_path: &Path, url: &str, output: &str) -> Res<()> {
    Command::new(ytdlp_path)
        .args(["-t", "mp4", "-o", output, url])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()?;
    Ok(())
}
