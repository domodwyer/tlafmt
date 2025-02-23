use tree_sitter::Node;

use crate::{get_str, token::Token, Error, Renderer};

/// Emit a [`Token::Comment`] for `def`, after processing to adjust a comment
/// that may be attributed to an indented list that is actually adjacent to the
/// next AST node.
pub(super) fn format_comment<'a, W>(
    def: Node<'_>,
    input: &'a str,
    writer: &mut Renderer<'a, W>,
) -> Result<(), Error>
where
    W: std::io::Write,
{
    // Block comments should be rendered "as-is" without additional indentation.
    //
    // If a comment was not indented, it should be rendered without
    // formatter-added indentation below this branch.
    if def.kind() != "block_comment" && def.start_position().column != 0 {
        writer.push(Token::Comment(get_str(&def, input)))?;
        return Ok(());
    }

    let orig = writer.indent_get();

    writer.indent_set(0);
    let ret = writer.push(Token::Comment(get_str(&def, input)));
    writer.indent_set(orig);

    ret.map_err(Into::into)
}
