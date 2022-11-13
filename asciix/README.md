# Asciix
The asciinema player for frames generated with [asciic](../asciic)

## Installation
You will need `mpv` installed on your system if you want to play videos with sound.
`cargo` and `rustc` are also required, since we're compiling from source.
```sh
cargo install --git https://github.com/S0raWasTaken/bad_apple asciix
```

## Usage
> --help output:
```yml
USAGE:
    asciix <path> [framerate]

ARGS:
    <path>         path to the frames directory
    <framerate>    framerate to play the ascii. Default: 30 [default: 30]

OPTIONS:
    -h, --help       Print help information
    -V, --version    Print version information
```

Example:
```sh
asciix bad_apple.mp4 30 # frames per second
```

## Copying
Read [here](../README.md)
