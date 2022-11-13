# asciic
An asciinema compiler.

This project was made as a helper to compile videos into asciinema, mainly focused in making the [bad apple](https://www.youtube.com/watch?v=UkgK8eUdpAo) video play on a text interface (TTY/framebuffer/console/etc).
<br>It is one of the steps I took before working on my bad apple kernel & OS (WIP)

## Installation instructions
You must have `cargo`, `rustc` and `ffmpeg` (binary) installed.
```sh
cargo install --git https://github.com/S0raWasTaken/bad_apple asciic
```

## Usage
> --help output:
```yml
USAGE:
    asciic [OPTIONS] [ARGS]

ARGS:
    <video>         Input video to transform in asciinema
    <output-dir>    Output directory
                    Creates a directory if it doesn't exist

OPTIONS:
    -h, --help                 Print help information
    -i <image>                 compiles a single image
    -s, --size <frame-size>    The ratio that each frame should be resized [default: 216x56]
    -V, --version              Print version information
```

Examples:
> Compiling a normal video:
```sh
asciic video.mp4 output-dir/
```

> Compiling an image:
```sh
asciic -i image.png
```

> Passing the frame size argument:
```sh
asciic video.mp4 output-dir/ -i 500x150
# This command gives out a warning about things getting wonky at high image sizes,
# but you can safely ignore them if you want :)
```
