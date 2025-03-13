use crate::token::Token;

use super::Indent;

/// Process the token buffer, reducing the indentation of excessively indented
/// blocks relative to the previous line.
///
/// An excessively indented block is one in which all tokens within it are more
/// than 1 level deeper in indentation than the parent. It rewrites this:
///
/// ```text
///    LET n == Len(InitVals)
///            gg == CHOOSE g:
///                    \E e \in PosReal:
///                        /\ \A r \in OpenInterval(a - e, b + e):
///                                    D[X |-> 42] = 0
/// ```
///
/// Into this:
///
/// ```text
///    LET n == Len(InitVals)
///        gg == CHOOSE g:
///            \E e \in PosReal:
///                /\ \A r \in OpenInterval(a - e, b + e):
///                     D[X |-> 42] = 0
/// ```
///
pub(super) fn limit_indents(buf: &mut [(Token<'_>, Indent)]) {
    recurse(buf);
}

/// The indentation rewriter state.
///
/// In these comments, the "current" indentation level refers to the indentation
/// level at which the first token in a call to [`recurse()`] is observed.
///
/// The "current block" is all nodes indented at the "current" indentation level
/// or greater.
///
/// ```text
///                           Current Indent
///                                Level
///                                  │
///                                  ▼
///                               ┌─
///                               │  A
///                               │
///                               │  A
///                               │
///                               │     B
///                               │
///                     Current   │        C
///                      Block    │
///                               │     B
///                               │
///                               │     B
///                               │
///                               │  A
///                               └─
///
/// ```
///
#[derive(Debug)]
enum State {
    /// Consume tokens that are at the same indent level as the current block.
    ///
    /// Transitions to [`State::Scanning`] when excessive indentation is found.
    Skipping,

    /// Potentially excessive indentation has been found.
    ///
    /// Scan through tokens looking for the minimum indent that exceeds the
    /// current indent level, at which point transition to [`State::Rewriting`]
    /// to correct it if it is `> current + 1`, else transitions back to
    /// [`State::Skipping`].
    ScanningMin {
        /// The index at which the transition to this state occurred, which is
        /// the index where the excessive indentation begins.
        start_index: usize,

        /// The min observed indention level within the nested block.
        min: Indent,
    },

    /// Adjust any indentation level at or above the current block's indent
    /// depth by reducing indentation by the number of levels specified in
    /// `delta`.
    ///
    /// Once the current block ends (the indention is strictly less than the
    /// current block), the state transitions back to [`State::Skipping`].
    Rewriting {
        /// The index at which this first token of excessive indentation was
        /// found.
        start_index: usize,

        /// The (positive) adjustment to subtract from the nested block
        /// indentation level.
        delta: Indent,
    },
}

fn recurse(buf: &mut [(Token<'_>, Indent)]) -> usize {
    // Extract the current indentation depth for this call to operate relative
    // to.
    let current_depth = match buf.first() {
        Some((_, v)) => *v,
        None => {
            return 0;
        }
    };

    // The rewrite state.
    let mut state = State::Skipping;

    // Indentation is only effective immediately following a newline token,
    // therefore only those tokens are considered when advancing the FSM or
    // rewriting indentation levels.
    let mut last_was_newline = false;

    let mut i = 0;
    while i < buf.len() {
        // Observe if this token is a newline, and skip visiting it if the last
        // token was not a newline.
        let last = last_was_newline;
        last_was_newline = matches!(buf[i].0, Token::Newline | Token::SourceNewline);
        if !last {
            i += 1;
            continue;
        }

        // Extract the indentation level of this token.
        let this = buf[i].1;

        match state {
            // Skip tokens at the same indentation level.
            State::Skipping if this == current_depth => {}

            // Recurse into an nested block of +1 depth for processing.
            State::Skipping if this == current_depth + 1 => {
                i += recurse(&mut buf[i..]);
                continue;
            }

            // This MAY be excessively indented.
            //
            // To confirm, all tokens that are indented at this depth must be
            // visited.
            //
            // An example of valid indentation that appears excessive without
            // visiting all child nodes is:
            //
            //      Op == [
            //              {
            //          }
            //      ]
            //
            // The transition from the first line to the second looks excessive,
            // but a trailing bracket on the second to last line causes the
            // indentation to be valid, as there are nodes at all indent depths.
            State::Skipping if this > current_depth + 1 => {
                // Transition to the "scanning" state to find the minimum indent
                // indentation that is contained within the parent block.
                state = State::ScanningMin {
                    start_index: i,
                    min: this,
                }
            }

            // The indent depth has fallen below the current indent depth of
            // this block.
            State::Skipping => {
                debug_assert!(this < current_depth);

                // Optimisation: return the number of nodes visited at least
                // once to allow the caller to advance past the nodes this call
                // visited.
                return i;
            }

            // The excessively indented, nested block was fully visited (by
            // virtue of indentation dropping back to the current block's
            // indentation level or less) and it did not take an early state
            // transition back to skipping, meaning it is confirmed to be
            // excessively indented.
            State::ScanningMin { start_index, min } if this <= current_depth => {
                // Reset the index back to the first occurrence of `child`,
                // which is guaranteed to have followed a newline.
                i = start_index;
                last_was_newline = true;

                // And begin reducing the indentation by the specified amount.
                state = State::Rewriting {
                    start_index,
                    delta: min - current_depth - Indent(1),
                };
                continue;
            }

            // The early return when a possibly excessively indented, nested
            // block contains a node at current + 1 meaning it was appropriately
            // indented.
            State::ScanningMin { start_index, min } if min == current_depth + 1 => {
                state = State::Skipping;
                i = start_index + recurse(&mut buf[start_index..]);
                continue;
            }

            // Continue scanning within the nested block, looking for the min
            // depth.
            State::ScanningMin { start_index, min } => {
                state = State::ScanningMin {
                    start_index,
                    min: std::cmp::min(min, this),
                }
            }

            // Apply a delta adjustment to reduce this node's indentation level.
            State::Rewriting { delta, .. } if this > current_depth => {
                buf[i].1 = this - delta;
            }

            // This node reaches the end of the nested block.
            //
            // After processing the nested block, it needs to be recursed into
            // to correct any further nested, excessively indented blocks
            // relative to it.
            State::Rewriting { start_index, .. } => {
                state = State::Skipping;
                i = start_index + recurse(&mut buf[start_index..]);
                continue;
            }
        };

        i += 1;
    }

    i
}

#[cfg(test)]
mod tests {
    use crate::assert_rewrite;

    use super::*;

    #[test]
    fn test_no_op() {
        let tokens = [
            (Token::Newline, Indent(1)),
            (Token::Bang, Indent(1)),
            (Token::Newline, Indent(1)),
            (Token::Bang, Indent(1)),
            (Token::Newline, Indent(1)),
            (Token::Bang, Indent(1)),
        ];

        let mut got = tokens.clone();
        limit_indents(&mut got);
        assert_eq!(got, tokens);
    }

    #[test]
    fn test_dedent_many() {
        let mut tokens = [
            (Token::Newline, Indent(1)),
            (Token::Bang, Indent(1)),
            (Token::Newline, Indent(3)),
            (Token::Bang, Indent(3)),
            (Token::Newline, Indent(5)),
            (Token::Bang, Indent(5)),
            (Token::Newline, Indent(3)),
            (Token::Bang, Indent(3)),
            (Token::Newline, Indent(1)),
            (Token::Bang, Indent(1)),
        ];

        limit_indents(&mut tokens);
        assert_eq!(
            tokens,
            [
                (Token::Newline, Indent(1)),
                (Token::Bang, Indent(1)),
                (Token::Newline, Indent(3)), // Newline tokens are not rewrote
                (Token::Bang, Indent(2)),
                (Token::Newline, Indent(5)),
                (Token::Bang, Indent(3)),
                (Token::Newline, Indent(3)),
                (Token::Bang, Indent(2)),
                (Token::Newline, Indent(1)),
                (Token::Bang, Indent(1)),
            ]
        );
    }

    #[test]
    fn test_dedent_one() {
        let mut tokens = [
            (Token::Newline, Indent(1)),
            (Token::Bang, Indent(1)),
            (Token::Newline, Indent(3)),
            (Token::Bang, Indent(3)),
            (Token::Newline, Indent(1)),
            (Token::Bang, Indent(1)),
        ];

        limit_indents(&mut tokens);
        assert_eq!(
            tokens,
            [
                (Token::Newline, Indent(1)),
                (Token::Bang, Indent(1)),
                (Token::Newline, Indent(3)),
                (Token::Bang, Indent(2)),
                (Token::Newline, Indent(1)),
                (Token::Bang, Indent(1)),
            ]
        );
    }

    #[test]
    fn test_step_jump() {
        let mut tokens = [
            (Token::Newline, Indent(1)),
            (Token::Bang, Indent(1)),
            (Token::Newline, Indent(2)),
            (Token::Bang, Indent(2)),
            (Token::Newline, Indent(5)),
            (Token::Bang, Indent(5)),
            (Token::Newline, Indent(5)),
            (Token::Bang, Indent(5)),
            (Token::Newline, Indent(1)),
            (Token::Bang, Indent(1)),
        ];

        limit_indents(&mut tokens);
        assert_eq!(
            tokens,
            [
                (Token::Newline, Indent(1)),
                (Token::Bang, Indent(1)),
                (Token::Newline, Indent(2)),
                (Token::Bang, Indent(2)),
                (Token::Newline, Indent(5)),
                (Token::Bang, Indent(3)),
                (Token::Newline, Indent(5)),
                (Token::Bang, Indent(3)),
                (Token::Newline, Indent(1)),
                (Token::Bang, Indent(1)),
            ]
        );
    }

    #[test]
    fn test_deferred_indent_use() {
        let tokens = [
            (Token::Newline, Indent(1)),
            (Token::Bang, Indent(1)),
            (Token::Newline, Indent(3)),
            (Token::Bang, Indent(3)),
            (Token::Newline, Indent(2)),
            (Token::Bang, Indent(2)),
        ];

        let mut got = tokens.clone();
        limit_indents(&mut got);
        assert_eq!(got, tokens);
    }

    #[test]
    fn test_let_in_record_literal() {
        assert_rewrite!(
            r"
---- MODULE Bananas ----
AppendEntries(i, j) ==
    /\ LET prevLogIndex == nextIndex[i][j] - 1
           prevLogTerm == IF prevLogIndex > 0 THEN
                              log[i][prevLogIndex].term
                          ELSE
                              0
           \* Send up to 1 entry, constrained by the end of the log.
           lastEntry == Min({Len(log[i]), nextIndex[i][j]})
           entries == SubSeq(log[i], nextIndex[i][j], lastEntry)
       IN Send([mtype          |-> AppendEntriesRequest,
                mterm          |-> currentTerm[i],
                mprevLogIndex  |-> prevLogIndex,
                mdest          |-> j])
====
"
        )
    }
}
