use std::fs::{read_dir, read_to_string};

use proc_macro::TokenStream;

/// # Panics
/// Panics if no directory or an invalid directory is specified
#[proc_macro]
pub fn link_frames(items: TokenStream) -> TokenStream {
    let frames_dir = items.into_iter().next().unwrap();

    let dir = read_dir(frames_dir.to_string().replace('"', "")).unwrap();
    let mut ret = String::from("&[");

    let mut entries = dir
        .filter_map(Result::ok)
        .filter(|e| e.file_name() != *"audio.mp3")
        .collect::<Vec<_>>();

    entries.sort_by_key(|k| {
        k.path()
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .parse::<u32>()
            .unwrap()
    });

    for entry in entries {
        ret.push_str(&format!("\"{}\",", read_to_string(entry.path()).unwrap()));
    }

    ret.push(']');
    ret.parse().unwrap()
}
