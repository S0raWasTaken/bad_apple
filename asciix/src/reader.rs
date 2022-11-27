use std::{ffi::OsString, fs::File, io::Read, process::exit};

use tar::{Archive, Entry};
use zstd::decode_all;

use crate::{bidirectional_channel::BiChannel, BoxResult};

macro_rules! closure_error {
    ($x:expr) => {
        match $x {
            Ok(res) => res,
            Err(e) => {
                eprintln!("{e:#?}");
                exit(7);
            }
        }
    };
}

pub fn manage_buffer(
    signal_recv: &BiChannel<Vec<u8>, bool>,
    tar_file: File,
    mut frame: Vec<u8>,
) -> BoxResult<()> {
    // Spawn a new thread to receive ticks from the receiver and update the index
    let mut archive = Archive::new(tar_file);
    let mut files = archive
        .entries()?
        .map(|e| closure_error!(e))
        .map(|mut e| {
            let file_stem = get_file_stem(&e).unwrap();

            let mut content = Vec::new();
            closure_error!(e.read_to_end(&mut content));

            if file_stem == *"audio" {
                return (0, content);
            }

            let file_number = closure_error!(file_stem.to_str().unwrap().parse::<usize>());

            (file_number, content)
        })
        .collect::<Vec<_>>();

    drop(archive);

    files.sort_by_key(|e| e.0);

    // Now wait for `next_frame` calls
    for (x, entry) in files {
        if x == 0 {
            signal_recv.recv()?; // First entry is audio
            signal_recv.send(entry)?;
            continue;
        }

        let content = decode_all(entry.as_slice())?;

        if signal_recv.recv()? {
            signal_recv.send(frame.clone())?;
            frame = content;
        } else {
            frame = content;
        }
    }

    // Display last frame
    if signal_recv.recv()? {
        signal_recv.send(frame)?;
    }

    Ok(())
}

#[inline]
pub fn next_frame(bi_channel: &BiChannel<bool, Vec<u8>>) -> Option<Vec<u8>> {
    bi_channel.send_recv(true)
}

#[inline]
fn get_file_stem(e: &'_ Entry<File>) -> Option<OsString> {
    Some(e.header().path().ok()?.file_stem()?.to_os_string())
}
