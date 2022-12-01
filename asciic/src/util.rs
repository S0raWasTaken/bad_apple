use std::{
    fs::{remove_dir_all, File},
    io,
    path::Path,
    process::{abort, Command, Stdio},
    thread::sleep,
    time::Duration,
};

use tar::{Builder, Header};

pub fn clean_abort(tmp_path: &Path) -> ! {
    sleep(Duration::from_secs(2));
    clean(tmp_path);
    eprintln!("\nAborting!");
    abort();
}

pub fn clean(tmp_path: &Path) {
    eprintln!("\n\nCleaning up...");
    remove_dir_all(tmp_path).unwrap();
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
    data: &Vec<u8>,
) -> io::Result<()> {
    let mut header = Header::new_gnu();
    header.set_size(data.len() as u64);
    header.set_cksum();

    tar_archive.append_data(&mut header, path, data.as_slice())
}

pub fn ffmpeg(args: &[&str], extra_flags: &[&String]) -> Result<(), Box<dyn std::error::Error>> {
    let mut command = Command::new("ffmpeg");
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

#[inline]
pub fn max_sub(a: u8, b: u8) -> u8 {
    a.max(b) - a.min(b)
}
