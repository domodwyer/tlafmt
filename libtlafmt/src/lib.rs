#![doc = include_str!("../README.md")]

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

mod ast_format;
mod helpers;
mod renderer;
mod test_utils;
mod token;

use std::{borrow::Cow, io::Write};

use ast_format::format_node;
use helpers::{EmptyLines, INDENT_STR};
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

/// A parsed TLA file ready for formatting.
#[derive(Debug)]
pub struct ParsedFile<'a> {
    t: Tree,
    input: Cow<'a, str>,
}

impl<'a> ParsedFile<'a> {
    /// Parse the `input` TLA spec into an AST.
    pub fn new(input: &'a str) -> Result<Self, Error> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_tlaplus::LANGUAGE.into())
            .expect("error loading TLA+ grammar");

        // Normalise tab characters - if a mixture of tab and space are used,
        // the AST may produce incorrect nodes. Specifically conj_items using
        // tabs can become a bound_infix_op instead of conj_list, see
        // `test_mixed_tabs_spaces`.
        let input = match input.contains("\t") {
            true => Cow::Owned(input.replace("\t", INDENT_STR)),
            false => Cow::Borrowed(input),
        };

        Ok(Self {
            t: parser.parse(input.as_bytes(), None).ok_or(Error::Parse)?,
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
        format_node(self.t.root_node(), &self.input, &mut empty_lines, &mut out)?;

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

        glob!("../", "tests/corpus/*.tla", |path| {
            let input = fs::read_to_string(path).expect("read test corpus file");
            assert_rewrite!(&input);
        });
    }

    /// A test where the second conj item uses a tab to position the bullet.
    /// With tabs=4 these causes the bullets to align, but a conj_list is not
    /// emitted in the AST.
    ///
    /// ```text
    ///
    ///             ---- MODULE bananas ----
    ///             Op == /\ A = 1
    /// tab ->          /\ B = 2
    ///             ====
    ///
    /// ```
    ///
    /// Emits the following AST nodes:
    ///
    ///     operator_definition
    ///     name: identifier
    ///     def_eq
    ///     definition: bound_infix_op
    ///         lhs: conj_list
    ///         conj_item
    ///             bullet_conj
    ///             bound_infix_op
    ///             lhs: identifier_ref
    ///             symbol: eq
    ///             rhs: nat_number
    ///         symbol: land
    ///         rhs: bound_infix_op
    ///         lhs: identifier_ref
    ///         symbol: eq
    ///         rhs: nat_number
    ///
    /// Note the definition is for bounded_infix_op, with a list of 1 item (the
    /// lhs) with an op of "land" for the second item.
    ///
    /// ```text
    ///
    ///             ---- MODULE Bananas ------
    ///             Op == /\ A = 1
    ///                 /\ B = 2
    ///             =====
    ///
    /// ```
    ///
    /// Compared to the above, that uses only whitespace for alignment:
    ///
    ///     operator_definition
    ///     name: identifier
    ///     def_eq
    ///     definition: conj_list
    ///         conj_item
    ///         bullet_conj
    ///         bound_infix_op
    ///             lhs: identifier_ref
    ///             symbol: eq
    ///             rhs: nat_number
    ///         conj_item
    ///         bullet_conj
    ///         bound_infix_op
    ///             lhs: identifier_ref
    ///             symbol: eq
    ///             rhs: nat_number
    ///
    /// Where the definition node is correctly labelled as a "conj_list" kind,
    /// with two conj_item nodes.
    #[test]
    fn test_mixed_tabs_spaces() {
        assert_rewrite!(
            "\
---- MODULE Bananas ------
X == /\\ x = 4
\t /\\ y = 2
====="
        );
    }
}
