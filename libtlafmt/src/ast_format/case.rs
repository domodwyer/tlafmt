use tree_sitter::Node;

use crate::{format_node, token::Token, EmptyLines, Error, Renderer};

/// Format a CASE block for `def`, indenting the arms and ensuring each arm is
/// spaced by a single newline.
pub(super) fn format_case<'a, W>(
    def: Node<'a>,
    input: &'a str,
    empty_lines: &mut EmptyLines,
    writer: &mut Renderer<'a, W>,
) -> Result<(), Error>
where
    W: std::io::Write,
{
    let mut c = def.walk();
    let mut iter = def.named_children(&mut c).peekable();

    empty_lines.maybe_insert(&def, writer)?;
    writer.indent_inc();

    writer.push(Token::KeywordCase)?;

    // True when the nodes being visited are part of the initial CASE
    // <condition> and false once the first case_box is observed.
    let mut in_condition_expr = true;

    while let Some(n) = iter.next() {
        match n.kind() {
            "case_box" => {
                writer.push(Token::Newline)?;
                writer.push(Token::CaseBox)?;
            }
            "other_arm" | "case_arm" => {
                empty_lines.suppress(&n);
                if let Some(next) = iter.peek() {
                    empty_lines.suppress(next);
                }

                writer.indent_inc();
                format_node(n, input, empty_lines, writer)?;
                writer.indent_dec();

                if in_condition_expr {
                    writer.indent_inc();
                    in_condition_expr = false;
                }
            }
            _ => format_node(n, input, empty_lines, writer)?,
        }
    }

    // Once for the CASE, once for the arms.
    writer.indent_dec();
    writer.indent_dec();

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::assert_rewrite;

    #[test]
    fn test_typical() {
        assert_rewrite!(
            r#"
---- MODULE B ----
X == CASE self \in RM -> "RS"
            [] self = 0 -> "TS"
            [] self = 10 -> "BTS"
====
"#
        );
    }

    #[test]
    fn test_comments() {
        assert_rewrite!(
            r#"
---- MODULE B ----
X == CASE self \in RM -> "RS"
            \* Something
            [] self = 0 -> "TS"
            (* bananas *)
            [] self = 10 -> "BTS" \* platanos
            [] self = 10 -> "BTS"
====
"#
        );
    }

    #[test]
    fn test_multiline_pointer() {
        assert_rewrite!(
            r#"
---- MODULE B ----
X == CASE self \in RM -> "RS"
            [] self = 0 -> "TS"
            [] self = 10 ->
                 "BTS"
====
"#
        );
    }

    #[test]
    fn test_conj_arms() {
        assert_rewrite!(
            r#"
---- MODULE B ----
X == CASE self \in RM -> "RS"
            [] /\ self = 0
               /\ self = 0 -> "TS"
            [] self = 10 -> "BTS" /\ A = 42
====
"#
        );
    }

    #[test]
    fn test_keyword_line() {
        assert_rewrite!(
            r#"
---- MODULE B ----
X == CASE
self \in RM -> "RS"
        [] self = 0 -> "TS"
            [] self = 10 -> "BTS"
====
"#
        );
    }

    #[test]
    fn test_exploded() {
        assert_rewrite!(
            r#"
---- MODULE B ----
X ==
CASE
self \in RM -> "RS"
[] self = 0 -> "TS"
[] self = 10 -> "BTS"
====
"#
        );
    }

    #[test]
    fn test_box_arm_newlines() {
        assert_rewrite!(
            r#"
---- MODULE B ----
X ==
CASE
self \in RM -> "RS"
[]
        self = 0 -> "TS"

        []

self = 10 -> "BTS"
====
"#
        );
    }

    #[test]
    fn test_other() {
        assert_rewrite!(
            r#"
---- MODULE B ----
X == CASE \/ x = 0 \/ y = 0
          \/ x > N \/ y > N
          \/ ~grid[<<x, y>>] -> 0
        [] OTHER -> 1
====
"#
        );
    }

    /// Discovered by fuzzing, this test reproduces a CASE node that contains an
    /// ERROR child node.
    #[test]
    fn test_fuzz_error_node_kind() {
        assert_rewrite!(
            r#"
---- MODULE B ----
==CASE>d->0
====
"#
        );
    }
}
