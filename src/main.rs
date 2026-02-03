#![deny(clippy::disallowed_methods, clippy::disallowed_types)]

use clap::Parser;
use color_eyre::eyre::Result;
use log::LevelFilter;
use simplelog::{ColorChoice, CombinedLogger, Config, TermLogger, TerminalMode};
use std::path::PathBuf;
use std::process::exit;

#[derive(Parser, Debug)]
#[command(name = "one2html")]
pub(crate) struct Opt {
    /// Input files (`.one` or `.onetoc2` files)
    #[arg(short, long, required = true, value_name = "FILE", num_args = 1..)]
    pub(crate) input: Vec<PathBuf>,

    /// Output directory
    #[arg(short, long, value_name = "DIR")]
    pub(crate) output: PathBuf,
}

#[cfg(feature = "backtrace")]
fn main() {
    if let Err(e) = _main() {
        eprintln!("{:?}", e);

        if let Some(bt) = e
            .downcast_ref::<onenote_parser::errors::Error>()
            .and_then(std::error::Error::source)
        {
            eprintln!();
            eprintln!("Caused by:");
            eprintln!("{}", bt)
        }

        exit(1);
    }
}

#[cfg(not(feature = "backtrace"))]
fn main() {
    if let Err(e) = _main() {
        eprintln!("{:?}", e);

        exit(1);
    }
}

#[allow(clippy::disallowed_methods)]
fn _main() -> Result<()> {
    CombinedLogger::init(vec![TermLogger::new(
        LevelFilter::Warn,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )])?;

    let opt: Opt = Opt::parse();

    color_eyre::install()?;

    let output_dir = opt.output;
    assert!(!output_dir.is_file());

    for path in opt.input {
        one2html::convert(&path, &output_dir, onenote_parser::fs::NativeFs {})?;
    }

    Ok(())
}
