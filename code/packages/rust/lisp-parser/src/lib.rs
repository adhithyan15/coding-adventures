//! # Lisp Parser -- parses token streams into S-expression ASTs.
//!
//! ## Lisp's Grammar is Beautifully Simple
//!
//! Lisp has one of the simplest grammars of any programming language. Where
//! CSS has ~36 grammar rules and Starlark has ~25, Lisp has just **6**:
//!
//! ```text
//! program   = { sexpr } ;          -- A program is zero or more S-expressions
//! sexpr     = atom | list | quoted ;  -- An S-expression is an atom, list, or quoted form
//! atom      = NUMBER | SYMBOL | STRING ; -- Atoms are terminal values
//! list      = LPAREN { sexpr } RPAREN ;  -- Lists are parenthesized sequences
//! quoted    = QUOTE sexpr ;         -- 'x is sugar for (quote x)
//! ```
//!
//! This simplicity is the genius of Lisp: the syntax is so uniform that code
//! and data share the same structure. A function call `(+ 1 2)` has the same
//! shape as a data list `(1 2 3)`. This property is called **homoiconicity**.
//!
//! ## What the Parser Produces
//!
//! The parser transforms a flat token stream into a tree of [`SExpr`] nodes:
//!
//! ```text
//! Input:  (+ (* 2 3) 4)
//!
//! Tokens: LPAREN SYMBOL(+) LPAREN SYMBOL(*) NUMBER(2) NUMBER(3) RPAREN
//!         NUMBER(4) RPAREN
//!
//! AST:    List([
//!           Atom(Symbol, "+"),
//!           List([
//!             Atom(Symbol, "*"),
//!             Atom(Number, "2"),
//!             Atom(Number, "3"),
//!           ]),
//!           Atom(Number, "4"),
//!         ])
//! ```
//!
//! ## Dotted Pairs
//!
//! Lisp supports dotted pair notation for constructing cons cells directly:
//!
//! ```text
//! (a . b)     -- a cons cell with car=a and cdr=b
//! (1 2 . 3)   -- a list ending with 3 instead of nil
//! ```
//!
//! The parser detects the DOT token inside a list and produces a
//! [`SExpr::DottedPair`] node.

mod _grammar;

use lisp_lexer::{tokenize, Token, TokenType, LexerError};
use std::fmt;
use std::sync::OnceLock;

use _grammar::parser_grammar;
use grammar_tools::parser_grammar::ParserGrammar;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode, GrammarParser};

// ============================================================================
// Section 1: AST Types
// ============================================================================
//
// The AST is built from a single enum, `SExpr`, which represents all possible
// Lisp values. This is much simpler than the generic ASTNode used by the
// Python parser -- we take advantage of Rust's enum system to make the tree
// structure explicit and type-safe.
// ============================================================================

/// The kind of atom (terminal value) in a Lisp expression.
///
/// Atoms are the leaves of the S-expression tree. They have no children --
/// they are raw values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AtomKind {
    /// An integer literal. The string value is the number text (e.g., `"42"`, `"-7"`).
    Number,
    /// A symbol (identifier or operator name). Example values: `"define"`, `"+"`, `"car"`.
    Symbol,
    /// A string literal. The value includes surrounding quotes (e.g., `"\"hello\""`).
    String,
}

/// An S-expression -- the universal data structure of Lisp.
///
/// Everything in Lisp is either an atom (a single value) or a list (a
/// collection of S-expressions). This enum captures all possible forms:
///
/// - **Atom**: a number, symbol, or string
/// - **List**: a parenthesized sequence of S-expressions
/// - **DottedPair**: a list ending with `. value` instead of nil
/// - **Quoted**: `'expr` syntactic sugar for `(quote expr)`
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SExpr {
    /// A terminal value: number, symbol, or string.
    ///
    /// ```text
    /// 42      -> Atom(Number, "42")
    /// define  -> Atom(Symbol, "define")
    /// "hello" -> Atom(String, "\"hello\"")
    /// ```
    Atom(AtomKind, std::string::String),

    /// A parenthesized list of S-expressions.
    ///
    /// ```text
    /// (+ 1 2)   -> List([Atom(Symbol, "+"), Atom(Number, "1"), Atom(Number, "2")])
    /// ()        -> List([])
    /// ```
    List(Vec<SExpr>),

    /// A dotted pair: `(elements... . final)`.
    ///
    /// ```text
    /// (a . b)   -> DottedPair([Atom(Symbol, "a")], Atom(Symbol, "b"))
    /// (1 2 . 3) -> DottedPair([Atom(Number, "1"), Atom(Number, "2")], Atom(Number, "3"))
    /// ```
    DottedPair(Vec<SExpr>, Box<SExpr>),

    /// A quoted form: `'expr` is sugar for `(quote expr)`.
    ///
    /// ```text
    /// 'foo      -> Quoted(Atom(Symbol, "foo"))
    /// '(1 2 3)  -> Quoted(List([...]))
    /// ```
    Quoted(Box<SExpr>),
}

impl SExpr {
    /// Recursively collect all atom values in this S-expression tree.
    ///
    /// This is useful for testing -- it lets us verify that the parser
    /// captured all the terminal values without worrying about tree structure.
    pub fn find_atoms(&self) -> Vec<std::string::String> {
        let mut results = Vec::new();
        self.collect_atoms(&mut results);
        results
    }

    fn collect_atoms(&self, results: &mut Vec<std::string::String>) {
        match self {
            SExpr::Atom(_, value) => results.push(value.clone()),
            SExpr::List(children) => {
                for child in children {
                    child.collect_atoms(results);
                }
            }
            SExpr::DottedPair(children, last) => {
                for child in children {
                    child.collect_atoms(results);
                }
                last.collect_atoms(results);
            }
            SExpr::Quoted(inner) => inner.collect_atoms(results),
        }
    }

    /// Count how many times a particular node kind appears in the tree.
    pub fn count_lists(&self) -> usize {
        match self {
            SExpr::List(children) => {
                1 + children.iter().map(|c| c.count_lists()).sum::<usize>()
            }
            SExpr::DottedPair(children, last) => {
                1 + children.iter().map(|c| c.count_lists()).sum::<usize>()
                    + last.count_lists()
            }
            SExpr::Quoted(inner) => inner.count_lists(),
            SExpr::Atom(_, _) => 0,
        }
    }

    /// Count how many quoted nodes appear in the tree.
    pub fn count_quoted(&self) -> usize {
        match self {
            SExpr::Quoted(inner) => 1 + inner.count_quoted(),
            SExpr::List(children) => {
                children.iter().map(|c| c.count_quoted()).sum::<usize>()
            }
            SExpr::DottedPair(children, last) => {
                children.iter().map(|c| c.count_quoted()).sum::<usize>()
                    + last.count_quoted()
            }
            SExpr::Atom(_, _) => 0,
        }
    }
}

// ============================================================================
// Section 2: Error Type
// ============================================================================

/// An error that occurs during parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// A human-readable description of what went wrong.
    pub message: std::string::String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ParseError: {}", self.message)
    }
}

impl std::error::Error for ParseError {}

impl From<LexerError> for ParseError {
    fn from(err: LexerError) -> Self {
        ParseError {
            message: format!("Lexer error: {}", err),
        }
    }
}

// ============================================================================
// Section 3: Grammar-driven parser
// ============================================================================
//
// This crate is a *thin wrapper*, not a hand-written parser. The
// S-expression grammar lives in `code/grammars/lisp/lisp.grammar`,
// compiled to the checked-in `src/_grammar.rs`. Parsing happens in two
// stages: the shared [`parser::grammar_parser::GrammarParser`] turns the
// token stream into a generic concrete syntax tree, then the extractors
// below lower that CST onto this crate's public [`SExpr`] API, which is
// unchanged (as are all downstream crates).
//
// A nesting-depth pre-scan guards the engine, whose default configuration
// recurses without a limit: over-deep input gets a catchable error
// instead of aborting the process.
// ============================================================================

static LISP_PARSER_GRAMMAR: OnceLock<ParserGrammar> = OnceLock::new();

fn lisp_parser_grammar() -> &'static ParserGrammar {
    LISP_PARSER_GRAMMAR.get_or_init(parser_grammar)
}

/// Borrow the compiled `lisp.grammar` rules.
///
/// Exposed for tooling that wants to introspect the grammar without
/// re-parsing the canonical `.grammar` file.
pub fn lisp_parser_grammar_spec() -> &'static ParserGrammar {
    lisp_parser_grammar()
}

/// Maximum structural nesting (parens + pending quotes) admitted before
/// the grammar parser runs.
///
/// The engine recurses without a limit of its own (its default cap is
/// effectively infinite), so deep input would overflow the native stack
/// -- an uncatchable abort. Each structural level costs the engine
/// several rule frames, so 48 keeps worst-case rule depth (~147) clear
/// of the engine's documented debug-build overflow band (~192+) with
/// margin to spare, while still dwarfing any real program. Flat lists
/// stay cheap however long: siblings share one level.
pub const MAX_NESTING_DEPTH: usize = 48;

/// Reject over-deep token streams before invoking the engine.
///
/// Counts combined paren depth plus pending quotes (`''''x` nests via
/// `quoted = QUOTE sexpr` without consuming a paren). A completed
/// S-expression discharges the quotes wrapping it; closing a list pops
/// to its paren.
fn check_nesting_depth(tokens: &[Token]) -> Result<(), ParseError> {
    // 'p' = open paren (list), 'q' = pending quote.
    let mut stack: Vec<u8> = Vec::new();

    for token in tokens {
        match token.token_type {
            TokenType::LParen => stack.push(b'p'),
            TokenType::RParen => {
                while matches!(stack.last(), Some(b'q')) {
                    stack.pop();
                }
                if matches!(stack.last(), Some(b'p')) {
                    stack.pop();
                }
                while matches!(stack.last(), Some(b'q')) {
                    stack.pop();
                }
            }
            TokenType::Quote => stack.push(b'q'),
            TokenType::Number | TokenType::Symbol | TokenType::String => {
                while matches!(stack.last(), Some(b'q')) {
                    stack.pop();
                }
            }
            TokenType::Dot | TokenType::Eof => {}
        }

        if stack.len() > MAX_NESTING_DEPTH {
            return Err(ParseError {
                message: format!(
                    "structural nesting (parens + quotes) exceeds MAX_NESTING_DEPTH \
                     ({MAX_NESTING_DEPTH}) — refusing to invoke the parser to avoid \
                     stack overflow"
                ),
            });
        }
    }
    Ok(())
}

/// Convert lisp tokens to engine tokens.
///
/// Engine matching keys on `type_name`; positions are synthesized
/// (this API never reported any) since only the message survives the
/// mapping below.
fn to_engine_tokens(tokens: &[Token]) -> Vec<lexer::token::Token> {
    tokens
        .iter()
        .enumerate()
        .map(|(i, token)| {
            let name = match token.token_type {
                TokenType::Number => "NUMBER",
                TokenType::Symbol => "SYMBOL",
                TokenType::String => "STRING",
                TokenType::LParen => "LPAREN",
                TokenType::RParen => "RPAREN",
                TokenType::Quote => "QUOTE",
                TokenType::Dot => "DOT",
                TokenType::Eof => "EOF",
            };
            lexer::token::Token {
                type_: if token.token_type == TokenType::Eof {
                    lexer::token::TokenType::Eof
                } else {
                    lexer::token::TokenType::Name
                },
                type_name: Some(name.to_string()),
                value: token.value.clone(),
                line: 1,
                column: i + 1,
                flags: None,
                cv: None,
            }
        })
        .collect()
}

/// Lower one `sexpr` CST node to an [`SExpr`].
fn extract_sexpr(node: &GrammarASTNode) -> Result<SExpr, ParseError> {
    let [child] = node.children.as_slice() else {
        return Err(internal(format!(
            "sexpr node has {} children, want 1",
            node.children.len()
        )));
    };
    let ASTNodeOrToken::Node(inner) = child else {
        return Err(internal("sexpr child is a token, want a rule node"));
    };
    match inner.rule_name.as_str() {
        "atom" => extract_atom(inner),
        "list" => extract_list(inner),
        "quoted" => extract_quoted(inner),
        other => Err(internal(format!("sexpr child is rule {other:?}"))),
    }
}

/// Lower an `atom` CST node (single token child) to an [`SExpr::Atom`].
fn extract_atom(node: &GrammarASTNode) -> Result<SExpr, ParseError> {
    let Some(token) = node.token() else {
        return Err(internal("atom node is not a leaf"));
    };
    let kind = match token.type_name.as_deref() {
        Some("NUMBER") => AtomKind::Number,
        Some("SYMBOL") => AtomKind::Symbol,
        Some("STRING") => AtomKind::String,
        other => return Err(internal(format!("atom token is {other:?}"))),
    };
    Ok(SExpr::Atom(kind, token.value.clone()))
}

/// Lower a `list` CST node to [`SExpr::List`] or [`SExpr::DottedPair`].
///
/// Shape: `LPAREN list_body RPAREN`, where `list_body` holds element
/// sexprs with at most one `DOT sexpr` tail. A leading dot cannot occur:
/// the grammar requires an element before `DOT`, so `(. b)` fails
/// parsing (the hand reader accepted it as an empty-headed pair).
fn extract_list(node: &GrammarASTNode) -> Result<SExpr, ParseError> {
    let body = node.children.iter().find_map(|child| match child {
        ASTNodeOrToken::Node(inner) if inner.rule_name == "list_body" => Some(inner),
        _ => None,
    });
    let Some(body) = body else {
        return Err(internal("list node has no list_body child"));
    };
    let mut elements = Vec::new();
    let mut dot_value: Option<SExpr> = None;
    let mut children = body.children.iter().peekable();
    while let Some(child) = children.next() {
        match child {
            ASTNodeOrToken::Node(inner) => {
                elements.push(extract_sexpr(inner)?);
            }
            ASTNodeOrToken::Token(token) if is_dot(token) => {
                let Some(ASTNodeOrToken::Node(tail)) = children.next() else {
                    return Err(internal("DOT tail is not a rule node"));
                };
                if dot_value.replace(extract_sexpr(tail)?).is_some() {
                    return Err(internal("list_body has two DOT tails"));
                }
            }
            ASTNodeOrToken::Token(token) => {
                return Err(internal(format!(
                    "list_body child is a {:?} token",
                    token.type_name
                )));
            }
        }
    }
    match dot_value {
        Some(tail) => Ok(SExpr::DottedPair(elements, Box::new(tail))),
        None => Ok(SExpr::List(elements)),
    }
}

/// Whether an engine token is the DOT marker.
fn is_dot(token: &lexer::token::Token) -> bool {
    token.type_name.as_deref() == Some("DOT")
}

/// Lower a `quoted` CST node (`QUOTE sexpr`) to [`SExpr::Quoted`].
fn extract_quoted(node: &GrammarASTNode) -> Result<SExpr, ParseError> {
    let mut inner = None;
    for child in &node.children {
        if let ASTNodeOrToken::Node(sexpr) = child {
            if inner.replace(sexpr).is_some() {
                return Err(internal("quoted node has two sexprs"));
            }
        }
    }
    let Some(inner) = inner else {
        return Err(internal("quoted node has no sexpr child"));
    };
    Ok(SExpr::Quoted(Box::new(extract_sexpr(inner)?)))
}

/// Lower a `program` CST node to the top-level expression list.
fn extract_program(node: &GrammarASTNode) -> Result<Vec<SExpr>, ParseError> {
    node.children
        .iter()
        .map(|child| match child {
            ASTNodeOrToken::Node(inner) => extract_sexpr(inner),
            ASTNodeOrToken::Token(token) => Err(internal(format!(
                "program child is a {:?} token",
                token.type_name
            ))),
        })
        .collect()
}

/// An extractor internal error: the CST has a shape the fixed grammar
/// cannot produce. Unreachable unless grammar and extractor drift apart.
fn internal(message: impl Into<String>) -> ParseError {
    ParseError {
        message: format!("Internal error: {}", message.into()),
    }
}

// ============================================================================
// Section 4: Public API
// ============================================================================

/// Parse Lisp source code into a vector of S-expressions.
///
/// This is the main entry point for the Lisp parser. It tokenizes the source
/// text and then parses the token stream into an AST.
///
/// # Returns
///
/// A vector of top-level S-expressions. Each element represents one
/// top-level form in the source code.
///
/// # Errors
///
/// Returns `ParseError` if the source has syntax errors (unmatched parentheses,
/// unexpected tokens, etc.) or lexer errors (unrecognized characters).
///
/// # Examples
///
/// ```
/// use lisp_parser::{parse, SExpr, AtomKind};
///
/// let program = parse("(+ 1 2)").unwrap();
/// assert_eq!(program.len(), 1);
///
/// let atoms: Vec<String> = program[0].find_atoms();
/// assert_eq!(atoms, vec!["+", "1", "2"]);
/// ```
pub fn parse(source: &str) -> Result<Vec<SExpr>, ParseError> {
    let tokens = tokenize(source)?;
    parse_tokens(tokens)
}

/// Parse a pre-tokenized token stream into S-expressions.
///
/// Use this when you already have tokens from the lexer and want to
/// skip re-tokenization.
pub fn parse_tokens(tokens: Vec<Token>) -> Result<Vec<SExpr>, ParseError> {
    if tokens.is_empty() {
        // `program = { sexpr }` matches empty input. (The hand reader
        // indexed past the end here -- a panic this migration retires.)
        return Ok(Vec::new());
    }
    check_nesting_depth(&tokens)?;
    let engine_tokens = to_engine_tokens(&tokens);
    let mut parser =
        GrammarParser::new(engine_tokens, lisp_parser_grammar().clone());
    let cst = parser
        .parse()
        .map_err(|e| ParseError { message: e.message })?;
    extract_program(&cst)
}

// ============================================================================
// Section 5: Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: collect all atom values from a program.
    fn find_all_atoms(program: &[SExpr]) -> Vec<std::string::String> {
        let mut results = Vec::new();
        for expr in program {
            results.extend(expr.find_atoms());
        }
        results
    }

    /// Helper: count all list nodes in a program.
    fn count_all_lists(program: &[SExpr]) -> usize {
        program.iter().map(|e| e.count_lists()).sum()
    }

    /// Helper: count all quoted nodes in a program.
    fn count_all_quoted(program: &[SExpr]) -> usize {
        program.iter().map(|e| e.count_quoted()).sum()
    }

    // =====================================================================
    // Basic Structure
    // =====================================================================

    #[test]
    fn test_empty_program() {
        let program = parse("").unwrap();
        assert_eq!(program.len(), 0);
    }

    #[test]
    fn test_multiple_top_level() {
        let program = parse("1 2 3").unwrap();
        assert_eq!(program.len(), 3);
    }

    // =====================================================================
    // Atoms
    // =====================================================================

    #[test]
    fn test_number() {
        let program = parse("42").unwrap();
        assert_eq!(find_all_atoms(&program), vec!["42"]);
    }

    #[test]
    fn test_negative_number() {
        let program = parse("-7").unwrap();
        assert_eq!(find_all_atoms(&program), vec!["-7"]);
    }

    #[test]
    fn test_symbol() {
        let program = parse("define").unwrap();
        assert_eq!(find_all_atoms(&program), vec!["define"]);
    }

    #[test]
    fn test_operator_symbol() {
        let program = parse("+").unwrap();
        assert_eq!(find_all_atoms(&program), vec!["+"]);
    }

    #[test]
    fn test_string() {
        let program = parse("\"hello\"").unwrap();
        let atoms = find_all_atoms(&program);
        assert_eq!(atoms.len(), 1);
    }

    // =====================================================================
    // Lists
    // =====================================================================

    #[test]
    fn test_empty_list() {
        let program = parse("()").unwrap();
        assert_eq!(count_all_lists(&program), 1);
    }

    #[test]
    fn test_simple_list() {
        let program = parse("(1 2 3)").unwrap();
        assert_eq!(find_all_atoms(&program), vec!["1", "2", "3"]);
    }

    #[test]
    fn test_nested_list() {
        let program = parse("((1 2) (3 4))").unwrap();
        assert_eq!(count_all_lists(&program), 3); // outer + 2 inner
    }

    #[test]
    fn test_function_call() {
        let program = parse("(+ 1 2)").unwrap();
        assert_eq!(find_all_atoms(&program), vec!["+", "1", "2"]);
    }

    #[test]
    fn test_define() {
        let program = parse("(define x 42)").unwrap();
        assert_eq!(find_all_atoms(&program), vec!["define", "x", "42"]);
    }

    #[test]
    fn test_deeply_nested() {
        let program = parse("(+ (* 2 3) (- 10 4))").unwrap();
        assert_eq!(
            find_all_atoms(&program),
            vec!["+", "*", "2", "3", "-", "10", "4"]
        );
    }

    // =====================================================================
    // Quoted Forms
    // =====================================================================

    #[test]
    fn test_quoted_symbol() {
        let program = parse("'foo").unwrap();
        assert_eq!(count_all_quoted(&program), 1);
        assert_eq!(find_all_atoms(&program), vec!["foo"]);
    }

    #[test]
    fn test_quoted_list() {
        let program = parse("'(1 2 3)").unwrap();
        assert_eq!(count_all_quoted(&program), 1);
        assert_eq!(find_all_atoms(&program), vec!["1", "2", "3"]);
    }

    #[test]
    fn test_quoted_in_expression() {
        let program = parse("(eq 'foo 'bar)").unwrap();
        assert_eq!(count_all_quoted(&program), 2);
    }

    // =====================================================================
    // Dotted Pairs
    // =====================================================================

    #[test]
    fn test_simple_dotted_pair() {
        let program = parse("(a . b)").unwrap();
        assert_eq!(find_all_atoms(&program), vec!["a", "b"]);
        match &program[0] {
            SExpr::DottedPair(_, _) => {} // expected
            other => panic!("Expected DottedPair, got {:?}", other),
        }
    }

    #[test]
    fn test_numeric_dotted_pair() {
        let program = parse("(1 . 2)").unwrap();
        assert_eq!(find_all_atoms(&program), vec!["1", "2"]);
    }

    // =====================================================================
    // Complex Expressions
    // =====================================================================

    #[test]
    fn test_lambda() {
        let program = parse("(lambda (x) (* x x))").unwrap();
        let atoms = find_all_atoms(&program);
        assert!(atoms.contains(&"lambda".to_string()));
        assert!(atoms.contains(&"x".to_string()));
        assert!(atoms.contains(&"*".to_string()));
    }

    #[test]
    fn test_cond() {
        let program = parse("(cond ((eq x 0) 1) (t x))").unwrap();
        let atoms = find_all_atoms(&program);
        assert!(atoms.contains(&"cond".to_string()));
        assert!(atoms.contains(&"eq".to_string()));
        assert!(atoms.contains(&"t".to_string()));
    }

    #[test]
    fn test_factorial() {
        let source = r#"
        (define factorial
          (lambda (n)
            (cond ((eq n 0) 1)
                  (t (* n (factorial (- n 1)))))))
        "#;
        let program = parse(source).unwrap();
        assert_eq!(program.len(), 1);
        let atoms = find_all_atoms(&program);
        assert!(atoms.contains(&"define".to_string()));
        assert!(atoms.contains(&"factorial".to_string()));
        assert!(atoms.contains(&"lambda".to_string()));
        assert!(atoms.contains(&"cond".to_string()));
    }

    #[test]
    fn test_multiple_definitions() {
        let source = r#"
        (define x 10)
        (define y 20)
        (+ x y)
        "#;
        let program = parse(source).unwrap();
        assert_eq!(program.len(), 3);
    }

    #[test]
    fn test_cons_car_cdr() {
        let program = parse("(car (cons 1 2))").unwrap();
        assert_eq!(find_all_atoms(&program), vec!["car", "cons", "1", "2"]);
    }

    // =====================================================================
    // Error Cases
    // =====================================================================

    #[test]
    fn test_unmatched_lparen() {
        let result = parse("(+ 1 2");
        assert!(result.is_err());
    }

    #[test]
    fn test_unexpected_rparen() {
        let result = parse(")");
        assert!(result.is_err());
    }

    // =====================================================================
    // Grammar-migration pins
    // =====================================================================

    #[test]
    fn checked_in_grammar_matches_lisp_grammar() {
        // Drift guard: the compiled `_grammar.rs` must equal a fresh
        // parse of the canonical source.
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let grammar_path = manifest
            .parent() // rust
            .unwrap()
            .parent() // packages
            .unwrap()
            .parent() // code
            .unwrap()
            .join("grammars/lisp/lisp.grammar");
        let source = std::fs::read_to_string(&grammar_path).unwrap();
        let fresh =
            grammar_tools::parser_grammar::parse_parser_grammar(&source).unwrap();
        assert_eq!(&fresh, lisp_parser_grammar_spec());
    }

    #[test]
    fn nesting_at_the_limit_parses() {
        let open = "(".repeat(MAX_NESTING_DEPTH);
        let close = ")".repeat(MAX_NESTING_DEPTH);
        let source = format!("{open}x{close}");
        assert!(parse(&source).is_ok());
    }

    #[test]
    fn nesting_past_the_limit_fails_closed() {
        let open = "(".repeat(MAX_NESTING_DEPTH + 1);
        let close = ")".repeat(MAX_NESTING_DEPTH + 1);
        let source = format!("{open}x{close}");
        let err = parse(&source).unwrap_err();
        assert!(err.message.contains("MAX_NESTING_DEPTH"), "{}", err.message);
    }

    #[test]
    fn quote_runs_count_toward_nesting() {
        // `quoted = QUOTE sexpr` recurses without consuming a paren, so
        // pending quotes count like parens.
        let source = format!("{}x", "'".repeat(MAX_NESTING_DEPTH + 1));
        let err = parse(&source).unwrap_err();
        assert!(err.message.contains("MAX_NESTING_DEPTH"), "{}", err.message);
        let ok_source = format!("{}x", "'".repeat(MAX_NESTING_DEPTH));
        assert!(parse(&ok_source).is_ok());
    }

    #[test]
    fn leading_dot_is_an_error() {
        // The hand reader accepted `(. b)` as an empty-headed pair; the
        // grammar requires an element before DOT (`list_body = [ sexpr
        // { sexpr } [ DOT sexpr ] ]`), and the grammar governs.
        assert!(parse("(. b)").is_err());
    }

    #[test]
    fn empty_token_stream_parses_to_empty_program() {
        // Security review: the hand reader panicked indexing past the
        // end; the grammar accepts empty input, so return it.
        let program = parse_tokens(Vec::new()).unwrap();
        assert!(program.is_empty());
    }

    #[test]
    fn spec_accessor_exposes_grammar() {
        let names: Vec<&str> = lisp_parser_grammar_spec()
            .rules
            .iter()
            .map(|r| r.name.as_str())
            .collect();
        assert!(names.contains(&"program"));
        assert!(names.contains(&"sexpr"));
        assert!(names.contains(&"list_body"));
    }
}
