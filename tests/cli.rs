use assert_cmd::Command;
use predicates::prelude::*;

fn cmd() -> Command {
    Command::cargo_bin("tlafmt").unwrap()
}

/// This test asserts what is part of the CLI and the documentation for it.
///
/// As changes are made, this help text will need updating, which helps
/// highlight any changes to the public interface.
#[test]
fn test_help_text() {
    let stderr = String::from_utf8(
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
        stderr,
        "\
Formatter of TLA+ specs

Usage: tlafmt [OPTIONS] <FILE>

Arguments:
  <FILE>  Path to the TLA+ file to format

Options:
      --check    Check the input file and exit with an error (code 3) if it needs formatting
  -h, --help     Print help
  -V, --version  Print version
"
    );
}

/// Check mode behaviour for formatted and unformatted input files.
#[test]
fn test_check_mode() {
    // Failure case from test corpus
    cmd()
        .arg("--check")
        .arg("libtlafmt/tests/corpus/differential_equations.tla")
        .assert()
        .failure()
        .stdout(predicate::eq(""))
        .stderr(predicate::eq("input file needs formatting\n"))
        .code(predicate::eq(3));

    // Success from corpus snapshot test output.
    cmd()
        .arg("--check")
        .arg("libtlafmt/src/snapshots/libtlafmt__tests__corpus@differential_equations.tla.snap")
        .assert()
        .success()
        .stdout(predicate::eq(""))
        .stderr(predicate::eq(""))
        .code(predicate::eq(0));
}
