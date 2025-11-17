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

    #[arg(long, default_value = " .:-+=#@")]
    pub charset: String,
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
