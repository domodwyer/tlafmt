use tree_sitter::Node;

/// Positional metadata for a token.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Position {
    /// The absolute position where this token appears in the source / input
    /// spec.
    Source { row: usize, col: usize },

    /// The amount of padding whitespace to precede to this token (relative
    /// positioning w.r.t previous token).
    Relative(usize),
}

impl Position {
    pub(crate) fn unwrap_row(&self) -> usize {
        match self {
            Self::Source { row, .. } => *row,
            _ => unreachable!(),
        }
    }

    pub(crate) fn unwrap_col(&self) -> usize {
        match self {
            Self::Source { col, .. } => *col,
            _ => unreachable!(),
        }
    }
}

impl From<&Node<'_>> for Position {
    fn from(n: &Node<'_>) -> Self {
        let pos = n.start_position();
        Self::Source {
            row: pos.row,
            col: pos.column,
        }
    }
}

/// The formatter token to be rendered.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Token<'a> {
    /// A raw string to print.
    ///
    /// NOTE: this str may contain newlines.
    Raw(&'a str),

    /// A module header (`--- MODULE name ---`).
    ModuleHeader(&'a str),

    /// A comment (inline or box).
    ///
    /// NOTE: this str may contain newlines.
    Comment(&'a str, Position),

    /// A linebreak specified in the input sourcecode.
    SourceNewline,

    /// An explicit newline inserted by the formatter.
    Newline,

    /// A `CHOOSE` operator.
    KeywordChoose,

    /// A `LET` operator.
    KeywordLet,

    /// A `IN` operator.
    KeywordIn,

    /// A `UNCHANGED` sequence.
    KeywordUnchanged,

    /// A `LOCAL` keyword.
    KeywordLocal,

    /// A `INSTANCE` keyword.
    KeywordInstance,

    /// A `DOMAIN` keyword.
    KeywordDomain,

    /// A `SUBSET` keyword.
    KeywordSubset,

    /// A `IF`.
    KeywordIf,

    /// A `THEN`.
    KeywordThen,

    /// A `ELSE`.
    KeywordElse,

    /// A `CASE`.
    KeywordCase,

    /// A "EXTENDS" sequence.
    KeywordExtends,

    /// A "CONSTANT" sequence.
    KeywordConstant,

    /// A "CONSTANTS" sequence.
    KeywordConstants,

    /// A "VARIABLE" sequence.
    KeywordVariable,

    /// A "VARIABLES" sequence.
    KeywordVariables,

    /// A "EXCEPT" sequence.
    KeywordExcept,

    /// A `ENABLED` sequence.
    KeywordEnabled,

    /// A `THEOREM` sequence.
    KeywordTheorem,

    /// A `\E` sequence.
    Exists,

    /// A `[]` sequence for a CASE statement.
    CaseBox,

    /// A `->` sequence for a CASE statement.
    CaseArrow,

    /// A `\A` sequence.
    All,

    /// A `\in sequence.
    SetIn,

    /// A `\notin` sequence.
    SetNotIn,

    /// A `/\` sequence.
    And,

    /// A `\/` sequence.
    Or,

    /// A `->` sequence.
    MapsTo,

    /// A `:>` sequence.
    MapTo,

    /// A `|->` sequence.
    AllMapsTo,

    /// A identifier name.
    Ident(&'a str),

    /// A literal number or string.
    Lit(&'a str),

    /// A `(`.
    ParenOpen,

    /// A `)`.
    ParenClose,

    /// A comma `,`.
    Comma,

    /// A `:`.
    SemiColon,

    /// A `+`.
    Plus,

    /// A `-`.
    Minus,

    /// A `*`.
    Multiply,

    /// A `=` sequence.
    Eq,

    /// A `==` sequence.
    Eq2,

    /// A `/=` sequence.
    NotEq,

    /// A `\subseteq` sequence.
    SubsetEq,

    /// A `.` sequence.
    Dot,

    /// A `..` sequence.
    Dots2,

    /// A `@`.
    At,

    /// A `[`.
    SquareOpen,

    /// A `]`.
    SquareClose,

    /// A `{`.
    CurlyOpen,

    /// A `}`.
    CurlyClose,

    /// A `<<` sequence.
    AngleOpen,

    /// A `>>` sequence.
    AngleClose,

    /// A `\o` sequence.
    AppendShort,

    /// A `Real` sequence.
    Real,

    /// An `Int` sequence.
    Int,

    /// A `Nat` sequence.
    Nat,

    /// A `<`.
    GreaterThan,

    /// A `>=`.
    GreaterThanEqual,

    /// A `<`.
    LessThan,

    /// A `<=`.
    LessThanEqual,

    /// A `~`.
    Not,

    /// A `\`.
    SetMinus,

    /// A `/`.
    Divide,

    /// A dividing line composed of `-----` or `=====`.
    LineDivider(char),

    /// A prime var marker `'`.
    Prime,

    /// A `[]` sequence.
    Always,

    /// A `<>` sequence.
    Eventually,

    /// A `=>` sequence.
    Implies,

    /// A `!`.
    Bang,

    /// A `TRUE`.
    True,

    /// A `FALSE`.
    False,

    /// A `WF_` sequence.
    WeakFairness,

    /// A `SF_` sequence.
    StrongFairness,

    /// A `\union` sequence.
    Union,

    /// A `\intersect` sequence.
    Intersect,

    /// A `@@` sequence.
    Compose,

    /// A `[Next]_` sequence.
    StepOrStutter(&'a str),
}

impl Token<'_> {
    /// Returns true when `self` and `next` are allowed to appear in sequence.
    ///
    /// If false, the caller is expected to drop `self` when rendering.
    pub(crate) fn can_precede(&self, next: &Self) -> bool {
        match (self, next) {
            (Token::Comma, Token::ParenClose) => false,
            _ => true,
        }
    }

    /// Returns true when `self` and `next` should be delimited by a space
    /// character when rendered.
    pub(crate) fn delimiting_space_len(&self, next: &Self) -> usize {
        match (self, next) {
            (_, Token::ModuleHeader(_)) => 0,

            // Comments with explicit whitespace padding render the provided
            // amount of space.
            (_, Token::Comment(_, Position::Relative(v))) => *v,

            // Newlines are never automatically followed by whitespace.
            (Token::Newline | Token::SourceNewline, _) => 0,

            (Token::Raw(_), Token::Newline | Token::SourceNewline) => 0,
            (Token::Raw(s), _) if s.ends_with("\n") => 0,
            (Token::Raw(_), _) => 1,
            (_, Token::Raw(_)) => 1,

            // These tokens can never be followed by a space, irrespective of
            // the next token.
            (Token::ParenOpen | Token::SquareOpen | Token::CurlyOpen | Token::Dots2, _) => 0,

            // These tokens allow immediate dots, to support <<thing>>.field
            (
                Token::ParenClose | Token::SquareClose | Token::CurlyClose | Token::AngleClose,
                Token::Dot,
            ) => 0,

            // No space between Fn[application].
            (Token::Ident(_), Token::SquareOpen) => 0,

            // No space between x[1][2].
            (Token::SquareClose, Token::SquareOpen) => 0,

            (Token::AngleOpen, Token::AngleClose) => 0,

            // A `record.field` sequence.
            (Token::Ident(_), Token::Dot) => 0,
            (Token::Dot, Token::Ident(_)) => 0,

            // A `EXCEPT !.ok` sequence or `EXCEPT ![x]`.
            (Token::Bang, Token::Dot | Token::SquareOpen) => 0,

            // Any "not" sequence such as `~(thing)`.
            (Token::Not, _) => 0,

            // Chained liveness tokens are not space delimited, nor should there
            // be a space before the [Next] portion in `<>[][Next]_vars`.
            //
            // Likewise a THEOREM == Spec => []Op should not be space delimited
            // between [] and the ident.
            (
                Token::Eventually | Token::Always,
                Token::Eventually | Token::Always | Token::StepOrStutter(_) | Token::Ident(_),
            ) => 0,

            // Fairness bounds must not be space delimited.
            (Token::WeakFairness | Token::StrongFairness, _) => 0,

            // Unchanged vars `[Next]_vars`.
            (Token::StepOrStutter(_), _) => 0,

            // No space between these tokens and an opening paren.
            (
                Token::Ident(_)
                | Token::ParenClose
                | Token::SquareClose
                | Token::CurlyClose
                | Token::Always
                | Token::Eventually,
                Token::ParenOpen,
            ) => 0,

            // No space between any token and the listed tokens.
            (
                _,
                Token::SourceNewline
                | Token::Newline
                | Token::ParenClose
                | Token::SquareClose
                | Token::CurlyClose
                | Token::Comma
                | Token::Dots2
                | Token::SemiColon
                | Token::Prime,
            ) => 0,

            // All other tokens can be delimited by whitespace.
            _ => 1,
        }
    }
}
