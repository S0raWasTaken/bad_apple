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
    asciic [OPTIONS] [video] [output] [-- <ffmpeg-flags>...]

ARGS:
    <video>              Input video to transform in asciinema
    <output>             Output file name [default: output]
    <ffmpeg-flags>...    Pass extra flags to ffmpeg

OPTIONS:
    -c
            Colorize output

    -h, --help
            Print help information

    -i, --image <image>
            Compiles a single image

    -n, --skip-compression
            Disables compression on colored outputs

        --no-audio
            skips audio generation

        --paint-fg
            Paints the foreground instead of background

    -s, --size <frame-size>
            The ratio that each frame should be resized [default: 216x56]

    -t, --threshold <compression-threshold>
            Manually sets the compression threshold [default: 10]

    -V, --version
            Print version information

```

Examples:
> Compiling a normal video:
```sh
asciic video.mp4 output.bapple
```

> Compiling a colored video:
```sh
asciic -c video.mp4 output.bapple
```

> Compiling an image:
```sh
asciic -i image.png
# Output will be available in image.txt
```

> Compiling a colored image:
```sh
asciic -i image.png -c --skip-compression
# We skip the color compression step, since it's a single image
```

> Passing the frame size argument:
```sh
asciic video.mp4 output.bapple -s 500x150
# This command gives out a warning about things getting wonky at high image sizes,
# but you can safely ignore them if you want :)
```

## Copying
Read [here](https://github.com/S0raWasTaken/bad_apple#copying)
