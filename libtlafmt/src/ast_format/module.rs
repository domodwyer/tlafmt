use std::{io::Write, iter::Peekable};

use tree_sitter::Node;

use crate::{ast_format::format_node, get_str, helpers::EmptyLines, token::Token, Error, Renderer};

/// Format a TLA module.
pub(super) fn format_module<'a, 'b: 'a, W>(
    n: Node<'b>,
    input: &'a str,
    empty_lines: &mut EmptyLines,
    mut out: &mut Renderer<'a, W>,
) -> Result<(), Error>
where
    W: Write,
{
    assert_eq!(n.kind(), "module"); // Validated by caller.

    let mut c = n.walk();
    let mut iter = c.node().named_children(&mut c).peekable();

    while let Some(node) = iter.peek() {
        // Emit newlines, squashing repeated newlines to at most 1 empty line.
        empty_lines.maybe_insert(node, &mut out)?;

        match node.kind() {
            "header_line" => format_module_header(&mut iter, input, &mut out),
            _ => format_node(iter.next().unwrap(), input, empty_lines, out),
        }?;
    }

    out.flush()?;
    Ok(())
}

/// Consume the module header nodes from `iter`, printing a normalised module
/// header.
fn format_module_header<'a, W>(
    iter: &mut Peekable<impl ExactSizeIterator<Item = Node<'a>>>,
    input: &'a str,
    out: &mut Renderer<'a, W>,
) -> Result<(), Error>
where
    W: Write,
{
    // Greedily try and consume the header line, which is composed of 3 nodes:
    //
    //   * header_line
    //   * identifier
    //   * header_line
    //
    // All must be consumed from the iterator.
    let left = iter.next().unwrap();
    assert_eq!(left.kind(), "header_line"); // Validated by caller.

    let ident = match iter.next_if(|v| v.kind() == "identifier") {
        Some(v) => v,
        None => return Err(Error::ModuleHeader),
    };

    match iter.next_if(|v| v.kind() == "header_line") {
        Some(_) => {}
        None => return Err(Error::ModuleHeader),
    };

    let name = get_str(&ident, input).trim_ascii();

    out.push(Token::ModuleHeader(name))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::assert_rewrite;

    #[test]
    fn test_module_header_length_normalisation() {
        assert_rewrite!(
            "\
Some bananas
---- MODULE DifferentialEquations ------
LOCAL X == 42
====="
        );
    }

    #[test]
    fn test_module_header_whitespace_trim() {
        assert_rewrite!(
            "\
     ----    MODULE     DifferentialEquations   ------
====="
        );
    }

    /// Reduce overly long module lines to fit into the desired line width.
    #[test]
    fn test_module_header_length_trim() {
        assert_rewrite!(
            "\
---------------------------------- MODULE DifferentialEquations ----------------------------------
====="
        );
    }

    #[test]
    fn test_module_header_even_odd_length() {
        // Line is perfectly even.
        assert_rewrite!(
            "\
--------------------------------- MODULE A ---------------------------------
================================================================================"
        );
        // Text is odd and equal dashes would result in an excessively long line.
        assert_rewrite!(
            "\
--------------------------------- MODULE AB ---------------------------------
================================================================================"
        );
    }

    #[test]
    fn test_newline_squashing() {
        assert_rewrite!(
            "\
----------------------------------- MODULE A -----------------------------------
LOCAL C == 42
LOCAL A == 42



\\* Comment

LOCAL T == 13
(* Another *)




(* BANANAS *)
LOCAL S == 24


================================================================================"
        );
    }
}
