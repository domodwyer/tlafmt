use tree_sitter::Node;

use crate::{
    ast_format::{case::format_case, format_comment, format_module, list_item::format_list_item},
    get_str,
    helpers::EmptyLines,
    token::Token,
    Error, Renderer,
};

/// Format an arbitrary AST node.
pub(crate) fn format_node<'a, 'b: 'a, W>(
    def: Node<'b>,
    input: &'a str,
    empty_lines: &mut EmptyLines,
    writer: &mut Renderer<'a, W>,
) -> Result<(), Error>
where
    W: std::io::Write,
{
    empty_lines.maybe_insert(&def, writer)?;

    // When false, child nodes will be indented from `def` when rendered.
    let mut skip_indent = false;

    // For debug purposes, read the current indentation and assert the
    // indentation matches at the end of this fn to ensure balanced inc / dec.
    let indent_depth = writer.indent_get();

    // Some tokens can be extracted one-to-one from the AST.
    if let Some(t) = into_output_token(&def, input) {
        match t {
            // Suppress empty an ident that results in extraneous spacing.
            Token::Ident("") => return Ok(()),

            // Dedent these tokens from their bodies.
            Token::KeywordExcept
            | Token::KeywordVariable
            | Token::KeywordVariables
            | Token::KeywordConstant
            | Token::KeywordConstants
            | Token::KeywordExtends => {
                writer.indent_dec();
                writer.push(t)?;
                writer.indent_inc();
            }

            // Indent the bodies of parentheses.
            Token::ParenOpen | Token::CurlyOpen | Token::AngleOpen | Token::SquareOpen => {
                writer.push(t)?;
                writer.indent_inc();
            }

            // And dedent them when closing the parentheses.
            Token::ParenClose | Token::CurlyClose | Token::AngleClose | Token::SquareClose => {
                writer.indent_dec();
                writer.push(t)?;
            }

            // All other tokens are emitted with default indentation.
            t => writer.push(t)?,
        };

        return Ok(());
    }

    // These nodes emit no output directly, but cause indention of any children
    // which are output.
    //
    // Indentation is only increased if one of these nodes has not already
    // increased the indentation within the same line (which would cause double
    // indentation in the child).
    let may_indent = [
        // These tokens never indent as they are explicitly handled with
        // precedence in the match statement below, and exist in this list only
        // for indent suppression.
        "disj_item",
        "conj_item",
        "let_in",
        // ---
        "bound_infix_op",
        "bound_op",
        "except",
        "extends",
        "choose",
        "record_literal",
        "constant_declaration",
        "variable_declaration",
        "bounded_quantification",
        "quantifier_bound",
        "function_definition",
        "function_literal",
        "if_then_else",
        "finite_set_literal",
        "operator_definition",
        "set_of_functions",
        "set_of_records",
        "set_map",
    ];

    // Some tokens require processing before they can be emitted, to manage
    // their positioning or indentation.
    match def.kind() {
        // A module has specialised lowering that recursively calls this fn, and
        // is the entry point into parsing a TLA spec.
        "module" => {
            return format_module(def, input, empty_lines, writer);
        }

        // Comments must be processed to correct indentation caused by their
        // position in the AST, which does not necessarily reflect their
        // position in the source code.
        "comment" | "block_comment" | "extramodular_text" => {
            return format_comment(def, input, writer);
        }

        "case" => {
            return format_case(def, input, empty_lines, writer);
        }

        // A `[ident]_vars` sequence.
        "]_" => return Ok(()), // Part of the AST that is emitted below.
        "step_expr_or_stutter" => {
            let ident = def.named_child(0).ok_or(Error::StepOrStutter)?;
            writer.push(Token::StepOrStutter(get_str(&ident, input)))?;

            let vars = def.named_child(1).ok_or(Error::StepOrStutter)?;
            format_node(vars, input, empty_lines, writer)?;

            return Ok(());
        }

        // Conjunction and disjunction always appear on newlines and are never
        // explicitly indented - the parent list nodes are indented instead.
        "conj_item" | "disj_item" => {
            return format_list_item(def, input, empty_lines, writer);
        }

        // These are always indented.
        "disj_list" | "conj_list" | "let_in" => skip_indent = false,

        // Operators are not indented if they are the top level definition, and
        // are indented if they are within a definition (excluding LOCALs).
        "operator_definition"
            if def
                .parent()
                .is_some_and(|v| matches!(v.kind(), "module" | "local_definition")) =>
        {
            skip_indent = true;
        }

        // String literals.
        "string" => {
            writer.push(Token::Lit(get_str(&def, input)))?;
            return Ok(());
        }

        // Node types that have their children indented when rendered, iff the
        // indentation was not already increased on this line.
        v if may_indent.contains(&v) => {
            // Walk the tree upwards whilst the parent remains on the current
            // line.
            let mut ptr = def.parent();
            while let Some(p) = ptr {
                // Indentation may be changed at the AST node, which is always
                // reached at the start point of the span.
                if p.start_position().row != def.start_position().row {
                    break;
                }

                // Siblings cannot affect the indentation of this node - they
                // increase, format, and decrease indentation before this node
                // is formatted.
                if may_indent.contains(&p.kind()) {
                    skip_indent = true;
                    break;
                }

                ptr = p.parent();
            }
        }

        // Nodes that never increase the indentation depth.
        "source_file"
        | "case_arm"
        | "case_box"
        | "function_evaluation"
        | "except_update_record_field"
        | "except_update_specifier"
        | "except_update_fn_appl"
        | "except_update"
        | "record_value"
        | "always"
        | "eventually"
        | "boolean"
        | "fairness"
        | "bound_postfix_op"
        | "bullet_conj"
        | "bullet_disj"
        | "bound_op"
        | "bound_prefix_op"
        | "tuple_literal"
        | "parentheses"
        | "local_definition"
        | "set_filter"
        | "subexpr_component"
        | "infix_op_symbol"
        | "instance"
        | "domain"
        | "theorem" => {
            skip_indent = true;
        }

        // Syntax errors reported by the AST parser.
        "ERROR" => {
            #[cfg(not(fuzzing))] // No output during fuzzing for faster execs.
            eprintln!(
                "[ERROR] syntax parsing error for {:?} => '{}'",
                def,
                get_str(&def, input)
            );
            writer.push(Token::Raw(get_str(&def, input)))?;
            return Ok(());
        }

        // Unformatted nodes that are printed as-is.
        _ => {
            #[cfg(not(fuzzing))] // No output during fuzzing for faster execs.
            eprintln!(
                "[WARN] unformatted node {:?} => '{}'",
                def,
                get_str(&def, input)
            );
            writer.push(Token::Raw(get_str(&def, input)))?;
            return Ok(());
        }
    }

    // Begin rewriting the definition body.
    let mut c = def.walk();
    let iter = def.children(&mut c);

    for n in iter {
        empty_lines.maybe_insert(&n, writer)?;

        if !skip_indent {
            writer.indent_inc();
        }

        format_node(n, input, empty_lines, writer)?;

        if !skip_indent {
            writer.indent_dec();
        }
    }

    debug_assert_eq!(indent_depth, writer.indent_get());

    Ok(())
}

/// Returns a [`Token`] if [`Node`] can be directly mapped to an output token.
fn into_output_token<'a>(node: &Node<'_>, input: &'a str) -> Option<Token<'a>> {
    Some(match node.kind() {
        "LET" => Token::KeywordLet,
        "IN" => Token::KeywordIn,
        "CHOOSE" => Token::KeywordChoose,
        "LOCAL" => Token::KeywordLocal,
        "IF" => Token::KeywordIf,
        "THEN" => Token::KeywordThen,
        "ELSE" => Token::KeywordElse,
        "CASE" => Token::KeywordCase,
        "INSTANCE" => Token::KeywordInstance,
        "EXTENDS" => Token::KeywordExtends,
        "CONSTANT" => Token::KeywordConstant,
        "CONSTANTS" => Token::KeywordConstants,
        "VARIABLE" => Token::KeywordVariable,
        "VARIABLES" => Token::KeywordVariables,
        "EXCEPT" => Token::KeywordExcept,
        "unchanged" => Token::KeywordUnchanged,
        "THEOREM" => Token::KeywordTheorem,
        "powerset" => Token::KeywordSubset,
        "domain" => Token::KeywordDomain,
        "enabled" => Token::KeywordEnabled,
        "union" => Token::KeywordUnion,
        "implies" => Token::Implies,
        "compose" => Token::Compose,
        "TRUE" => Token::True,
        "FALSE" => Token::False,
        "exists" => Token::Exists,
        "case_arrow" => Token::CaseArrow,
        "in" | "set_in" => Token::SetIn,
        "notin" => Token::SetNotIn,
        "forall" => Token::All,
        "/\\" | "land" => Token::And,
        "\\/" | "lor" => Token::Or,
        "lnot" => Token::Not,
        "eq" | "=" => Token::Eq,
        "def_eq" => Token::Eq2,
        "neq" => Token::NotEq,
        "identifier" | "identifier_ref" => Token::Ident(get_str(node, input)),
        "nat_number" => Token::Lit(get_str(node, input)),
        "prev_func_val" => Token::At,
        ":" => Token::SemiColon,
        "!" => Token::Bang,
        "(" => Token::ParenOpen,
        ")" => Token::ParenClose,
        "," => Token::Comma,
        "[" => Token::SquareOpen,
        "]" => Token::SquareClose,
        "{" => Token::CurlyOpen,
        "}" => Token::CurlyClose,
        "plus" => Token::Plus,
        "minus" => Token::Minus,
        "mul" => Token::Multiply,
        "map_to" => Token::MapTo,
        "maps_to" => Token::MapsTo,
        "all_map_to" => Token::AllMapsTo,
        "." => Token::Dot,
        "dots_2" => Token::Dots2,
        "langle_bracket" => Token::AngleOpen,
        "rangle_bracket" => Token::AngleClose,
        "circ" => Token::AppendShort,
        "gt" => Token::GreaterThan,
        "geq" => Token::GreaterThanEqual,
        "lt" => Token::LessThan,
        "leq" => Token::LessThanEqual,
        "real_number_set" => Token::Real,
        "int_number_set" => Token::Int,
        "nat_number_set" => Token::Nat,
        "setminus" => Token::SetMinus,
        "slash" => Token::Divide,
        "single_line" => Token::LineDivider('-'),
        "double_line" => Token::LineDivider('='),
        "prime" => Token::Prime,
        "[]" => Token::Always,
        "<>" => Token::Eventually,
        "cup" => Token::Union,
        "cap" => Token::Intersect,
        "subseteq" => Token::SubsetEq,
        "WF_" => Token::WeakFairness,
        "SF_" => Token::StrongFairness,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use crate::assert_rewrite;

    #[test]
    fn test_basic_def() {
        assert_rewrite!(
            "\
    ---- MODULE Bananas ------
    X == 42
    ====="
        );
    }

    #[test]
    fn test_basic_conj() {
        assert_rewrite!(
            r"
    ---- MODULE Bananas ------
    X == /\ x = 4
         /\ y = 2
    ====="
        );
    }

    #[test]
    fn test_one_line() {
        assert_rewrite!(
            "\
    ---- MODULE Bananas ------
    X(a, b) == a = 42 /\\ b = 13
    ====="
        );
    }

    #[test]
    fn test_long_line() {
        assert_rewrite!(
            "\
    ---- MODULE Bananas ------
    X(a, b) == a = 42 /\\ b = 13 /\\ b = 13 /\\ b = 13 /\\ b = 13 /\\ b = 13 /\\ b = 13 /\\ b = 13
    ====="
        );
    }

    #[test]
    fn test_multi_line_conjunction() {
        assert_rewrite!(
            r"
---- MODULE Bananas ------
X(a, b) == /\ a = 42
           /\ b = 13 /\ b = 13
           /\ b = 13 /\ b = 13 /\ b = 13
           /\ b = 13

           /\ b = 13
====="
        );
    }

    #[test]
    fn test_let_in_big() {
        assert_rewrite!(
            "\
----------------------- MODULE Bananas --------------------
Integrate(D, a, b, InitVals) ==
  LET n == Len(InitVals)
      gg == CHOOSE g :
              \\E e \\in PosReal :
                 /\\ g \\in [0..n -> [OpenInterval(a-e, b+e) -> Real]]
                 /\\ \\A i \\in 1..n : /\\ IsDeriv(i, g[i], g[0])
                                    /\\ g[i-1][a] = InitVals[i]
                 /\\ \\A r \\in OpenInterval(a-e, b+e) :
                        D[ <<r>> \\o [i \\in 1..(n+1) |-> g[i-1][r]] ] = 0
  IN  [i \\in 1..n |-> gg[i-1][b]]
===="
        );
    }

    #[test]
    fn test_let_in_small() {
        assert_rewrite!(
            "\
----------------------- MODULE Bananas --------------------
Integrate(D, a, b, InitVals) ==
  LET n == Len(InitVals)
      gg == CHOOSE g :
              \\E e \\in PosReal :
                 /\\ \\A r \\in OpenInterval(a-e, b+e) :
                        D[ <<r>> \\o [i \\in 1..(n+1) |-> g[i-1][r]] ] = 0
  IN  [i \\in 1..n |-> gg[i-1][b]]
===="
        );
    }

    #[test]
    fn test_operator_short_form_args() {
        assert_rewrite!(
            "\
    ---- MODULE Bananas ------
    DoStuff(
		b,


		a  ,
		n, A,

		N     ,
		a,
		S


) == 42
    ====="
        );
    }

    #[test]
    fn test_operator_long_form_args() {
        assert_rewrite!(
            "\
    ---- MODULE Bananas ------
    DoStuff(b,a  , n,A, N     ,a,S    ) == 42
    ====="
        );
    }

    #[test]
    fn test_operator_short_form_inline_comment() {
        assert_rewrite!(
            "\
    ---- MODULE Bananas ------
    DoStuff(b,a  , n,A, N     ,a,S (* are great*)) == 42
    ====="
        );
    }

    #[test]
    fn test_operator_preceding_line_comment() {
        assert_rewrite!(
            "\
 ---- MODULE Bananas ------
   (* Y this *)     DoStuff(b,a  , n,A, N     ,a,S    ) == 42
    ====="
        );
    }

    #[test]
    fn test_operator_succeeding_line_comment() {
        assert_rewrite!(
            "\
 ---- MODULE Bananas ------
       DoStuff(b,a  , n,A, N     ,a,S    ) == (* Y this *)  42
    ====="
        );
    }

    #[test]
    fn test_operator_succeeding_args_comment() {
        assert_rewrite!(
            "\
 ---- MODULE Bananas ------
    DoStuff(b,a  , n,A, N     ,a,S    )  (* Y this *)    == 42
    ====="
        );
    }

    #[test]
    fn test_operator_preceding_line_comment_no_args() {
        assert_rewrite!(
            "\
 ---- MODULE Bananas ------
   (* Y this *)     DoStuff == 42
    ====="
        );
    }

    #[test]
    fn test_operator_succeeding_line_comment_no_args() {
        assert_rewrite!(
            "\
 ---- MODULE Bananas ------
       DoStuff == (* Y this *)  42
    ====="
        );
    }

    #[test]
    fn test_operator_succeeding_args_comment_no_args() {
        assert_rewrite!(
            "\
 ---- MODULE Bananas ------
    DoStuff  (* Y this *)    == 42
    ====="
        );
    }

    #[test]
    fn test_operator_normalise_open_arg_list() {
        assert_rewrite!(
            "\
    ---- MODULE Bananas ------
DoStuff
(b,a  , n,A, N     ,a,S    ) == 42
    ====="
        );
    }

    #[test]
    fn test_operator_open_arg_list_comment() {
        assert_rewrite!(
            "\
    ---- MODULE Bananas ------
DoStuff (*comment*)
(b,a  , n,A, N     ,a,S    ) == 42
    ====="
        );
    }

    #[test]
    fn test_set_map() {
        assert_rewrite!(
            r"
---- MODULE Bananas ------
Init == {x[banana]: banana \in Box}
====="
        );
    }

    #[test]
    fn test_empty_set_seq() {
        assert_rewrite!(
            r"
---- MODULE Bananas ------
Init == /\ A = {42}
        /\ B = {}
        /\ C = <<42>>
        /\ D = <<>>
====="
        );
    }

    #[test]
    fn test_if_then_else() {
        assert_rewrite!(
            r"
---- MODULE Bananas ------
DoStuff ==
    IF x THEN
        42
    ELSE
        24

MoreStuff = IF x THEN 42 ELSE 24
====="
        );
    }

    #[test]
    fn test_bounded_quantifier_with_list() {
        assert_rewrite!(
            r"
---- MODULE Bananas ------
Next ==
    \/ ResetCounters
    \/ \E t \in Threads:
        \/ ThreadReadHealth(t)
        \/ ThreadShouldProbe_health(t)

LOCAL IsFirstDeriv(df, f) ==
    /\ df \in [f -> Real]
    /\ \A r \in f:
        \A e \in PosReal:
            \E d \in PosReal:
                \A s \in Nbhd(r, d) \ { r }:
                    (f[s] - f[r]) /(s - r) \in Nbhd(df[r], e)

Integrate(D, a, b, InitVals) ==
    LET n == Len(InitVals)
        gg == CHOOSE g:
            \E e \in PosReal:
                /\ \A r \in OpenInterval(a - e, b + e):
                    D[<< r >> \o [i \in 1..(n + 1) |-> g[i - 1][r]]] = 0
    IN [i \in 1..n |-> gg[i - 1][b]]
====="
        );
    }

    #[test]
    fn test_spec_theorem() {
        assert_rewrite!(
            r"
---- MODULE Bananas ------
Spec == \* Initialize state with Init and transition with Next.
    Init /\ [][Next]_<<store, tx, snapshotStore, written, missed>>
----------------------------------------------------------------------------
THEOREM Spec => [](TypeInvariant /\ TxLifecycle)
=============================================================================
"
        );
    }

    #[test]
    fn test_newline_indent_non_list() {
        assert_rewrite!(
            r#"
---- MODULE Bananas ----
NoVal ==
    CHOOSE v :

    v \notin
Val

   \* Bananas are good
Store ==
    [  Key ->
Val      \cup   {   NoVal}  ]

   \* Set map
Map ==
{
P(x)  :   x \in S
}

Record ==
[
  name   |->"bananas" ,
    version |-> 0
    ]
====
"#
        );
    }

    #[test]
    fn test_lor() {
        assert_rewrite!(
            r"
---- MODULE B ----
X == A \/ B
====
"
        );
    }

    #[test]
    fn test_string() {
        assert_rewrite!(
            r#"
---- MODULE B ----
X == "bananas"
====
"#
        );
    }

    #[test]
    fn test_spec_always_true_ident() {
        assert_rewrite!(
            r#"
---- MODULE B ----
LEMMA TypeCorrect == Spec => []TypeInv
THEOREM DeadlockFreedom == Spec => [] Invariant
====
"#
        );
    }

    /// Reproducer for https://github.com/domodwyer/tlafmt/issues/25.
    #[test]
    fn test_bounded_quantification() {
        assert_rewrite!(
            r#"
---- MODULE B ----
FairSpec ==
    /\ Spec

    \* Assert that producers take steps should their  Put  action be (continuously)
    \* enabled. This is the basic case of fairness that rules out stuttering, i.e.,
    \* assert global progress.
    /\ \A t \in Producers:
            WF_vars(Put(t,t))
    \* Stipulates that  Get  actions (consumers!) will eventually notify *all*
    \* waiting producers. In other words, given repeated  Get  actions (we don't
    \* care which consumer, thus, existential quantification), all waiting
    \* producers will eventually be notified.  Because  Get  actions are not
    \* continuously enabled (the buffer might be empty), weak fairness is not
    \* strong enough. Obviously, no real system scheduler implements such an
    \* inefficient "policy".
    \* This fairness constraint was initially proposed by Leslie Lamport, although
    \* with the minor typo "in" instead of "notin", which happens to hold for
    \* configurations with at most two producers.
    /\ \A t \in Producers:
            SF_vars(\E self \in Consumers: Get(self) /\ t \notin waitSet')

    \* See notes above (except swap "producer" with "consumer").
    /\ \A t \in Consumers:
            WF_vars(Get(t))
    /\ \A t \in Consumers:
            SF_vars(\E self \in Producers: Put(self, self) /\ t \notin waitSet')
====
"#
        );
    }

    /// Reproducer for https://github.com/domodwyer/tlafmt/issues/24.
    #[test]
    fn test_let_in_no_in() {
        assert_rewrite!(
            r#"
---- MODULE B ----
Send ==
    /\ i <= n
    /\ \E v \in 0..n: i <= v /\ v <= i + l /\ v \notin got
    /\ LET k == CHOOSE val \in 0..n:(val \in i..i + l /\ val \notin got) IN
    /\ chan' = chan \union {<< k, INPUT[k] >>}
    /\ UNCHANGED << i, INPUT, OUTPUT, got, ack, complete >>
====
"#
        );
    }

    #[test]
    fn test_number_sets() {
        assert_rewrite!(
            r"
---- MODULE B ----
EXTENDS Integers
X == [version: Int]
X == [version: Real]
X == [version: Nat]
====
"
        );
    }

    #[test]
    fn test_inline_comment_empty_ident() {
        assert_rewrite!(
            r#"
---- MODULE B ----
Inv == /\ TypeOK
       /\ \A i \in Procs :
\*             /\ (pc[i] \in {"ncs", "e1", "e2"}) => (num[i] = 0)
====
"#
        );
    }
}
