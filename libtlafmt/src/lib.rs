mod ast_format;
mod helpers;
mod renderer;
mod test_utils;
mod token;

use std::io::Write;

use ast_format::format_node;
use helpers::EmptyLines;
use renderer::Renderer;
use thiserror::Error;
use tree_sitter::{Node, Parser, Tree};

const LINE_WIDTH: usize = 80;

/// Errors during AST parsing, lowering or rendering.
#[derive(Debug, Error)]
pub enum Error {
    /// An I/O error writing to the output sink.
    #[error("data store disconnected")]
    IO(#[from] std::io::Error),

    /// The input cannot be parsed into an AST.
    #[error("unknown parser error")]
    Parse,

    /// The module header is malformed.
    #[error("invalid module header")]
    ModuleHeader,

    /// A `[Next]_vars` sequence is malformed.
    #[error("invalid step-or-stutter sequence")]
    StepOrStutter,
}

/// A parsed TLA file.
///
/// Formatting a TLA file occurs in three phases:
///
///   1. The input file is parsed into an abstract syntax tree.
///   2. The AST is then lowered into a formatter-specific representation.
///   3. The format representation is rendered into output text.
///
/// Step (1) is performed when calling [`ParsedFile::new()`] to initialise a new
/// instance, and steps (2) and (3) are performed when [`ParsedFile::format()`]
/// is called.
#[derive(Debug)]
pub struct ParsedFile<'a> {
    t: Tree,
    input: &'a str,
}

impl<'a> ParsedFile<'a> {
    /// Parse the `input` TLA spec into an AST.
    pub fn new(input: &'a str) -> Result<Self, Error> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_tlaplus::LANGUAGE.into())
            .expect("error loading TLA+ grammar");

        Ok(Self {
            t: parser.parse(input, None).ok_or(Error::Parse)?,
            input,
        })
    }

    /// Format and render the parsed spec into `out`.
    ///
    /// # Errors
    ///
    /// If formatting fails `out` may contain partial content.
    pub fn format<W>(&self, out: W) -> Result<(), Error>
    where
        W: Write,
    {
        let mut out = Renderer::new(out);
        let mut empty_lines = EmptyLines::default();

        // Lower the AST into a series of formatter tokens wrote to `out`.
        format_node(self.t.root_node(), self.input, &mut empty_lines, &mut out)?;

        out.flush()?;

        Ok(())
    }
}

/// Return the content in `input` for `node.`
fn get_str<'a>(node: &Node<'_>, input: &'a str) -> &'a str {
    &input[node.byte_range()]
}

#[cfg(test)]
mod tests {
    use insta::glob;

    use super::*;

    /// Execute the formatter against each file in the TLA spec corpus, and
    /// compare the output against a reference copy.
    ///
    /// If this test fails, use "cargo insta review" to inspect any output
    /// changes.
    #[test]
    fn test_corpus() {
        use std::fs;

        glob!("../../", "tests/corpus/*.tla", |path| {
            let input = fs::read_to_string(path).expect("read test corpus file");
            assert_rewrite!(&input);
        });
    }
}
