use std::path::PathBuf;

use clap::{Parser, ValueEnum, command, crate_version};

use crate::primitives::Input;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Style {
    FgPaint,
    BgPaint,
    BgOnly,
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
}

impl Args {
    // Sorts out video or image + calls yt-dlp in case a link is passed
    pub fn handle_input(&self) -> Input {
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
