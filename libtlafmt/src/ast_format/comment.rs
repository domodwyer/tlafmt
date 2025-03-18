use tree_sitter::Node;

use crate::{
    get_str,
    helpers::Indent,
    token::{Position, Token},
    Error, Renderer,
};

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
    if def.start_position().column != 0 {
        // Special case indentation within an operator body to ensure a comment
        // appears indented if it is the first statement in the body.
        if def
            .parent()
            .is_some_and(|v| v.kind() == "operator_definition")
        {
            let orig = writer.indent_get();
            writer.indent_set(std::cmp::max(Indent::new(1), orig));
            writer.push(Token::Comment(get_str(&def, input), Position::from(&def)))?;
            writer.indent_set(orig);
        } else {
            writer.push(Token::Comment(get_str(&def, input), Position::from(&def)))?;
        }

        return Ok(());
    }

    let orig = writer.indent_get();

    writer.indent_set(Indent::new(0));
    let ret = writer.push(Token::Comment(get_str(&def, input), Position::from(&def)));
    writer.indent_set(orig);

    ret.map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use crate::assert_rewrite;

    #[test]
    fn test_comment_let_in_list() {
        assert_rewrite!(
            r"
---- MODULE Bananas ------
A ==
    LET B == 42
    IN /\ C = D

\* Where is this placed?
B == 24
=============================================================================
"
        );
    }

    #[test]
    fn test_comment_with_lists() {
        assert_rewrite!(
            r"
---- MODULE Bananas ------
DoStuff ==
    /\ A' = 1
    \* Comment in a list.
    /\ B' = 2

\* Some comment for the operator.
Another == 42
====="
        );
    }

    #[test]
    fn test_operator_long_form_inline_comment() {
        assert_rewrite!(
            "\
---- MODULE Bananas ------
DoStuff(
b,
a  , \\* platanos?
n,

A,		N     ,
a,
S (* are great
    dont
    you think*)
) == 42
====="
        );
    }

    #[test]
    fn test_block() {
        assert_rewrite!(
            r"
---- MODULE Bananas ------
X ==
    /\ A = 42
    (* There's a block comment here *)
    /\ B = A
====="
        );
    }
}
