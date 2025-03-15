mod comment;
mod indent;

use std::{
    io::Write,
    ops::{Add, Sub},
};

use comment::align_comments;
use indent::limit_indents;

use crate::{helpers::IndentDecorator, token::Token, LINE_WIDTH};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
struct Indent(u8);

impl Add<u8> for Indent {
    type Output = Self;

    fn add(self, rhs: u8) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl Sub for Indent {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

/// A renderer of [`Token`] instances, writing the resulting output to `W`.
pub(crate) struct Renderer<'a, W> {
    /// The current indentation depth.
    indent_depth: Indent,

    /// A decorator over `W` that emits indentation after every rendered
    /// newline.
    indent: IndentDecorator<W>,

    /// A buffer containing the lowered formatter [`Token`] to write to `W`.
    buf: Vec<(Token<'a>, Indent)>,

    /// True when the last token wrote to `ident` was a newline.
    last_token_was_newline: bool,
}

impl<'a, W> Renderer<'a, W>
where
    W: std::io::Write,
{
    /// Initialise a [`Renderer`] to write to `out`.
    pub(crate) fn new(out: W) -> Self {
        Self {
            indent_depth: Indent(0),
            indent: IndentDecorator::new(out),
            buf: Default::default(),
            last_token_was_newline: false,
        }
    }

    /// Read the current indentation depth.
    pub(crate) fn indent_get(&self) -> u8 {
        self.indent_depth.0
    }

    /// Increase the indentation depth.
    pub(crate) fn indent_inc(&mut self) {
        self.indent_depth.0 += 1;
    }

    /// Decrease the indentation depth.
    pub(crate) fn indent_set(&mut self, v: u8) {
        self.indent_depth.0 = v;
    }

    /// Decrement the indentation depth.
    ///
    /// # Panics
    ///
    /// Panics if the indentation depth is 0.
    pub(crate) fn indent_dec(&mut self) {
        debug_assert_ne!(self.indent_depth.0, 0);

        self.indent_depth.0 -= 1;
    }

    /// Add `t` to the render queue.
    pub(crate) fn push(&mut self, t: Token<'a>) -> Result<(), std::io::Error> {
        self.buf.push((t, self.indent_depth));

        Ok(())
    }

    /// Flush the queue of [`Token`], rendering them to the output sink.
    pub(crate) fn flush(mut self) -> Result<(), std::io::Error> {
        // Find consecutive lines that contain end-of-line comments that are
        // aligned vertically and rewrite them to preserve their alignment after
        // their respective lines are formatted.
        align_comments(&mut self.buf);

        // Rewrite indentation levels if necessary, to prevent blocks from being
        // excessively indented.
        limit_indents(&mut self.buf);

        let mut iter = self.buf.drain(..).peekable();

        while let Some((t, indent_depth)) = iter.next() {
            self.indent.set(indent_depth.0);

            // If this token cannot appear before the next token, skip rendering
            // this one.
            if let Some((next, _)) = iter.peek() {
                if !t.can_precede(next) {
                    continue;
                }
            }

            let s = match &t {
                Token::StepOrStutter(ident) => {
                    let s = format!("[{ident}]_");
                    debug_assert_eq!(s.len(), token_len(&t));

                    self.indent.write_all(s.as_bytes())?;
                    continue;
                }

                Token::Newline if self.last_token_was_newline => {
                    // Prevent the formatter from unintentionally inserting a
                    // forced newline in addition to a newline that was in the
                    // source spec.
                    continue;
                }

                Token::ModuleHeader(name) => {
                    let s = render_module_header(name);
                    debug_assert_eq!(s.len(), token_len(&t));

                    self.indent.write_all(s.as_bytes())?;

                    continue;
                }

                Token::LineDivider(c) => {
                    let s = std::iter::repeat(c).take(LINE_WIDTH).collect::<String>();
                    debug_assert_eq!(s.len(), token_len(&t));

                    self.indent.write_all(s.as_bytes())?;
                    continue;
                }

                Token::Comment(s, _) => {
                    self.last_token_was_newline = false;

                    // Indentation must be removed from lines that follow the
                    // first (only applicable in a block comment).
                    //
                    // If this does not happen, indentation will be added each
                    // time the document is formatted.

                    let mut comment_parts = s.split("\n");

                    // Write the first line with whatever indentation is set.
                    self.indent
                        .write_all(comment_parts.next().unwrap().as_bytes())?;

                    // Write the rest of the lines without indentation.
                    let orig = self.indent_depth.0;
                    self.indent.set(0);
                    for v in comment_parts {
                        self.indent.write_all(b"\n")?;
                        self.indent.write_all(v.as_bytes())?;
                    }
                    self.indent.set(orig);

                    // Optionally add a space delimiter.
                    if let Some(n) = iter.peek().map(|(v, _)| t.delimiting_space_len(v)) {
                        self.indent.write_all(&b" ".repeat(n))?;
                    }
                    continue;
                }
                Token::Raw(s) => s,
                Token::Prime => "'",
                Token::Always => "[]",
                Token::Eventually => "<>",
                Token::Newline | Token::SourceNewline => "\n",
                Token::KeywordChoose => "CHOOSE",
                Token::KeywordLet => "LET",
                Token::KeywordIn => "IN",
                Token::KeywordLocal => "LOCAL",
                Token::KeywordInstance => "INSTANCE",
                Token::KeywordDomain => "DOMAIN",
                Token::KeywordIf => "IF",
                Token::KeywordThen => "THEN",
                Token::KeywordElse => "ELSE",
                Token::KeywordExtends => "EXTENDS",
                Token::KeywordConstant => "CONSTANT",
                Token::KeywordConstants => "CONSTANTS",
                Token::KeywordVariable => "VARIABLE",
                Token::KeywordVariables => "VARIABLES",
                Token::KeywordExcept => "EXCEPT",
                Token::KeywordUnchanged => "UNCHANGED",
                Token::KeywordEnabled => "ENABLED",
                Token::KeywordSubset => "SUBSET",
                Token::MapTo => ":>",
                Token::MapsTo => "->",
                Token::AllMapsTo => "|->",
                Token::Compose => "@@",
                Token::Exists => r"\E",
                Token::All => r"\A",
                Token::SetIn => r"\in",
                Token::SetNotIn => r"\notin",
                Token::And => r"/\",
                Token::Or => r"\/",
                Token::Not => "~",
                Token::Ident(s) => s,
                Token::Lit(s) => s,
                Token::ParenOpen => "(",
                Token::ParenClose => ")",
                Token::SquareOpen => "[",
                Token::SquareClose => "]",
                Token::CurlyOpen => "{",
                Token::CurlyClose => "}",
                Token::AngleOpen => "<<",
                Token::AngleClose => ">>",
                Token::Comma => ",",
                Token::SemiColon => ":",
                Token::Bang => "!",
                Token::At => "@",
                Token::Eq => "=",
                Token::Eq2 => "==",
                Token::NotEq => "/=",
                Token::SubsetEq => r"\subseteq",
                Token::Dot => ".",
                Token::Dots2 => "..",
                Token::Plus => "+",
                Token::Minus => "-",
                Token::Multiply => "*",
                Token::AppendShort => r"\o",
                Token::Real => r"Real",
                Token::GreaterThan => ">",
                Token::GreaterThanEqual => ">=",
                Token::LessThan => "<",
                Token::LessThanEqual => "<=",
                Token::SetMinus => r"\",
                Token::Divide => r"/",
                Token::True => "TRUE",
                Token::False => "FALSE",
                Token::Union => r"\union",
                Token::Intersect => r"\intersect",
                Token::WeakFairness => "WF_",
                Token::StrongFairness => "SF_",
                Token::Implies => "=>",
                Token::KeywordTheorem => "THEOREM",
            };

            // Invariant: the rendered text must match the reported token
            // length.
            debug_assert!(s.len() == token_len(&t) || is_newline(&t), "{s:?}");

            // Write the rendered token.
            self.indent.write_all(s.as_bytes())?;

            // Insert a space if this node and the next node can be space
            // delimited.
            if let Some(n) = iter.peek().map(|(v, _)| t.delimiting_space_len(v)) {
                self.indent.write_all(&b" ".repeat(n))?;
            }

            self.last_token_was_newline = is_newline(&t);
        }

        Ok(())
    }
}

fn is_newline(t: &Token) -> bool {
    matches!(t, Token::Newline | Token::SourceNewline)
}

/// Render a module header line for `name`.
fn render_module_header(name: &&str) -> String {
    const MODULE: &str = " MODULE ";
    let line_len = LINE_WIDTH
        .checked_sub(name.len())
        .and_then(|v| v.checked_sub(MODULE.len() + 1))
        .and_then(|v| v.checked_div(2))
        .unwrap_or(1);

    // Sometimes the module dashes can't be exactly equal and fill up the entire
    // line because the "MODULE" and name have an odd length.
    //
    // When this happens, pad the right-side line with an extra dash.
    let mut right_extra = 0;
    if (line_len * 2) + MODULE.len() + 1 + name.len() == LINE_WIDTH - 1 {
        right_extra = 1;
    }

    format!(
        "{}{MODULE}{name} {}",
        "-".repeat(line_len),
        "-".repeat(line_len + right_extra)
    )
}

/// Return the size of a rendered [`Token`], exclusive of whitespace delimiters.
fn token_len(t: &Token<'_>) -> usize {
    match t {
        Token::Raw(s) => s.len(),
        Token::ModuleHeader(name) => render_module_header(name).len(),
        Token::Comment(s, _) => s.len(),
        Token::Newline | Token::SourceNewline => 0,
        Token::KeywordChoose => 6,
        Token::KeywordLet => 3,
        Token::KeywordIn => 2,
        Token::KeywordUnchanged => 9,
        Token::KeywordExtends => 7,
        Token::KeywordConstant => 8,
        Token::KeywordConstants => 9,
        Token::KeywordVariable => 8,
        Token::KeywordVariables => 9,
        Token::KeywordExcept => 6,
        Token::KeywordEnabled => 7,
        Token::KeywordTheorem => 7,
        Token::KeywordLocal => 5,
        Token::KeywordInstance => 8,
        Token::KeywordDomain => 6,
        Token::KeywordSubset => 6,
        Token::KeywordIf => 2,
        Token::KeywordThen => 4,
        Token::KeywordElse => 4,
        Token::Exists => 2,
        Token::All => 2,
        Token::SetIn => 3,
        Token::SetNotIn => 6,
        Token::And => 2,
        Token::Or => 2,
        Token::MapsTo => 2,
        Token::MapTo => 2,
        Token::AllMapsTo => 3,
        Token::Ident(s) => s.len(),
        Token::Lit(s) => s.len(),
        Token::ParenOpen => 1,
        Token::ParenClose => 1,
        Token::Comma => 1,
        Token::SemiColon => 1,
        Token::Plus => 1,
        Token::Minus => 1,
        Token::Multiply => 1,
        Token::Eq => 1,
        Token::Eq2 => 2,
        Token::NotEq => 2,
        Token::SubsetEq => 9,
        Token::Dot => 1,
        Token::Dots2 => 2,
        Token::At => 1,
        Token::SquareOpen => 1,
        Token::SquareClose => 1,
        Token::CurlyOpen => 1,
        Token::CurlyClose => 1,
        Token::AngleOpen => 2,
        Token::AngleClose => 2,
        Token::AppendShort => 2,
        Token::Real => 4,
        Token::GreaterThan => 1,
        Token::GreaterThanEqual => 2,
        Token::LessThan => 1,
        Token::LessThanEqual => 2,
        Token::Not => 1,
        Token::SetMinus => 1,
        Token::Divide => 1,
        Token::LineDivider(_) => LINE_WIDTH,
        Token::Prime => 1,
        Token::Always => 2,
        Token::Eventually => 2,
        Token::Implies => 2,
        Token::Bang => 1,
        Token::True => 4,
        Token::False => 5,
        Token::WeakFairness => 3,
        Token::StrongFairness => 3,
        Token::Union => 6,
        Token::Intersect => 10,
        Token::Compose => 2,
        Token::StepOrStutter(s) => s.len() + 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn format<'a>(tokens: impl IntoIterator<Item = Token<'a>>) -> String {
        let mut buf = Vec::new();
        let mut w = Renderer::new(&mut buf);

        for t in tokens {
            w.push(t).unwrap();
        }

        w.flush().unwrap();

        String::from_utf8(buf).expect("valid utf8 output")
    }

    #[test]
    fn test_write_line() {
        let output = format([Token::Raw("testing")]);
        assert_eq!(output, "testing");
    }

    #[test]
    fn test_liveness() {
        let output = format([
            Token::Eq2,
            Token::Eventually,
            Token::Always,
            Token::ParenOpen,
            Token::Ident("bananas"),
            Token::ParenClose,
        ]);
        assert_eq!(output, "== <>[](bananas)");
    }

    #[test]
    fn test_spec_next() {
        let output = format([
            Token::And,
            Token::Always,
            Token::StepOrStutter("Next"),
            Token::Ident("vars"),
        ]);
        assert_eq!(output, r"/\ [][Next]_vars");
    }

    #[test]
    fn test_range() {
        let output = format([Token::Ident("n"), Token::Dots2, Token::Ident("m")]);
        assert_eq!(output, "n..m");

        let output = format([
            Token::Ident("n"),
            Token::Dots2,
            Token::ParenOpen,
            Token::Ident("m"),
            Token::Plus,
            Token::Lit("1"),
            Token::ParenClose,
        ]);
        assert_eq!(output, "n..(m + 1)");
    }

    #[test]
    fn test_op_params() {
        let output = format([
            Token::Ident("bananas"),
            Token::ParenOpen,
            Token::Ident("A"),
            Token::Comma,
            Token::Ident("B"),
            Token::Comma,
            Token::Ident("C"),
            Token::Comma,
            Token::ParenClose,
        ]);
        assert_eq!(output, "bananas(A, B, C)");
    }

    #[test]
    fn test_fn_application() {
        let output = format([
            Token::Ident("bananas"),
            Token::SquareOpen,
            Token::Ident("platanos"),
            Token::SquareClose,
            Token::SquareOpen,
            Token::Lit("42"),
            Token::SquareClose,
        ]);
        assert_eq!(output, "bananas[platanos][42]");
    }

    #[test]
    fn test_record_fields() {
        let output = format([
            Token::Ident("bananas"),
            Token::Dot,
            Token::Ident("platanos"),
        ]);
        assert_eq!(output, "bananas.platanos");
    }

    #[test]
    fn test_except_bang() {
        let output = format([
            Token::KeywordExcept,
            Token::Bang,
            Token::Dot,
            Token::Ident("bananas"),
        ]);
        assert_eq!(output, "EXCEPT !.bananas");

        let output = format([
            Token::KeywordExcept,
            Token::Bang,
            Token::SquareOpen,
            Token::Ident("bananas"),
            Token::SquareClose,
        ]);
        assert_eq!(output, "EXCEPT ![bananas]");
    }

    #[test]
    fn test_gt_lt_parens() {
        let input = [
            (Token::GreaterThan, ">"),
            (Token::GreaterThanEqual, ">="),
            (Token::LessThan, "<"),
            (Token::LessThanEqual, "<="),
        ];

        for (token, symbol) in input {
            let output = format([
                Token::Ident("bananas"),
                token,
                Token::ParenOpen,
                Token::Ident("n"),
                Token::Plus,
                Token::Lit("1"),
                Token::ParenClose,
            ]);
            assert_eq!(output, format!("bananas {symbol} (n + 1)"));
        }
    }

    #[test]
    fn test_not() {
        let output = format([Token::Not, Token::Ident("bananas")]);
        assert_eq!(output, "~bananas");
    }

    #[test]
    fn test_fairness() {
        let output = format([Token::WeakFairness, Token::Ident("vars")]);
        assert_eq!(output, "WF_vars");

        let output = format([Token::StrongFairness, Token::Ident("vars")]);
        assert_eq!(output, "SF_vars");
    }

    #[test]
    fn test_stutter() {
        let output = format([Token::StepOrStutter("Next"), Token::Ident("vars")]);
        assert_eq!(output, "[Next]_vars");
    }

    #[test]
    fn test_bounded_quantification() {
        let output: String = format([
            Token::Exists,
            Token::Ident("t"),
            Token::SetIn,
            Token::Ident("vars"),
            Token::SemiColon,
            Token::Ident("bananas"),
        ]);
        assert_eq!(output, r"\E t \in vars: bananas");
    }

    #[test]
    fn test_prime_var() {
        let output: String = format([
            Token::Ident("bananas"),
            Token::Prime,
            Token::Eq,
            Token::Lit("42"),
        ]);
        assert_eq!(output, "bananas' = 42");
    }

    #[test]
    fn test_eq_paren() {
        let parens = [
            (Token::ParenOpen, "("),
            (Token::SquareOpen, "["),
            (Token::AngleOpen, "<< "), // Extra space
        ];

        let eq = [(Token::Eq, "="), (Token::NotEq, "/="), (Token::Eq2, "==")];

        for (paren, p_symbol) in parens {
            for (eq, e_symbol) in &eq {
                let output: String = format([
                    eq.clone(),
                    paren.clone(),
                    Token::KeywordEnabled,
                    Token::Ident("Bananas"),
                ]);

                assert_eq!(output, format!("{e_symbol} {p_symbol}ENABLED Bananas"));
            }
        }
    }

    #[test]
    fn test_record_paren() {
        let parens = [
            (Token::ParenClose, ")"),
            (Token::SquareClose, "]"),
            (Token::AngleClose, ">>"), // Extra space
        ];

        for (paren, p_symbol) in parens {
            let output: String = format([paren.clone(), Token::Dot, Token::Ident("bananas")]);
            assert_eq!(output, format!("{p_symbol}.bananas"));
        }
    }

    #[test]
    fn test_compose_paren() {
        let output: String = format([
            Token::Ident("bananas"),
            Token::Compose,
            Token::ParenOpen,
            Token::Ident("x"),
            Token::MapTo,
            Token::Lit("42"),
            Token::ParenClose,
        ]);
        assert_eq!(output, "bananas @@ (x :> 42)");
    }

    /// This test covers the unhandled tokens in the AST that are emitted as-is.
    /// Spacing is a best guess in this case, as the original may or may not
    /// have contained spacing between or after the raw token.
    #[test]
    fn test_raw_tokens() {
        let output: String = format([Token::Ident("bananas"), Token::Raw("!!!"), Token::Lit("42")]);
        assert_eq!(output, "bananas !!! 42");
    }

    /// An unhandled node that follows a newline should not be space delimited -
    /// it leads to an unexpected space before the content of a line.
    #[test]
    fn test_raw_tokens_with_newlines() {
        let output: String = format([Token::Newline, Token::Raw("!!!"), Token::Lit("42")]);
        assert_eq!(output, "\n!!! 42");

        let output: String = format([Token::SourceNewline, Token::Raw("!!!"), Token::Lit("42")]);
        assert_eq!(output, "\n!!! 42");

        let output: String = format([Token::Raw("!!!"), Token::Newline]);
        assert_eq!(output, "!!!\n");

        let output: String = format([Token::Raw("!!!"), Token::SourceNewline]);
        assert_eq!(output, "!!!\n");

        let output: String = format([Token::Raw("!!!\n"), Token::Ident("bananas")]);
        assert_eq!(output, "!!!\nbananas");

        let output: String = format([Token::Raw("!!!\n"), Token::Ident("bananas")]);
        assert_eq!(output, "!!!\nbananas");
    }

    /// An unhandled node that follows a newline should not be space delimited -
    /// it leads to an unexpected space before the content of a line.
    #[test]
    fn test_newline_line_divider() {
        let output: String = format([Token::Raw("!!!"), Token::Newline, Token::LineDivider('-')]);
        assert_eq!(
            output,
            "!!!\n--------------------------------------------------------------------------------"
        );

        let output: String = format([Token::Raw("!!!"), Token::Newline, Token::LineDivider('=')]);
        assert_eq!(
            output,
            "!!!\n================================================================================"
        );
    }
}
