use std::{
    error::Error,
    fs::remove_dir_all,
    sync::{Arc, atomic::Ordering::Relaxed},
};

use clap::Parser;

use crate::{cli::Args, colours::YELLOW, primitives::AsciiCompiler};

type Res<T> = Result<T, Box<dyn Error>>;

mod cli;
mod colours;
mod primitives;
fn main() -> Res<()> {
    let program = Arc::new(AsciiCompiler::new(Args::parse())?);
    register_ctrl_c_handle(program)?;
    Ok(())
}

fn abort_cleanly(program: Arc<AsciiCompiler>) -> ! {
    cleanup(program);
    eprintln!("{YELLOW}Cleanup successful, now aborting...");
    todo!()
}

fn cleanup(program: Arc<AsciiCompiler>) {
    eprintln!("\n\n{YELLOW}Cleaning up...");
    let tmp_dir_path = program.temp_dir.path();
    remove_dir_all(tmp_dir_path).unwrap_or_else(|_| {
        panic!(
            "remove_dir_all() failed. Check for littering on {tmp_dir_path:?}"
        )
    });
}

fn register_ctrl_c_handle(program: Arc<AsciiCompiler>) -> Res<()> {
    ctrlc::set_handler(move || {
        program.stop_handle.store(true, Relaxed);
        abort_cleanly(program.clone());
    })?;
    Ok(())
}
