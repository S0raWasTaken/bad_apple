use std::fmt::Write as FmtWrite;
use std::fs::{File, read_dir, remove_file};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::Relaxed;
use std::{path::PathBuf, sync::atomic::AtomicBool};

use clap::crate_version;
use image::{GenericImageView, ImageReader, imageops::FilterType::Nearest};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use ron::ser::PrettyConfig;
use serde::Serialize;
use tar::{Builder, Header};
use tempfile::TempDir;
use zstd::encode_all;

use crate::children::{ffprobe, yt_dlp};
use crate::colours::{BOLD, RESET, YELLOW};
use crate::installer::Dependencies;
use crate::{
    Res,
    children::ffmpeg,
    cli::{Args, Style},
};

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

pub struct Charset(pub Vec<u8>, pub Vec<char>, pub char);

impl Charset {
    pub fn match_char(&self, brightness: u8) -> char {
        self.0
            .iter()
            .zip(self.1.iter())
            .find(|(threshold, _)| brightness <= **threshold)
            .map_or(self.2, |(_, c)| *c)
    }

    fn mkcharset(spec: &str) -> Res<Self> {
        let mut chars: Vec<char> = spec.chars().collect();
        chars.insert(0, ' ');

        let steps = chars.len();
        let mut thresholds = Vec::with_capacity(steps);

        for i in 0..steps {
            let t =
                (i as f32 / (steps - 1).max(1) as f32 * 250.0).round() as u8;
            thresholds.push(t);
        }

        let last = *chars.last().unwrap();
        Ok(Self(thresholds, chars, last))
    }
}

pub struct AsciiCompiler {
    pub stop_handle: AtomicBool,
    pub temp_dir: TempDir,
    dimensions: (u32, u32),
    dependencies: Dependencies,

    // Set by args from now on ↓
    input: Input,
    colorize: bool,
    no_audio: bool,
    style: Style,
    output: PathBuf,
    charset: Charset,
    threshold: u8,
}

impl AsciiCompiler {
    pub fn new(args: Args) -> Res<Self> {
        let stop_handle = AtomicBool::new(false);
        let temp_dir = TempDir::new_in(&args.temp)?;

        let (input, output) = args.handle_io();
        let colorize = args.colorize;
        let style = args.style;
        let no_audio = args.no_audio;
        let threshold = args.threshold;

        let charset = Charset::mkcharset(&args.charset)?;

        let Some(dimensions) = term_size::dimensions().and_then(|(w, h)| {
            Some((u32::try_from(w).ok()?, u32::try_from(h).ok()?))
        }) else {
            return Err("Could not detect the terminal's window size.".into());
        };

        let dependencies = Dependencies::setup()?;

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

        self.make_video(&temporary_video)?;

        remove_file(temporary_video)?;
        Ok(())
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

        let processed = AtomicUsize::new(0);
        let total = frames.len();

        // The compiler wanted to end its own life, so I'm giving
        // this variable an explicit type.
        let frames: Vec<(PathBuf, Vec<u8>)> = frames
            .into_par_iter()
            .map(|entry| -> Res<(PathBuf, Vec<u8>)> {
                if self.stop_handle.load(Relaxed) {
                    return Err("Stopped.".into());
                }
                let now = processed.fetch_add(1, Relaxed);

                print!("\rProcessing: {}% {now}/{total}", (100 * now) / total);
                let uncompressed_frame: String = self.make_frame(&entry)?;
                Ok((
                    entry.clone(),
                    encode_all(uncompressed_frame.as_bytes(), 1)?,
                ))
            })
            .collect::<Res<_>>()?;

        let mut processed = 0;
        // Let's write to the tar archive in a single thread, for obvious reasons.
        for (path, compressed_frame) in frames {
            processed += 1;
            print!(
                "\rLinking: {}% {processed}/{total}",
                (100 * processed) / total
            );

            let mut inside_path = PathBuf::from(".");
            inside_path.set_file_name(path.file_stem().unwrap());
            inside_path.set_extension("zst");

            add_file(&mut tar_archive, inside_path, &compressed_frame)?;
        }

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

    fn make_frame(&self, frame: &PathBuf) -> Res<String> {
        let resized_image = ImageReader::open(frame)?.decode()?.resize_exact(
            self.dimensions.0,
            self.dimensions.1,
            Nearest,
        );

        let mut frame = String::new();
        let mut last_colorized_pixel = resized_image.get_pixel(0, 0).0;

        for y in 0..self.dimensions.1 {
            for x in 0..self.dimensions.0 {
                let current_pixel = resized_image.get_pixel(x, y).0;
                let [r, g, b, _] = current_pixel;
                let brightness = r.max(g).max(b);

                let char = self.charset.match_char(brightness);
                if !self.colorize {
                    frame.push(char);
                    continue;
                }

                let char = match self.style {
                    Style::BgPaint | Style::FgPaint => char,
                    Style::BgOnly => ' ',
                };

                let max_colour_diff = Self::get_max_colour_diff(
                    current_pixel,
                    last_colorized_pixel,
                );

                if max_colour_diff > self.threshold || x == 0 {
                    write!(
                        frame,
                        "\x1b[{}8;2;{r};{g};{b}m{char}",
                        self.style.ansi()
                    )?;
                    last_colorized_pixel = current_pixel;
                } else {
                    frame.push(char)
                }
            }
            if self.colorize {
                frame.push_str(&format!("{RESET}\n"));
            } else {
                frame.push('\n');
            }
        }
        Ok(frame)
    }

    #[inline]
    fn make_image(&self, image: &PathBuf) -> Res<()> {
        File::create(self.output.clone())?
            .write_all(self.make_frame(image)?.as_bytes())?;
        Ok(())
    }
    #[inline]
    fn get_max_colour_diff(pixel_a: [u8; 4], pixel_b: [u8; 4]) -> u8 {
        let [r1, g1, b1, _] = pixel_a;
        let [r2, g2, b2, _] = pixel_b;
        r1.abs_diff(r2).max(g1.abs_diff(g2)).max(b1.abs_diff(b2))
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
                &format!("{}/%03d.png", self.temp_dir.path().to_str().unwrap()),
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
