use tree_sitter::Node;

use crate::{format_node, token::Token, EmptyLines, Error, Renderer};

/// Render a conjunctive or disjunctive list item, indenting the body of the
/// item by 1.
pub(super) fn format_list_item<'a, W>(
    def: Node<'a>,
    input: &'a str,
    empty_lines: &mut EmptyLines,
    writer: &mut Renderer<'a, W>,
) -> Result<(), Error>
where
    W: std::io::Write,
{
    empty_lines.maybe_insert(&def, writer)?;
    writer.push(Token::Newline)?;

    let mut c = def.walk();
    let iter = def.named_children(&mut c).peekable();
    for n in iter {
        match n.kind() {
            "bullet_conj" => {
                writer.push(Token::And)?;
                writer.indent_inc();
            }
            "bullet_disj" => {
                writer.push(Token::Or)?;
                writer.indent_inc();
            }
            _ => format_node(n, input, empty_lines, writer)?,
        }
    }

    writer.indent_dec();

    Ok(())
}
