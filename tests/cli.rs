use std::path::PathBuf;

use assert_cmd::Command;
use predicates::prelude::*;
use pretty_assertions::assert_eq;
use tempfile::TempDir;

/// Path to an unformatted file.
const BAD_PATH: &str = "libtlafmt/tests/corpus/differential_equations.tla";

/// Path to a formatted file.
///
/// NOTE: this file includes a YAML header that does not appear in the input
/// file.
const GOOD_PATH: &str = "libtlafmt/tests/snapshots/format__corpus@differential_equations.tla.snap";

fn cmd() -> Command {
    Command::cargo_bin("tlafmt").unwrap()
}

fn dir() -> TempDir {
    tempfile::Builder::new()
        .prefix(".tlafmt-test")
        .tempdir_in("./")
        .expect("cannot create tempdir for test state")
}

/// A helper to format `path` and return the stdout (formatted result).
fn format(path: &str) -> String {
    String::from_utf8(
        cmd()
            .arg(path)
            .assert()
            .success()
            .get_output()
            .stdout
            .clone(),
    )
    .unwrap()
}

/// This test asserts what is part of the CLI and the documentation for it.
///
/// As changes are made, this help text will need updating, which helps
/// highlight any changes to the public interface.
#[test]
fn test_help_text() {
    let stdout = String::from_utf8(
        cmd()
            .arg("--help")
            .assert()
            .success()
            .get_output()
            .stdout
            .clone(),
    )
    .unwrap();

    assert_eq!(
        "\
A formatter for TLA+ specs

Usage: tlafmt [OPTIONS] [FILE]

Arguments:
  [FILE]  Path to the TLA+ file to format

Options:
  -c, --check     Check the input file and exit with an error (code 3) if it needs formatting
  -i, --in-place  Overwrite the source file with the formatted output instead of printing it to stdout
      --stdin     Read the input file from stdin instead of the filesystem
  -h, --help      Print help
  -V, --version   Print version
",
    stdout
    );
}

/// Check mode behaviour for formatted and unformatted input files.
#[test]
fn test_check_mode() {
    // Failure case from test corpus
    cmd()
        .arg("--check")
        .arg(BAD_PATH)
        .assert()
        .failure()
        .stdout(predicate::eq(""))
        .stderr(predicate::eq("input file needs formatting\n"))
        .code(predicate::eq(3));

    // Success from corpus snapshot test output.
    cmd()
        .arg("--check")
        .arg(GOOD_PATH)
        .assert()
        .success()
        .stdout(predicate::eq(""))
        .stderr(predicate::eq(""))
        .code(predicate::eq(0));
}

#[test]
fn test_in_place() {
    let wd = dir();

    let mut file = PathBuf::from(wd.path());
    file.push("test.rs");

    std::fs::copy(BAD_PATH, &file).expect("cannot copy file for test");

    // Run the formatter with --in-place
    cmd()
        .arg("--in-place")
        .arg(file.to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::eq(""))
        .stderr(predicate::eq(""))
        .code(predicate::eq(0));

    // Run the formatter without --in-place to obtain the control output.
    let control = format(BAD_PATH);

    let got = std::fs::read_to_string(file).unwrap();

    // Confirm the file matches the formatted sample file.
    assert_eq!(control, got);
}

/// Reject --in-place with --check.
#[test]
fn test_in_place_conflicts_check() {
    let unformatted = std::fs::read_to_string(BAD_PATH).unwrap();

    // Run the formatter reading from --stdin.
    cmd()
        .arg("--in-place")
        .arg("--check")
        .arg(BAD_PATH)
        .write_stdin(unformatted)
        .assert()
        .failure()
        .stdout(predicate::eq(""))
        .stderr(predicate::eq(
            "\
error: the argument '--in-place' cannot be used with '--check'

Usage: tlafmt --in-place <FILE>

For more information, try '--help'.
",
        ))
        .code(predicate::eq(2));
}

/// Support reading from stdin, instead of using a file path.
#[test]
fn test_from_stdin() {
    let unformatted = std::fs::read_to_string(BAD_PATH).unwrap();
    let control = format(BAD_PATH);

    // Run the formatter reading from --stdin.
    cmd()
        .arg("--stdin")
        .write_stdin(unformatted.clone())
        .assert()
        .success()
        .stdout(predicate::eq(control.clone()))
        .stderr(predicate::eq(""))
        .code(predicate::eq(0));

    // Run a check reading from --stdin.
    cmd() // Unformatted
        .arg("--stdin")
        .arg("--check")
        .write_stdin(unformatted)
        .assert()
        .failure()
        .stdout(predicate::eq(""))
        .stderr(predicate::eq("input file needs formatting\n"))
        .code(predicate::eq(3));
    cmd() // Already formatted
        .arg("--stdin")
        .arg("--check")
        .write_stdin(control)
        .assert()
        .success()
        .stdout(predicate::eq(""))
        .stderr(predicate::eq(""))
        .code(predicate::eq(0));
}

/// Reject --stdin with a file path.
#[test]
fn test_from_stdin_conflicts_path() {
    let unformatted = std::fs::read_to_string(BAD_PATH).unwrap();

    // Run the formatter reading from --stdin.
    cmd()
        .arg("--stdin")
        .arg(BAD_PATH)
        .write_stdin(unformatted)
        .assert()
        .failure()
        .stdout(predicate::eq(""))
        .stderr(predicate::eq(
            "\
error: the argument '--stdin' cannot be used with '[FILE]'

Usage: tlafmt --stdin [FILE]

For more information, try '--help'.
",
        ))
        .code(predicate::eq(2));
}

/// Reject --stdin with --in-place.
#[test]
fn test_from_stdin_conflicts_in_place() {
    let unformatted = std::fs::read_to_string(BAD_PATH).unwrap();

    // Run the formatter reading from --stdin.
    cmd()
        .arg("--stdin")
        .arg("--in-place")
        .write_stdin(unformatted)
        .assert()
        .failure()
        .stdout(predicate::eq(""))
        .stderr(predicate::eq(
            "\
error: the argument '--stdin' cannot be used with '--in-place'

Usage: tlafmt --stdin [FILE]

For more information, try '--help'.
",
        ))
        .code(predicate::eq(2));
}
