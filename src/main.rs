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

    /// Overwrite the source file with the formatted output instead of printing
    /// it to stdout.
    #[arg(short, long, conflicts_with = "check")]
    in_place: bool,
}

#[derive(Debug, Error)]
enum Error {
    #[error("failed to read input file: {0}")]
    ReadFile(std::io::Error),

    #[error("formatting error: {0}")]
    Format(#[from] libtlafmt::Error),

    /// Creating a temporary file for --in-place output.
    #[error("failed to create temporary file in current dir: {0}")]
    CreateTempFile(std::io::Error),

    /// Persisting the formatted output for --in-place.
    #[error("failed to persist formatted output: {0}")]
    SaveTempFile(std::io::Error),
}

fn main() -> Result<(), Error> {
    let args = Args::parse();

    let input = std::fs::read_to_string(&args.file).map_err(Error::ReadFile)?;
    let parsed = ParsedFile::new(input.as_str())?;

    if args.check {
        assert!(!args.in_place);
        return check(&input, parsed);
    }

    if args.in_place {
        assert!(!args.check);
        return in_place(args, &parsed);
    }

    parsed.format(std::io::stdout().lock())?;
    Ok(())
}

fn in_place(args: Args, parsed: &ParsedFile<'_>) -> Result<(), Error> {
    // For in-place output, first render to a temporary file and then move it to
    // the input path (somewhat) atomically to prevent a ctrl+c or crash during
    // execution from causing the input file to be only half populated.
    let mut file = tempfile::Builder::new()
        .prefix(".tlafmt")
        // 6 bytes of randomness here
        .suffix(".rs")
        // Tempfiles across filesystems can be problematic, so use ./
        .tempfile_in("./")
        .map_err(Error::CreateTempFile)?;

    parsed.format(&mut file)?;

    file.persist(args.file)
        .map_err(|v| Error::SaveTempFile(v.error))?;

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
