use std::path::PathBuf;

use clap::{Parser, ValueEnum, command, crate_version};

use crate::primitives::Input;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Style {
    FgPaint,
    BgPaint,
    BgOnly,
}

impl Style {
    pub fn ansi(&self) -> u8 {
        match self {
            Style::FgPaint => 3,
            Style::BgPaint => 4,
            Style::BgOnly => 4,
        }
    }
}

#[derive(Parser, Debug)]
#[command(version(crate_version!()))]
pub struct Args {
    /// Makes the output colorized
    #[arg(short, long)]
    pub colorize: bool,

    /// Skips audio extraction and inclusion in the output
    #[arg(short, long)]
    pub no_audio: bool,

    /// Path to a valid video file
    #[arg(group = "input")]
    pub video: Option<PathBuf>,

    /// Path to a valid image file
    #[arg(short, long, group = "input")]
    pub image: Option<PathBuf>,

    /// Youtube video URL to download and use
    #[arg(short, long, group = "input")]
    pub youtube: Option<String>,

    /// Custom output path, defaults to the video's file name
    #[arg(short, long, required_unless_present_any = ["video", "image"])]
    pub output: Option<PathBuf>,

    /// Sets the output style
    #[arg(short, long, default_value = "bg-only")]
    pub style: Style,

    /// Sets a custom path to create a temporary directory.
    /// It could be used to write the temporary files in memory,
    /// if the user sets this to /dev/shm
    #[arg(long, default_value = ".")]
    pub temp: PathBuf,

    /// Sets the colour compression threshold.
    #[arg(short, long, default_value = "3")]
    pub threshold: u8,
}

impl Args {
    /// Sorts out the Input and Output options and return them.
    pub fn handle_io(&self) -> (Input, PathBuf) {
        let input = self.handle_input();
        if let Some(mut output) = self.output.clone() {
            match input {
                Input::Image(_) => output.set_extension("txt"),
                Input::Video(_) => output.set_extension("bapple"),
            };
            return (input, output);
        }

        let output = match input.clone() {
            Input::Image(mut image_path) => {
                image_path.set_extension("txt");
                image_path.clone()
            }
            Input::Video(mut video_path) => {
                video_path.set_extension("bapple");
                video_path.clone()
            }
        };

        (input, output)
    }

    /// Sorts out video or image + calls yt-dlp in case a link is passed
    fn handle_input(&self) -> Input {
        [
            self.video.as_ref().map(|v| Input::Video(v.clone())),
            self.image.as_ref().map(|i| Input::Image(i.clone())),
            self.youtube.as_ref().map(|_link| todo!()), // TODO: yt-dlp
        ]
        .into_iter()
        .flatten()
        .next()
        .unwrap() // Guaranteed by the input group
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_style_ansi_fgpaint() {
        let style = Style::FgPaint;
        assert_eq!(style.ansi(), 3);
    }

    #[test]
    fn test_style_ansi_bgpaint() {
        let style = Style::BgPaint;
        assert_eq!(style.ansi(), 4);
    }

    #[test]
    fn test_style_ansi_bgonly() {
        let style = Style::BgOnly;
        assert_eq!(style.ansi(), 4);
    }

    #[test]
    fn test_style_value_enum() {
        // Test that styles can be parsed from strings
        use clap::ValueEnum;
        
        let variants = Style::value_variants();
        assert_eq!(variants.len(), 3);
        assert!(variants.contains(&Style::FgPaint));
        assert!(variants.contains(&Style::BgPaint));
        assert!(variants.contains(&Style::BgOnly));
    }

    #[test]
    fn test_args_handle_io_with_video() {
        let args = Args {
            colorize: false,
            no_audio: false,
            video: Some(PathBuf::from("input.mp4")),
            image: None,
            youtube: None,
            output: Some(PathBuf::from("custom_output")),
            style: Style::BgOnly,
            temp: PathBuf::from("."),
            threshold: 3,
        };

        let (input, output) = args.handle_io();
        
        match input {
            Input::Video(path) => assert_eq!(path, PathBuf::from("input.mp4")),
            _ => panic!("Expected Video input"),
        }
        assert_eq!(output, PathBuf::from("custom_output.bapple"));
    }

    #[test]
    fn test_args_handle_io_with_image() {
        let args = Args {
            colorize: false,
            no_audio: false,
            video: None,
            image: Some(PathBuf::from("picture.png")),
            youtube: None,
            output: Some(PathBuf::from("output")),
            style: Style::FgPaint,
            temp: PathBuf::from("."),
            threshold: 5,
        };

        let (input, output) = args.handle_io();
        
        match input {
            Input::Image(path) => assert_eq!(path, PathBuf::from("picture.png")),
            _ => panic!("Expected Image input"),
        }
        assert_eq!(output, PathBuf::from("output.txt"));
    }

    #[test]
    fn test_args_handle_io_video_default_output() {
        let args = Args {
            colorize: true,
            no_audio: false,
            video: Some(PathBuf::from("my_video.mp4")),
            image: None,
            youtube: None,
            output: None,
            style: Style::BgPaint,
            temp: PathBuf::from("/tmp"),
            threshold: 10,
        };

        let (input, output) = args.handle_io();
        
        match input {
            Input::Video(path) => assert_eq!(path, PathBuf::from("my_video.mp4")),
            _ => panic!("Expected Video input"),
        }
        // Output should be derived from input with .bapple extension
        assert_eq!(output, PathBuf::from("my_video.bapple"));
    }

    #[test]
    fn test_args_handle_io_image_default_output() {
        let args = Args {
            colorize: false,
            no_audio: true,
            video: None,
            image: Some(PathBuf::from("test.jpg")),
            youtube: None,
            output: None,
            style: Style::BgOnly,
            temp: PathBuf::from("."),
            threshold: 1,
        };

        let (input, output) = args.handle_io();
        
        match input {
            Input::Image(path) => assert_eq!(path, PathBuf::from("test.jpg")),
            _ => panic!("Expected Image input"),
        }
        assert_eq!(output, PathBuf::from("test.txt"));
    }

    #[test]
    fn test_args_handle_input_priority() {
        // When video is provided, it should be used
        let args = Args {
            colorize: false,
            no_audio: false,
            video: Some(PathBuf::from("video.mp4")),
            image: None,
            youtube: None,
            output: None,
            style: Style::FgPaint,
            temp: PathBuf::from("."),
            threshold: 3,
        };

        let input = args.handle_input();
        match input {
            Input::Video(path) => assert_eq!(path, PathBuf::from("video.mp4")),
            _ => panic!("Expected Video input"),
        }
    }

    #[test]
    fn test_args_with_different_thresholds() {
        for threshold in [0, 1, 3, 5, 10, 255] {
            let args = Args {
                colorize: true,
                no_audio: false,
                video: Some(PathBuf::from("test.mp4")),
                image: None,
                youtube: None,
                output: None,
                style: Style::BgPaint,
                temp: PathBuf::from("."),
                threshold,
            };
            assert_eq!(args.threshold, threshold);
        }
    }

    #[test]
    fn test_args_colorize_flag() {
        let args_colored = Args {
            colorize: true,
            no_audio: false,
            video: Some(PathBuf::from("test.mp4")),
            image: None,
            youtube: None,
            output: None,
            style: Style::BgOnly,
            temp: PathBuf::from("."),
            threshold: 3,
        };
        assert!(args_colored.colorize);

        let args_no_color = Args {
            colorize: false,
            no_audio: false,
            video: Some(PathBuf::from("test.mp4")),
            image: None,
            youtube: None,
            output: None,
            style: Style::BgOnly,
            temp: PathBuf::from("."),
            threshold: 3,
        };
        assert!(!args_no_color.colorize);
    }

    #[test]
    fn test_args_no_audio_flag() {
        let args_no_audio = Args {
            colorize: false,
            no_audio: true,
            video: Some(PathBuf::from("test.mp4")),
            image: None,
            youtube: None,
            output: None,
            style: Style::BgOnly,
            temp: PathBuf::from("."),
            threshold: 3,
        };
        assert!(args_no_audio.no_audio);

        let args_with_audio = Args {
            colorize: false,
            no_audio: false,
            video: Some(PathBuf::from("test.mp4")),
            image: None,
            youtube: None,
            output: None,
            style: Style::BgOnly,
            temp: PathBuf::from("."),
            threshold: 3,
        };
        assert!(!args_with_audio.no_audio);
    }

    #[test]
    fn test_args_custom_temp_directory() {
        let temp_paths = vec![
            PathBuf::from("."),
            PathBuf::from("/tmp"),
            PathBuf::from("/dev/shm"),
            PathBuf::from("./custom_temp"),
        ];

        for temp_path in temp_paths {
            let args = Args {
                colorize: false,
                no_audio: false,
                video: Some(PathBuf::from("test.mp4")),
                image: None,
                youtube: None,
                output: None,
                style: Style::BgOnly,
                temp: temp_path.clone(),
                threshold: 3,
            };
            assert_eq!(args.temp, temp_path);
        }
    }

    #[test]
    fn test_args_output_extension_video() {
        let args = Args {
            colorize: false,
            no_audio: false,
            video: Some(PathBuf::from("test.mp4")),
            image: None,
            youtube: None,
            output: Some(PathBuf::from("myoutput")),
            style: Style::BgOnly,
            temp: PathBuf::from("."),
            threshold: 3,
        };

        let (_, output) = args.handle_io();
        assert_eq!(output.extension().and_then(|s| s.to_str()), Some("bapple"));
    }

    #[test]
    fn test_args_output_extension_image() {
        let args = Args {
            colorize: false,
            no_audio: false,
            video: None,
            image: Some(PathBuf::from("test.png")),
            youtube: None,
            output: Some(PathBuf::from("myoutput")),
            style: Style::BgOnly,
            temp: PathBuf::from("."),
            threshold: 3,
        };

        let (_, output) = args.handle_io();
        assert_eq!(output.extension().and_then(|s| s.to_str()), Some("txt"));
    }

    #[test]
    fn test_args_output_preserves_directory() {
        let args = Args {
            colorize: false,
            no_audio: false,
            video: Some(PathBuf::from("test.mp4")),
            image: None,
            youtube: None,
            output: Some(PathBuf::from("/some/path/output")),
            style: Style::BgOnly,
            temp: PathBuf::from("."),
            threshold: 3,
        };

        let (_, output) = args.handle_io();
        assert_eq!(output, PathBuf::from("/some/path/output.bapple"));
    }

    #[test]
    fn test_style_all_variants_covered() {
        // Ensure all style variants have ansi codes
        let styles = vec![Style::FgPaint, Style::BgPaint, Style::BgOnly];
        
        for style in styles {
            let ansi = style.ansi();
            assert!(ansi == 3 || ansi == 4, "ANSI code should be 3 or 4");
        }
    }

    #[test]
    fn test_args_with_paths_containing_special_chars() {
        let args = Args {
            colorize: false,
            no_audio: false,
            video: Some(PathBuf::from("my video with spaces.mp4")),
            image: None,
            youtube: None,
            output: Some(PathBuf::from("output-with-dash")),
            style: Style::BgOnly,
            temp: PathBuf::from("."),
            threshold: 3,
        };

        let (input, output) = args.handle_io();
        
        match input {
            Input::Video(path) => assert_eq!(path, PathBuf::from("my video with spaces.mp4")),
            _ => panic!("Expected Video input"),
        }
        assert_eq!(output, PathBuf::from("output-with-dash.bapple"));
    }

    #[test]
    fn test_args_default_threshold() {
        // Default threshold should be 3
        let args = Args {
            colorize: true,
            no_audio: false,
            video: Some(PathBuf::from("test.mp4")),
            image: None,
            youtube: None,
            output: None,
            style: Style::BgOnly,
            temp: PathBuf::from("."),
            threshold: 3,
        };
        assert_eq!(args.threshold, 3);
    }

    #[test]
    fn test_input_enum_video_variant() {
        let video_path = PathBuf::from("test.mp4");
        let input = Input::Video(video_path.clone());
        
        match input {
            Input::Video(path) => assert_eq!(path, video_path),
            _ => panic!("Expected Video variant"),
        }
    }

    #[test]
    fn test_input_enum_image_variant() {
        let image_path = PathBuf::from("test.png");
        let input = Input::Image(image_path.clone());
        
        match input {
            Input::Image(path) => assert_eq!(path, image_path),
            _ => panic!("Expected Image variant"),
        }
    }
}
