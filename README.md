# Bad Apple
An asciinema compiler.

This project was made as a helper to compile videos into asciinema, mainly focused in making the [bad apple](https://www.youtube.com/watch?v=UkgK8eUdpAo) video play on a text interface (TTY/framebuffer/console/etc).
<br>It is one of the steps I took before working on my bad apple kernel & OS (WIP)

## Installation instructions
You must have cargo, rustc and FFMPEG (binary) installed.
```sh
cargo install --git https://github.com/S0raWasTaken/bad_apple
```

## Usage
> --help output:
```yml
USAGE:
    bad_apple <video> <output-dir>

ARGS:
    <video>         Input video to transform in asciinema
    <output-dir>    Output directory
                    Creates a directory if it doesn't exist

OPTIONS:
    -h, --help       Print help information
    -V, --version    Print version information
```

Example:
```sh
bad_apple video.mp4 output-dir/
```
