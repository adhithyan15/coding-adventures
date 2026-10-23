//! # Parser Grammar — parsing and validating `.grammar` files (EBNF notation).
//!
//! A `.grammar` file describes the syntactic structure of a programming language
//! using EBNF (Extended Backus-Naur Form). Where a `.tokens` file says "these
//! are the words," a `.grammar` file says "these are the sentences."
//!
//! # EBNF: a brief history
//!
//! BNF (Backus-Naur Form) was invented in the late 1950s by John Backus and
//! Peter Naur to describe the syntax of ALGOL 60. It was one of the first
//! formal notations for programming language grammars. EBNF extends BNF with
//! three conveniences:
//!
//! ```text
//! { x }   — zero or more repetitions of x (replaces recursive rules)
//! [ x ]   — optional x (shorthand for x | epsilon)
//! ( x )   — grouping (to clarify precedence in alternations)
//! ```
//!
//! These extensions do not add any theoretical power — anything expressible in
//! EBNF can be written in plain BNF — but they make grammars dramatically more
//! readable. Compare:
//!
//! ```text
//! BNF:   statements ::= <empty> | statement statements
//! EBNF:  statements = { statement } ;
//! ```
//!
//! # The recursive descent parser
//!
//! This module contains a hand-written recursive descent parser for the EBNF
//! notation used in `.grammar` files. This is the "chicken-and-egg" solution:
//! we need a parser to read grammar files, so we write one by hand.
//!
//! A recursive descent parser works by having one function per grammar rule.
//! Each function:
//!   1. Looks at the current token (character or word)
//!   2. Decides which alternative to take
//!   3. Calls other parsing functions as needed
//!   4. Returns an AST node
//!
//! For our EBNF parser, the grammar of the grammar (the "meta-grammar") is:
//!
//! ```text
//! grammar_file  = { rule } ;
//! rule          = rule_name "=" body ";" ;
//! body          = sequence { "|" sequence } ;
//! sequence      = { element } ;
//! element       = rule_ref | token_ref | literal | predicate
//!               | "{" body "}"
//!               | "[" body "]"
//!               | "(" body ")" ;
//! rule_ref      = lowercase_identifier ;
//! token_ref     = UPPERCASE_IDENTIFIER ;
//! literal       = '"' characters '"' ;
//! predicate     = "@" lowercase_identifier ;   (F11 adjacency predicates)
//! ```
//!
//! Each level of this meta-grammar becomes a method in our parser.
//!
//! # Why Rust enums are perfect here
//!
//! In Go, we used an interface with a marker method to simulate a sum type.
//! In TypeScript, we used discriminated unions with a `type` field.
//! In Rust, we have **real algebraic data types** via `enum`. Each variant
//! carries its own data, and the compiler enforces exhaustive matching.
//! This is the cleanest representation of all the implementations.

use std::collections::HashSet;
use std::fmt;

// ===========================================================================
// Error type
// ===========================================================================

/// Error returned when a `.grammar` file cannot be parsed.
///
/// Includes the 1-based line number where the problem occurred.
#[derive(Debug, Clone, PartialEq)]
pub struct ParserGrammarError {
    pub message: String,
    pub line_number: usize,
}

impl fmt::Display for ParserGrammarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Line {}: {}", self.line_number, self.message)
    }
}

impl std::error::Error for ParserGrammarError {}

// ===========================================================================
// AST node types (the "grammar elements")
// ===========================================================================
// These variants form a tree that represents the parsed body of a grammar
// rule. Together they can express anything that EBNF can express.
//
// Rust's enum with data is the perfect fit for this: each variant carries
// exactly the data it needs, the compiler enforces exhaustive matching,
// and there is no need for a marker trait or discriminant field.
// ===========================================================================

/// A node in the grammar's abstract syntax tree.
///
/// Each variant corresponds to one kind of EBNF construct:
///
/// | Variant          | EBNF syntax       | Meaning                          |
/// |------------------|--------------------|----------------------------------|
/// | `RuleReference`  | `expression`       | Reference to another rule        |
/// | `TokenReference` | `NUMBER`           | Reference to a token type        |
/// | `Literal`        | `"+"` or `"if"`    | A literal string match           |
/// | `Sequence`       | `A B C`            | Elements in order                |
/// | `Alternation`    | `A \| B \| C`      | Choice between alternatives      |
/// | `Repetition`     | `{ A }`            | Zero or more of A                |
/// | `Optional`       | `[ A ]`            | Zero or one of A                 |
/// | `Group`          | `( A )`            | Explicit grouping                |
#[derive(Debug, Clone, PartialEq)]
pub enum GrammarElement {
    /// A reference to another grammar rule (lowercase name).
    ///
    /// In EBNF, `expression` refers to the rule named "expression". Rule
    /// references are lowercase by convention, which distinguishes them
    /// from token references (UPPERCASE).
    RuleReference { name: String },

    /// A reference to a token type (UPPERCASE name).
    ///
    /// In EBNF, `NUMBER` refers to the token type NUMBER from the `.tokens`
    /// file. Token references are UPPERCASE by convention.
    TokenReference { name: String },

    /// A literal string match, written as `"..."` in EBNF.
    ///
    /// Less common than token references — usually you define tokens in
    /// the `.tokens` file and reference them by name. But sometimes it is
    /// convenient to write a literal directly in the grammar.
    Literal { value: String },

    /// A sequence of elements that must appear in order.
    ///
    /// In EBNF, juxtaposition means sequencing: `A B C` means "A followed
    /// by B followed by C." This is the most fundamental combinator.
    Sequence { elements: Vec<GrammarElement> },

    /// A choice between alternatives, written with `|` in EBNF.
    ///
    /// `A | B | C` means "either A, or B, or C." The parser tries each
    /// alternative in order.
    Alternation { choices: Vec<GrammarElement> },

    /// Zero-or-more repetition, written as `{ x }` in EBNF.
    ///
    /// `{ statement }` means "zero or more statements." This replaces
    /// the recursive rules that plain BNF requires.
    Repetition { element: Box<GrammarElement> },

    /// Optional element, written as `[ x ]` in EBNF.
    ///
    /// `[ ELSE block ]` means "optionally an ELSE followed by a block."
    /// Equivalent to `x | epsilon` in BNF.
    Optional { element: Box<GrammarElement> },

    /// Explicit grouping, written as `( x )` in EBNF.
    ///
    /// `( PLUS | MINUS )` groups the alternation so it can be used as a
    /// single element in a sequence: `term { ( PLUS | MINUS ) term }`.
    Group { element: Box<GrammarElement> },

    /// Positive lookahead predicate, written as `&element` in the grammar.
    ///
    /// Succeeds (producing no AST children) if element matches at the current
    /// position. Fails if element does not match. Does NOT consume input.
    ///
    /// This is a standard PEG operator. Example:
    ///
    /// ```text
    /// arrow_params = LPAREN [ param_list ] RPAREN &ARROW ;
    /// ```
    ///
    /// The `&ARROW` predicate checks that `=>` follows without consuming it.
    PositiveLookahead { element: Box<GrammarElement> },

    /// Negative lookahead predicate, written as `!element` in the grammar.
    ///
    /// Succeeds (producing no AST children) if element does NOT match at
    /// the current position. Fails if element does match. Does NOT consume
    /// input.
    ///
    /// Example:
    ///
    /// ```text
    /// postfix_expr = primary !NEWLINE ( "++" | "--" ) ;
    /// ```
    ///
    /// The `!NEWLINE` predicate ensures no line terminator before `++` or `--`.
    NegativeLookahead { element: Box<GrammarElement> },

    /// One-or-more repetition, written as `{ element }+` in the grammar.
    ///
    /// Like zero-or-more `{ element }` but requires at least one match.
    /// Fails if the first match fails.
    OneOrMore { element: Box<GrammarElement> },

    /// Separated repetition, written as `{ element // separator }` in the
    /// grammar.
    ///
    /// Matches zero or more occurrences of `element` separated by `separator`.
    /// With `at_least_one = true` (from `{ element // separator }+`), requires
    /// at least one occurrence.
    ///
    /// Example:
    /// ```text
    /// args = { expression // COMMA } ;
    /// ```
    /// is equivalent to:
    /// ```text
    /// args = [ expression { COMMA expression } ] ;
    /// ```
    SeparatedRepetition {
        element: Box<GrammarElement>,
        separator: Box<GrammarElement>,
        at_least_one: bool,
    },

    /// A token-adjacency predicate, written `@name` in the grammar (F11,
    /// LANG84 extension E6).
    ///
    /// Like `&x` and `!x` it is **zero-width**: it consumes no token and adds
    /// no AST children. Instead of trying to match an element, it asks a
    /// question about the *gap* in front of the next token, using the flags
    /// the lexer recorded on that token:
    ///
    /// ```text
    /// return_statement = "return" [ @no_newline_before expression ] SEMICOLON ;
    /// command_arg      = @space_before MINUS @no_space_before primary ;
    /// ```
    ///
    /// The first rule is ECMAScript's `[no LineTerminator here]`; the second
    /// is Ruby's `f -1` (a space before the minus, none after it). See
    /// [`AdjacencyPredicate`] for the full list.
    AdjacencyPredicate { predicate: AdjacencyPredicate },
}

// ===========================================================================
// Token-adjacency predicates (F11 / LANG84 E6)
// ===========================================================================
// The lexer throws whitespace and comments away, but it remembers two facts
// about each gap it threw away, as bits on the token that follows the gap:
//
//   source        tokens and the gap facts recorded on them
//   ------        -----------------------------------------
//   f -1          f   -(space)   1
//   f - 1         f   -(space)   1(space)
//   f-1           f   -          1
//   return⏎x      return         x(space, newline)
//
// A predicate is a yes/no question about one of those gaps. "Before" asks
// about the gap in front of the next token, "after next" about the gap in
// front of the token after that. Each question exists in both polarities so
// that a grammar can say what it means without a double negative:
//
//   @space_before       @no_space_before        (trivia before next token?)
//   @newline_before     @no_newline_before      (line break before next?)
//   @space_after_next   @no_space_after_next    (trivia before the one after?)
//
// `!@p` (negative lookahead over a predicate) is always equal to `@no_p`.
// ===========================================================================

/// Which gap fact an [`GrammarElement::AdjacencyPredicate`] tests.
///
/// The grammar-tools crate does not depend on the lexer, so the answer is
/// computed from two booleans — "was there trivia?" and "was there a line
/// terminator?" — which the parser reads from the token's flag bits and
/// hands to [`AdjacencyPredicate::holds`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AdjacencyPredicate {
    /// `@space_before`: whitespace, a comment or a line break precedes the
    /// next token.
    SpaceBefore,
    /// `@no_space_before`: the next token touches the previous one.
    NoSpaceBefore,
    /// `@newline_before`: a line terminator precedes the next token.
    NewlineBefore,
    /// `@no_newline_before`: no line terminator precedes the next token
    /// (ECMAScript's `[no LineTerminator here]`).
    NoNewlineBefore,
    /// `@space_after_next`: trivia precedes the token *after* the next one.
    SpaceAfterNext,
    /// `@no_space_after_next`: the token after the next one touches the
    /// next one.
    NoSpaceAfterNext,
}

impl AdjacencyPredicate {
    /// Every predicate, in the order the documentation lists them.
    pub const ALL: [AdjacencyPredicate; 6] = [
        AdjacencyPredicate::SpaceBefore,
        AdjacencyPredicate::NoSpaceBefore,
        AdjacencyPredicate::NewlineBefore,
        AdjacencyPredicate::NoNewlineBefore,
        AdjacencyPredicate::SpaceAfterNext,
        AdjacencyPredicate::NoSpaceAfterNext,
    ];

    /// The name as written after `@` in a `.grammar` file.
    pub fn name(self) -> &'static str {
        match self {
            AdjacencyPredicate::SpaceBefore => "space_before",
            AdjacencyPredicate::NoSpaceBefore => "no_space_before",
            AdjacencyPredicate::NewlineBefore => "newline_before",
            AdjacencyPredicate::NoNewlineBefore => "no_newline_before",
            AdjacencyPredicate::SpaceAfterNext => "space_after_next",
            AdjacencyPredicate::NoSpaceAfterNext => "no_space_after_next",
        }
    }

    /// The Rust variant name, used by the grammar compiler.
    pub fn variant_name(self) -> &'static str {
        match self {
            AdjacencyPredicate::SpaceBefore => "SpaceBefore",
            AdjacencyPredicate::NoSpaceBefore => "NoSpaceBefore",
            AdjacencyPredicate::NewlineBefore => "NewlineBefore",
            AdjacencyPredicate::NoNewlineBefore => "NoNewlineBefore",
            AdjacencyPredicate::SpaceAfterNext => "SpaceAfterNext",
            AdjacencyPredicate::NoSpaceAfterNext => "NoSpaceAfterNext",
        }
    }

    /// Look a predicate up by its `.grammar` name (without the `@`).
    pub fn from_name(name: &str) -> Option<AdjacencyPredicate> {
        AdjacencyPredicate::ALL.into_iter().find(|p| p.name() == name)
    }

    /// Whether the predicate inspects the token *after* the next token
    /// rather than the next token itself.
    pub fn reads_after_next(self) -> bool {
        matches!(
            self,
            AdjacencyPredicate::SpaceAfterNext | AdjacencyPredicate::NoSpaceAfterNext
        )
    }

    /// Decide the predicate from the facts about the inspected gap.
    ///
    /// | predicate              | true when       |
    /// |------------------------|-----------------|
    /// | `space_before`         | `space`         |
    /// | `no_space_before`      | `!space`        |
    /// | `newline_before`       | `newline`       |
    /// | `no_newline_before`    | `!newline`      |
    /// | `space_after_next`     | `space`         |
    /// | `no_space_after_next`  | `!space`        |
    ///
    /// (`newline` implies `space` for lexer-produced tokens, because a line
    /// break is itself trivia, but this function does not rely on that.)
    pub fn holds(self, space: bool, newline: bool) -> bool {
        match self {
            AdjacencyPredicate::SpaceBefore | AdjacencyPredicate::SpaceAfterNext => space,
            AdjacencyPredicate::NoSpaceBefore | AdjacencyPredicate::NoSpaceAfterNext => !space,
            AdjacencyPredicate::NewlineBefore => newline,
            AdjacencyPredicate::NoNewlineBefore => !newline,
        }
    }
}

impl fmt::Display for AdjacencyPredicate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@{}", self.name())
    }
}

// ===========================================================================
// Data model for the complete grammar
// ===========================================================================

/// A single rule from a `.grammar` file.
///
/// # Fields
///
/// - `name` — The rule name (lowercase identifier).
/// - `body` — The parsed EBNF body as a tree of [`GrammarElement`] nodes.
/// - `line_number` — The 1-based line number where this rule appeared.
#[derive(Debug, Clone, PartialEq)]
pub struct GrammarRule {
    pub name: String,
    pub body: GrammarElement,
    pub line_number: usize,
}

/// The complete contents of a parsed `.grammar` file.
///
/// # Fields
///
/// - `rules` — Ordered list of grammar rules. The first rule is the
///   entry point (start symbol) of the grammar.
/// - `version` — The grammar version number, set by the `# @version N`
///   magic comment at the top of the file. A value of `0` means "no
///   version declared" (use latest semantics).
///
/// # Magic comments
///
/// Lines of the form `# @key value` before or between rules carry
/// structured metadata. Unknown keys are silently ignored. Example:
///
/// ```text
/// # @version 3
/// expression = term { PLUS term } ;
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ParserGrammar {
    pub rules: Vec<GrammarRule>,
    /// Grammar version declared via `# @version N`. Zero means unset.
    pub version: u32,
}

// ===========================================================================
// AST traversal helpers
// ===========================================================================

/// Return the set of all defined rule names.
pub fn rule_names(grammar: &ParserGrammar) -> HashSet<String> {
    grammar.rules.iter().map(|r| r.name.clone()).collect()
}

/// Return all UPPERCASE token names referenced anywhere in the grammar.
///
/// These should correspond to token names in the `.tokens` file.
pub fn grammar_token_references(grammar: &ParserGrammar) -> HashSet<String> {
    let mut refs = HashSet::new();
    for rule in &grammar.rules {
        collect_token_refs(&rule.body, &mut refs);
    }
    refs
}

/// Return all lowercase rule names referenced anywhere in the grammar.
///
/// These should correspond to other rule names in this grammar.
pub fn grammar_rule_references(grammar: &ParserGrammar) -> HashSet<String> {
    let mut refs = HashSet::new();
    for rule in &grammar.rules {
        collect_rule_refs(&rule.body, &mut refs);
    }
    refs
}

// ===========================================================================
// Internal AST walkers
// ===========================================================================
// These functions walk the grammar element tree to collect references.
// They use Rust's `match` on the enum — the compiler guarantees we handle
// every variant, which is safer than the Go interface or TypeScript switch.
// ===========================================================================

fn collect_token_refs(node: &GrammarElement, refs: &mut HashSet<String>) {
    match node {
        GrammarElement::TokenReference { name } => {
            refs.insert(name.clone());
        }
        GrammarElement::RuleReference { .. }
        | GrammarElement::Literal { .. }
        | GrammarElement::AdjacencyPredicate { .. } => {
            // Leaf nodes that do not contain token references.
        }
        GrammarElement::Sequence { elements } => {
            for e in elements {
                collect_token_refs(e, refs);
            }
        }
        GrammarElement::Alternation { choices } => {
            for c in choices {
                collect_token_refs(c, refs);
            }
        }
        GrammarElement::Repetition { element }
        | GrammarElement::Optional { element }
        | GrammarElement::Group { element }
        | GrammarElement::PositiveLookahead { element }
        | GrammarElement::NegativeLookahead { element }
        | GrammarElement::OneOrMore { element } => {
            collect_token_refs(element, refs);
        }
        GrammarElement::SeparatedRepetition { element, separator, .. } => {
            collect_token_refs(element, refs);
            collect_token_refs(separator, refs);
        }
    }
}

fn collect_rule_refs(node: &GrammarElement, refs: &mut HashSet<String>) {
    match node {
        GrammarElement::RuleReference { name } => {
            refs.insert(name.clone());
        }
        GrammarElement::TokenReference { .. }
        | GrammarElement::Literal { .. }
        | GrammarElement::AdjacencyPredicate { .. } => {
            // Leaf nodes that do not contain rule references.
        }
        GrammarElement::Sequence { elements } => {
            for e in elements {
                collect_rule_refs(e, refs);
            }
        }
        GrammarElement::Alternation { choices } => {
            for c in choices {
                collect_rule_refs(c, refs);
            }
        }
        GrammarElement::Repetition { element }
        | GrammarElement::Optional { element }
        | GrammarElement::Group { element }
        | GrammarElement::PositiveLookahead { element }
        | GrammarElement::NegativeLookahead { element }
        | GrammarElement::OneOrMore { element } => {
            collect_rule_refs(element, refs);
        }
        GrammarElement::SeparatedRepetition { element, separator, .. } => {
            collect_rule_refs(element, refs);
            collect_rule_refs(separator, refs);
        }
    }
}

// ===========================================================================
// Tokenizer for .grammar files
// ===========================================================================
// Before we can parse the EBNF, we need to break the raw text into tokens.
// This is a simple hand-written tokenizer — much simpler than the lexers we
// are trying to generate, because the grammar notation uses only a few token
// types.
//
// Token types:
//   IDENT     — an identifier (rule name or token reference)
//   STRING    — a quoted literal "..."
//   EQUALS    — the = sign separating rule name from body
//   SEMI      — the ; terminating a rule
//   PIPE      — the | alternation operator
//   LBRACE / RBRACE — { }
//   LBRACKET / RBRACKET — [ ]
//   LPAREN / RPAREN — ( )
//   AT        — an adjacency predicate `@name` (F11); value = the name
//   EOF       — end of input
// ===========================================================================

#[derive(Debug, Clone)]
struct Token {
    kind: TokenKind,
    value: String,
    line: usize,
}

/// The kinds of tokens that appear in `.grammar` files.
///
/// Using an enum (rather than strings like the Go/TypeScript versions)
/// gives us compile-time exhaustiveness checking: if we add a new token
/// kind, the compiler will tell us every `match` that needs updating.
#[derive(Debug, Clone, PartialEq)]
enum TokenKind {
    Ident,
    StringLiteral,
    Equals,
    Semi,
    Pipe,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    LParen,
    RParen,
    /// `&` — prefix for positive lookahead predicates.
    Ampersand,
    /// `!` — prefix for negative lookahead predicates.
    Bang,
    /// `+` — suffix for one-or-more repetition.
    Plus,
    /// `//` — separator operator inside repetition braces.
    DoubleSlash,
    /// `@name` — a token-adjacency predicate (F11). The token's value is the
    /// name without the `@`, which may be empty if nothing followed it.
    At,
    Eof,
}

fn tokenize_grammar(source: &str) -> Result<Vec<Token>, ParserGrammarError> {
    let mut tokens = Vec::new();
    let lines: Vec<&str> = source.split('\n').collect();

    for (line_idx, raw_line) in lines.iter().enumerate() {
        let line_number = line_idx + 1;
        let line = raw_line.trim_end();
        let stripped = line.trim();

        // Skip blanks and comments.
        if stripped.is_empty() || stripped.starts_with('#') {
            continue;
        }

        let bytes = line.as_bytes();
        let mut i = 0;

        while i < bytes.len() {
            let ch = bytes[i];

            // Skip whitespace.
            if ch == b' ' || ch == b'\t' {
                i += 1;
                continue;
            }

            // Skip inline comments.
            if ch == b'#' {
                break; // Rest of line is a comment.
            }

            // Single-character tokens.
            // Each punctuation character in the EBNF notation maps to
            // exactly one token kind. This is the simplest possible
            // tokenizer design.
            match ch {
                b'=' => {
                    tokens.push(Token { kind: TokenKind::Equals, value: "=".to_string(), line: line_number });
                    i += 1;
                }
                b';' => {
                    tokens.push(Token { kind: TokenKind::Semi, value: ";".to_string(), line: line_number });
                    i += 1;
                }
                b'|' => {
                    tokens.push(Token { kind: TokenKind::Pipe, value: "|".to_string(), line: line_number });
                    i += 1;
                }
                b'{' => {
                    tokens.push(Token { kind: TokenKind::LBrace, value: "{".to_string(), line: line_number });
                    i += 1;
                }
                b'}' => {
                    tokens.push(Token { kind: TokenKind::RBrace, value: "}".to_string(), line: line_number });
                    i += 1;
                }
                b'[' => {
                    tokens.push(Token { kind: TokenKind::LBracket, value: "[".to_string(), line: line_number });
                    i += 1;
                }
                b']' => {
                    tokens.push(Token { kind: TokenKind::RBracket, value: "]".to_string(), line: line_number });
                    i += 1;
                }
                b'(' => {
                    tokens.push(Token { kind: TokenKind::LParen, value: "(".to_string(), line: line_number });
                    i += 1;
                }
                b')' => {
                    tokens.push(Token { kind: TokenKind::RParen, value: ")".to_string(), line: line_number });
                    i += 1;
                }
                // Lookahead predicates.
                b'&' => {
                    tokens.push(Token { kind: TokenKind::Ampersand, value: "&".to_string(), line: line_number });
                    i += 1;
                }
                b'!' => {
                    tokens.push(Token { kind: TokenKind::Bang, value: "!".to_string(), line: line_number });
                    i += 1;
                }
                // One-or-more suffix.
                b'+' => {
                    tokens.push(Token { kind: TokenKind::Plus, value: "+".to_string(), line: line_number });
                    i += 1;
                }
                // Adjacency predicate: `@` immediately followed by a name.
                // The name is checked by the parser, which reports unknown
                // names with the list of valid ones.
                b'@' => {
                    let start = i + 1;
                    let mut j = start;
                    while j < bytes.len()
                        && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_')
                    {
                        j += 1;
                    }
                    let value = String::from_utf8_lossy(&bytes[start..j]).to_string();
                    // `@asi(T, C…)` is the other half of LANG84 E6. Its
                    // argument list (with commas) is not even tokenizable
                    // yet, so say so here rather than fail on the comma.
                    if value == "asi" {
                        return Err(ParserGrammarError {
                            message: "'@asi' (automatic semicolon insertion, LANG84 E6) is \
                                      not implemented yet; only the adjacency predicates are"
                                .to_string(),
                            line_number,
                        });
                    }
                    tokens.push(Token { kind: TokenKind::At, value, line: line_number });
                    i = j;
                }
                // Separator operator (// inside repetition braces).
                b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
                    tokens.push(Token { kind: TokenKind::DoubleSlash, value: "//".to_string(), line: line_number });
                    i += 2;
                }
                // Quoted string literal.
                // We scan forward until we find the closing quote,
                // handling escaped characters along the way.
                b'"' => {
                    let mut j = i + 1;
                    while j < bytes.len() && bytes[j] != b'"' {
                        if bytes[j] == b'\\' {
                            j += 1; // Skip escaped character.
                        }
                        j += 1;
                    }
                    if j >= bytes.len() {
                        return Err(ParserGrammarError {
                            message: "Unterminated string literal".to_string(),
                            line_number,
                        });
                    }
                    // Store the string content without quotes.
                    let value = String::from_utf8_lossy(&bytes[i + 1..j]).to_string();
                    tokens.push(Token { kind: TokenKind::StringLiteral, value, line: line_number });
                    i = j + 1;
                }
                // Identifier (rule name or token reference).
                // Identifiers start with a letter or underscore, followed
                // by letters, digits, or underscores.
                _ if ch.is_ascii_alphabetic() || ch == b'_' => {
                    let start = i;
                    while i < bytes.len()
                        && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_')
                    {
                        i += 1;
                    }
                    let value = String::from_utf8_lossy(&bytes[start..i]).to_string();
                    tokens.push(Token { kind: TokenKind::Ident, value, line: line_number });
                }
                _ => {
                    return Err(ParserGrammarError {
                        message: format!("Unexpected character: '{}'", ch as char),
                        line_number,
                    });
                }
            }
        }
    }

    // Sentinel token that simplifies the parser — it can always peek()
    // without worrying about running off the end.
    tokens.push(Token {
        kind: TokenKind::Eof,
        value: String::new(),
        line: lines.len(),
    });

    Ok(tokens)
}

// ===========================================================================
// Recursive descent parser for EBNF
// ===========================================================================
// The parser consumes the token list produced by tokenize_grammar and
// builds a tree of GrammarElement nodes. Each method corresponds to one
// level of the meta-grammar:
//
//   parse            ->  { rule }
//   parse_rule       ->  name "=" body ";"
//   parse_body       ->  sequence { "|" sequence }
//   parse_sequence   ->  { element }
//   parse_element    ->  ident | string | "{" body "}" | "[" body "]" | "(" body ")"
// ===========================================================================

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    /// Look at the current token without consuming it.
    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    /// Consume and return the current token.
    fn advance(&mut self) -> Token {
        let tok = self.tokens[self.pos].clone();
        self.pos += 1;
        tok
    }

    /// Consume a token of the expected kind, or return an error.
    fn expect(&mut self, kind: TokenKind) -> Result<Token, ParserGrammarError> {
        let tok = self.advance();
        if tok.kind != kind {
            return Err(ParserGrammarError {
                message: format!("Expected {:?}, got {:?} ('{}')", kind, tok.kind, tok.value),
                line_number: tok.line,
            });
        }
        Ok(tok)
    }

    // --- Top level: grammar file = { rule } ---

    /// Parse all rules in the grammar file.
    fn parse(&mut self) -> Result<Vec<GrammarRule>, ParserGrammarError> {
        let mut rules = Vec::new();
        while self.peek().kind != TokenKind::Eof {
            rules.push(self.parse_rule()?);
        }
        Ok(rules)
    }

    // --- rule = name "=" body ";" ---

    /// Parse a single grammar rule.
    fn parse_rule(&mut self) -> Result<GrammarRule, ParserGrammarError> {
        let name_tok = self.expect(TokenKind::Ident)?;
        self.expect(TokenKind::Equals)?;
        let body = self.parse_body()?;
        self.expect(TokenKind::Semi)?;
        Ok(GrammarRule {
            name: name_tok.value,
            body,
            line_number: name_tok.line,
        })
    }

    // --- body = sequence { "|" sequence } ---

    /// Parse alternation: one or more sequences separated by '|'.
    ///
    /// If there is only one sequence (no '|'), we return it directly
    /// rather than wrapping it in an Alternation node. This keeps the
    /// AST clean — a rule like `factor = NUMBER ;` produces a simple
    /// TokenReference, not an Alternation with one choice containing a
    /// Sequence with one element.
    fn parse_body(&mut self) -> Result<GrammarElement, ParserGrammarError> {
        let first = self.parse_sequence()?;
        let mut alternatives = vec![first];

        while self.peek().kind == TokenKind::Pipe {
            self.advance(); // consume '|'
            alternatives.push(self.parse_sequence()?);
        }

        if alternatives.len() == 1 {
            Ok(alternatives.into_iter().next().unwrap())
        } else {
            Ok(GrammarElement::Alternation { choices: alternatives })
        }
    }

    // --- sequence = { element } ---

    /// Parse a sequence of elements.
    ///
    /// A sequence ends when we hit something that cannot start an element:
    /// '|', ';', '}', ']', ')' or EOF. If the sequence has only one
    /// element, we return it directly (no Sequence wrapper).
    fn parse_sequence(&mut self) -> Result<GrammarElement, ParserGrammarError> {
        let mut elements = Vec::new();

        loop {
            match self.peek().kind {
                TokenKind::Pipe
                | TokenKind::Semi
                | TokenKind::RBrace
                | TokenKind::RBracket
                | TokenKind::RParen
                | TokenKind::DoubleSlash
                | TokenKind::Eof => break,
                _ => elements.push(self.parse_element()?),
            }
        }

        if elements.is_empty() {
            return Err(ParserGrammarError {
                message: "Expected at least one element in sequence".to_string(),
                line_number: self.peek().line,
            });
        }

        if elements.len() == 1 {
            Ok(elements.into_iter().next().unwrap())
        } else {
            Ok(GrammarElement::Sequence { elements })
        }
    }

    // --- element = ident | string | "{" body "}" | "[" body "]" | "(" body ")" ---

    /// Parse a single grammar element.
    ///
    /// This is where the recursive descent happens: braces, brackets,
    /// and parentheses cause us to recurse back into parse_body.
    fn parse_element(&mut self) -> Result<GrammarElement, ParserGrammarError> {
        let tok = self.peek().clone();

        match tok.kind {
            // --- Lookahead predicates: & (positive) and ! (negative) ---
            // These are prefix operators that match without consuming input.
            TokenKind::Ampersand => {
                self.advance();
                let inner = self.parse_element()?;
                Ok(GrammarElement::PositiveLookahead { element: Box::new(inner) })
            }

            TokenKind::Bang => {
                self.advance();
                let inner = self.parse_element()?;
                Ok(GrammarElement::NegativeLookahead { element: Box::new(inner) })
            }

            // Identifier: could be a rule reference (lowercase) or
            // a token reference (UPPERCASE).
            TokenKind::Ident => {
                self.advance();
                // UPPERCASE names starting with A-Z are token references.
                // lowercase names are rule references.
                let first_char = tok.value.chars().next().unwrap_or('a');
                if first_char.is_ascii_uppercase() && tok.value == tok.value.to_uppercase() {
                    Ok(GrammarElement::TokenReference { name: tok.value })
                } else {
                    Ok(GrammarElement::RuleReference { name: tok.value })
                }
            }

            // Quoted string literal.
            TokenKind::StringLiteral => {
                self.advance();
                Ok(GrammarElement::Literal { value: tok.value })
            }

            // `@name` — a zero-width token-adjacency predicate (F11).
            TokenKind::At => {
                self.advance();
                if let Some(predicate) = AdjacencyPredicate::from_name(&tok.value) {
                    return Ok(GrammarElement::AdjacencyPredicate { predicate });
                }
                let valid: Vec<String> =
                    AdjacencyPredicate::ALL.iter().map(|p| p.to_string()).collect();
                let message = if tok.value.is_empty() {
                    format!("Expected a predicate name after '@' (one of {})", valid.join(", "))
                } else {
                    format!(
                        "Unknown predicate '@{}' (expected one of {})",
                        tok.value,
                        valid.join(", ")
                    )
                };
                Err(ParserGrammarError { message, line_number: tok.line })
            }

            // { body } — zero-or-more repetition, with extensions:
            //   { body }+         — one-or-more repetition
            //   { body // sep }   — separated repetition (zero-or-more)
            //   { body // sep }+  — separated repetition (one-or-more)
            TokenKind::LBrace => {
                self.advance();
                let body = self.parse_body()?;

                // Check for separator: { element // separator }
                if self.peek().kind == TokenKind::DoubleSlash {
                    self.advance(); // consume //
                    let separator = self.parse_body()?;
                    self.expect(TokenKind::RBrace)?;
                    // Check for + suffix (one-or-more)
                    let at_least_one = self.peek().kind == TokenKind::Plus;
                    if at_least_one {
                        self.advance();
                    }
                    return Ok(GrammarElement::SeparatedRepetition {
                        element: Box::new(body),
                        separator: Box::new(separator),
                        at_least_one,
                    });
                }

                self.expect(TokenKind::RBrace)?;
                // Check for + suffix: { element }+
                if self.peek().kind == TokenKind::Plus {
                    self.advance();
                    Ok(GrammarElement::OneOrMore { element: Box::new(body) })
                } else {
                    Ok(GrammarElement::Repetition { element: Box::new(body) })
                }
            }

            // [ body ] — optional.
            TokenKind::LBracket => {
                self.advance();
                let body = self.parse_body()?;
                self.expect(TokenKind::RBracket)?;
                Ok(GrammarElement::Optional { element: Box::new(body) })
            }

            // ( body ) — grouping.
            TokenKind::LParen => {
                self.advance();
                let body = self.parse_body()?;
                self.expect(TokenKind::RParen)?;
                Ok(GrammarElement::Group { element: Box::new(body) })
            }

            _ => Err(ParserGrammarError {
                message: format!("Unexpected token: {:?} ('{}')", tok.kind, tok.value),
                line_number: tok.line,
            }),
        }
    }
}

// ===========================================================================
// Public API
// ===========================================================================

/// Parse the text of a `.grammar` file into a [`ParserGrammar`].
///
/// This function tokenizes the source, then runs a recursive descent
/// parser over the token stream to produce an AST of grammar elements.
///
/// # Errors
///
/// Returns [`ParserGrammarError`] if the source cannot be parsed.
///
/// # Example
///
/// ```
/// use grammar_tools::parser_grammar::parse_parser_grammar;
///
/// let source = r#"
/// expression = term { PLUS term } ;
/// term       = NUMBER ;
/// "#;
///
/// let grammar = parse_parser_grammar(source).unwrap();
/// assert_eq!(grammar.rules.len(), 2);
/// assert_eq!(grammar.rules[0].name, "expression");
/// ```
pub fn parse_parser_grammar(source: &str) -> Result<ParserGrammar, ParserGrammarError> {
    // --- Pre-pass: scan source lines for magic comments ---
    //
    // Magic comments have the form `# @key value`. They must be processed
    // before we feed the source into the tokenizer, because the tokenizer
    // simply discards comment lines without inspecting their content.
    //
    // Parsing strategy (pure string ops, no regex):
    //   1. Iterate over lines, trim each one.
    //   2. If a line starts with '#', strip '#' and leading whitespace.
    //   3. If the next character is '@', extract key and value.
    //   4. Dispatch on key; unknown keys are silently ignored.
    let mut version: u32 = 0;
    for raw_line in source.split('\n') {
        let stripped = raw_line.trim();
        if let Some(after_hash) = stripped.strip_prefix('#') {
            let after_hash = after_hash.trim_start();
            if let Some(rest) = after_hash.strip_prefix('@') {
                // skip '@'
                let key_end = rest.find(|c: char| c.is_whitespace()).unwrap_or(rest.len());
                let key = &rest[..key_end];
                let value = rest[key_end..].trim();
                if key == "version" {
                    if let Ok(v) = value.parse::<u32>() {
                        version = v;
                    }
                    // Malformed version values are silently ignored.
                }
                // Unknown keys are silently ignored for forward-compatibility.
            }
        }
    }

    let tokens = tokenize_grammar(source)?;
    let mut parser = Parser::new(tokens);
    let rules = parser.parse()?;
    Ok(ParserGrammar { rules, version })
}

// ===========================================================================
// Validator
// ===========================================================================

/// Check a parsed [`ParserGrammar`] for common problems.
///
/// Validation checks:
///
/// - **Undefined rule references**: A lowercase name is used in a rule
///   body but never defined as a rule.
/// - **Undefined token references**: An UPPERCASE name is used but does
///   not appear in the provided `token_names_set`. (Only checked if
///   the set is provided.)
/// - **Duplicate rule names**: Two rules with the same name.
/// - **Non-lowercase rule names**: By convention, rule names are lowercase.
/// - **Unreachable rules**: A rule that is defined but never referenced
///   by any other rule. The first rule (start symbol) is exempt.
///
/// Returns a list of warning/error strings. An empty list means no issues.
pub fn validate_parser_grammar(
    grammar: &ParserGrammar,
    token_names_set: Option<&HashSet<String>>,
) -> Vec<String> {
    let mut issues = Vec::new();
    let defined = rule_names(grammar);
    let referenced_rules = grammar_rule_references(grammar);
    let referenced_tokens = grammar_token_references(grammar);

    // --- Duplicate rule names ---
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for rule in &grammar.rules {
        if let Some(&first_line) = seen.get(&rule.name) {
            issues.push(format!(
                "Line {}: Duplicate rule name '{}' (first defined on line {})",
                rule.line_number, rule.name, first_line
            ));
        } else {
            seen.insert(rule.name.clone(), rule.line_number);
        }
    }

    // --- Non-lowercase rule names ---
    for rule in &grammar.rules {
        if rule.name != rule.name.to_lowercase() {
            issues.push(format!(
                "Line {}: Rule name '{}' should be lowercase",
                rule.line_number, rule.name
            ));
        }
    }

    // --- Undefined rule references ---
    let mut sorted_rule_refs: Vec<&String> = referenced_rules.iter().collect();
    sorted_rule_refs.sort();
    for ref_name in sorted_rule_refs {
        if !defined.contains(ref_name.as_str()) {
            issues.push(format!("Undefined rule reference: '{}'", ref_name));
        }
    }

    // --- Undefined token references ---
    if let Some(token_set) = token_names_set {
        // Synthetic tokens are always valid — the lexer produces these
        // implicitly without needing a .tokens definition:
        //   NEWLINE — emitted at bare '\n' when skip pattern excludes newlines
        //   INDENT/DEDENT — emitted in indentation mode
        //   EOF — always emitted at end of input
        let synthetic_tokens: HashSet<String> = ["NEWLINE", "INDENT", "DEDENT", "EOF"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let mut sorted_token_refs: Vec<&String> = referenced_tokens.iter().collect();
        sorted_token_refs.sort();
        for ref_name in sorted_token_refs {
            if !token_set.contains(ref_name.as_str()) && !synthetic_tokens.contains(ref_name.as_str()) {
                issues.push(format!("Undefined token reference: '{}'", ref_name));
            }
        }
    }

    // --- Repetitions whose body can match without consuming (F11) ---
    //
    // `{ [ X ] }`, `{ @space_before | X }` or `{ p }` with `p = &X` can
    // succeed on an iteration that consumes nothing. The parser no longer
    // hangs on them (it stops the repetition after such an iteration), but
    // the body is almost certainly not what the author meant.
    //
    // Policy: an error when the nullable body involves an adjacency
    // predicate (new F11 syntax, so no existing grammar is affected); a
    // `Warning:` otherwise, because pre-F11 grammars in this repository
    // already contain nullable repetition bodies that parse correctly.
    let facts = RepetitionFacts {
        nullable: nullable_rules(grammar),
        with_predicate: predicate_rules(grammar),
    };
    for rule in &grammar.rules {
        check_nullable_repetitions(&rule.body, rule, &facts, &mut issues);
    }

    // --- Unreachable rules ---
    if !grammar.rules.is_empty() {
        let start_rule = &grammar.rules[0].name;
        for rule in &grammar.rules {
            if rule.name != *start_rule && !referenced_rules.contains(&rule.name) {
                issues.push(format!(
                    "Line {}: Rule '{}' is defined but never referenced (unreachable)",
                    rule.line_number, rule.name
                ));
            }
        }
    }

    issues
}

/// The rules that can succeed without consuming a token ("nullable" rules),
/// computed as a least fixpoint.
///
/// ```text
///   element                     nullable when
///   -------                     -------------
///   NAME, "x"                   never
///   @p, &x, !x                  always (zero-width)
///   [ x ], { x }                always (zero matches is a match)
///   { x }+, ( x )               x is
///   { x // s }                  always;  { x // s }+ : x is
///   a b c                       every element is
///   a | b | c                   any choice is
///   rule                        the rule's body is (fixpoint)
/// ```
///
/// Each round adds at least one rule or stops, so this takes at most
/// `rules.len() + 1` rounds.
pub fn nullable_rules(grammar: &ParserGrammar) -> HashSet<String> {
    let mut nullable = HashSet::new();
    loop {
        let mut changed = false;
        for rule in &grammar.rules {
            if !nullable.contains(&rule.name) && is_nullable(&rule.body, &nullable) {
                nullable.insert(rule.name.clone());
                changed = true;
            }
        }
        if !changed {
            return nullable;
        }
    }
}

/// Whether `element` can succeed without consuming a token, given the set
/// of nullable rules (see [`nullable_rules`]).
fn is_nullable(element: &GrammarElement, nullable: &HashSet<String>) -> bool {
    match element {
        GrammarElement::AdjacencyPredicate { .. }
        | GrammarElement::PositiveLookahead { .. }
        | GrammarElement::NegativeLookahead { .. }
        | GrammarElement::Optional { .. }
        | GrammarElement::Repetition { .. } => true,
        GrammarElement::OneOrMore { element } | GrammarElement::Group { element } => {
            is_nullable(element, nullable)
        }
        GrammarElement::SeparatedRepetition { element, at_least_one, .. } => {
            !*at_least_one || is_nullable(element, nullable)
        }
        GrammarElement::Sequence { elements } => elements.iter().all(|e| is_nullable(e, nullable)),
        GrammarElement::Alternation { choices } => choices.iter().any(|c| is_nullable(c, nullable)),
        GrammarElement::RuleReference { name } => nullable.contains(name),
        GrammarElement::TokenReference { .. } | GrammarElement::Literal { .. } => false,
    }
}

/// The rules whose body contains an adjacency predicate, directly or through
/// the rules it references (least fixpoint, like [`nullable_rules`]).
fn predicate_rules(grammar: &ParserGrammar) -> HashSet<String> {
    let mut with = HashSet::new();
    loop {
        let mut changed = false;
        for rule in &grammar.rules {
            if !with.contains(&rule.name) && contains_predicate(&rule.body, &with) {
                with.insert(rule.name.clone());
                changed = true;
            }
        }
        if !changed {
            return with;
        }
    }
}

/// Whether `element` contains an adjacency predicate, directly or through a
/// rule in `with`.
fn contains_predicate(element: &GrammarElement, with: &HashSet<String>) -> bool {
    match element {
        GrammarElement::AdjacencyPredicate { .. } => true,
        GrammarElement::RuleReference { name } => with.contains(name),
        GrammarElement::TokenReference { .. } | GrammarElement::Literal { .. } => false,
        GrammarElement::Sequence { elements } => elements.iter().any(|e| contains_predicate(e, with)),
        GrammarElement::Alternation { choices } => choices.iter().any(|c| contains_predicate(c, with)),
        GrammarElement::Repetition { element }
        | GrammarElement::Optional { element }
        | GrammarElement::Group { element }
        | GrammarElement::PositiveLookahead { element }
        | GrammarElement::NegativeLookahead { element }
        | GrammarElement::OneOrMore { element } => contains_predicate(element, with),
        GrammarElement::SeparatedRepetition { element, separator, .. } => {
            contains_predicate(element, with) || contains_predicate(separator, with)
        }
    }
}

/// Facts the repetition check needs about the whole grammar.
struct RepetitionFacts {
    nullable: HashSet<String>,
    with_predicate: HashSet<String>,
}

/// Walk `element` and report every repetition whose body can match without
/// consuming a token (see the F11 policy note in `validate_parser_grammar`).
fn check_nullable_repetitions(
    element: &GrammarElement,
    rule: &GrammarRule,
    facts: &RepetitionFacts,
    issues: &mut Vec<String>,
) {
    let mut report = |body: &GrammarElement, nullable_body: bool| {
        if !nullable_body {
            return;
        }
        let severity = if contains_predicate(body, &facts.with_predicate) {
            ""
        } else {
            "Warning: "
        };
        issues.push(format!(
            "{}Line {}: Rule '{}' repeats a body that can match without consuming a \
             token; the parser stops such a repetition after its first empty \
             iteration instead of looping",
            severity, rule.line_number, rule.name
        ));
    };
    match element {
        GrammarElement::Repetition { element: body }
        | GrammarElement::OneOrMore { element: body } => {
            report(body, is_nullable(body, &facts.nullable));
            check_nullable_repetitions(body, rule, facts, issues);
        }
        GrammarElement::SeparatedRepetition { element: body, separator, .. } => {
            let both = is_nullable(body, &facts.nullable) && is_nullable(separator, &facts.nullable);
            if both {
                let joined = GrammarElement::Sequence {
                    elements: vec![(**separator).clone(), (**body).clone()],
                };
                report(&joined, true);
            }
            check_nullable_repetitions(body, rule, facts, issues);
            check_nullable_repetitions(separator, rule, facts, issues);
        }
        GrammarElement::Sequence { elements } => {
            for e in elements {
                check_nullable_repetitions(e, rule, facts, issues);
            }
        }
        GrammarElement::Alternation { choices } => {
            for c in choices {
                check_nullable_repetitions(c, rule, facts, issues);
            }
        }
        GrammarElement::Group { element: inner }
        | GrammarElement::Optional { element: inner }
        | GrammarElement::PositiveLookahead { element: inner }
        | GrammarElement::NegativeLookahead { element: inner } => {
            check_nullable_repetitions(inner, rule, facts, issues);
        }
        GrammarElement::RuleReference { .. }
        | GrammarElement::TokenReference { .. }
        | GrammarElement::Literal { .. }
        | GrammarElement::AdjacencyPredicate { .. } => {}
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Parsing: happy paths
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_simple_rule() {
        // The simplest possible grammar: one rule, one token reference.
        let grammar = parse_parser_grammar("term = NUMBER ;").unwrap();
        assert_eq!(grammar.rules.len(), 1);
        assert_eq!(grammar.rules[0].name, "term");
        assert_eq!(
            grammar.rules[0].body,
            GrammarElement::TokenReference { name: "NUMBER".to_string() }
        );
    }

    #[test]
    fn test_parse_alternation() {
        // The pipe operator creates an Alternation node.
        let grammar = parse_parser_grammar("factor = NUMBER | NAME ;").unwrap();
        assert_eq!(grammar.rules.len(), 1);
        match &grammar.rules[0].body {
            GrammarElement::Alternation { choices } => {
                assert_eq!(choices.len(), 2);
                assert_eq!(choices[0], GrammarElement::TokenReference { name: "NUMBER".to_string() });
                assert_eq!(choices[1], GrammarElement::TokenReference { name: "NAME".to_string() });
            }
            other => panic!("Expected Alternation, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_sequence() {
        // Multiple elements in a row create a Sequence node.
        let grammar = parse_parser_grammar("assignment = NAME EQUALS expression ;").unwrap();
        match &grammar.rules[0].body {
            GrammarElement::Sequence { elements } => {
                assert_eq!(elements.len(), 3);
                assert_eq!(elements[0], GrammarElement::TokenReference { name: "NAME".to_string() });
                assert_eq!(elements[1], GrammarElement::TokenReference { name: "EQUALS".to_string() });
                assert_eq!(elements[2], GrammarElement::RuleReference { name: "expression".to_string() });
            }
            other => panic!("Expected Sequence, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_repetition() {
        // Braces create a Repetition node: { PLUS term }
        let grammar = parse_parser_grammar("expression = term { PLUS term } ;").unwrap();
        match &grammar.rules[0].body {
            GrammarElement::Sequence { elements } => {
                assert_eq!(elements.len(), 2);
                // The second element should be a Repetition.
                match &elements[1] {
                    GrammarElement::Repetition { element } => {
                        match element.as_ref() {
                            GrammarElement::Sequence { elements: inner } => {
                                assert_eq!(inner.len(), 2);
                            }
                            other => panic!("Expected inner Sequence, got {:?}", other),
                        }
                    }
                    other => panic!("Expected Repetition, got {:?}", other),
                }
            }
            other => panic!("Expected Sequence, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_optional() {
        // Brackets create an Optional node: [ ELSE block ]
        let grammar = parse_parser_grammar("if_stmt = IF expression [ ELSE block ] ;").unwrap();
        match &grammar.rules[0].body {
            GrammarElement::Sequence { elements } => {
                assert_eq!(elements.len(), 3);
                match &elements[2] {
                    GrammarElement::Optional { .. } => {} // correct
                    other => panic!("Expected Optional, got {:?}", other),
                }
            }
            other => panic!("Expected Sequence, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_group() {
        // Parentheses create a Group node: ( PLUS | MINUS )
        let grammar = parse_parser_grammar("expression = term { ( PLUS | MINUS ) term } ;").unwrap();
        match &grammar.rules[0].body {
            GrammarElement::Sequence { elements } => {
                assert_eq!(elements.len(), 2);
                // Inside the Repetition, first element should be a Group.
                match &elements[1] {
                    GrammarElement::Repetition { element } => {
                        match element.as_ref() {
                            GrammarElement::Sequence { elements: inner } => {
                                match &inner[0] {
                                    GrammarElement::Group { element } => {
                                        match element.as_ref() {
                                            GrammarElement::Alternation { choices } => {
                                                assert_eq!(choices.len(), 2);
                                            }
                                            other => panic!("Expected Alternation in Group, got {:?}", other),
                                        }
                                    }
                                    other => panic!("Expected Group, got {:?}", other),
                                }
                            }
                            other => panic!("Expected inner Sequence, got {:?}", other),
                        }
                    }
                    other => panic!("Expected Repetition, got {:?}", other),
                }
            }
            other => panic!("Expected Sequence, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_literal() {
        // String literals in the grammar are represented as Literal nodes.
        let grammar = parse_parser_grammar(r#"op = "+" | "-" ;"#).unwrap();
        match &grammar.rules[0].body {
            GrammarElement::Alternation { choices } => {
                assert_eq!(choices[0], GrammarElement::Literal { value: "+".to_string() });
                assert_eq!(choices[1], GrammarElement::Literal { value: "-".to_string() });
            }
            other => panic!("Expected Alternation, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_rule_reference() {
        // Lowercase identifiers are rule references.
        let grammar = parse_parser_grammar("program = { statement } ;").unwrap();
        match &grammar.rules[0].body {
            GrammarElement::Repetition { element } => {
                assert_eq!(
                    element.as_ref(),
                    &GrammarElement::RuleReference { name: "statement".to_string() }
                );
            }
            other => panic!("Expected Repetition, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_multiple_rules() {
        // A grammar file typically has many rules.
        let source = r#"
expression = term { PLUS term } ;
term = NUMBER ;
"#;
        let grammar = parse_parser_grammar(source).unwrap();
        assert_eq!(grammar.rules.len(), 2);
        assert_eq!(grammar.rules[0].name, "expression");
        assert_eq!(grammar.rules[1].name, "term");
    }

    #[test]
    fn test_parse_comments_and_blanks() {
        // Comments and blank lines are ignored in grammar files.
        let source = "# This is a comment\n\nterm = NUMBER ; # inline comment\n";
        let grammar = parse_parser_grammar(source).unwrap();
        assert_eq!(grammar.rules.len(), 1);
    }

    #[test]
    fn test_parse_empty_source() {
        // An empty source produces an empty grammar.
        let grammar = parse_parser_grammar("").unwrap();
        assert!(grammar.rules.is_empty());
    }

    #[test]
    fn test_parse_full_python_grammar() {
        // Parse the actual Python grammar to ensure our parser handles
        // a realistic grammar with sequences, alternations, repetitions,
        // optionals, and groups.
        let source = r#"
program      = { statement } ;
statement    = assignment | expression_stmt ;
assignment   = NAME EQUALS expression ;
expression_stmt = expression ;
expression   = term { ( PLUS | MINUS ) term } ;
term         = factor { ( STAR | SLASH ) factor } ;
factor       = NUMBER | STRING | NAME | LPAREN expression RPAREN ;
"#;
        let grammar = parse_parser_grammar(source).unwrap();
        assert_eq!(grammar.rules.len(), 7);
        assert_eq!(grammar.rules[0].name, "program");
        assert_eq!(grammar.rules[6].name, "factor");
    }

    // -----------------------------------------------------------------------
    // Parsing: error cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_error_missing_semicolon() {
        // A rule without a terminating ';' is an error.
        let result = parse_parser_grammar("term = NUMBER");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("Expected Semi"));
    }

    #[test]
    fn test_error_missing_equals() {
        // A rule without '=' between name and body.
        let result = parse_parser_grammar("term NUMBER ;");
        assert!(result.is_err());
    }

    #[test]
    fn test_error_unterminated_string() {
        // A string literal without a closing quote.
        let result = parse_parser_grammar(r#"op = "unclosed ;"#);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Unterminated"));
    }

    #[test]
    fn test_error_unexpected_character() {
        // Characters that are not part of the EBNF notation.
        // (`@` used to be the example here; F11 gave it a meaning.)
        let result = parse_parser_grammar("term = $ ;");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Unexpected character"));
    }

    #[test]
    fn test_error_unclosed_brace() {
        // An opening brace without a matching closing brace.
        let result = parse_parser_grammar("term = { NUMBER ;");
        assert!(result.is_err());
    }

    #[test]
    fn test_error_unclosed_bracket() {
        // An opening bracket without a matching closing bracket.
        let result = parse_parser_grammar("term = [ NUMBER ;");
        assert!(result.is_err());
    }

    #[test]
    fn test_error_unclosed_paren() {
        // An opening parenthesis without a matching closing one.
        let result = parse_parser_grammar("term = ( NUMBER ;");
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Validation
    // -----------------------------------------------------------------------

    #[test]
    fn test_validate_no_issues() {
        // A well-formed grammar produces no validation issues.
        let grammar = parse_parser_grammar(
            "expression = term ; term = NUMBER ;",
        ).unwrap();
        let token_set: HashSet<String> = ["NUMBER"].iter().map(|s| s.to_string()).collect();
        let issues = validate_parser_grammar(&grammar, Some(&token_set));
        assert!(issues.is_empty());
    }

    #[test]
    fn test_validate_undefined_rule_reference() {
        // Referencing a rule that does not exist is an error.
        let grammar = parse_parser_grammar("expression = term ;").unwrap();
        let issues = validate_parser_grammar(&grammar, None);
        assert!(issues.iter().any(|i| i.contains("Undefined rule reference") && i.contains("term")));
    }

    #[test]
    fn test_validate_undefined_token_reference() {
        // Referencing a token that does not exist is an error (when token set provided).
        let grammar = parse_parser_grammar("expression = NUMBER ;").unwrap();
        let token_set: HashSet<String> = HashSet::new(); // No tokens defined
        let issues = validate_parser_grammar(&grammar, Some(&token_set));
        assert!(issues.iter().any(|i| i.contains("Undefined token reference") && i.contains("NUMBER")));
    }

    #[test]
    fn test_validate_duplicate_rule_names() {
        // Two rules with the same name triggers a duplicate warning.
        let grammar = parse_parser_grammar(
            "term = NUMBER ; term = NAME ;",
        ).unwrap();
        let issues = validate_parser_grammar(&grammar, None);
        assert!(issues.iter().any(|i| i.contains("Duplicate rule name")));
    }

    #[test]
    fn test_validate_unreachable_rule() {
        // A rule that is never referenced (and is not the start rule) is unreachable.
        let grammar = parse_parser_grammar(
            "expression = NUMBER ; orphan = NAME ;",
        ).unwrap();
        let issues = validate_parser_grammar(&grammar, None);
        assert!(issues.iter().any(|i| i.contains("unreachable") && i.contains("orphan")));
    }

    #[test]
    fn test_validate_non_lowercase_rule_name() {
        // Rule names should be lowercase by convention.
        let grammar = parse_parser_grammar("Expression = NUMBER ;").unwrap();
        let issues = validate_parser_grammar(&grammar, None);
        assert!(issues.iter().any(|i| i.contains("should be lowercase")));
    }

    // -----------------------------------------------------------------------
    // Helper functions
    // -----------------------------------------------------------------------

    #[test]
    fn test_rule_names_helper() {
        let grammar = parse_parser_grammar(
            "expression = term ; term = NUMBER ;",
        ).unwrap();
        let names = rule_names(&grammar);
        assert!(names.contains("expression"));
        assert!(names.contains("term"));
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn test_grammar_token_references_helper() {
        let grammar = parse_parser_grammar(
            "expression = term { PLUS term } ; term = NUMBER ;",
        ).unwrap();
        let refs = grammar_token_references(&grammar);
        assert!(refs.contains("PLUS"));
        assert!(refs.contains("NUMBER"));
        assert_eq!(refs.len(), 2);
    }

    #[test]
    fn test_grammar_rule_references_helper() {
        let grammar = parse_parser_grammar(
            "expression = term { PLUS term } ; term = NUMBER ;",
        ).unwrap();
        let refs = grammar_rule_references(&grammar);
        assert!(refs.contains("term"));
        assert_eq!(refs.len(), 1);
    }

    // -----------------------------------------------------------------------
    // Magic comments: ParserGrammar
    // -----------------------------------------------------------------------

    #[test]
    fn test_parser_magic_comment_version() {
        // `# @version N` at the top of a .grammar file sets the version field.
        let source = "# @version 1\nterm = NUMBER ;";
        let grammar = parse_parser_grammar(source).unwrap();
        assert_eq!(grammar.version, 1);
        // Normal rules still parsed.
        assert_eq!(grammar.rules.len(), 1);
        assert_eq!(grammar.rules[0].name, "term");
    }

    #[test]
    fn test_parser_magic_comment_version_default() {
        // When no # @version line is present, version defaults to 0.
        let source = "term = NUMBER ;";
        let grammar = parse_parser_grammar(source).unwrap();
        assert_eq!(grammar.version, 0);
    }

    #[test]
    fn test_parser_magic_comment_unknown_key_silently_ignored() {
        // Unknown `# @key value` lines are silently ignored for
        // forward-compatibility.
        let source = "# @future_directive foo\nterm = NUMBER ;";
        let grammar = parse_parser_grammar(source).unwrap();
        assert_eq!(grammar.version, 0);
        assert_eq!(grammar.rules.len(), 1);
    }

    // -----------------------------------------------------------------------
    // F11: token-adjacency predicates (`@space_before`, ...)
    // -----------------------------------------------------------------------

    fn pred(p: AdjacencyPredicate) -> GrammarElement {
        GrammarElement::AdjacencyPredicate { predicate: p }
    }

    fn tok_ref(name: &str) -> GrammarElement {
        GrammarElement::TokenReference { name: name.to_string() }
    }

    #[test]
    fn test_each_adjacency_predicate_parses_by_name() {
        for p in AdjacencyPredicate::ALL {
            let source = format!("r = @{} NAME ;", p.name());
            let g = parse_parser_grammar(&source).unwrap();
            assert_eq!(
                g.rules[0].body,
                GrammarElement::Sequence { elements: vec![pred(p), tok_ref("NAME")] },
                "predicate @{}",
                p.name()
            );
            // Name and variant lookups are mutually consistent.
            assert_eq!(AdjacencyPredicate::from_name(p.name()), Some(p));
            assert_eq!(p.to_string(), format!("@{}", p.name()));
        }
    }

    #[test]
    fn test_adjacency_predicate_holds_truth_table() {
        use AdjacencyPredicate::*;
        // (space, newline) combinations a lexer can produce: none, space only,
        // space + newline. Newline without space is included for robustness.
        let cases = [(false, false), (true, false), (true, true), (false, true)];
        for (space, newline) in cases {
            assert_eq!(SpaceBefore.holds(space, newline), space);
            assert_eq!(NoSpaceBefore.holds(space, newline), !space);
            assert_eq!(SpaceAfterNext.holds(space, newline), space);
            assert_eq!(NoSpaceAfterNext.holds(space, newline), !space);
            assert_eq!(NewlineBefore.holds(space, newline), newline);
            assert_eq!(NoNewlineBefore.holds(space, newline), !newline);
        }
        assert!(SpaceAfterNext.reads_after_next());
        assert!(NoSpaceAfterNext.reads_after_next());
        assert!(!SpaceBefore.reads_after_next());
        assert!(!NoNewlineBefore.reads_after_next());
    }

    #[test]
    fn test_lang84_e6_examples_parse_verbatim() {
        // The examples from LANG84 §E6 (minus `@asi`, which is not part of F11).
        let source = r#"
return_statement = "return" [ @no_newline_before expression ] SEMICOLON ;
postfix_expr     = NAME [ @no_newline_before ( "++" | "--" ) ] ;
command_arg      = @space_before !@space_after_next MINUS primary ;
expression       = NAME ;
primary          = NUMBER ;
"#;
        let g = parse_parser_grammar(source).unwrap();
        assert_eq!(
            g.rules[0].body,
            GrammarElement::Sequence {
                elements: vec![
                    GrammarElement::Literal { value: "return".to_string() },
                    GrammarElement::Optional {
                        element: Box::new(GrammarElement::Sequence {
                            elements: vec![
                                pred(AdjacencyPredicate::NoNewlineBefore),
                                GrammarElement::RuleReference { name: "expression".to_string() },
                            ],
                        }),
                    },
                    tok_ref("SEMICOLON"),
                ],
            }
        );
        assert_eq!(
            g.rules[2].body,
            GrammarElement::Sequence {
                elements: vec![
                    pred(AdjacencyPredicate::SpaceBefore),
                    GrammarElement::NegativeLookahead {
                        element: Box::new(pred(AdjacencyPredicate::SpaceAfterNext)),
                    },
                    tok_ref("MINUS"),
                    GrammarElement::RuleReference { name: "primary".to_string() },
                ],
            }
        );
    }

    #[test]
    fn test_adjacency_predicate_after_a_token_and_in_groups() {
        // A predicate written after a token tests the gap that follows it.
        let g = parse_parser_grammar(
            r#"r = MINUS @no_space_before NUMBER | { @space_before NAME }+ | &@newline_before NAME ;"#,
        )
        .unwrap();
        let GrammarElement::Alternation { choices } = &g.rules[0].body else {
            panic!("expected alternation, got {:?}", g.rules[0].body);
        };
        assert_eq!(
            choices[0],
            GrammarElement::Sequence {
                elements: vec![tok_ref("MINUS"), pred(AdjacencyPredicate::NoSpaceBefore), tok_ref("NUMBER")],
            }
        );
        assert_eq!(
            choices[1],
            GrammarElement::OneOrMore {
                element: Box::new(GrammarElement::Sequence {
                    elements: vec![pred(AdjacencyPredicate::SpaceBefore), tok_ref("NAME")],
                }),
            }
        );
        assert_eq!(
            choices[2],
            GrammarElement::Sequence {
                elements: vec![
                    GrammarElement::PositiveLookahead {
                        element: Box::new(pred(AdjacencyPredicate::NewlineBefore)),
                    },
                    tok_ref("NAME"),
                ],
            }
        );
        // A predicate before a literal.
        let g = parse_parser_grammar(r#"r = NAME @no_newline_before "++" ;"#).unwrap();
        assert_eq!(
            g.rules[0].body,
            GrammarElement::Sequence {
                elements: vec![
                    tok_ref("NAME"),
                    pred(AdjacencyPredicate::NoNewlineBefore),
                    GrammarElement::Literal { value: "++".to_string() },
                ],
            }
        );
    }

    #[test]
    fn test_unknown_adjacency_predicate_is_rejected_with_valid_names() {
        let err = parse_parser_grammar("r = NAME\n  @spaces_before NAME ;").unwrap_err();
        assert_eq!(err.line_number, 2);
        assert!(err.message.contains("Unknown predicate '@spaces_before'"), "{}", err.message);
        for p in AdjacencyPredicate::ALL {
            assert!(err.message.contains(&p.to_string()), "{}", err.message);
        }
        // Case-sensitive.
        let err = parse_parser_grammar("r = @Space_Before NAME ;").unwrap_err();
        assert!(err.message.contains("Unknown predicate '@Space_Before'"), "{}", err.message);
    }

    #[test]
    fn test_bare_at_sign_is_rejected() {
        let err = parse_parser_grammar("r = @ NAME ;").unwrap_err();
        assert!(err.message.contains("Expected a predicate name after '@'"), "{}", err.message);
    }

    #[test]
    fn test_asi_is_rejected_as_not_yet_implemented() {
        let err = parse_parser_grammar("end = SEMICOLON | @asi(SEMICOLON, RBRACE) ;").unwrap_err();
        assert!(err.message.contains("'@asi'"), "{}", err.message);
        assert!(err.message.contains("not implemented yet"), "{}", err.message);
    }

    #[test]
    fn test_e7_binding_forms_are_rejected() {
        // LANG84 E7 syntax is not implemented; it must fail loudly, not parse
        // into something else.
        let err = parse_parser_grammar("param = NAME@declare ;").unwrap_err();
        assert!(err.message.contains("Unknown predicate '@declare'"), "{}", err.message);
    }

    #[test]
    fn test_at_sign_inside_literal_or_comment_is_not_a_predicate() {
        let g = parse_parser_grammar("# @version 2\nr = \"@space_before\" ; # @asi\n").unwrap();
        assert_eq!(g.version, 2);
        assert_eq!(g.rules[0].body, GrammarElement::Literal { value: "@space_before".to_string() });
    }

    #[test]
    fn test_adjacency_predicates_add_no_references() {
        let g = parse_parser_grammar("r = @space_before NAME @no_newline_before other ;\nother = NUMBER ;")
            .unwrap();
        let tokens = grammar_token_references(&g);
        assert_eq!(tokens, ["NAME", "NUMBER"].iter().map(|s| s.to_string()).collect());
        let rules = grammar_rule_references(&g);
        assert_eq!(rules, ["other"].iter().map(|s| s.to_string()).collect());
        let token_set: HashSet<String> = ["NAME", "NUMBER"].iter().map(|s| s.to_string()).collect();
        assert!(validate_parser_grammar(&g, Some(&token_set)).is_empty());
    }

    fn nullable_issues(grammar: &str) -> Vec<String> {
        let g = parse_parser_grammar(grammar).unwrap();
        validate_parser_grammar(&g, None)
            .into_iter()
            .filter(|i| i.contains("without consuming a token"))
            .collect()
    }

    #[test]
    fn test_validation_rejects_nullable_repetitions_with_predicates() {
        for grammar in [
            // Every path zero-width.
            "r = NAME { @space_before } ;",
            "r = NAME { @space_before !NAME }+ ;",
            "r = NAME { ( @newline_before | &NAME ) } ;",
            "r = NAME { @space_before // @no_space_before } ;",
            "r = NAME [ { [ @no_space_before ] } ] ;",
            // Only *some* path is empty (security review probes).
            "prog = NAME { @space_before | NAME } SEMI ;",
            "prog = NAME { @space_before [ NAME ] } SEMI ;",
            "prog = NAME { !@no_space_before | NAME } SEMI ;",
            // Nullable through a rule reference.
            "prog = NAME { p } SEMI ;\np = @space_before ;",
            "prog = NAME { p } SEMI ;\np = q ;\nq = [ @space_before NAME ] ;",
        ] {
            let issues = nullable_issues(grammar);
            assert_eq!(issues.len(), 1, "{}: {:?}", grammar, issues);
            assert!(issues[0].starts_with("Line 1: Rule '"), "{}", issues[0]);
            assert!(issues[0].contains("stops such a repetition"), "{}", issues[0]);
        }
    }

    #[test]
    fn test_validation_warns_on_nullable_repetitions_without_predicates() {
        // Pre-F11 shapes. The C# grammars' `switch_block` has exactly the
        // second one; the parser now stops after an empty iteration, so these
        // are warnings, not errors.
        for grammar in [
            "r = NAME { [ NAME ] } ;",
            "block = { section } ;\nsection = { label } { NAME } ;\nlabel = COLON ;",
        ] {
            let issues = nullable_issues(grammar);
            assert_eq!(issues.len(), 1, "{}: {:?}", grammar, issues);
            assert!(issues[0].starts_with("Warning: Line 1: Rule '"), "{}", issues[0]);
        }
    }

    #[test]
    fn test_separated_repetition_with_a_consuming_element_is_fine() {
        // The separator may be nullable as long as the element consumes.
        assert!(nullable_issues("r = { NAME // [ COMMA ] }+ ;").is_empty());
        assert!(nullable_issues("r = { NAME // @no_space_before } ;").is_empty());
    }

    #[test]
    fn test_nullable_rules_fixpoint() {
        let g = parse_parser_grammar(
            "a = b c ;\nb = [ NAME ] ;\nc = d | NAME ;\nd = &NAME ;\ne = NAME b ;\nf = { NAME // COMMA }+ ;\nh = { b }+ ;",
        )
        .unwrap();
        let nullable = nullable_rules(&g);
        let mut names: Vec<&str> = nullable.iter().map(|s| s.as_str()).collect();
        names.sort();
        assert_eq!(names, vec!["a", "b", "c", "d", "h"]);
    }

    #[test]
    fn test_validation_accepts_predicates_next_to_consuming_elements() {
        for body in [
            "{ @space_before NAME }",
            "{ @space_before NAME | COMMA }",
            "{ NAME // @no_space_before }",
            "{ @no_space_before // COMMA }",
            "@no_newline_before",
            "[ @space_before ]",
        ] {
            let g = parse_parser_grammar(&format!("r = NAME {} ;", body)).unwrap();
            assert!(validate_parser_grammar(&g, None).is_empty(), "{}", body);
        }
    }
}
