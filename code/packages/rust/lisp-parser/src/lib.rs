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

use lisp_lexer::{tokenize, Token, TokenType, LexerError};
use std::fmt;

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
// Section 3: The Parser
// ============================================================================
//
// The parser is a recursive descent parser -- one function per grammar rule.
// It walks through the token list with a position index, consuming tokens
// as it goes.
//
// Recursive descent is the natural fit for Lisp because the grammar is so
// simple. Each rule maps directly to a function:
//
//   program -> parse_program()
//   sexpr   -> parse_sexpr()
//   atom    -> (handled inline in parse_sexpr)
//   list    -> parse_list()
//   quoted  -> (handled inline in parse_sexpr)
// ============================================================================

/// Internal parser state: the token stream and current position.
struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    /// Peek at the current token without consuming it.
    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    /// Consume the current token and advance to the next.
    fn advance(&mut self) -> &Token {
        let token = &self.tokens[self.pos];
        self.pos += 1;
        token
    }

    /// Expect and consume a specific token type, or return an error.
    fn expect(&mut self, expected: TokenType) -> Result<&Token, ParseError> {
        if self.peek().token_type == expected {
            Ok(self.advance())
        } else {
            Err(ParseError {
                message: format!(
                    "Expected {:?}, got {:?} ({:?})",
                    expected,
                    self.peek().token_type,
                    self.peek().value,
                ),
            })
        }
    }

    /// Parse a complete program: zero or more S-expressions.
    ///
    /// Grammar: `program = { sexpr } ;`
    ///
    /// We keep parsing S-expressions until we hit EOF.
    fn parse_program(&mut self) -> Result<Vec<SExpr>, ParseError> {
        let mut expressions = Vec::new();
        while self.peek().token_type != TokenType::Eof {
            expressions.push(self.parse_sexpr()?);
        }
        Ok(expressions)
    }

    /// Parse a single S-expression.
    ///
    /// Grammar: `sexpr = atom | list | quoted ;`
    ///
    /// We look at the current token to decide which case applies:
    /// - `(` -> it's a list
    /// - `'` -> it's a quoted form
    /// - NUMBER, SYMBOL, STRING -> it's an atom
    fn parse_sexpr(&mut self) -> Result<SExpr, ParseError> {
        match self.peek().token_type {
            // -----------------------------------------------------------------
            // Case 1: List -- starts with `(`
            // -----------------------------------------------------------------
            TokenType::LParen => self.parse_list(),

            // -----------------------------------------------------------------
            // Case 2: Quoted form -- starts with `'`
            // -----------------------------------------------------------------
            // `'expr` is syntactic sugar for `(quote expr)`. We consume the
            // quote token and recursively parse the following S-expression.
            // -----------------------------------------------------------------
            TokenType::Quote => {
                self.advance(); // consume the quote
                let inner = self.parse_sexpr()?;
                Ok(SExpr::Quoted(Box::new(inner)))
            }

            // -----------------------------------------------------------------
            // Case 3: Atoms -- terminal values
            // -----------------------------------------------------------------
            TokenType::Number => {
                let value = self.advance().value.clone();
                Ok(SExpr::Atom(AtomKind::Number, value))
            }
            TokenType::Symbol => {
                let value = self.advance().value.clone();
                Ok(SExpr::Atom(AtomKind::Symbol, value))
            }
            TokenType::String => {
                let value = self.advance().value.clone();
                Ok(SExpr::Atom(AtomKind::String, value))
            }

            // -----------------------------------------------------------------
            // Error: unexpected token
            // -----------------------------------------------------------------
            _ => Err(ParseError {
                message: format!(
                    "Unexpected token: {:?} ({:?})",
                    self.peek().token_type,
                    self.peek().value,
                ),
            }),
        }
    }

    /// Parse a list: `LPAREN { sexpr } RPAREN`
    ///
    /// This also handles dotted pairs: `LPAREN sexpr { sexpr } DOT sexpr RPAREN`
    ///
    /// Lists are the core structure of Lisp. They can contain any number of
    /// S-expressions, and optionally end with a dot followed by a final
    /// S-expression (creating a dotted pair / improper list).
    fn parse_list(&mut self) -> Result<SExpr, ParseError> {
        self.expect(TokenType::LParen)?;

        let mut elements = Vec::new();
        let mut is_dotted = false;
        let mut dot_value: Option<SExpr> = None;

        // Parse elements until we hit `)` or EOF
        while self.peek().token_type != TokenType::RParen
            && self.peek().token_type != TokenType::Eof
        {
            // Check for dot (dotted pair notation)
            if self.peek().token_type == TokenType::Dot {
                self.advance(); // consume the dot
                is_dotted = true;
                dot_value = Some(self.parse_sexpr()?);
                break;
            }
            elements.push(self.parse_sexpr()?);
        }

        self.expect(TokenType::RParen)?;

        if is_dotted {
            Ok(SExpr::DottedPair(
                elements,
                Box::new(dot_value.unwrap()),
            ))
        } else {
            Ok(SExpr::List(elements))
        }
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
    let mut parser = Parser::new(tokens);
    parser.parse_program()
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
}
