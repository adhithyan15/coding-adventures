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
//! element       = rule_ref | token_ref | literal
//!               | "{" body "}"
//!               | "[" body "]"
//!               | "(" body ")" ;
//! rule_ref      = lowercase_identifier ;
//! token_ref     = UPPERCASE_IDENTIFIER ;
//! literal       = '"' characters '"' ;
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
        GrammarElement::RuleReference { .. } | GrammarElement::Literal { .. } => {
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
        | GrammarElement::Group { element } => {
            collect_token_refs(element, refs);
        }
    }
}

fn collect_rule_refs(node: &GrammarElement, refs: &mut HashSet<String>) {
    match node {
        GrammarElement::RuleReference { name } => {
            refs.insert(name.clone());
        }
        GrammarElement::TokenReference { .. } | GrammarElement::Literal { .. } => {
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
        | GrammarElement::Group { element } => {
            collect_rule_refs(element, refs);
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

            // { body } — zero-or-more repetition.
            TokenKind::LBrace => {
                self.advance();
                let body = self.parse_body()?;
                self.expect(TokenKind::RBrace)?;
                Ok(GrammarElement::Repetition { element: Box::new(body) })
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
        if stripped.starts_with('#') {
            let after_hash = stripped[1..].trim_start();
            if after_hash.starts_with('@') {
                let rest = &after_hash[1..]; // skip '@'
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
        let result = parse_parser_grammar("term = @ ;");
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
}
