//! # `ascii_linker`
//!
//! A procedural macro crate for embedding ASCII art animation frames from `.bapple` files
//! directly into your Rust binary at compile time.
//!
//! ## Overview
//!
//! This crate provides two macros for working with `.bapple` files:
//! - `link_frames!` - Embeds decompressed ASCII frames as strings
//! - `embed_full!` - Embeds compressed frames with audio data and timing metadata
//!
//! ## File Format
//!
//! The `.bapple` format is a tar archive where:
//! - Each entry represents a frame of ASCII art (zstd-compressed)
//! - A special file named "metadata" contains timing information in RON format
//! - A special file named "audio" contains audio data
//! - All other entries are treated as animation frames
//!
//! ## Generating .bapple Files
//!
//! The `.bapple` files used by this crate are generated using [`asciic`](https://github.com/S0raWasTaken/bad_apple/tree/master/asciic),
//! an ASCII art animation compiler. See the `asciic` documentation for details on creating
//! `.bapple` files from video sources.
//!
//! ## Examples
//!
//! ### Using `link_frames!`
//!
//! ```rust
//! use ascii_linker::link_frames;
//!
//! // Embed frames at compile time
//! const FRAMES: &[&str] = link_frames!("./animations/bad_apple.bapple");
//!
//! fn main() {
//!     // Iterate through frames
//!     for (i, frame) in FRAMES.iter().enumerate() {
//!         println!("Frame {}: \n{}", i, frame);
//!     }
//! }
//! ```
//!
//! ### Using `embed_full!`
//!
//! When using `embed_full!`, you'll need to add `zstd` to your dependencies to decompress
//! frames at runtime:
//!
//! ```toml
//! [dependencies]
//! ascii_linker = "..."
//! zstd = "0.13"
//! ```
//!
//! ```rust
//! use ascii_linker::embed_full;
//!
//! // Embed compressed frames, audio, and timing
//! const BAPPLE: (&[&[u8]], &[u8], u64) = embed_full!("./animations/bad_apple.bapple");
//!
//! fn main() {
//!     let (frames, audio, frametime) = BAPPLE;
//!     
//!     println!("Total frames: {}", frames.len());
//!     println!("Audio size: {} bytes", audio.len());
//!     println!("Frame time: {}μs", frametime);
//!     
//!     // Decompress frames at runtime using zstd
//!     for compressed_frame in frames {
//!         let frame = zstd::decode_all(&compressed_frame[..]).unwrap();
//!         let frame_str = String::from_utf8(frame).unwrap();
//!         println!("{}", frame_str);
//!     }
//! }
//! ```

#![warn(clippy::pedantic)]

use proc_macro::TokenStream;
use ron::de::from_bytes;
use serde::Deserialize;
use std::fmt::Write;
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
/// use ascii_linker::link_frames;
///
/// // Basic usage - embed animation frames
/// const FRAMES: &[&str] = link_frames!("./path/to/animation.bapple");
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
/// - All frames are embedded in the final binary **decompressed**, increasing binary size significantly
/// - For smaller binaries, consider using `embed_full!` which keeps frames compressed
/// - The path is relative to the crate root where `Cargo.toml` is located
/// - This is ideal for small ASCII animations where runtime decompression overhead is undesirable
#[proc_macro]
pub fn link_frames(items: TokenStream) -> TokenStream {
    let file_path = items.into_iter().next().unwrap();
    let path_str = file_path.to_string().trim_matches('"').to_string();

    let mut tar: Archive<File> = Archive::new(File::open(path_str).unwrap());

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

        write!(ret, "\"{frame_as_str}\",").unwrap();
    }
    ret.push(']');
    ret.parse().unwrap()
}

/// Embeds a complete `.bapple` file (compressed frames, audio, and timing) at compile time.
///
/// This procedural macro reads a tar archive and extracts all components of an ASCII animation,
/// keeping frames in their compressed form for smaller binary size. This is more efficient than
/// `link_frames!` when binary size is a concern or when you want to control decompression at runtime.
///
/// # Arguments
///
/// * `file_path` - A string literal path to the `.bapple` file (relative to the crate root)
///
/// # Returns
///
/// Returns a tuple `(&[&[u8]], &[u8], u64)` containing:
/// - `&[&[u8]]` - Array of compressed frame data (zstd-compressed ASCII art)
/// - `&[u8]` - Raw audio data extracted from the "audio" entry
/// - `u64` - Frame time in microseconds (timing between frames)
///
/// # File Processing
///
/// - Opens the specified `.bapple` file as a tar archive
/// - Extracts the "metadata" entry and parses it as RON format to get timing information
/// - Extracts the "audio" entry as raw bytes
/// - Collects all other entries as compressed frame data
/// - Generates compile-time code that creates static byte arrays
///
/// # Panics
///
/// This macro will panic at compile time if:
/// - No file path is provided
/// - The specified file cannot be opened
/// - The file is not a valid tar archive
/// - The "metadata" entry is missing or malformed
/// - The frametime cannot be determined from metadata
///
/// # Dependencies
///
/// To decompress frames at runtime, add `zstd` to your `Cargo.toml`:
///
/// ```toml
/// [dependencies]
/// ascii_linker = "..."
/// zstd = "0.13"
/// ```
///
/// # Examples
///
/// ```no_run
/// use ascii_linker::embed_full;
///
/// // Embed the complete animation
/// const BAPPLE: (&[&[u8]], &[u8], u64) = embed_full!("./animations/bad_apple.bapple");
///
/// fn main() {
///     let (compressed_frames, audio, frametime_us) = BAPPLE;
///     
///     println!("Animation info:");
///     println!("  Frames: {}", compressed_frames.len());
///     println!("  Audio size: {} bytes", audio.len());
///     println!("  Frame time: {}μs ({} FPS)", frametime_us, 1_000_000 / frametime_us);
///     
///     // Decompress and display frames at runtime using zstd
///     for (i, compressed_frame) in compressed_frames.iter().enumerate() {
///         let decompressed = zstd::decode_all(&compressed_frame[..])
///             .expect("Failed to decompress frame");
///         let frame_str = String::from_utf8(decompressed)
///             .expect("Invalid UTF-8 in frame");
///         
///         println!("Frame {}: \n{}", i, frame_str);
///         
///         // Use frametime for animation timing
///         std::thread::sleep(std::time::Duration::from_micros(frametime_us));
///     }
/// }
/// ```
///
/// # Metadata Format
///
/// The "metadata" file should be in RON (Rusty Object Notation) format with the following structure:
///
/// ```ron
/// (
///     frametime: 33333,  // Microseconds per frame (e.g., 33333 = ~30 FPS)
///     fps: 0,            // DEPRECATED: legacy field, use frametime instead
/// )
/// ```
///
/// # Notes
///
/// - Frames remain **compressed** in the binary, significantly reducing binary size
/// - You must decompress frames at runtime using `zstd::decode_all()`
/// - The `fps` field in metadata is deprecated; `frametime` in microseconds is preferred
/// - The path is relative to the crate root where `Cargo.toml` is located
/// - This is ideal for larger animations or when you want to minimize binary size
/// - Audio data format depends on your `.bapple` file (commonly WAV or raw PCM)
#[proc_macro]
pub fn embed_full(items: TokenStream) -> TokenStream {
    let file_path = items.into_iter().next().unwrap();
    let path_str = file_path.to_string().trim_matches('"').to_string();

    let mut audio = Vec::new();
    let mut frametime = 0;

    let compressed_frames = Archive::new(File::open(path_str).unwrap())
        .entries()
        .unwrap()
        .filter_map(|e| process_frames(e, &mut audio, &mut frametime))
        .collect::<Vec<_>>();

    assert!(
        frametime != 0,
        ".bapple file is too old or it's corrupted.\n\
            Couldn't fetch the frametime info."
    );

    let mut audio_ret = String::from("&[");

    for byte in audio {
        write!(audio_ret, "{byte},").unwrap();
    }
    audio_ret.push(']');

    let mut compressed_frames_ret = String::from("&[");

    for frame_bytes in compressed_frames {
        let mut frame = String::from("&[");
        for byte in frame_bytes {
            write!(frame, "{byte},").unwrap();
        }
        frame.push(']');
        write!(compressed_frames_ret, "{frame},").unwrap();
    }
    compressed_frames_ret.push(']');

    format!("({compressed_frames_ret},{audio_ret},{frametime})")
        .parse()
        .unwrap()
}

// Borrowed from `bplay`
fn process_frames(
    entry: Result<Entry<'_, File>, std::io::Error>,
    audio: &mut Vec<u8>,
    outer_frametime: &mut u64,
) -> Option<Vec<u8>> {
    let mut entry = entry.ok()?;
    let file_stem = entry.header().path().ok()?.file_stem()?.to_os_string();

    let mut content = Vec::new();
    entry.read_to_end(&mut content).ok()?;

    if file_stem == *"audio" {
        *audio = content;

        return None;
    } else if file_stem == *"metadata" {
        let Metadata { frametime, fps } =
            from_bytes(&content).unwrap_or_default();
        if frametime != 0 {
            *outer_frametime = frametime;
        } else if fps != 0 {
            // DEPRECATED
            *outer_frametime = 1_000_000 / fps;
        }
        return None;
    }

    Some(content)
}

#[derive(Deserialize, Default)]
struct Metadata {
    frametime: u64,
    /// DEPRECATED
    fps: u64,
}
