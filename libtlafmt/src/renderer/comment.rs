use std::cmp::max;

use crate::{
    helpers::INDENT_STR,
    renderer::token_len,
    token::{Position, Token},
};

use super::{is_newline, Indent};

/// Scan `buf`, searching for comments that appear vertically aligned (same
/// column index) in the input source file and compute the appropriate amount of
/// padding for adjacent comments to either:
///
///   * Maintain the existing vertical position in source
///   * Increase the column index of all adjacent comments to maintain alignment
///
/// The [`Token::Comment`] that are aligned have their [`Position`] updated to
/// specify relative padding used during rendering in order to maintain the
/// above.
pub(super) fn align_comments(buf: &mut Vec<(Token<'_>, Indent)>) {
    let mut candidates = vec![];

    // Look for end-of-line comments in consecutive rows.
    let mut i = 0;
    while i < buf.len() {
        // If this token is a comment, extract the source position and indent
        // level for it.
        let (pos, indent) = match buf[i] {
            (Token::Comment(_, v), indent) => (v, indent),
            _ => {
                i += 1;
                continue;
            }
        };

        // Check this comment against the last candidate (if any).
        //
        // If this comment:
        //
        //   * Appears at the end of a line of text
        //   * The previous line also has an end-of-line comment
        //   * These two comments have the same column index
        //
        // Then they become realignment candidates, and their vertical alignment
        // will be preserved after formatting.

        // If the line delta between this comment and the last candidate is >1,
        // OR this comment is not vertically aligned with the previous, then
        // process the aligned candidate batch before continuing.
        if candidates
            .last()
            .map(|(_idx, pos, _)| pos)
            .is_some_and(|v: &crate::token::Position| {
                (v.unwrap_row() + 1) != pos.unwrap_row() || v.unwrap_col() != pos.unwrap_col()
            })
        {
            process_candidates(buf, &mut candidates);
            candidates.truncate(0);
        }

        // Only end-of-line comments (those followed by newlines) are valid for
        // adjacency alignment.
        if buf.get(i + 1).is_some_and(|(v, _)| is_newline(v)) {
            candidates.push((i, pos, indent));
        }

        i += 1;
    }

    process_candidates(buf, &mut candidates);
}

/// Process a set of comments that are vertically aligned in the source and
/// appear in `buf` to set the appropriate amount of padding on their comments
/// in order to maintain vertical alignment after their lines are formatted.
fn process_candidates(
    buf: &mut [(Token<'_>, Indent)],
    candidates: &mut [(usize, Position, Indent)],
) {
    if candidates.len() < 2 {
        return;
    }

    debug_assert!(buf.len() >= candidates.len());

    // Walk backwards from the first candidate index to find the start of the
    // line for the first candidate.
    let start = candidates[0].0;
    debug_assert!(matches!(buf[candidates[0].0].0, Token::Comment(..)));

    let start = (0..start).rev().find(|&i| is_newline(&buf[i].0));
    let start = match start {
        Some(v) => v,
        None => {
            // This can occur if the start of the document is an ERROR node,
            // followed by a comment on two lines.
            //
            // In this case, avoid trying to align an unparsed spec.
            return;
        }
    };

    // Define the exclusive upper bound token index - the last comment (which is
    // guaranteed to terminate the line it is on).
    let end = candidates
        .last()
        .expect("must have candidate for comment alignment")
        .0;
    debug_assert!(matches!(buf[end].0, Token::Comment(..)));
    debug_assert!(is_newline(&buf[end + 1].0));

    // Compute the post-formatting line lengths of each candidate line.
    let lines = Vec::with_capacity(candidates.len());
    let mut max_line = 0; // Maximum observed line length.

    let iter = buf[start..=end].iter();
    let lines = line_len(iter).fold(lines, |mut acc, v| {
        max_line = max(max_line, v);
        acc.push(v);
        acc
    });

    // Invariant: there is exactly one line length computed per candidate.
    debug_assert_eq!(lines.len(), candidates.len());

    // The new column index is the maximum of:
    //
    //   * all line lengths + 1 space.
    //   * the existing column index as appears in the input source code.
    //
    // This causes the location of the comment to match the pre-formatted output
    // UNLESS a now-formatted line pushes past the previous position, in which
    // case all aligned comments are pushed further back with it.
    let new_col = max(max_line + 1, candidates[0].1.unwrap_col());

    for (candidate_idx, (buf_idx, ..)) in candidates.iter().enumerate() {
        match &mut buf[*buf_idx] {
            (Token::Comment(_, pos), _) => {
                *pos = Position::Relative(new_col - lines[candidate_idx])
            }
            _ => unreachable!(),
        }
    }
}

// Consume one line from newline to line-ending comment from `iter` and return
// the line length up to, but not including the comment or its preceding space.
fn line_len<'a, T>(iter: T) -> impl Iterator<Item = usize>
where
    T: Iterator<Item = &'a (Token<'a>, Indent)>,
{
    let mut iter = iter.peekable();

    // Invariant: only called with a token iter that will yield a newline as the
    // indentation value is set by the first token after the newline.
    debug_assert!(iter.peek().is_some_and(|v| is_newline(&v.0)));

    let mut len = 0;
    std::iter::from_fn(move || {
        loop {
            let (t, _) = iter.next()?;

            // If a newline is observed, clear the accumulated line length and set
            // the indentation level from the first token on the next line (only the
            // first can set the line indentation).
            if is_newline(t) {
                len = iter.peek().unwrap().1 .0 as usize * INDENT_STR.len();
                continue;
            }

            // If this is the end of line comment, the length has been computed.
            if matches!(t, Token::Comment(..)) && iter.peek().is_some_and(|v| is_newline(&v.0))
                || iter.peek().is_none()
            {
                return Some(len.saturating_sub(1)); // -1 because spaces are inserted before comments or it was a newline.
            }

            // Apply the same filtering as when rendering occurs.
            let next = iter.peek();
            if let Some((next, _)) = next {
                if !t.can_precede(next) {
                    continue;
                }
            }

            len += token_len(t);

            // Account for any whitespace.
            if let Some(n) = next.map(|(v, _)| t.delimiting_space_len(v)) {
                len += n;
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use crate::assert_rewrite;

    use super::*;

    #[test]
    fn test_line_len_1() {
        let tokens = [
            (Token::Newline, Indent(255)),
            // The first token, which sets the indent level
            (Token::And, Indent(1)), // 4 + 2 + space
            // The rest, which do not affect indent.
            (Token::Ident("bananas"), Indent(255)), // 7 + space
            (Token::Eq, Indent(255)),               // 1 + space
            (Token::Lit("42"), Indent(255)),        // 2
        ]
        .iter()
        .peekable();

        let got = line_len(tokens).next().unwrap();
        assert_eq!(got, 16);
    }
    #[test]
    fn test_line_len_2() {
        let tokens = [
            (Token::SourceNewline, Indent(255)),
            // Immediately followed by second newline
            (Token::Newline, Indent(255)),
            // The first token, which sets the indent level
            (Token::And, Indent(1)), // 4 + 2 + space
            // The rest, which do not affect indent.
            (Token::Ident("platanos"), Indent(255)), // 8
            (Token::Prime, Indent(255)),             // 1 + space
            (Token::Eq, Indent(255)),                // 1 + space
            (Token::Lit("42"), Indent(255)),         // 2
        ]
        .iter()
        .peekable();

        let got = line_len(tokens).next().unwrap();
        assert_eq!(got, 18);
    }

    #[test]
    fn test_comment_manually_aligned() {
        assert_rewrite!(
            r"
---- MODULE bananas ----
Op == /\ bananas = 42       \* This is an important number.
      /\ platanos' = 42     \* That should be assigned here.
====
"
        );
    }

    #[test]
    fn test_comment_partially_aligned() {
        assert_rewrite!(
            r"
---- MODULE bananas ----
Op == /\ bananas = 42         \* This is an important number.
      /\ platanos' = 42       \* That should be assigned here.
      /\ platanos' = 42           \* That should be assigned here.
====
"
        );
    }

    #[test]
    fn test_comment_non_adjacent_partially_aligned() {
        assert_rewrite!(
            r"
---- MODULE bananas ----
Op == /\ bananas = 42       \* This is an important number.
      /\ platanos' = 42         \* That should be assigned here.
      /\ platanos' = 42     \* That should be assigned here.
====
"
        );
    }

    #[test]
    fn test_comment_manually_unaligned() {
        assert_rewrite!(
            r"
---- MODULE bananas ----
Op == /\ bananas = 42      \* This is an important number.
      /\ platanos' = 42     \* That should be assigned here.
====
"
        );
    }

    #[test]
    fn test_comment_align_push() {
        assert_rewrite!(
            r"
---- MODULE bananas ----
Op == /\ bananas = 42    \* This is an important number.
      /\x=4+1+1+1+1+1+1  \* That should be assigned here.
      /\ platanos' = 42  \* That should be assigned here.
====
"
        );
    }

    #[test]
    fn test_comment_align_shrink() {
        assert_rewrite!(
            r"
---- MODULE bananas ----
Op == /\ bananas=42         \* This is an important number.
      /\ platanos'  =  42   \* That should be assigned here.
====
"
        );
    }

    /// A test case discovered through fuzzing where the input string contains a
    /// NULL byte, but the parser recovers and emits a sequence of nodes that
    /// have no newline preceding the first comment.
    ///
    /// When scanned backwards there would be no newline found, and the line
    /// length calculation would be fed a buffer that does not start with a
    /// newline causing it to assert.
    #[test]
    fn test_fuzz_input_contains_null() {
        let s = String::from_utf8(vec![
            0x71, 0x00, 0x0a, 0x2a, 0x5c, 0x2a, 0x0a, 0x4a, 0x5c, 0x2a, 0x0a, 0x2b, 0x41, 0x7e,
            0x41,
        ])
        .unwrap();
        assert_rewrite!(&s);
    }
}
