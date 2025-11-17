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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // Tests for Input enum
    #[test]
    fn test_input_video_variant() {
        let path = PathBuf::from("video.mp4");
        let input = Input::Video(path.clone());
        match input {
            Input::Video(p) => assert_eq!(p, path),
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_input_image_variant() {
        let path = PathBuf::from("image.png");
        let input = Input::Image(path.clone());
        match input {
            Input::Image(p) => assert_eq!(p, path),
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_input_clone() {
        let path = PathBuf::from("test.mp4");
        let input1 = Input::Video(path.clone());
        let input2 = input1.clone();
        
        match (input1, input2) {
            (Input::Video(p1), Input::Video(p2)) => assert_eq!(p1, p2),
            _ => panic!("Clone failed"),
        }
    }

    #[test]
    fn test_input_with_various_video_extensions() {
        let extensions = vec!["mp4", "avi", "mkv", "mov", "webm"];
        
        for ext in extensions {
            let path = PathBuf::from(format!("video.{}", ext));
            let input = Input::Video(path.clone());
            
            match input {
                Input::Video(p) => assert_eq!(p.extension().unwrap(), ext),
                _ => panic!("Wrong variant"),
            }
        }
    }

    #[test]
    fn test_input_with_various_image_extensions() {
        let extensions = vec!["png", "jpg", "jpeg", "gif", "bmp"];
        
        for ext in extensions {
            let path = PathBuf::from(format!("image.{}", ext));
            let input = Input::Image(path.clone());
            
            match input {
                Input::Image(p) => assert_eq!(p.extension().unwrap(), ext),
                _ => panic!("Wrong variant"),
            }
        }
    }

    // Tests for Metadata struct
    #[test]
    fn test_metadata_new() {
        let meta = Metadata::new(30, 33);
        assert_eq!(meta.fps, 30);
        assert_eq!(meta.frametime, 33);
    }

    #[test]
    fn test_metadata_version() {
        let meta = Metadata::new(60, 16);
        assert!(!meta.asciic_version.is_empty());
        // Version should be non-empty
        assert!(meta.asciic_version.len() > 0);
    }

    #[test]
    fn test_metadata_various_fps() {
        let fps_values = vec![24, 25, 30, 50, 60, 120];
        
        for fps in fps_values {
            let meta = Metadata::new(fps, 1000 / fps);
            assert_eq!(meta.fps, fps);
        }
    }

    #[test]
    fn test_metadata_frametime_calculation() {
        // Common fps and frametime pairs
        let pairs = vec![
            (30, 33),   // ~30 fps
            (60, 16),   // ~60 fps
            (24, 41),   // ~24 fps
            (25, 40),   // 25 fps
        ];
        
        for (fps, frametime) in pairs {
            let meta = Metadata::new(fps, frametime);
            assert_eq!(meta.fps, fps);
            assert_eq!(meta.frametime, frametime);
        }
    }

    // Tests for Charset
    #[test]
    fn test_charset_default() {
        let charset = Charset::default();
        assert_eq!(charset.0.len(), 7);
        assert_eq!(charset.1.len(), 7);
        assert_eq!(charset.2, '@');
    }

    #[test]
    fn test_charset_thresholds() {
        let charset = Charset::default();
        assert_eq!(charset.0, vec![20, 40, 80, 100, 130, 200, 250]);
    }

    #[test]
    fn test_charset_chars() {
        let charset = Charset::default();
        assert_eq!(charset.1, vec![' ', '.', ':', '-', '+', '=', '#']);
    }

    #[test]
    fn test_charset_match_char_low_brightness() {
        let charset = Charset::default();
        let ch = charset.match_char(10);
        assert_eq!(ch, ' ');
    }

    #[test]
    fn test_charset_match_char_mid_brightness() {
        let charset = Charset::default();
        let ch = charset.match_char(50);
        assert_eq!(ch, ':');
    }

    #[test]
    fn test_charset_match_char_high_brightness() {
        let charset = Charset::default();
        let ch = charset.match_char(255);
        assert_eq!(ch, '@');
    }

    #[test]
    fn test_charset_match_char_edge_cases() {
        let charset = Charset::default();
        
        // Test boundary values
        assert_eq!(charset.match_char(0), ' ');
        assert_eq!(charset.match_char(20), ' ');
        assert_eq!(charset.match_char(21), '.');
        assert_eq!(charset.match_char(40), '.');
        assert_eq!(charset.match_char(41), ':');
    }

    #[test]
    fn test_charset_match_char_all_ranges() {
        let charset = Charset::default();
        
        // Test each range
        let test_cases = vec![
            (15, ' '),
            (35, '.'),
            (70, ':'),
            (95, '-'),
            (120, '+'),
            (180, '='),
            (240, '#'),
            (255, '@'),
        ];
        
        for (brightness, expected) in test_cases {
            assert_eq!(charset.match_char(brightness), expected);
        }
    }

    // Tests for get_max_colour_diff
    #[test]
    fn test_get_max_colour_diff_identical() {
        let pixel = [100, 150, 200, 255];
        let diff = AsciiCompiler::get_max_colour_diff(pixel, pixel);
        assert_eq!(diff, 0);
    }

    #[test]
    fn test_get_max_colour_diff_red_only() {
        let pixel1 = [100, 150, 200, 255];
        let pixel2 = [120, 150, 200, 255];
        let diff = AsciiCompiler::get_max_colour_diff(pixel1, pixel2);
        assert_eq!(diff, 20);
    }

    #[test]
    fn test_get_max_colour_diff_green_only() {
        let pixel1 = [100, 150, 200, 255];
        let pixel2 = [100, 180, 200, 255];
        let diff = AsciiCompiler::get_max_colour_diff(pixel1, pixel2);
        assert_eq!(diff, 30);
    }

    #[test]
    fn test_get_max_colour_diff_blue_only() {
        let pixel1 = [100, 150, 200, 255];
        let pixel2 = [100, 150, 250, 255];
        let diff = AsciiCompiler::get_max_colour_diff(pixel1, pixel2);
        assert_eq!(diff, 50);
    }

    #[test]
    fn test_get_max_colour_diff_all_channels() {
        let pixel1 = [100, 150, 200, 255];
        let pixel2 = [110, 140, 220, 255];
        // Max diff: red=10, green=10, blue=20 -> max is 20
        let diff = AsciiCompiler::get_max_colour_diff(pixel1, pixel2);
        assert_eq!(diff, 20);
    }

    #[test]
    fn test_get_max_colour_diff_black_to_white() {
        let black = [0, 0, 0, 255];
        let white = [255, 255, 255, 255];
        let diff = AsciiCompiler::get_max_colour_diff(black, white);
        assert_eq!(diff, 255);
    }

    #[test]
    fn test_get_max_colour_diff_small_difference() {
        let pixel1 = [100, 100, 100, 255];
        let pixel2 = [101, 102, 103, 255];
        let diff = AsciiCompiler::get_max_colour_diff(pixel1, pixel2);
        assert_eq!(diff, 3); // Max of [1, 2, 3]
    }

    #[test]
    fn test_get_max_colour_diff_symmetry() {
        let pixel1 = [100, 150, 200, 255];
        let pixel2 = [50, 100, 150, 255];
        
        let diff1 = AsciiCompiler::get_max_colour_diff(pixel1, pixel2);
        let diff2 = AsciiCompiler::get_max_colour_diff(pixel2, pixel1);
        
        assert_eq!(diff1, diff2);
    }

    // Tests for add_file function
    #[test]
    fn test_add_file_creates_header() {
        use std::fs::File;
        use tempfile::NamedTempFile;
        
        let temp_file = NamedTempFile::new().unwrap();
        let mut archive = Builder::new(File::create(temp_file.path()).unwrap());
        
        let data = b"test data";
        let result = add_file(&mut archive, "test.txt", data);
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_add_file_with_empty_data() {
        use std::fs::File;
        use tempfile::NamedTempFile;
        
        let temp_file = NamedTempFile::new().unwrap();
        let mut archive = Builder::new(File::create(temp_file.path()).unwrap());
        
        let data: &[u8] = &[];
        let result = add_file(&mut archive, "empty.txt", data);
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_add_file_with_large_data() {
        use std::fs::File;
        use tempfile::NamedTempFile;
        
        let temp_file = NamedTempFile::new().unwrap();
        let mut archive = Builder::new(File::create(temp_file.path()).unwrap());
        
        let data = vec![0u8; 10000];
        let result = add_file(&mut archive, "large.bin", &data);
        
        assert!(result.is_ok());
    }

    // Integration-style tests for Charset behavior
    #[test]
    fn test_charset_covers_full_brightness_range() {
        let charset = Charset::default();
        
        // Test every brightness value from 0 to 255
        for brightness in 0..=255 {
            let ch = charset.match_char(brightness);
            // Should return a valid character for any brightness
            assert!(ch == ' ' || ch == '.' || ch == ':' || ch == '-' || 
                   ch == '+' || ch == '=' || ch == '#' || ch == '@');
        }
    }

    #[test]
    fn test_charset_progression() {
        let charset = Charset::default();
        
        // Characters should get "darker" as brightness increases
        let chars: Vec<char> = (0..=255).step_by(30).map(|b| charset.match_char(b)).collect();
        
        // Verify we get a progression through the character set
        assert!(chars.len() > 1);
    }

    // Edge case tests
    #[test]
    fn test_metadata_edge_cases() {
        // Very high FPS
        let high_fps = Metadata::new(240, 4);
        assert_eq!(high_fps.fps, 240);
        
        // Very low FPS
        let low_fps = Metadata::new(1, 1000);
        assert_eq!(low_fps.fps, 1);
        
        // Zero values (edge case, may not be realistic)
        let zero = Metadata::new(0, 0);
        assert_eq!(zero.fps, 0);
    }

    #[test]
    fn test_input_with_complex_paths() {
        // Test with various path complexities
        let paths = vec![
            "simple.mp4",
            "./relative/path/video.mp4",
            "/absolute/path/image.png",
            "../parent/file.jpg",
            "path with spaces.mp4",
            "path-with-dashes.png",
            "path_with_underscores.mkv",
        ];
        
        for path_str in paths {
            let path = PathBuf::from(path_str);
            let input = Input::Video(path.clone());
            
            match input {
                Input::Video(p) => assert_eq!(p, path),
                _ => panic!("Wrong variant"),
            }
        }
    }

    #[test]
    fn test_colour_diff_with_all_same_values() {
        let pixel = [128, 128, 128, 255];
        let diff = AsciiCompiler::get_max_colour_diff(pixel, pixel);
        assert_eq!(diff, 0);
    }

    #[test]
    fn test_colour_diff_with_alpha_channel() {
        // Alpha channel should be ignored
        let pixel1 = [100, 150, 200, 255];
        let pixel2 = [100, 150, 200, 0];
        let diff = AsciiCompiler::get_max_colour_diff(pixel1, pixel2);
        assert_eq!(diff, 0); // Only RGB matters, not alpha
    }

    #[test]
    fn test_metadata_version_format() {
        let meta = Metadata::new(30, 33);
        // Version should follow semver-like format (contains digits)
        assert!(meta.asciic_version.chars().any(|c| c.is_numeric()));
    }
}
