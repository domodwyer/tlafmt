//   Copyright 2025 Dom Dwyer <dom@itsallbroken.com>
//
//   Licensed under the Apache License, Version 2.0 (the "License");
//   you may not use this file except in compliance with the License.
//   You may obtain a copy of the License at
//
//       http://www.apache.org/licenses/LICENSE-2.0
//
//   Unless required by applicable law or agreed to in writing, software
//   distributed under the License is distributed on an "AS IS" BASIS,
//   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//   See the License for the specific language governing permissions and
//   limitations under the License.

use std::{
    io::{BufWriter, IsTerminal, Write},
    path::PathBuf,
    string::FromUtf8Error,
};

use anstyle::Style;
use clap::{
    builder::styling::{AnsiColor, Color},
    Parser,
};
use libtlafmt::ParsedFile;
use thiserror::Error;

/// Formatter of TLA+ specs.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Path to the TLA+ file to format.
    #[arg(required_unless_present = "stdin", conflicts_with = "stdin")]
    file: Option<PathBuf>,

    /// Check the input file and print a diff of any changes that would be made.
    #[arg(short, long)]
    check: bool,

    /// Overwrite the source file with the formatted output instead of printing
    /// it to stdout.
    #[arg(short, long, conflicts_with = "check", conflicts_with = "stdin")]
    in_place: bool,

    /// Read the input file from stdin instead of the filesystem.
    #[arg(long)]
    stdin: bool,
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

    /// Flushing the formatted output through the buffered writer for
    /// --in-place.
    #[error("failed to flush formatted output: {0}")]
    FlushTempFile(std::io::Error),

    /// Persisting the formatted output for --in-place.
    #[error("failed to persist formatted output: {0}")]
    SaveTempFile(std::io::Error),

    /// A non-UTF8 string was generated (likely from non-UTF8 input).
    #[error("non-utf8 string found: {0}")]
    Utf8(#[from] FromUtf8Error),
}

fn main() -> Result<(), Error> {
    let args = Args::parse();

    let input = match args.file.as_ref() {
        Some(v) => std::fs::read_to_string(v),
        None => std::io::read_to_string(std::io::stdin().lock()),
    }
    .map_err(Error::ReadFile)?;

    let parsed = ParsedFile::new(input.as_str())?;

    if args.check {
        assert!(!args.in_place);
        return check(&input, parsed);
    }

    if args.in_place {
        assert!(!args.check);
        assert!(args.file.is_some()); // Not --stdin
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

    let mut buffered = BufWriter::new(&mut file);
    parsed.format(&mut buffered)?;

    buffered.flush().map_err(Error::FlushTempFile)?;
    drop(buffered);

    file.persist(args.file.unwrap())
        .map_err(|v| Error::SaveTempFile(v.error))?;

    Ok(())
}

fn check(input: &str, parsed: ParsedFile<'_>) -> Result<(), Error> {
    // Allocate a buffer to render the normalised spec into, which will be
    // approximately the same length as the input text.
    let mut buf = Vec::with_capacity(input.len());

    parsed.format(&mut buf)?;

    let input = input.trim_ascii();

    // If the strings match, return early.
    if buf.trim_ascii() == input.as_bytes() {
        return Ok(());
    }

    let buf = String::from_utf8(buf)?;
    let mut out = std::io::stderr().lock();

    // Define the styles used, or skip styling if used in a script.
    let style_none = Style::new();
    let (style_add, style_rem) = match out.is_terminal() {
        true => (
            Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green))),
            Style::new().fg_color(Some(Color::Ansi(AnsiColor::Red))),
        ),
        false => (style_none, style_none),
    };

    for diff in diff::lines(input, buf.trim_ascii()) {
        // Reset the colour of the next line.
        style_add
            .write_reset_to(&mut out)
            .expect("reset stderr colour");

        match diff {
            diff::Result::Left(l) => writeln!(&mut out, "{style_rem}- {}", l),
            diff::Result::Both(l, _) => writeln!(&mut out, "  {}", l),
            diff::Result::Right(r) => writeln!(&mut out, "{style_add}+ {}", r),
        }
        .expect("write to stderr")
    }

    std::process::exit(3);
}
