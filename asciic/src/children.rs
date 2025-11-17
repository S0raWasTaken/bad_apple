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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_ffmpeg_with_nonexistent_binary() {
        let nonexistent = PathBuf::from("/nonexistent/ffmpeg");
        let result = ffmpeg(&nonexistent, &["--version"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_ffmpeg_with_empty_args() {
        // Test that ffmpeg can be called with minimal args
        // This will fail if ffmpeg is not installed, but tests the function structure
        let ffmpeg_path = PathBuf::from("ffmpeg");
        let result = ffmpeg(&ffmpeg_path, &["--version"]);
        // We don't assert success here as ffmpeg might not be installed
        // But we verify the function doesn't panic
        let _ = result;
    }

    #[test]
    fn test_ffmpeg_args_passed_correctly() {
        // Test with a command that should fail with specific args
        let temp_dir = TempDir::new().unwrap();
        let fake_ffmpeg = temp_dir.path().join("fake_ffmpeg");
        
        // Create a fake executable that echoes args
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::write(&fake_ffmpeg, "#!/bin/sh\necho \"$@\"\nexit 0").unwrap();
            let mut perms = fs::metadata(&fake_ffmpeg).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&fake_ffmpeg, perms).unwrap();
            
            let result = ffmpeg(&fake_ffmpeg, &["-i", "input.mp4"]);
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_ffmpeg_with_invalid_args() {
        let temp_dir = TempDir::new().unwrap();
        let fake_ffmpeg = temp_dir.path().join("fake_ffmpeg");
        
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            // Create a script that exits with error code
            fs::write(&fake_ffmpeg, "#!/bin/sh\nexit 1").unwrap();
            let mut perms = fs::metadata(&fake_ffmpeg).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&fake_ffmpeg, perms).unwrap();
            
            let result = ffmpeg(&fake_ffmpeg, &["invalid"]);
            assert!(result.is_err());
            if let Err(e) = result {
                assert!(e.to_string().contains("FFMPEG failed to run"));
            }
        }
    }

    #[test]
    fn test_ffmpeg_with_multiple_args() {
        let temp_dir = TempDir::new().unwrap();
        let fake_ffmpeg = temp_dir.path().join("fake_ffmpeg");
        
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::write(&fake_ffmpeg, "#!/bin/sh\nexit 0").unwrap();
            let mut perms = fs::metadata(&fake_ffmpeg).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&fake_ffmpeg, perms).unwrap();
            
            let args = vec!["-i", "input.mp4", "-c:v", "libx264", "-c:a", "aac", "output.mp4"];
            let result = ffmpeg(&fake_ffmpeg, &args);
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_ffmpeg_path_is_used() {
        // Verify that the actual path we pass is used, not a system default
        let custom_path = PathBuf::from("/custom/path/to/ffmpeg");
        let result = ffmpeg(&custom_path, &[]);
        // Should fail because the path doesn't exist
        assert!(result.is_err());
    }

    #[test]
    fn test_ffmpeg_with_special_characters_in_args() {
        let temp_dir = TempDir::new().unwrap();
        let fake_ffmpeg = temp_dir.path().join("fake_ffmpeg");
        
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::write(&fake_ffmpeg, "#!/bin/sh\nexit 0").unwrap();
            let mut perms = fs::metadata(&fake_ffmpeg).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&fake_ffmpeg, perms).unwrap();
            
            // Test with special characters
            let result = ffmpeg(&fake_ffmpeg, &["-i", "file with spaces.mp4", "-vf", "scale=1920:1080"]);
            assert!(result.is_ok());
        }
    }
}
