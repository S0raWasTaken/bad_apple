//! Error types for ASCII art conversion operations.

use std::{error::Error, fmt, io};

/// Errors that can occur during ASCII art generation.
#[derive(Debug)]
pub enum AsciiError {
    /// An I/O error occurred while reading the image.
    Io(io::Error),

    /// The image format could not be determined or decoded.
    ImageFormat(image::ImageError),

    /// Dimensions were not set before calling `make_ascii()`.
    DimensionsNotSet,
}

impl fmt::Display for AsciiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "I/O error: {err}"),
            Self::ImageFormat(err) => write!(f, "Image format error: {err}"),
            Self::DimensionsNotSet => write!(
                f,
                "Dimensions not set: please call dimensions() with non-zero values before make_ascii()"
            ),
        }
    }
}

impl Error for AsciiError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::ImageFormat(err) => Some(err),
            Self::DimensionsNotSet => None,
        }
    }
}

impl From<io::Error> for AsciiError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<image::ImageError> for AsciiError {
    fn from(err: image::ImageError) -> Self {
        Self::ImageFormat(err)
    }
}
