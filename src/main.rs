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

    /// Check the input file and exit with an error (code 3) if it needs
    /// formatting.
    #[arg(long)]
    check: bool,
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
    let parsed = ParsedFile::new(input.as_str())?;

    if args.check {
        return check(&input, parsed);
    }

    parsed.format(std::io::stdout().lock())?;
    Ok(())
}

fn check(input: &str, parsed: ParsedFile<'_>) -> Result<(), Error> {
    // Allocate a buffer to render the normalised spec into, which will be
    // approximately the same length as the input text.
    let mut buf = Vec::with_capacity(input.len());

    parsed.format(&mut buf)?;

    if buf.trim_ascii() != input.trim_ascii().as_bytes() {
        eprintln!("input file needs formatting");
        std::process::exit(3);
    }

    Ok(())
}
