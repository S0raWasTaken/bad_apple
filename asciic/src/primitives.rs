use std::fmt::Write as FmtWrite;
use std::fs::{File, read_dir};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::Relaxed;
use std::{path::PathBuf, sync::atomic::AtomicBool};

use clap::crate_version;
use image::{GenericImageView, ImageReader, imageops::FilterType::Nearest};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::Serialize;
use tar::{Builder, Header};
use tempfile::TempDir;
use zstd::encode_all;

use crate::colours::RESET;
use crate::{
    Res,
    children::ffmpeg,
    cli::{Args, Style},
};

#[derive(Serialize)]
pub struct Metadata {
    frametime: usize,
    fps: usize,
    asciic_version: &'static str,
}
impl Metadata {
    pub fn new(fps: usize, frametime: usize) -> Self {
        Self { frametime, fps, asciic_version: crate_version!() }
    }
}

#[derive(Clone)]
pub enum Input {
    Video(PathBuf),
    Image(PathBuf),
}

pub struct Charset(pub Vec<u8>, pub Vec<char>, pub char);

impl Default for Charset {
    fn default() -> Self {
        Self(
            vec![20, 40, 80, 100, 130, 200, 250],
            vec![' ', '.', ':', '-', '+', '=', '#'],
            '@',
        )
    }
}

impl Charset {
    pub fn match_char(&self, brightness: u8) -> char {
        self.0
            .iter()
            .zip(self.1.clone())
            .find(|(threshold, _)| brightness <= **threshold)
            .map_or(self.2, |(_, c)| c)
    }
}

pub struct AsciiCompiler {
    pub stop_handle: AtomicBool,
    pub temp_dir: TempDir,
    dimensions: (u32, u32),

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

        // TODO: unhardcode this
        let charset = Charset::default();

        let Some(dimensions) = term_size::dimensions().map(|(w, h)| {
            (u32::try_from(w).unwrap(), u32::try_from(h).unwrap())
        }) else {
            return Err("Could not detect the terminal's window size.".into());
        };

        Ok(Self {
            stop_handle,
            temp_dir,
            dimensions,
            input,
            colorize,
            no_audio,
            style,
            output,
            charset,
            threshold,
        })
    }

    pub fn install_deps(&self) -> Res<()> {
        Ok(())
    }

    pub fn compile(&self) -> Res<()> {
        match &self.input {
            Input::Video(video) => self.make_video(video),
            Input::Image(image) => self.make_image(image),
        }?;
        Ok(())
    }

    fn make_video(&self, video: &PathBuf) -> Res<()> {
        let video_path = video.to_str().unwrap();
        self.split_video_frames(video_path)?;
        if !self.no_audio {
            self.extract_audio(video_path)?;
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

            add_file(&mut tar_archive, path, &compressed_frame)?;
        }

        if !self.no_audio {
            let mut audio = File::open(tmp_path.join("audio.mp3"))?;
            let mut data = Vec::new();
            audio.read_to_end(&mut data)?;

            add_file(&mut tar_archive, "audio.mp3", &data)?;
        }

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
                }

                let char = match self.style {
                    Style::BgPaint | Style::FgPaint => char,
                    Style::BgOnly => ' ',
                };

                let max_colour_diff = Self::get_max_colour_diff(
                    current_pixel,
                    last_colorized_pixel,
                );

                last_colorized_pixel = current_pixel;

                if max_colour_diff > self.threshold || x == 0 {
                    write!(
                        frame,
                        "\x1b[{}8;2;{r};{g};{b}m{char}",
                        self.style.ansi()
                    )?;
                } else {
                    frame.push(char)
                }
            }
            if self.colorize {
                frame.push_str("{RESET}\n");
            } else {
                frame.push('\n');
            }
        }
        Ok(frame)
    }

    fn make_image(&self, image: &PathBuf) -> Res<()> {
        let frame = self.make_frame(image)?;
        File::create(self.output.clone())?.write_all(frame.as_bytes())?;
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
        // TODO: Handle local yt-dlp, ffmpeg & ffprobe installation.
        ffmpeg(
            &PathBuf::from("ffmpeg"),
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
            &PathBuf::from("ffmpeg"),
            &[
                "-i",
                video_path,
                &format!("{}/audio.mp3", self.temp_dir.path().display()),
            ],
        )
    }
}

fn add_file(
    tar_archive: &mut Builder<File>,
    path: impl AsRef<Path>,
    compressed_frame: &[u8],
) -> Res<()> {
    let mut header = Header::new_gnu();
    header.set_size(compressed_frame.len() as u64);
    header.set_cksum();
    tar_archive.append_data(&mut header, path, compressed_frame)?;
    Ok(())
}
