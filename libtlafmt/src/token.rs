/// The formatter token to be rendered.
#[derive(Debug, Clone)]
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
    Comment(&'a str),

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
    pub(crate) fn is_space_delimited(&self, next: &Self) -> bool {
        match (self, next) {
            (_, Token::ModuleHeader(_)) => false,

            (Token::Raw(_), _) => true,
            (_, Token::Raw(_)) => true,

            // These tokens can never be followed by a space, irrespective of
            // the next token.
            (Token::ParenOpen | Token::SquareOpen | Token::CurlyOpen | Token::Dots2, _) => false,

            // No space between Fn[application].
            (Token::Ident(_), Token::SquareOpen) => false,

            // No space between x[1][2].
            (Token::SquareClose, Token::SquareOpen) => false,

            (Token::AngleOpen, Token::AngleClose) => false,

            // A `record.field` sequence.
            (Token::Ident(_), Token::Dot) => false,
            (Token::Dot, Token::Ident(_)) => false,

            // A `EXCEPT !.ok` sequence or `EXCEPT ![x]`.
            (Token::Bang, Token::Dot | Token::SquareOpen) => false,

            // A `something > (n + 1)` sequence.
            (
                Token::GreaterThan
                | Token::GreaterThanEqual
                | Token::LessThan
                | Token::LessThanEqual
                | Token::Compose,
                Token::ParenOpen,
            ) => true,

            // Any "not" sequence such as `~(thing)`.
            (Token::Not, _) => false,

            // Chained liveness tokens are not space delimited, nor should there
            // be a space before the [Next] portion in `<>[][Next]_vars`.
            (
                Token::Eventually | Token::Always,
                Token::Eventually | Token::Always | Token::StepOrStutter(_),
            ) => false,

            // Fairness bounds must not be space delimited.
            (Token::WeakFairness | Token::StrongFairness, _) => false,

            // Unchanged vars `[Next]_vars`.
            (Token::StepOrStutter(_), _) => false,

            // Eq followed by parens.
            (Token::Eq | Token::NotEq, Token::ParenOpen | Token::SquareOpen | Token::AngleOpen) => {
                true
            }

            // No space between this token and the listed tokens.
            _ if matches!(
                next,
                Token::SourceNewline
                    | Token::Newline
                    | Token::ParenOpen
                    | Token::ParenClose
                    | Token::SquareClose
                    | Token::CurlyClose
                    | Token::Comma
                    | Token::Dots2
                    | Token::SemiColon
                    | Token::Prime
            ) =>
            {
                false
            }

            // Newlines are never automatically followed by whitespace.
            (Token::SourceNewline | Token::Newline, _) => false,

            // All other tokens can be delimited by whitespace.
            _ => true,
        }
    }
}
