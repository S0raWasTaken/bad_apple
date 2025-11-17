use std::{
    path::Path,
    process::{Command, Stdio},
};

use crate::Res;

pub fn ffmpeg(ffmpeg_path: &Path, args: &[&str]) -> Res<()> {
    let mut command = Command::new(ffmpeg_path);
    command
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let output = command.output()?;

    if !output.status.success() {
        return Err("FFMPEG failed to run".into());
    }

    Ok(())
}
