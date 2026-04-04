//! # Hand-written recursive descent parser for a Python subset.
//!
//! This module implements a parser that converts a stream of tokens (produced
//! by the lexer) into an abstract syntax tree (AST). It handles a small but
//! representative subset of Python:
//!
//! - **Expressions**: arithmetic with `+`, `-`, `*`, `/`, parentheses
//! - **Literals**: numbers, strings, variable names
//! - **Statements**: assignments (`x = expr`) and expression statements
//! - **Programs**: sequences of statements separated by newlines
//!
//! # What is recursive descent parsing?
//!
//! Recursive descent is the simplest parsing technique. The idea:
//!
//! 1. Write one function for each grammar rule.
//! 2. Each function looks at the current token to decide what to do.
//! 3. Functions call each other recursively to handle nested structures.
//!
//! For example, the grammar rule:
//!
//! ```text
//! expression = term { ("+" | "-") term } ;
//! ```
//!
//! becomes a function `parse_expression()` that:
//! 1. Calls `parse_term()` to get the left operand.
//! 2. Checks if the next token is `+` or `-`.
//! 3. If so, calls `parse_term()` again for the right operand.
//! 4. Repeats step 2-3 for chained operations like `1 + 2 + 3`.
//!
//! # Operator precedence via grammar structure
//!
//! The grammar encodes operator precedence through its rule hierarchy:
//!
//! ```text
//! expression = term { ("+" | "-") term } ;     ← lowest precedence
//! term       = factor { ("*" | "/") factor } ;  ← higher precedence
//! factor     = NUMBER | STRING | NAME | "(" expression ")" ;  ← highest
//! ```
//!
//! Because `term` calls `factor` (not `expression`), multiplication binds
//! tighter than addition. And because `factor` can recurse back to
//! `expression` via parentheses, explicit grouping overrides precedence.
//!
//! This is the same technique used in virtually every hand-written parser,
//! from GCC to Python's own parser (before they switched to PEG).
//!
//! # Error handling
//!
//! Instead of panicking (like the Go implementation), this Rust version
//! returns `Result<ASTNode, ParseError>`. This is idiomatic Rust: errors
//! are values, not control flow. The `?` operator makes error propagation
//! concise — a function can say "try this, and if it fails, return the
//! error to my caller" with a single `?`.

use lexer::token::{Token, TokenType};
use crate::ast::ASTNode;
use std::fmt;

// ===========================================================================
// Error type
// ===========================================================================

/// An error encountered during parsing.
///
/// Contains a human-readable message and the token where the error occurred.
/// The token provides position information (line and column) for error
/// reporting.
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub token: Token,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at line {}, column {}",
            self.message, self.token.line, self.token.column
        )
    }
}

impl std::error::Error for ParseError {}

// ===========================================================================
// Parser struct
// ===========================================================================

/// A recursive descent parser for a Python-like language.
///
/// The parser holds a flat list of tokens and a cursor (`pos`) that advances
/// as tokens are consumed. This is the same design as the Go implementation,
/// but with Rust's `Result` type for error handling instead of panics.
///
/// # Lifetime of tokens
///
/// The parser takes ownership of the token vector. This is a deliberate
/// design choice: once you hand tokens to the parser, the parser is the
/// sole owner. This avoids lifetime annotations and makes the API simpler.
///
/// # Example
///
/// ```
/// use lexer::token::{Token, TokenType};
/// use lexer::tokenizer::Lexer;
/// use parser::parser::Parser;
/// use parser::ast::ASTNode;
///
/// let mut lexer = Lexer::new("1 + 2", None);
/// let tokens = lexer.tokenize().unwrap();
/// let mut parser = Parser::new(tokens);
/// let result = parser.parse().unwrap();
///
/// // The result is a Program containing one ExpressionStmt
/// match result {
///     ASTNode::Program(stmts) => assert_eq!(stmts.len(), 1),
///     _ => panic!("Expected Program"),
/// }
/// ```
pub struct Parser {
    /// The complete list of tokens to parse.
    tokens: Vec<Token>,
    /// Current position in the token list (0-based index).
    pos: usize,
}

impl Parser {
    /// Create a new parser for the given token list.
    ///
    /// The token list should end with an EOF token (which the lexer always
    /// provides). The parser starts at position 0.
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    // =======================================================================
    // Token navigation helpers
    // =======================================================================
    // These four methods are the parser's "eyes" — they let it look at tokens
    // and move through the stream. Every parsing method is built on top of
    // these primitives.

    /// Look at the current token without consuming it.
    ///
    /// If we have gone past the end (which should not happen if the token
    /// stream ends with EOF), returns the last token.
    fn peek(&self) -> &Token {
        if self.pos < self.tokens.len() {
            &self.tokens[self.pos]
        } else {
            &self.tokens[self.tokens.len() - 1]
        }
    }

    /// Consume the current token and advance to the next one.
    ///
    /// Returns a clone of the consumed token. We clone because the parser
    /// might need the token later (e.g., to extract a variable name or
    /// operator), but we also need to advance past it.
    fn advance(&mut self) -> Token {
        let token = self.peek().clone();
        self.pos += 1;
        token
    }

    /// Assert that the current token has the expected type, then consume it.
    ///
    /// If the token type does not match, returns a `ParseError` with a
    /// descriptive message including both what was expected and what was found.
    ///
    /// This is used for mandatory syntax elements like the `=` in assignments
    /// or the `)` closing a parenthesized expression.
    fn expect(&mut self, expected_type: TokenType) -> Result<Token, ParseError> {
        let token = self.peek().clone();
        if token.type_ != expected_type {
            return Err(ParseError {
                message: format!(
                    "Expected {}, got {} ({:?})",
                    expected_type, token.type_, token.value
                ),
                token,
            });
        }
        Ok(self.advance())
    }

    /// Try to match the current token against one of the given types.
    ///
    /// If the current token matches any of the types, consumes and returns it.
    /// Otherwise, returns `None` without consuming anything.
    ///
    /// This is the "optional" counterpart to `expect()`. It is used for
    /// operators in expression parsing, where the operator might or might not
    /// be present (e.g., the `+` in `1 + 2` vs. just `1`).
    fn match_token(&mut self, types: &[TokenType]) -> Option<Token> {
        let token = self.peek();
        for t in types {
            if token.type_ == *t {
                return Some(self.advance());
            }
        }
        None
    }

    /// Check if we have reached the end of the token stream.
    fn at_end(&self) -> bool {
        self.peek().type_ == TokenType::Eof
    }

    /// Skip over any newline tokens.
    ///
    /// Newlines separate statements, but consecutive newlines (blank lines)
    /// should be ignored. This method consumes all newline tokens at the
    /// current position.
    fn skip_newlines(&mut self) {
        while self.peek().type_ == TokenType::Newline {
            self.advance();
        }
    }

    // =======================================================================
    // Parsing methods — one per grammar rule
    // =======================================================================
    // The grammar for our language is:
    //
    //   program    = { statement } ;
    //   statement  = assignment | expression_stmt ;
    //   assignment = NAME "=" expression NEWLINE ;
    //   expr_stmt  = expression NEWLINE ;
    //   expression = term { ("+" | "-") term } ;
    //   term       = factor { ("*" | "/") factor } ;
    //   factor     = NUMBER | STRING | NAME | "(" expression ")" ;
    //
    // Each rule becomes a method below.

    /// Parse the entire token stream into a Program AST node.
    ///
    /// This is the main entry point. It parses statements until EOF.
    pub fn parse(&mut self) -> Result<ASTNode, ParseError> {
        self.parse_program()
    }

    /// Parse a program: a sequence of statements separated by newlines.
    ///
    /// ```text
    /// program = { statement } ;
    /// ```
    ///
    /// Blank lines (consecutive newlines) are skipped. The program ends
    /// when we reach EOF.
    fn parse_program(&mut self) -> Result<ASTNode, ParseError> {
        let mut statements = Vec::new();
        self.skip_newlines();

        while !self.at_end() {
            let stmt = self.parse_statement()?;
            statements.push(stmt);
            self.skip_newlines();
        }

        Ok(ASTNode::Program(statements))
    }

    /// Parse a single statement: either an assignment or an expression statement.
    ///
    /// ```text
    /// statement = assignment | expression_stmt ;
    /// ```
    ///
    /// # How do we tell them apart?
    ///
    /// An assignment starts with `NAME =`. We use a two-token lookahead:
    /// if the current token is a Name AND the next token is `=`, it's an
    /// assignment. Otherwise, it's an expression statement.
    ///
    /// This lookahead is cheap because we just index into the token array.
    fn parse_statement(&mut self) -> Result<ASTNode, ParseError> {
        // Two-token lookahead to distinguish assignment from expression.
        // An assignment looks like: NAME EQUALS ...
        if self.peek().type_ == TokenType::Name
            && self.pos + 1 < self.tokens.len()
            && self.tokens[self.pos + 1].type_ == TokenType::Equals
        {
            return self.parse_assignment();
        }
        self.parse_expression_stmt()
    }

    /// Parse an assignment: `NAME = expression`.
    ///
    /// ```text
    /// assignment = NAME "=" expression [NEWLINE] ;
    /// ```
    ///
    /// The newline at the end is optional (the last statement in a file
    /// might not have a trailing newline).
    fn parse_assignment(&mut self) -> Result<ASTNode, ParseError> {
        let name_token = self.expect(TokenType::Name)?;
        let target = name_token.value;

        self.expect(TokenType::Equals)?;

        let value = self.parse_expression()?;

        // Consume trailing newline if present (but not at EOF).
        if !self.at_end() {
            self.expect(TokenType::Newline)?;
        }

        Ok(ASTNode::Assignment {
            target,
            value: Box::new(value),
        })
    }

    /// Parse an expression statement: an expression on its own line.
    ///
    /// ```text
    /// expression_stmt = expression [NEWLINE] ;
    /// ```
    fn parse_expression_stmt(&mut self) -> Result<ASTNode, ParseError> {
        let expr = self.parse_expression()?;

        if !self.at_end() {
            self.expect(TokenType::Newline)?;
        }

        Ok(ASTNode::ExpressionStmt(Box::new(expr)))
    }

    /// Parse an expression: addition and subtraction.
    ///
    /// ```text
    /// expression = term { ("+" | "-") term } ;
    /// ```
    ///
    /// This handles the lowest-precedence binary operators. The `{ ... }`
    /// repetition means we keep looking for more `+` or `-` operators
    /// until we find something else.
    ///
    /// # Left-associativity
    ///
    /// The loop builds the tree left-to-right. For `1 + 2 + 3`:
    ///
    /// ```text
    /// Step 1: left = Number(1)
    /// Step 2: op = "+", right = Number(2) → left = BinaryOp(1 + 2)
    /// Step 3: op = "+", right = Number(3) → left = BinaryOp((1 + 2) + 3)
    /// ```
    ///
    /// This is correct: addition is left-associative.
    fn parse_expression(&mut self) -> Result<ASTNode, ParseError> {
        let mut left = self.parse_term()?;

        loop {
            let op_tok = self.match_token(&[TokenType::Plus, TokenType::Minus]);
            match op_tok {
                None => break,
                Some(tok) => {
                    let right = self.parse_term()?;
                    left = ASTNode::BinaryOp {
                        left: Box::new(left),
                        op: tok.value,
                        right: Box::new(right),
                    };
                }
            }
        }

        Ok(left)
    }

    /// Parse a term: multiplication and division.
    ///
    /// ```text
    /// term = factor { ("*" | "/") factor } ;
    /// ```
    ///
    /// This handles higher-precedence operators. Because `parse_expression`
    /// calls `parse_term`, and `parse_term` calls `parse_factor`, the
    /// recursive structure ensures `*` and `/` bind tighter than `+` and `-`.
    fn parse_term(&mut self) -> Result<ASTNode, ParseError> {
        let mut left = self.parse_factor()?;

        loop {
            let op_tok = self.match_token(&[TokenType::Star, TokenType::Slash]);
            match op_tok {
                None => break,
                Some(tok) => {
                    let right = self.parse_factor()?;
                    left = ASTNode::BinaryOp {
                        left: Box::new(left),
                        op: tok.value,
                        right: Box::new(right),
                    };
                }
            }
        }

        Ok(left)
    }

    /// Parse a factor: the highest-precedence level.
    ///
    /// ```text
    /// factor = NUMBER | STRING | NAME | "(" expression ")" ;
    /// ```
    ///
    /// Factors are the "atoms" of expressions — the smallest units that
    /// cannot be broken down further (unless they are parenthesized
    /// sub-expressions, which recurse back to `parse_expression`).
    ///
    /// # The parenthesized expression case
    ///
    /// When we see `(`, we recursively call `parse_expression()`. This
    /// means `(1 + 2) * 3` will:
    /// 1. `parse_factor` sees `(`, consumes it
    /// 2. Calls `parse_expression`, which parses `1 + 2`
    /// 3. `parse_factor` expects `)`, consumes it
    /// 4. Returns the `BinaryOp(1 + 2)` as the factor
    ///
    /// The caller (`parse_term`) then sees `*` and uses this as the left
    /// operand of multiplication. Parentheses override precedence!
    fn parse_factor(&mut self) -> Result<ASTNode, ParseError> {
        let token = self.peek().clone();

        match token.type_ {
            TokenType::Number => {
                self.advance();
                let val: f64 = token.value.parse().unwrap_or(0.0);
                Ok(ASTNode::Number(val))
            }

            TokenType::String => {
                self.advance();
                Ok(ASTNode::String(token.value))
            }

            TokenType::Name => {
                self.advance();
                Ok(ASTNode::Name(token.value))
            }

            TokenType::LParen => {
                self.advance(); // consume '('
                let expr = self.parse_expression()?;
                self.expect(TokenType::RParen)?; // consume ')'
                Ok(expr)
            }

            _ => Err(ParseError {
                message: format!(
                    "Unexpected token {} ({:?})",
                    token.type_, token.value
                ),
                token,
            }),
        }
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a token with default position (line 1, column 1).
    fn tok(type_: TokenType, value: &str) -> Token {
        Token {
            type_,
            value: value.to_string(),
            line: 1,
            column: 1,
            type_name: None, flags: None,
        }
    }

    /// Helper: create a token with specific position.
    fn tok_at(type_: TokenType, value: &str, line: usize, col: usize) -> Token {
        Token {
            type_,
            value: value.to_string(),
            line,
            column: col,
            type_name: None, flags: None,
        }
    }

    // -----------------------------------------------------------------------
    // Expression parsing tests
    // -----------------------------------------------------------------------

    /// Test parsing a simple addition: `1 + 2`.
    ///
    /// This is the canonical test for binary expression parsing.
    /// Expected AST:
    ///
    /// ```text
    /// Program([
    ///     ExpressionStmt(BinaryOp(Number(1) + Number(2)))
    /// ])
    /// ```
    #[test]
    fn test_parse_expression() {
        let tokens = vec![
            tok_at(TokenType::Number, "1", 1, 1),
            tok_at(TokenType::Plus, "+", 1, 3),
            tok_at(TokenType::Number, "2", 1, 5),
            tok_at(TokenType::Eof, "", 1, 6),
        ];

        let mut parser = Parser::new(tokens);
        let result = parser.parse().unwrap();

        let expected = ASTNode::Program(vec![
            ASTNode::ExpressionStmt(Box::new(ASTNode::BinaryOp {
                left: Box::new(ASTNode::Number(1.0)),
                op: "+".to_string(),
                right: Box::new(ASTNode::Number(2.0)),
            }))
        ]);

        assert_eq!(result, expected);
    }

    /// Test parsing an assignment: `x = 42`.
    ///
    /// Expected AST:
    ///
    /// ```text
    /// Program([
    ///     Assignment { target: "x", value: Number(42) }
    /// ])
    /// ```
    #[test]
    fn test_parse_assignment() {
        let tokens = vec![
            tok_at(TokenType::Name, "x", 1, 1),
            tok_at(TokenType::Equals, "=", 1, 3),
            tok_at(TokenType::Number, "42", 1, 5),
            tok_at(TokenType::Eof, "", 1, 7),
        ];

        let mut parser = Parser::new(tokens);
        let result = parser.parse().unwrap();

        match &result {
            ASTNode::Program(stmts) => {
                assert_eq!(stmts.len(), 1);
                match &stmts[0] {
                    ASTNode::Assignment { target, value } => {
                        assert_eq!(target, "x");
                        assert_eq!(**value, ASTNode::Number(42.0));
                    }
                    other => panic!("Expected Assignment, got {:?}", other),
                }
            }
            other => panic!("Expected Program, got {:?}", other),
        }
    }

    /// Test operator precedence: `1 + 2 * 3` should parse as `1 + (2 * 3)`.
    ///
    /// Multiplication binds tighter than addition because `parse_expression`
    /// calls `parse_term`, which handles `*` before returning.
    #[test]
    fn test_precedence() {
        let tokens = vec![
            tok(TokenType::Number, "1"),
            tok(TokenType::Plus, "+"),
            tok(TokenType::Number, "2"),
            tok(TokenType::Star, "*"),
            tok(TokenType::Number, "3"),
            tok(TokenType::Eof, ""),
        ];

        let mut parser = Parser::new(tokens);
        let result = parser.parse().unwrap();

        // The AST should be: 1 + (2 * 3), not (1 + 2) * 3
        let expected = ASTNode::Program(vec![
            ASTNode::ExpressionStmt(Box::new(ASTNode::BinaryOp {
                left: Box::new(ASTNode::Number(1.0)),
                op: "+".to_string(),
                right: Box::new(ASTNode::BinaryOp {
                    left: Box::new(ASTNode::Number(2.0)),
                    op: "*".to_string(),
                    right: Box::new(ASTNode::Number(3.0)),
                }),
            }))
        ]);

        assert_eq!(result, expected);
    }

    /// Test parenthesized expression: `(1 + 2) * 3`.
    ///
    /// Parentheses override the default precedence, making addition
    /// happen before multiplication.
    #[test]
    fn test_parentheses() {
        let tokens = vec![
            tok(TokenType::LParen, "("),
            tok(TokenType::Number, "1"),
            tok(TokenType::Plus, "+"),
            tok(TokenType::Number, "2"),
            tok(TokenType::RParen, ")"),
            tok(TokenType::Star, "*"),
            tok(TokenType::Number, "3"),
            tok(TokenType::Eof, ""),
        ];

        let mut parser = Parser::new(tokens);
        let result = parser.parse().unwrap();

        // The AST should be: (1 + 2) * 3
        let expected = ASTNode::Program(vec![
            ASTNode::ExpressionStmt(Box::new(ASTNode::BinaryOp {
                left: Box::new(ASTNode::BinaryOp {
                    left: Box::new(ASTNode::Number(1.0)),
                    op: "+".to_string(),
                    right: Box::new(ASTNode::Number(2.0)),
                }),
                op: "*".to_string(),
                right: Box::new(ASTNode::Number(3.0)),
            }))
        ]);

        assert_eq!(result, expected);
    }

    /// Test string literal parsing.
    #[test]
    fn test_string_literal() {
        let tokens = vec![
            tok(TokenType::String, "hello"),
            tok(TokenType::Eof, ""),
        ];

        let mut parser = Parser::new(tokens);
        let result = parser.parse().unwrap();

        let expected = ASTNode::Program(vec![
            ASTNode::ExpressionStmt(Box::new(ASTNode::String("hello".to_string())))
        ]);

        assert_eq!(result, expected);
    }

    /// Test variable name in expression.
    #[test]
    fn test_name_expression() {
        let tokens = vec![
            tok(TokenType::Name, "x"),
            tok(TokenType::Plus, "+"),
            tok(TokenType::Name, "y"),
            tok(TokenType::Eof, ""),
        ];

        let mut parser = Parser::new(tokens);
        let result = parser.parse().unwrap();

        let expected = ASTNode::Program(vec![
            ASTNode::ExpressionStmt(Box::new(ASTNode::BinaryOp {
                left: Box::new(ASTNode::Name("x".to_string())),
                op: "+".to_string(),
                right: Box::new(ASTNode::Name("y".to_string())),
            }))
        ]);

        assert_eq!(result, expected);
    }

    /// Test multi-statement program with newlines.
    #[test]
    fn test_multiple_statements() {
        let tokens = vec![
            tok(TokenType::Name, "x"),
            tok(TokenType::Equals, "="),
            tok(TokenType::Number, "1"),
            tok(TokenType::Newline, "\\n"),
            tok(TokenType::Name, "y"),
            tok(TokenType::Equals, "="),
            tok(TokenType::Number, "2"),
            tok(TokenType::Eof, ""),
        ];

        let mut parser = Parser::new(tokens);
        let result = parser.parse().unwrap();

        match &result {
            ASTNode::Program(stmts) => assert_eq!(stmts.len(), 2),
            other => panic!("Expected Program, got {:?}", other),
        }
    }

    /// Test that unexpected tokens produce a descriptive error.
    #[test]
    fn test_error_on_unexpected_token() {
        let tokens = vec![
            tok_at(TokenType::Plus, "+", 1, 1),
            tok_at(TokenType::Eof, "", 1, 2),
        ];

        let mut parser = Parser::new(tokens);
        let result = parser.parse();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("Unexpected token"));
    }

    /// Test assignment with a complex expression on the right side.
    #[test]
    fn test_assignment_with_expression() {
        let tokens = vec![
            tok(TokenType::Name, "result"),
            tok(TokenType::Equals, "="),
            tok(TokenType::Number, "10"),
            tok(TokenType::Plus, "+"),
            tok(TokenType::Number, "20"),
            tok(TokenType::Star, "*"),
            tok(TokenType::Number, "3"),
            tok(TokenType::Eof, ""),
        ];

        let mut parser = Parser::new(tokens);
        let result = parser.parse().unwrap();

        match &result {
            ASTNode::Program(stmts) => {
                assert_eq!(stmts.len(), 1);
                match &stmts[0] {
                    ASTNode::Assignment { target, value } => {
                        assert_eq!(target, "result");
                        // value should be: 10 + (20 * 3)
                        match value.as_ref() {
                            ASTNode::BinaryOp { op, .. } => assert_eq!(op, "+"),
                            other => panic!("Expected BinaryOp, got {:?}", other),
                        }
                    }
                    other => panic!("Expected Assignment, got {:?}", other),
                }
            }
            other => panic!("Expected Program, got {:?}", other),
        }
    }

    /// Test that the parser handles blank lines (consecutive newlines).
    #[test]
    fn test_blank_lines() {
        let tokens = vec![
            tok(TokenType::Newline, "\\n"),
            tok(TokenType::Newline, "\\n"),
            tok(TokenType::Number, "42"),
            tok(TokenType::Newline, "\\n"),
            tok(TokenType::Newline, "\\n"),
            tok(TokenType::Eof, ""),
        ];

        let mut parser = Parser::new(tokens);
        let result = parser.parse().unwrap();

        match &result {
            ASTNode::Program(stmts) => assert_eq!(stmts.len(), 1),
            other => panic!("Expected Program with 1 statement, got {:?}", other),
        }
    }

    /// Test division and subtraction operators.
    #[test]
    fn test_division_and_subtraction() {
        let tokens = vec![
            tok(TokenType::Number, "10"),
            tok(TokenType::Minus, "-"),
            tok(TokenType::Number, "3"),
            tok(TokenType::Slash, "/"),
            tok(TokenType::Number, "2"),
            tok(TokenType::Eof, ""),
        ];

        let mut parser = Parser::new(tokens);
        let result = parser.parse().unwrap();

        // Should be: 10 - (3 / 2) due to precedence
        let expected = ASTNode::Program(vec![
            ASTNode::ExpressionStmt(Box::new(ASTNode::BinaryOp {
                left: Box::new(ASTNode::Number(10.0)),
                op: "-".to_string(),
                right: Box::new(ASTNode::BinaryOp {
                    left: Box::new(ASTNode::Number(3.0)),
                    op: "/".to_string(),
                    right: Box::new(ASTNode::Number(2.0)),
                }),
            }))
        ]);

        assert_eq!(result, expected);
    }

    /// Integration test: tokenize and parse in sequence.
    #[test]
    fn test_lexer_to_parser_integration() {
        let source = "x = 1 + 2";
        let mut lexer = lexer::tokenizer::Lexer::new(source, None);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let result = parser.parse().unwrap();

        match &result {
            ASTNode::Program(stmts) => {
                assert_eq!(stmts.len(), 1);
                match &stmts[0] {
                    ASTNode::Assignment { target, value } => {
                        assert_eq!(target, "x");
                        match value.as_ref() {
                            ASTNode::BinaryOp { left, op, right } => {
                                assert_eq!(**left, ASTNode::Number(1.0));
                                assert_eq!(op, "+");
                                assert_eq!(**right, ASTNode::Number(2.0));
                            }
                            other => panic!("Expected BinaryOp, got {:?}", other),
                        }
                    }
                    other => panic!("Expected Assignment, got {:?}", other),
                }
            }
            other => panic!("Expected Program, got {:?}", other),
        }
    }
}
