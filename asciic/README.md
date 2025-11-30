# ASCII Compiler
This project started as a helper to compile videos into asciinema format, mainly focused on making 
[bad apple](https://www.youtube.com/watch?v=UkgK8eUdpAo) playable on a terminal.

Nowadays, it's gotten pretty big and full of features to make your own stuff in your own style.

## Preview
> Find video previews on [my youtube channel](https://www.youtube.com/@S0ra-)

<img width="400" height="333" alt="image" src="https://github.com/user-attachments/assets/4ac1d1e9-5cc8-44a6-afba-afefbf5ceb05" />
<img width="765" height="412" alt="image" src="https://github.com/user-attachments/assets/69be5e11-928f-405c-ba20-c52fb6224acd" />
<img width="765" height="412" alt="image" src="https://github.com/user-attachments/assets/d2447b34-2e35-45ea-b4d8-94ad0c6c04e3" />

## Installation

You must have `cargo` and `rustc` installed.

```sh
cargo install --git https://github.com/S0raWasTaken/bad_apple asciic
```

## Dependency management

This crate supports automatic FFMPEG & YT-DLP management on Windows and Linux.

Any other OS will have the flag `--use-system-binaries` forcefully set and will *fail* if it can't find one of these in your PATH:
- `ffmpeg`
- `ffprobe`
- `yt-dlp`

Of course, you don't need any of these to compile a single image, nor do you need to have yt-dlp if you don't plan on using the `--youtube` argument.

So yeah, if your OS supports Rust, it likely runs this crate :)

Automatically managed dependencies can be found here:
- `%APPDATA%\asciic-bin\` on Windows, or:
- `$HOME/.local/share/asciic-bin/` on Linux.

## Usage

> --help output:

```yml
Usage: asciic [OPTIONS] [VIDEO]

Arguments:
  [VIDEO]
          Path to a valid video file

Options:
  -c, --colorize
          Makes the output colorized

  -n, --no-audio
          Skips audio extraction and inclusion in the output

      --use-system-binaries
          Use ffmpeg, ffprobe and yt-dlp from the system PATH when available

  -i, --image <IMAGE>
          Path to a valid image file

  -y, --youtube <YOUTUBE>
          Youtube video URL to download and use

  -o, --output <OUTPUT>
          Custom output path, defaults to the video's file name

  -s, --style <STYLE>
          Sets the output style

          Possible values:
          - fg-paint: Paint the foreground (characters) with RGB colors. Characters vary based on brightness, and each character is colored
          - bg-paint: Paint the background with RGB colors while keeping characters visible. Characters vary based on brightness with colored backgrounds
          - bg-only:  Paint only the background with RGB colors using space characters. Creates a purely color-based representation without visible ASCII characters
          - mixed:    Paint both background and foreground. It darkens the background by a configurable percentage, so you can actually see the foreground characters
          
          [default: bg-only]

      --temp <TEMP>
          Sets a custom path to create a temporary directory. It could be used to write the temporary files in memory, if the user sets this to /dev/shm
          
          [default: .]

  -t, --threshold <THRESHOLD>
          Sets the colour compression threshold
          
          [default: 3]

      --charset <CHARSET>
          Custom charset for the output
          
          [default: .:-+=#@]

  -f, --filter-type <FILTER_TYPE>
          Set a custom filter type for image resizing
          
          [default: nearest]
          [possible values: nearest, triangle, catmull-rom, gaussian, lanczos3]

  -b, --bg-brightness <BACKGROUND_BRIGHTNESS>
          Pass a custom brightness value. Clamped between 0..1
          
          [default: 0.2]

  -l, --limit <FRAME_SIZE_LIMIT>
          Value in KiB for the uncompressed size limit per frame
          
          [default: 550]

      --skip-dynamic-compression
          Skips the new dynamic compression for videos entirely and rely purely on the normal threshold

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

### Examples

Compiling a normal video:
```sh
# outputs video.bapple
asciic video.mp4
```

Compiling a coloured video:
```sh
# outputs video.bapple
asciic -c video.mp4
```

Compiling an image:
```sh
# outputs image.txt
asciic -i image.png
```

Compiling a coloured image:
```sh
asciic -c -i image.png
```
