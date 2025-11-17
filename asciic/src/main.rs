use std::{
    error::Error,
    fs::remove_dir_all,
    process::exit,
    sync::{Arc, atomic::Ordering::Relaxed},
};

use clap::Parser;

use crate::{
    cli::Args,
    colours::{RED, YELLOW},
    primitives::AsciiCompiler,
};

type Res<T> = Result<T, Box<dyn Error + Send + Sync>>;

mod children;
mod cli;
mod colours;
mod installer;
mod primitives;
fn main() -> Res<()> {
    let ascii_compiler = Arc::new(AsciiCompiler::new(Args::parse())?);

    register_ctrl_c_handle(ascii_compiler.clone())?;

    ascii_compiler.compile()?;

    cleanup(ascii_compiler);
    Ok(())
}

fn abort_cleanly(ascii_compiler: Arc<AsciiCompiler>) -> ! {
    cleanup(ascii_compiler);
    eprintln!("{YELLOW}Cleanup successful, now aborting...");
    exit(1);
}

fn cleanup(ascii_compiler: Arc<AsciiCompiler>) {
    eprintln!("\n{YELLOW}Cleaning up...");
    let tmp_dir_path = ascii_compiler.temp_dir.path();

    // Manual cleanup, because we can't move temp_dir.
    remove_dir_all(tmp_dir_path).unwrap_or_else(|_| {
        panic!(
            "{RED}remove_dir_all() failed. Check for littering on {tmp_dir_path:?}"
        )
    });
}

fn register_ctrl_c_handle(ascii_compiler: Arc<AsciiCompiler>) -> Res<()> {
    ctrlc::set_handler(move || {
        ascii_compiler.stop_handle.store(true, Relaxed);
        abort_cleanly(ascii_compiler.clone());
    })?;
    Ok(())
}
