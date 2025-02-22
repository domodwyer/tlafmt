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
    let orig = writer.indent_get();

    // Block comments should be rendered "as-is" without additional indentation.
    if def.kind() == "block_comment" {
        writer.indent_set(0);
        writer.push(Token::Comment(get_str(&def, input)))?;
        writer.indent_set(orig);
        return Ok(());
    }

    // A fix for comments adjacent to the next node appearing at the tail of a
    // list node.
    let in_list = def
        .parent()
        .is_some_and(|v| v.kind() == "disj_list" || v.kind() == "conj_list");

    // Rewriting only occurs for disjunction / conjunction lists.
    if !in_list {
        writer.push(Token::Comment(get_str(&def, input)))?;
        return Ok(());
    }

    // And if the comment appears at the end of the list.
    if !is_comments_next(def) {
        writer.push(Token::Comment(get_str(&def, input)))?;
        return Ok(());
    }

    writer.indent_set(0);

    let ret = writer.push(Token::Comment(get_str(&def, input)));

    writer.indent_set(orig);

    ret.map_err(Into::into)
}

/// Return `true` if the next sibling nodes of `ptr` are exclusively comments,
/// or no siblings exist.
fn is_comments_next(mut ptr: Node<'_>) -> bool {
    while let Some(n) = ptr.next_sibling() {
        if n.kind() != "comment" {
            return false;
        }

        ptr = n;
    }

    true
}
