use std::fs::{File, read_dir};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::atomic::AtomicU8;
use std::{path::PathBuf, sync::atomic::AtomicBool};

use clap::crate_version;
use indicatif::{ProgressBar, ProgressStyle};
use libasciic::{AsciiBuilder, FilterType, Style};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use ron::ser::PrettyConfig;
use serde::Serialize;
use tar::{Builder, Header};
use tempfile::TempDir;
use zstd::encode_all;

use crate::children::{ffprobe, yt_dlp};
use crate::colours::{BOLD, RESET, YELLOW};
use crate::installer::Dependencies;
use crate::{Res, children::ffmpeg, cli::Args};

#[derive(Serialize)]
pub struct Metadata {
    frametime: u64,
    fps: u64,
    asciic_version: &'static str,
}
impl Metadata {
    pub fn new(fps: u64, frametime: u64) -> Self {
        Self { frametime, fps, asciic_version: crate_version!() }
    }
}

#[derive(Clone)]
pub enum Input {
    Video(PathBuf),
    Image(PathBuf),
    YoutubeLink(String),
}

const FILE_STEM_NAN: &str = "\x1b[31m\
Frame processing failed.
One of the file stems was not a valid number.
This might be an FFMPEG related issue.
If you ever see this message, please open an \
issue in https://github.com/S0raWasTaken/bad_apple \
\x1b[0m";

pub struct AsciiCompiler {
    pub stop_handle: AtomicBool,
    pub temp_dir: TempDir,
    dimensions: (u32, u32),
    dependencies: Dependencies,

    // Set by args from now on ↓
    pub input: Input,
    colorize: bool,
    no_audio: bool,
    style: Style,
    pub output: PathBuf,
    charset: String,
    threshold: AtomicU8,
    filter_type: FilterType,
}

impl AsciiCompiler {
    pub fn new(args: Args) -> Res<Self> {
        let stop_handle = AtomicBool::new(false);
        let temp_dir = TempDir::new_in(&args.temp)?;

        let (input, output) = args.handle_io();
        let colorize = args.colorize;
        let style = args.style;
        let no_audio = args.no_audio;
        let threshold = AtomicU8::new(args.threshold);

        let charset = args.charset;

        let Some(dimensions) = term_size::dimensions().and_then(|(w, h)| {
            Some((u32::try_from(w).ok()?, u32::try_from(h).ok()?))
        }) else {
            return Err("Could not detect the terminal's window size.".into());
        };

        let dependencies =
            Dependencies::setup(&input, args.use_system_binaries)?;

        let filter_type = args.filter_type.into();

        Ok(Self {
            stop_handle,
            temp_dir,
            dimensions,
            dependencies,
            input,
            colorize,
            no_audio,
            style,
            output,
            charset,
            threshold,
            filter_type,
        })
    }

    pub fn compile(&self) -> Res<()> {
        match &self.input {
            Input::Video(video) => self.make_video(video),
            Input::Image(image) => self.make_image(image),
            Input::YoutubeLink(link) => self.make_youtube(link),
        }?;

        println!(
            "\n\n{YELLOW}{BOLD}-> Done! <-\n\
            {RESET}{YELLOW}>> Output available at {RESET}{BOLD}{}{RESET}",
            self.output.display()
        );
        Ok(())
    }

    fn make_youtube(&self, link: &str) -> Res<()> {
        let mut temporary_video = self.output.clone();
        temporary_video.set_extension("mp4");

        yt_dlp(
            &self.dependencies.ytdlp,
            link,
            &temporary_video.to_string_lossy(),
        )?;

        self.make_video(&temporary_video)
    }

    fn make_video(&self, video: &Path) -> Res<()> {
        let video_path = video.to_string_lossy();
        let (fps, frametime): (u64, u64) =
            ffprobe(&self.dependencies.ffprobe, &video_path)?;
        self.split_video_frames(&video_path)?;
        if !self.no_audio {
            self.extract_audio(&video_path)?;
        }

        let mut tar_archive = Builder::new(File::create(self.output.clone())?);

        let tmp_path = self.temp_dir.path();
        let frames = read_dir(tmp_path)?
            .filter_map(Result::ok)
            .filter(|e| e.file_name() != *"audio.mp3")
            .map(|e| e.path())
            .collect::<Vec<_>>();

        let total = frames.len();
        let pb = ProgressBar::new(total as u64);
        pb.set_style(ProgressStyle::with_template(
            "{spinner:.green} {msg} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
        )
        .unwrap()
        .progress_chars("██░"));
        pb.set_message("Processing frames");

        // The compiler wanted to end its own life, so I'm giving
        // this variable an explicit type.
        let mut frames: Vec<(PathBuf, Vec<u8>)> = frames
            .into_par_iter()
            .map(|entry| -> Res<(PathBuf, Vec<u8>)> {
                if self.stop_handle.load(Relaxed) {
                    return Err("Stopped.".into());
                }

                // Early fail: filename must be a number
                {
                    let _: u64 = entry
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .and_then(|stem| stem.parse().ok())
                        .ok_or(FILE_STEM_NAN)?;
                }

                pb.inc(1);
                let uncompressed_frame: String = self.make_frame(&entry)?;
                Ok((
                    entry.clone(),
                    encode_all(uncompressed_frame.as_bytes(), 1)?,
                ))
            })
            .collect::<Res<_>>()?;

        pb.finish_with_message("Frames processed");

        frames.sort_by_key(|(path, _)| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap() // Should've failed early if the file stem is NaN
        });

        let pb_link = ProgressBar::new(total as u64);
        pb_link.set_style(ProgressStyle::with_template(
            "{spinner:.green} {msg} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
        )
        .unwrap()
        .progress_chars("██░"));
        pb_link.set_message("Linking frames");

        // Let's write to the tar archive in a single thread, for obvious reasons.
        for (path, compressed_frame) in frames {
            pb_link.inc(1);

            let mut inside_path = PathBuf::from(".");
            inside_path.set_file_name(path.file_stem().unwrap());
            inside_path.set_extension("zst");

            add_file(&mut tar_archive, inside_path, &compressed_frame)?;
        }
        pb_link.finish_with_message("Linking done");

        if !self.no_audio {
            let mut audio = File::open(tmp_path.join("audio.mp3"))?;
            let mut data = Vec::new();
            audio.read_to_end(&mut data)?;

            add_file(&mut tar_archive, "audio.mp3", &data)?;
        }

        let metadata = ron::Options::default().to_string_pretty(
            &Metadata::new(fps, frametime),
            PrettyConfig::default(),
        )?;

        add_file(&mut tar_archive, "metadata.ron", metadata.as_bytes())?;
        tar_archive.finish()?;

        Ok(())
    }

    #[inline]
    fn make_frame(&self, frame: &PathBuf) -> Res<String> {
        AsciiBuilder::new(File::open(frame)?)?
            .dimensions(self.dimensions.0, self.dimensions.1)
            .charset(&self.charset)?
            .style(self.style)
            .colorize(self.colorize)
            .filter_type(self.filter_type)
            .threshold(self.threshold.load(Relaxed))
            .make_ascii()
    }

    #[inline]
    fn make_image(&self, image: &PathBuf) -> Res<()> {
        self.threshold.store(0, Relaxed);
        File::create(self.output.clone())?
            .write_all(self.make_frame(image)?.as_bytes())?;
        Ok(())
    }

    #[inline]
    fn split_video_frames(&self, video_path: &str) -> Res<()> {
        ffmpeg(
            &self.dependencies.ffmpeg,
            &[
                "-r",
                "1",
                "-i",
                video_path,
                "-r",
                "1",
                &format!("{}/%03d.png", self.temp_dir.path().to_string_lossy()),
            ],
        )
    }

    #[inline]
    fn extract_audio(&self, video_path: &str) -> Res<()> {
        ffmpeg(
            &self.dependencies.ffmpeg,
            &[
                "-i",
                video_path,
                &format!(
                    "{}/audio.mp3",
                    self.temp_dir.path().to_string_lossy()
                ),
            ],
        )
    }
}

fn add_file(
    tar_archive: &mut Builder<File>,
    path: impl AsRef<Path>,
    data: &[u8],
) -> Res<()> {
    let mut header = Header::new_gnu();
    header.set_size(data.len() as u64);
    header.set_cksum();
    tar_archive.append_data(&mut header, path, data)?;
    Ok(())
}
