//! A library for converting images to ASCII art with optional colorization.
//!
//! This crate provides a builder-pattern API for converting raster images into
//! ASCII art representations. It supports various character sets, color styles,
//! and compression options for optimized ANSI output.
//!
//! # Examples
//!
//! Basic usage:
//!
//! ```no_run
//! use std::fs::File;
//! use libasciic::{AsciiBuilder, Style};
//!
//! let file = File::open("image.png")?;
//! let ascii = AsciiBuilder::new(file)?
//!     .dimensions(80, 40)
//!     .colorize(true)
//!     .style(Style::FgPaint)
//!     .threshold(10)
//!     .make_ascii()?;
//!
//! println!("{}", ascii);
//! # Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
//! ```
#![warn(clippy::pedantic)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

mod error;

use std::io::{BufReader, Read, Seek};

use image::{GenericImageView, ImageReader};

pub use image::imageops::FilterType;

use crate::error::AsciiError;

type Res<T> = Result<T, AsciiError>;
/// Defines how colors are applied to ASCII art output.
///
/// Different styles control whether characters themselves carry color information
/// or if colors are applied to the background.
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub enum Style {
    /// Paint the foreground (characters) with RGB colors.
    /// Characters vary based on brightness, and each character is colored.
    FgPaint,

    /// Paint the background with RGB colors while keeping characters visible.
    /// Characters vary based on brightness with colored backgrounds.
    BgPaint,

    /// Paint only the background with RGB colors using space characters.
    /// Creates a purely color-based representation without visible ASCII characters.
    BgOnly,

    /// Paint both background and foreground.
    /// It darkens the background by a configurable percentage, so you can actually see the
    /// foreground characters.
    ///
    /// Be very mindful that **this doubles the amount of ansi control strings**, so it costs
    /// a lot more to print the output.
    Mixed,
}

impl Style {
    /// Generates an ANSI escape sequence to colorize a character.
    ///
    /// # Arguments
    ///
    /// * `char` - The character to colorize
    /// * `rgb` - RGBA color values [red, green, blue, alpha]
    /// * `factor` - Background brightness factor (0.0-1.0), only used for [`Style::Mixed`]
    ///
    /// # Returns
    ///
    /// A string containing the ANSI escape sequence and the character.
    #[must_use]
    pub fn colorize(&self, char: char, rgb: [u8; 4], factor: f32) -> String {
        if let Self::Mixed = self {
            return format!(
                "\x1b[38;2;{}\x1b[48;2;{}{char}",
                Self::rgb_to_string(rgb),
                Self::reduce_brightness(rgb, factor)
            );
        }
        format!("\x1b[{}8;2;{}{char}", self.ansi(), Self::rgb_to_string(rgb))
    }

    fn ansi(self) -> u8 {
        match self {
            Style::FgPaint => 3,
            Style::BgPaint | Style::BgOnly => 4,
            Style::Mixed => {
                unreachable!("Mixed does not require Style::ansi()")
            }
        }
    }

    #[inline]
    fn reduce_brightness([r, g, b, _]: [u8; 4], factor: f32) -> String {
        let [r, g, b] = [f32::from(r), f32::from(g), f32::from(b)];
        Self::rgb_to_string([
            (r * factor) as u8,
            (g * factor) as u8,
            (b * factor) as u8,
            0,
        ])
    }

    #[inline]
    fn rgb_to_string([r, g, b, _]: [u8; 4]) -> String {
        format!("{r};{g};{b}m")
    }
}

/// Internal character set mapping brightness levels to ASCII characters.
///
/// Maps pixel brightness values (0-255) to appropriate characters based on
/// configured thresholds. Characters are ordered from darkest to brightest.
#[derive(Debug, Clone)]
pub struct Charset(Vec<u8>, Vec<char>, char);

impl Charset {
    /// Finds the appropriate character for a given brightness level.
    ///
    /// # Arguments
    ///
    /// * `brightness` - Pixel brightness value (0-255)
    ///
    /// # Returns
    ///
    /// The character that best represents this brightness level.
    #[must_use]
    pub fn match_char(&self, brightness: u8) -> char {
        self.0
            .iter()
            .zip(self.1.iter())
            .find(|(threshold, _)| brightness <= **threshold)
            .map_or(self.2, |(_, c)| *c)
    }

    /// Creates a new character set from a specification string.
    ///
    /// # Arguments
    ///
    /// * `spec` - A string of characters ordered from darkest to brightest.
    ///   A space character is automatically prepended for the darkest value.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use libasciic::Charset;
    /// let charset = Charset::mkcharset(".:-+=#@")?;
    /// # Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    /// ```
    ///
    /// # Returns
    ///
    /// A `Charset` with evenly distributed brightness thresholds.
    #[must_use]
    pub fn mkcharset(spec: &str) -> Self {
        let mut chars: Vec<char> = spec.chars().collect();
        chars.insert(0, ' ');

        let steps = chars.len();
        let mut thresholds = Vec::with_capacity(steps);

        for i in 0..steps {
            let t =
                (i as f32 / (steps - 1).max(1) as f32 * 250.0).round() as u8;
            thresholds.push(t);
        }

        // I add an element at the start regardless, so it should never panic.
        #[allow(clippy::missing_panics_doc)]
        let last = *chars.last().unwrap();

        Self(thresholds, chars, last)
    }
}

/// Builder for converting images to ASCII art.
///
/// Provides a fluent API for configuring ASCII art generation with support for
/// dimensions, colorization, character sets, and compression.
///
/// # Type Parameters
///
/// * `R` - A readable and seekable source (e.g., `File`, `Cursor<Vec<u8>>`)
///
/// # Examples
///
/// ```no_run
/// use std::fs::File;
/// use libasciic::{AsciiBuilder, Style, FilterType};
///
/// let file = File::open("photo.jpg")?;
/// let ascii = AsciiBuilder::new(file)?
///     .dimensions(100, 50)
///     .colorize(true)
///     .style(Style::BgPaint)
///     .threshold(15)
///     .charset(".:;+=xX$@")?
///     .filter_type(FilterType::Lanczos3)
///     .make_ascii()?;
/// # Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
/// ```
pub struct AsciiBuilder<R: Read + Seek> {
    image: BufReader<R>,
    dimensions: (u32, u32),
    compression_threshold: u8,
    charset: Charset,
    style: Style,
    colour: bool,
    filter_type: FilterType,
    background_brightness: f32,
}

impl<R: Read + Seek> AsciiBuilder<R> {
    /// Creates a new ASCII art builder from an image source.
    ///
    /// # Arguments
    ///
    /// * `image` - A readable and seekable image source
    ///
    /// # Returns
    ///
    /// A builder with default settings:
    /// - No dimensions set (must be configured before calling `make_ascii`)
    /// - Default charset: `.:-+=#@`
    /// - No colorization
    /// - Foreground paint style
    /// - Nearest neighbor filtering
    /// - Zero compression threshold
    pub fn new(image: R) -> Self {
        Self {
            image: BufReader::new(image),
            dimensions: (0, 0),
            compression_threshold: 0,
            charset: Charset::mkcharset(".:-+=#@"),
            style: Style::FgPaint,
            colour: false,
            filter_type: FilterType::Nearest,
            background_brightness: 0.2,
        }
    }

    /// Generates the ASCII art string from the configured image.
    ///
    /// Decodes the image, resizes it to the specified dimensions, and converts
    /// each pixel to an appropriate ASCII character based on brightness and color.
    ///
    /// # Returns
    ///
    /// A string containing the ASCII art with optional ANSI color codes.
    /// Each line is terminated with `\n`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Dimensions have not been set (are `(0, 0)`)
    /// - Image format cannot be determined
    /// - Image decoding fails
    pub fn make_ascii(self) -> Res<String> {
        if self.dimensions.0 == 0 || self.dimensions.1 == 0 {
            return Err(AsciiError::DimensionsNotSet);
        }

        let resized_image = ImageReader::new(self.image)
            .with_guessed_format()?
            .decode()?
            .resize_exact(
                self.dimensions.0,
                self.dimensions.1,
                self.filter_type,
            );

        let mut frame = String::new();
        let mut last_colorized_pixel = resized_image.get_pixel(0, 0).0;

        for y in 0..self.dimensions.1 {
            for x in 0..self.dimensions.0 {
                let current_pixel = resized_image.get_pixel(x, y).0;
                let [r, g, b, _] = current_pixel;
                let brightness = r.max(g).max(b);

                let char = self.charset.match_char(brightness);

                if !self.colour {
                    frame.push(char);
                    continue;
                }

                let char = match self.style {
                    Style::BgOnly => ' ',
                    _ => char,
                };

                let should_colorize =
                    max_colour_diff(current_pixel, last_colorized_pixel)
                        > self.compression_threshold
                        || x == 0;

                if should_colorize {
                    frame.push_str(&self.style.colorize(
                        char,
                        current_pixel,
                        self.background_brightness,
                    ));
                    last_colorized_pixel = current_pixel;
                } else {
                    frame.push(char);
                }
            }
            if self.colour {
                frame.push_str("\x1b[0m");
            }

            if y != self.dimensions.1 - 1 {
                frame.push('\n');
            }
        }

        Ok(frame)
    }

    /// Enables or disables ANSI color output.
    ///
    /// # Arguments
    ///
    /// * `colorize` - `true` to enable RGB colors, `false` for monochrome
    ///
    /// # Returns
    ///
    /// The builder for method chaining.
    #[inline]
    #[must_use]
    pub fn colorize(mut self, colorize: bool) -> Self {
        self.colour = colorize;
        self
    }

    /// Sets the output dimensions for the ASCII art.
    ///
    /// # Arguments
    ///
    /// * `width` - Number of characters per line
    /// * `height` - Number of lines
    ///
    /// # Returns
    ///
    /// The builder for method chaining.
    ///
    /// # Notes
    ///
    /// Must be called before `make_ascii()`. Consider that characters are typically
    /// taller than they are wide, so you may want to adjust the aspect ratio.
    #[inline]
    #[must_use]
    pub fn dimensions(mut self, width: u32, height: u32) -> Self {
        self.dimensions = (width, height);
        self
    }

    /// Sets the color compression threshold.
    ///
    /// # Arguments
    ///
    /// * `threshold` - Maximum color difference (0-255) before emitting new ANSI codes.
    ///   Higher values reduce output size but decrease color accuracy.
    ///   A value of 0 emits color codes for every pixel change.
    ///
    /// # Returns
    ///
    /// The builder for method chaining.
    ///
    /// # Notes
    ///
    /// Only applies when colorization is enabled. Useful for reducing the size of
    /// colored ASCII art output by avoiding redundant ANSI escape sequences.
    #[inline]
    #[must_use]
    pub fn threshold(mut self, threshold: u8) -> Self {
        self.compression_threshold = threshold;
        self
    }

    /// Sets a custom character set for brightness mapping.
    ///
    /// # Arguments
    ///
    /// * `charset` - Characters ordered from darkest to brightest (space is added automatically)
    ///
    /// # Returns
    ///
    /// The builder for method chaining.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use std::fs::File;
    /// # use libasciic::AsciiBuilder;
    /// # let file = File::open("image.png")?;
    /// let builder = AsciiBuilder::new(file)?
    ///     .charset(".'`^\",:;Il!i><~+_-?][}{1)(|\\/tfjrxnuvczXYUJCLQ0OZmwqpdbkhao*#MW&8%B@$")?;
    /// # Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    /// ```
    #[inline]
    #[must_use]
    pub fn charset(mut self, charset: &str) -> Self {
        self.charset = Charset::mkcharset(charset);
        self
    }

    /// Sets the color application style.
    ///
    /// # Arguments
    ///
    /// * `style` - The style to use (see [`Style`] for options)
    ///
    /// # Returns
    ///
    /// The builder for method chaining.
    #[inline]
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Sets the image resampling filter type.
    ///
    /// # Arguments
    ///
    /// * `filter_type` - The filter to use when resizing (from `image::imageops::FilterType`)
    ///
    /// # Returns
    ///
    /// The builder for method chaining.
    ///
    /// # Notes
    ///
    /// - `Nearest`: Fastest but lowest quality
    /// - `Triangle`: Good balance of speed and quality
    /// - `CatmullRom`: High quality
    /// - `Lanczos3`: Highest quality but slowest
    #[inline]
    #[must_use]
    pub fn filter_type(mut self, filter_type: FilterType) -> Self {
        self.filter_type = filter_type;
        self
    }

    /// Sets the background brightness factor for Mixed style.
    ///
    /// # Arguments
    ///
    /// * `factor` - A value between 0.0 and 1.0 that controls background brightness.
    ///   - `1.0` keeps the background at full brightness (same as foreground)
    ///   - `0.5` reduces background to 50% brightness
    ///   - `0.2` (default) reduces background to 20% brightness
    ///   - `0.0` makes the background completely black
    ///
    /// # Returns
    ///
    /// The builder for method chaining.
    ///
    /// # Notes
    ///
    /// Only applies when using [`Style::Mixed`]. Lower values create more contrast
    /// between foreground characters and background, making text more readable.
    #[inline]
    #[must_use]
    pub fn background_brightness(mut self, factor: f32) -> Self {
        self.background_brightness = factor.clamp(0.0, 1.0);
        self
    }
}

#[inline]
fn max_colour_diff(pixel_a: [u8; 4], pixel_b: [u8; 4]) -> u8 {
    let [r1, g1, b1, _] = pixel_a;
    let [r2, g2, b2, _] = pixel_b;
    r1.abs_diff(r2).max(g1.abs_diff(g2)).max(b1.abs_diff(b2))
}
