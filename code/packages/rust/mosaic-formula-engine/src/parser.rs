//! Formula parser — converts a token stream into an abstract syntax tree.
//!
//! The parser implements *recursive descent* — a family of parsing algorithms
//! where each grammar rule becomes a function that calls other functions for
//! sub-rules.  Recursive descent is easy to read and write, produces great
//! error messages, and handles all LL(k) grammars (grammars that can be
//! parsed by looking at the next k tokens without backtracking).
//!
//! Our grammar (from FE01):
//!
//! ```text
//! expr     = term { ('+' | '-') term }
//! term     = factor { ('*' | '/') factor }
//! factor   = NUMBER | STRING | BOOL | cell_ref | range | func_call
//!          | '(' expr ')' | '-' factor
//! func_call = IDENT '(' [arg {',' arg}] ')'
//! arg      = range | expr
//! range    = CellRef ':' CellRef
//! ```
//!
//! The grammar is left-recursive in `expr` and `term` but we handle that by
//! looping: instead of recursing left, we parse the first operand and then
//! consume operator–operand pairs in a `while` loop.
//!
//! # Operator precedence (highest first)
//!
//! 1. Unary minus (`-factor`)
//! 2. `*` and `/` (parsed in `term`)
//! 3. `+` and `-` (parsed in `expr`)

use crate::lexer::Token;
use crate::FormulaError;

/// The abstract syntax tree (AST) for a formula expression.
///
/// An AST is a tree where each node represents a syntactic construct.
/// For example, `1 + 2 * 3` becomes:
///
/// ```text
/// Add(
///   Number(1),
///   Mul(Number(2), Number(3))
/// )
/// ```
///
/// because `*` binds tighter than `+`.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// A numeric literal, e.g. `3.14`.
    Number(f64),
    /// A string literal, e.g. `"hello"`.
    Str(String),
    /// A boolean literal: `TRUE` or `FALSE`.
    Bool(bool),
    /// A reference to a single cell, e.g. `A1`.
    CellRef(String),
    /// A range of cells, e.g. `A1:C3`. Ranges are only valid inside
    /// function arguments.
    Range(String, String),
    /// Binary addition.
    Add(Box<Expr>, Box<Expr>),
    /// Binary subtraction.
    Sub(Box<Expr>, Box<Expr>),
    /// Binary multiplication.
    Mul(Box<Expr>, Box<Expr>),
    /// Binary division.
    Div(Box<Expr>, Box<Expr>),
    /// Unary negation.
    Neg(Box<Expr>),
    /// A function call, e.g. `SUM(A1:B2, 3)`.
    FuncCall {
        name: String,
        args: Vec<Expr>,
    },
}

/// A parser wraps a slice of tokens and a current position.
///
/// The parser is *not* public; external code calls the free function
/// [`parse`] instead.
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
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    /// Consume and return the current token, advancing the position.
    fn advance(&mut self) -> Token {
        let tok = self.tokens.get(self.pos).cloned().unwrap_or(Token::Eof);
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    /// Consume the current token if it matches `expected`, or return an error.
    fn expect(&mut self, expected: &Token) -> Result<(), FormulaError> {
        if std::mem::discriminant(self.peek()) == std::mem::discriminant(expected) {
            self.advance();
            Ok(())
        } else {
            Err(FormulaError::Parse)
        }
    }

    // ── Grammar rules ────────────────────────────────────────────────────

    /// Parse: `expr = term { ('+' | '-') term }`
    ///
    /// We handle addition and subtraction here (lowest precedence of the
    /// binary operators).  The loop consumes as many `+ term` or `- term`
    /// suffixes as are present.
    fn parse_expr(&mut self) -> Result<Expr, FormulaError> {
        let mut left = self.parse_term()?;
        loop {
            match self.peek() {
                Token::Plus => {
                    self.advance();
                    let right = self.parse_term()?;
                    left = Expr::Add(Box::new(left), Box::new(right));
                }
                Token::Minus => {
                    self.advance();
                    let right = self.parse_term()?;
                    left = Expr::Sub(Box::new(left), Box::new(right));
                }
                _ => break,
            }
        }
        Ok(left)
    }

    /// Parse: `term = factor { ('*' | '/') factor }`
    ///
    /// Multiplication and division bind more tightly than addition and
    /// subtraction, so we handle them in a separate function called by
    /// `parse_expr`.
    fn parse_term(&mut self) -> Result<Expr, FormulaError> {
        let mut left = self.parse_factor()?;
        loop {
            match self.peek() {
                Token::Star => {
                    self.advance();
                    let right = self.parse_factor()?;
                    left = Expr::Mul(Box::new(left), Box::new(right));
                }
                Token::Slash => {
                    self.advance();
                    let right = self.parse_factor()?;
                    left = Expr::Div(Box::new(left), Box::new(right));
                }
                _ => break,
            }
        }
        Ok(left)
    }

    /// Parse: `factor = NUMBER | STRING | BOOL | cell_ref_or_range | func_call
    ///                  | '(' expr ')' | '-' factor`
    ///
    /// This is the "atoms" level of the grammar — the things that can appear
    /// on either side of a `*` or `/`.
    fn parse_factor(&mut self) -> Result<Expr, FormulaError> {
        match self.peek().clone() {
            Token::Number(n) => {
                self.advance();
                Ok(Expr::Number(n))
            }
            Token::Str(s) => {
                self.advance();
                Ok(Expr::Str(s))
            }
            Token::Bool(b) => {
                self.advance();
                Ok(Expr::Bool(b))
            }

            // Cell reference — might be the start of a range (A1:B2).
            // We peek ahead: if the next token after the CellRef is a Colon,
            // it's a range.  Otherwise it's just a cell reference.
            // Ranges as standalone expressions are rejected at eval time (they
            // are only valid inside function arguments); the parser accepts
            // them here for simplicity.
            Token::CellRef(name) => {
                self.advance();
                if matches!(self.peek(), Token::Colon) {
                    self.advance(); // consume ':'
                    if let Token::CellRef(end_name) = self.peek().clone() {
                        self.advance();
                        Ok(Expr::Range(name, end_name))
                    } else {
                        Err(FormulaError::Parse)
                    }
                } else {
                    Ok(Expr::CellRef(name))
                }
            }

            // Function call: IDENT '(' [args] ')'
            Token::Ident(func_name) => {
                self.advance();
                self.expect(&Token::LParen)?;
                let args = self.parse_arg_list()?;
                self.expect(&Token::RParen)?;
                Ok(Expr::FuncCall { name: func_name, args })
            }

            // Parenthesised expression: '(' expr ')'
            Token::LParen => {
                self.advance();
                let inner = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                Ok(inner)
            }

            // Unary minus: '-' factor
            Token::Minus => {
                self.advance();
                let operand = self.parse_factor()?;
                Ok(Expr::Neg(Box::new(operand)))
            }

            _ => Err(FormulaError::Parse),
        }
    }

    /// Parse a comma-separated list of function arguments.
    ///
    /// Each argument can be a range (e.g. `A1:C3`) or an expression.
    /// The list may be empty (no arguments).
    fn parse_arg_list(&mut self) -> Result<Vec<Expr>, FormulaError> {
        let mut args = Vec::new();

        // Empty argument list.
        if matches!(self.peek(), Token::RParen) {
            return Ok(args);
        }

        // Parse first argument.
        args.push(self.parse_expr()?);

        // Parse subsequent arguments separated by commas.
        while matches!(self.peek(), Token::Comma) {
            self.advance(); // consume ','
            args.push(self.parse_expr()?);
        }

        Ok(args)
    }

    /// Parse the complete formula and verify no trailing tokens remain.
    fn parse_all(&mut self) -> Result<Expr, FormulaError> {
        let expr = self.parse_expr()?;
        // After parsing, the only acceptable remaining token is Eof.
        if !matches!(self.peek(), Token::Eof) {
            return Err(FormulaError::Parse);
        }
        Ok(expr)
    }
}

/// Parse a token stream produced by the lexer into an [`Expr`] AST.
///
/// The `tokens` slice should come from [`crate::lexer::tokenize`].  This
/// function is the main entry point used by the engine.
pub fn parse(tokens: Vec<Token>) -> Result<Expr, FormulaError> {
    let mut parser = Parser::new(tokens);
    parser.parse_all()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;

    fn parse_formula(s: &str) -> Result<Expr, FormulaError> {
        let tokens = tokenize(s)?;
        parse(tokens)
    }

    #[test]
    fn test_parse_number() {
        assert_eq!(parse_formula("42").unwrap(), Expr::Number(42.0));
    }

    #[test]
    fn test_parse_addition() {
        let expr = parse_formula("1 + 2").unwrap();
        assert_eq!(expr, Expr::Add(Box::new(Expr::Number(1.0)), Box::new(Expr::Number(2.0))));
    }

    #[test]
    fn test_parse_precedence() {
        // 1 + 2 * 3  =>  Add(1, Mul(2, 3))
        let expr = parse_formula("1 + 2 * 3").unwrap();
        match expr {
            Expr::Add(left, right) => {
                assert_eq!(*left, Expr::Number(1.0));
                match *right {
                    Expr::Mul(a, b) => {
                        assert_eq!(*a, Expr::Number(2.0));
                        assert_eq!(*b, Expr::Number(3.0));
                    }
                    _ => panic!("expected Mul"),
                }
            }
            _ => panic!("expected Add"),
        }
    }

    #[test]
    fn test_parse_cell_ref() {
        let expr = parse_formula("A1").unwrap();
        assert_eq!(expr, Expr::CellRef("A1".to_string()));
    }

    #[test]
    fn test_parse_range() {
        let expr = parse_formula("A1:C3").unwrap();
        assert_eq!(expr, Expr::Range("A1".to_string(), "C3".to_string()));
    }

    #[test]
    fn test_parse_func_call() {
        let expr = parse_formula("SUM(A1, A2)").unwrap();
        match expr {
            Expr::FuncCall { name, args } => {
                assert_eq!(name, "SUM");
                assert_eq!(args.len(), 2);
            }
            _ => panic!("expected FuncCall"),
        }
    }

    #[test]
    fn test_parse_unary_neg() {
        let expr = parse_formula("-5").unwrap();
        assert_eq!(expr, Expr::Neg(Box::new(Expr::Number(5.0))));
    }

    #[test]
    fn test_parse_parens() {
        // (1 + 2) * 3  =>  Mul(Add(1,2), 3)
        let expr = parse_formula("(1 + 2) * 3").unwrap();
        match expr {
            Expr::Mul(left, right) => {
                assert_eq!(*left, Expr::Add(Box::new(Expr::Number(1.0)), Box::new(Expr::Number(2.0))));
                assert_eq!(*right, Expr::Number(3.0));
            }
            _ => panic!("expected Mul"),
        }
    }

    #[test]
    fn test_parse_error_trailing_token() {
        // "1 2" has a trailing token after parsing "1".
        assert!(parse_formula("1 2").is_err());
    }

    #[test]
    fn test_parse_empty() {
        // Completely empty formula.
        assert!(parse_formula("").is_err());
    }
}
