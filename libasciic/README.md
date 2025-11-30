# libasciic

A Rust library for converting images to ASCII art with optional ANSI colorization.

## Features

- 🎨 **Full RGB Color Support** - Generate colorized ASCII art with 24-bit ANSI colors
- 🎭 **Multiple Styles** - Choose between foreground painting, background painting, or background-only modes
- 🔧 **Customizable Character Sets** - Use any character progression for brightness mapping
- 📦 **Compression** - Smart color compression to reduce output size
- 🖼️ **Multiple Formats** - Supports all image formats handled by the `image` crate
- ⚡ **Flexible Resizing** - Various resampling filters from nearest neighbor to Lanczos3

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
libasciic = "1.1.0"
```

## Quick Start

```rust
use std::fs::File;
use libasciic::{AsciiBuilder, Style};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open("image.png")?;
    
    let ascii = AsciiBuilder::new(file)
        .dimensions(80, 40)
        .colorize(true)
        .style(Style::FgPaint)
        .make_ascii()?;
    
    println!("{}", ascii);
    Ok(())
}
```

## Examples

### Basic Monochrome ASCII Art

```rust
use std::fs::File;
use libasciic::AsciiBuilder;

let file = File::open("photo.jpg")?;
let ascii = AsciiBuilder::new(file)
    .dimensions(100, 50)
    .make_ascii()?;

println!("{}", ascii);
```

### Colorized with Custom Character Set

```rust
use std::fs::File;
use libasciic::{AsciiBuilder, Style};

let file = File::open("image.png")?;
let ascii = AsciiBuilder::new(file)
    .dimensions(120, 60)
    .colorize(true)
    .style(Style::BgPaint)
    .charset(".:;+=xX$@")
    .threshold(10)  // Reduce output size
    .make_ascii()?;

print!("{}", ascii);
```

### Background-Only Mode (Pure Color Blocks)

```rust
use std::fs::File;
use libasciic::{AsciiBuilder, Style};

let file = File::open("artwork.png")?;
let ascii = AsciiBuilder::new(file)
    .dimensions(80, 40)
    .colorize(true)
    .style(Style::BgOnly)
    .make_ascii()?;

print!("{}", ascii);
```

### High-Quality Resampling

```rust
use std::fs::File;
use libasciic::{AsciiBuilder, FilterType};

let file = File::open("photo.jpg")?;
let ascii = AsciiBuilder::new(file)
    .dimensions(150, 75)
    .filter_type(FilterType::Lanczos3)
    .colorize(true)
    .make_ascii()?;

print!("{}", ascii);
```

## API Overview

### `AsciiBuilder`

The main builder struct for creating ASCII art. All methods are chainable.

#### Methods

- **`new(image: R) -> Self`** - Create a new builder from an image source
- **`dimensions(width: u32, height: u32) -> Self`** - Set output dimensions (required)
- **`colorize(bool) -> Self`** - Enable/disable ANSI color output
- **`style(Style) -> Self`** - Set the color style
- **`charset(&str) -> Self`** - Set custom character set for brightness mapping
- **`threshold(u8) -> Self`** - Set color compression threshold (0-255)
- **`filter_type(FilterType) -> Self`** - Set image resampling filter
- **`make_ascii(self) -> Result<String>`** - Generate the ASCII art

### `Style` Enum

Controls how colors are applied to the output:

- **`FgPaint`** - Color the characters (foreground)
- **`BgPaint`** - Color the background while keeping characters visible
- **`BgOnly`** - Use only colored backgrounds with space characters

### Character Sets

The default character set is `.:-+=#@`, ordered from darkest to brightest. A space character is automatically prepended for the darkest values.

Custom character sets can be provided in the same format:

```rust
.charset(".'`^\",:;Il!i><~+_-?][}{1)(|\\/tfjrxnuvczXYUJCLQ0OZmwqpdbkhao*#MW&8%B@$")
```

### Compression Threshold

The threshold parameter controls color compression when colorization is enabled:

- **`0`** - No compression (emit ANSI codes for every pixel)
- **`1-255`** - Only emit new color codes when the color difference exceeds this value

Higher values reduce output size but may decrease color accuracy. Recommended range: 5-20.

## Tips

- Terminal characters are typically taller than they are wide, so you may want to adjust your aspect ratio accordingly
- For better quality, use `FilterType::Lanczos3` or `FilterType::CatmullRom` when downscaling
- Use compression (`threshold()`) to significantly reduce the size of colorized output
- The `BgOnly` style works well for creating color-accurate terminal images

## License

MIT

## Author

S0ra (S0raWasTaken)

## Repository

[https://github.com/S0raWasTaken/bad_apple](https://github.com/S0raWasTaken/bad_apple)
