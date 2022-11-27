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
    asciix [OPTIONS] <file> [framerate]

ARGS:
    <file>         path to the .bapple file
    <framerate>    framerate to play the ascii. Default: 30 [default: 30]

OPTIONS:
    -h, --help       Print help information
        --loop       loops the stream
    -V, --version    Print version information
```

Examples:
```sh
asciix video.bapple 30 # frames per second
```

Loop a video/gif
```sh
asciix video.bapple --loop
```

## Copying
Read [here](https://github.com/S0raWasTaken/bad_apple#copying)
