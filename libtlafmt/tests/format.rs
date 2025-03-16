//! Test formatting the embedded TLA+ corpus.

include!("../src/test_utils.rs"); // Pull in private assert_rewrite!

use thiserror as _;
use tree_sitter as _;
use tree_sitter_tlaplus as _;

use insta::glob;
use libtlafmt::ParsedFile;

/// Execute the formatter against each file in the TLA spec corpus, and compare
/// the output against a reference copy.
///
/// Additionally re-run the formatter against the output of the first run to
/// ensure stable output between repeated runs.
///
/// If this test fails, use "cargo insta review" to inspect any output changes.
#[test]
fn test_corpus() {
    use std::fs;

    glob!("../", "tests/corpus/*.tla", |path| {
        let input = fs::read_to_string(path).expect("read test corpus file");
        assert_rewrite!(&input);
    });
}
