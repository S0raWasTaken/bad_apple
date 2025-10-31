use std::{
    error::Error,
    fs::{File, remove_dir_all, remove_file},
    io,
    path::Path,
    process::{Command, Stdio, abort},
    str::from_utf8,
    thread::sleep,
    time::Duration,
};

use tar::{Builder, Header};

pub fn clean_abort(tmp_path: &Path, should_delete_video: bool, video_path: &str) -> ! {
    sleep(Duration::from_secs(2));
    clean(tmp_path, should_delete_video, video_path);
    eprintln!("\nAborting!");
    abort();
}

pub fn clean(tmp_path: &Path, should_delete_video: bool, video_path: &str) {
    eprintln!("\n\nCleaning up...");
    remove_dir_all(tmp_path).unwrap_or_else(|_| {
        panic!(
            "remove_dir_all() failed. Check for littering on {}",
            tmp_path.display()
        )
    });
    if should_delete_video {
        remove_file(video_path)
            .unwrap_or_else(|_| panic!("remove_file() failed to delete {video_path}."));
    }
}

#[inline]
pub fn pause() -> ! {
    loop {
        sleep(Duration::from_millis(5));
    }
}

pub fn add_file(
    tar_archive: &mut Builder<File>,
    path: impl AsRef<Path>,
    data: &[u8],
) -> io::Result<()> {
    let mut header = Header::new_gnu();
    header.set_size(data.len() as u64);
    header.set_cksum();

    tar_archive.append_data(&mut header, path, data)
}

pub fn yt_dlp(ytdlp_path: &Path, url: &str, output: &str) -> Result<(), Box<dyn Error>> {
    Command::new(ytdlp_path)
        .args(["-t", "mp4", "-o", output, url])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()?;
    Ok(())
}

pub fn ffmpeg(
    ffmpeg_path: &Path,
    args: &[&str],
    extra_flags: &[&String],
) -> Result<(), Box<dyn Error>> {
    let mut command = Command::new(ffmpeg_path);
    command
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    if !extra_flags.is_empty() {
        command.args(extra_flags);
    }

    let output = command.output()?;

    if !output.status.success() {
        return Err("FFMPEG failed to run".into());
    }

    Ok(())
}

pub fn probe_fps(path: &str, ffprobe_path: &Path) -> Result<usize, Box<dyn Error>> {
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
            path,
        ])
        .output()?;
    let rate = from_utf8(&output.stdout)?.trim();
    // Expecting 30000/1001 or something like 60/1
    let [number, denominator, ..] = rate.split('/').collect::<Vec<_>>()[..] else {
        return Err("Couldn't automatically detect the stream's framerate.".into());
    };

    // I'm literally rounding it, shut up clippy.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Ok((number.parse::<f64>()? / denominator.parse::<f64>()?).round() as usize)
}

#[inline]
pub fn max_sub(a: u8, b: u8) -> u8 {
    a.max(b) - a.min(b)
}
