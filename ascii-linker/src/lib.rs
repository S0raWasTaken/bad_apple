//! # ascii_linker
//!
//! A procedural macro crate for embedding ASCII art animation frames from `.bapple` files
//! directly into your Rust binary at compile time.
//!
//! ## Overview
//!
//! This crate provides the `link_bapple!` macro which reads a `.bapple` file (a tar archive
//! containing zstd-compressed ASCII art frames) and converts it into a static string array
//! that can be embedded in your compiled binary.
//!
//! ## File Format
//!
//! The `.bapple` format is a tar archive where:
//! - Each entry represents a frame of ASCII art (zstd-compressed)
//! - Special files named "metadata" and "audio" are ignored
//! - All other entries are treated as animation frames
//!
//! ## Example
//!
//! ```rust
//! use ascii_linker::link_bapple;
//!
//! // Embed frames at compile time
//! const FRAMES: &[&str] = link_bapple!("./animations/bad_apple.bapple");
//!
//! fn main() {
//!     // Iterate through frames
//!     for (i, frame) in FRAMES.iter().enumerate() {
//!         println!("Frame {}: \n{}", i, frame);
//!     }
//! }
//! ```

use proc_macro::TokenStream;
use std::fs::File;
use std::io::Read;
use tar::{Archive, Entry};
use zstd::decode_all;

/// Embeds ASCII art animation frames from a `.bapple` file into your binary at compile time.
///
/// This procedural macro reads a tar archive containing zstd-compressed ASCII art frames
/// and generates a static string slice array (`&[&str]`) containing all the decompressed frames.
///
/// # Arguments
///
/// * `file_path` - A string literal path to the `.bapple` file (relative to the crate root)
///
/// # Returns
///
/// Returns a `&[&str]` where each element is a complete ASCII art frame as a string.
///
/// # File Processing
///
/// - Opens the specified `.bapple` file as a tar archive
/// - Iterates through all entries in the archive
/// - Skips entries named "metadata" or "audio"
/// - Decompresses each frame using zstd
/// - Converts the decompressed bytes to UTF-8 strings
/// - Generates compile-time code that creates a static array of string slices
///
/// # Panics
///
/// This macro will panic at compile time if:
/// - No file path is provided
/// - The specified file cannot be opened
/// - The file is not a valid tar archive
/// - Any frame cannot be read or decompressed
/// - The decompressed content is not valid UTF-8
///
/// # Examples
///
/// ```no_run
/// use ascii_linker::link_bapple;
///
/// // Basic usage - embed animation frames
/// const FRAMES: &[&str] = link_bapple!("./path/to/animation.bapple");
///
/// // Access individual frames
/// println!("{}", FRAMES[0]);
///
/// // Get frame count
/// println!("Total frames: {}", FRAMES.len());
/// ```
///
/// # Notes
///
/// - The macro executes at compile time, so the file must exist when building
/// - All frames are embedded in the final binary, increasing its size
/// - The path is relative to the crate root where `Cargo.toml` is located
/// - This is ideal for bundling small to medium-sized ASCII animations
#[proc_macro]
pub fn link_bapple(items: TokenStream) -> TokenStream {
    let file_path = items.into_iter().next().unwrap();

    let mut tar: Archive<File> =
        Archive::new(File::open(file_path.to_string()).unwrap());

    let mut ret = String::from("&[");

    for frame in tar.entries().unwrap() {
        let mut frame: Entry<'_, File> = frame.unwrap();

        let file_stem =
            frame.header().path().unwrap().file_stem().unwrap().to_os_string();

        if file_stem == *"metadata" || file_stem == *"audio" {
            continue;
        }

        let mut content = Vec::new();
        frame.read_to_end(&mut content).unwrap();

        let frame_as_str =
            String::from_utf8(decode_all(&*content).unwrap()).unwrap();

        ret.push_str(&format!("\"{}\",", frame_as_str));
    }
    ret.push(']');
    ret.parse().unwrap()
}
