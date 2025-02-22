use std::path::PathBuf;

use clap::Parser;
use libtlafmt::ParsedFile;
use thiserror::Error;

/// Formatter of TLA+ specs.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Path to the TLA+ file to format.
    #[arg()]
    file: PathBuf,
}

#[derive(Debug, Error)]
enum Error {
    #[error("failed to read input file: {0}")]
    ReadFile(std::io::Error),

    #[error("formatting error: {0}")]
    Format(#[from] libtlafmt::Error),
}

fn main() -> Result<(), Error> {
    let args = Args::parse();
    let input = std::fs::read_to_string(args.file).map_err(Error::ReadFile)?;

    ParsedFile::new(input.as_str())
        .unwrap()
        .format(std::io::stdout().lock())
        .expect("format error");

    Ok(())
}
