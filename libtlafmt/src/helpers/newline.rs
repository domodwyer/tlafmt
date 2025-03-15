//! Newline preservation and squashing helper.

use tree_sitter::Node;

use crate::{token::Token, Renderer};

#[derive(Debug, Default)]
pub(crate) struct EmptyLines(usize);

impl EmptyLines {
    /// Observe the position of this next `node` and emit empty lines if
    /// required.
    ///
    /// This call will preserve the (lack of) existing newlines, but squashes
    /// consecutive empty lines to at most 1.
    pub(crate) fn maybe_insert<W>(
        &mut self,
        node: &Node<'_>,
        out: &mut Renderer<'_, W>,
    ) -> Result<bool, std::io::Error>
    where
        W: std::io::Write,
    {
        // Calculate the number of lines between the last observed node, and
        // this one.
        let existing = node.start_position().row.saturating_sub(self.0);

        // Track the end position of this new node.
        self.0 = node.end_position().row;

        match existing {
            0 => return Ok(false),
            1 => out.push(Token::SourceNewline)?,
            _ => {
                // Squash to at most 1 empty line.
                out.push(Token::SourceNewline)?;
                out.push(Token::SourceNewline)?
            }
        }

        Ok(true)
    }

    /// Observe `node` and do not allow it or any lines prior to it to insert a
    /// linebreak if when observed later a node would have caused a line break
    /// to be inserted.
    pub(crate) fn suppress(&mut self, node: &Node<'_>) {
        self.0 = node.end_position().row;
    }
}
