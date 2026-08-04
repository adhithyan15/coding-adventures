//! Hand-rolled Pratt parser for Excel-style formulas.
//!
//! Phase-1 scope:
//! - Numeric literals (integer + decimal + scientific).
//! - String literals: `"hello"`, with `""` as the escape for a quote.
//! - Boolean literals (case-insensitive `TRUE` / `FALSE`).
//! - Error literals (`#REF!`, `#NAME?`, `#DIV/0!`, `#VALUE!`, `#N/A`,
//!   `#NUM!`, `#NULL!`).
//! - Cell references in A1 notation (`A1`, `$B$5`, `AA17`).
//! - Range references (`A1:B10`).
//! - Function calls (`SUM(A1:A10)`, `IF(A1>0, "yes", "no")`).
//! - All binary operators (`+`, `-`, `*`, `/`, `^`, `&`, `=`, `<>`,
//!   `<`, `<=`, `>`, `>=`), with Excel precedence.
//! - Unary `+` and `-`.
//! - Postfix `%` (divide by 100).
//! - Parenthesised sub-expressions.
//! - Optional leading `=` (formulas with or without).
//!
//! Out of scope for Phase 1 (queued for Phase 2):
//! - Structured table references (`Table1[Col]`).
//! - R1C1 notation.
//! - 3-D refs (`Sheet1:Sheet3!A1`).
//! - Array literals `{1, 2; 3, 4}`.
//! - Implicit intersection.

use crate::address::{CellAddress, CellRange};
use crate::ast::{BinaryOp, FormulaAst, UnaryOp};
use crate::cell::CellValue;
use crate::errors::SpreadsheetError;

/// Parser errors. These are distinct from `SpreadsheetError` because
/// they arise *before* a formula is evaluated.
#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    /// Unexpected end of input.
    UnexpectedEof,
    /// Unexpected character at the given byte offset.
    UnexpectedChar {
        /// 0-based byte offset.
        position: usize,
        /// The character.
        ch: char,
    },
    /// Expected one thing, got another.
    ExpectedToken {
        /// What was expected (e.g. `")"` or `"number"`).
        expected: &'static str,
        /// What was actually found.
        found: String,
    },
    /// Malformed numeric literal.
    BadNumber {
        /// The text that failed to parse.
        text: String,
    },
    /// Malformed string literal (unterminated, etc.).
    BadString,
    /// Malformed cell or range reference.
    BadReference {
        /// The bad reference text.
        text: String,
    },
    /// The formula nests deeper than [`MAX_PARSE_DEPTH`]. Rejecting
    /// over-deep input keeps the recursive-descent parser (and the
    /// recursive `evaluate` / `collect_refs` walks over the resulting
    /// AST) from overflowing the native stack on adversarial input
    /// such as `=((((((…))))))` or `=-----…1`.
    TooDeep,
}

impl core::fmt::Display for ParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ParseError::UnexpectedEof => f.write_str("unexpected end of formula"),
            ParseError::UnexpectedChar { position, ch } => {
                write!(f, "unexpected '{ch}' at position {position}")
            }
            ParseError::ExpectedToken { expected, found } => {
                write!(f, "expected {expected}, found '{found}'")
            }
            ParseError::BadNumber { text } => write!(f, "bad number: '{text}'"),
            ParseError::BadString => f.write_str("malformed string literal"),
            ParseError::BadReference { text } => write!(f, "bad reference: '{text}'"),
            ParseError::TooDeep => f.write_str("formula nested too deeply"),
        }
    }
}

/// Maximum expression-nesting depth the parser will accept. Each
/// parenthesis, unary operator, and function-argument level descends
/// one frame in the recursive-descent parser; capping the depth bounds
/// stack usage on adversarial input and, transitively, bounds the
/// depth of the recursive `evaluate` and `collect_refs` AST walks.
const MAX_PARSE_DEPTH: u32 = 256;

impl std::error::Error for ParseError {}

/// Parse an Excel-style formula. The leading `=` is optional.
pub fn parse(input: &str) -> Result<FormulaAst, ParseError> {
    let trimmed = input.trim();
    let body = trimmed.strip_prefix('=').unwrap_or(trimmed);
    let mut p = Parser::new(body);
    let ast = p.parse_expr(0)?;
    p.skip_whitespace();
    if !p.is_eof() {
        let ch = p.peek().unwrap();
        return Err(ParseError::UnexpectedChar {
            position: p.pos,
            ch,
        });
    }
    Ok(ast)
}

struct Parser<'a> {
    input: &'a [u8],
    pos: usize,
    /// Current expression-nesting depth, bounded by [`MAX_PARSE_DEPTH`].
    depth: u32,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input: input.as_bytes(),
            pos: 0,
            depth: 0,
        }
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn peek(&self) -> Option<char> {
        self.input.get(self.pos).map(|&b| b as char)
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.input.get(self.pos + offset).map(|&b| b as char)
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += 1;
        Some(c)
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_ascii_whitespace() {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    /// Pratt-style expression parser. `min_prec` is the minimum
    /// binary precedence we'll accept.
    ///
    /// Every recursive descent (parentheses, unary operators, function
    /// arguments) re-enters through here, so a single depth guard on
    /// this method bounds the parser's total stack usage. Without it,
    /// input like `=((((((…))))))` overflows the native stack — an
    /// uncatchable abort, and a WASM trap, on the host.
    fn parse_expr(&mut self, min_prec: u8) -> Result<FormulaAst, ParseError> {
        self.depth += 1;
        if self.depth > MAX_PARSE_DEPTH {
            self.depth -= 1;
            return Err(ParseError::TooDeep);
        }
        let result = self.parse_expr_inner(min_prec);
        self.depth -= 1;
        result
    }

    fn parse_expr_inner(&mut self, min_prec: u8) -> Result<FormulaAst, ParseError> {
        let mut lhs = self.parse_prefix()?;
        loop {
            // Postfix `%` binds tighter than any binary.
            self.skip_whitespace();
            if self.peek() == Some('%') {
                self.advance();
                lhs = FormulaAst::Percent(Box::new(lhs));
                continue;
            }
            // Binary operators.
            self.skip_whitespace();
            let (op, op_len) = match self.peek_binary_op() {
                Some(o) => o,
                None => break,
            };
            let prec = op.precedence();
            if prec < min_prec {
                break;
            }
            self.pos += op_len;
            let next_min = if op.right_associative() {
                prec
            } else {
                prec + 1
            };
            let rhs = self.parse_expr(next_min)?;
            lhs = FormulaAst::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn peek_binary_op(&self) -> Option<(BinaryOp, usize)> {
        // Order matters — try two-char operators first.
        let two = match self.peek_at(0).zip(self.peek_at(1)) {
            Some(('<', '>')) => Some(BinaryOp::Ne),
            Some(('<', '=')) => Some(BinaryOp::Le),
            Some(('>', '=')) => Some(BinaryOp::Ge),
            _ => None,
        };
        if let Some(op) = two {
            return Some((op, 2));
        }
        let one = match self.peek_at(0)? {
            '+' => BinaryOp::Add,
            '-' => BinaryOp::Sub,
            '*' => BinaryOp::Mul,
            '/' => BinaryOp::Div,
            '^' => BinaryOp::Pow,
            '&' => BinaryOp::Concat,
            '=' => BinaryOp::Eq,
            '<' => BinaryOp::Lt,
            '>' => BinaryOp::Gt,
            _ => return None,
        };
        Some((one, 1))
    }

    /// Prefix: unary +/-, parenthesised expr, literal, ref, function.
    fn parse_prefix(&mut self) -> Result<FormulaAst, ParseError> {
        self.skip_whitespace();
        let c = self.peek().ok_or(ParseError::UnexpectedEof)?;
        match c {
            '-' => {
                self.advance();
                let inner = self.parse_expr(BinaryOp::Mul.precedence())?;
                Ok(FormulaAst::Unary {
                    op: UnaryOp::Negate,
                    operand: Box::new(inner),
                })
            }
            '+' => {
                self.advance();
                let inner = self.parse_expr(BinaryOp::Mul.precedence())?;
                Ok(FormulaAst::Unary {
                    op: UnaryOp::Plus,
                    operand: Box::new(inner),
                })
            }
            '(' => {
                self.advance();
                let inner = self.parse_expr(0)?;
                self.skip_whitespace();
                if self.advance() != Some(')') {
                    return Err(ParseError::ExpectedToken {
                        expected: ")",
                        found: "end of input".into(),
                    });
                }
                Ok(inner)
            }
            '"' => self.parse_string(),
            '\'' => self.parse_quoted_sheet_ref(),
            '#' => self.parse_error_literal(),
            '0'..='9' | '.' => self.parse_number(),
            c if c.is_ascii_alphabetic() || c == '$' || c == '@' => self.parse_ident_or_ref(),
            _ => Err(ParseError::UnexpectedChar {
                position: self.pos,
                ch: c,
            }),
        }
    }

    fn parse_string(&mut self) -> Result<FormulaAst, ParseError> {
        // Opening quote.
        self.advance();
        let mut out = String::new();
        loop {
            match self.advance() {
                None => return Err(ParseError::BadString),
                Some('"') => {
                    // Escaped quote `""`?
                    if self.peek() == Some('"') {
                        out.push('"');
                        self.advance();
                    } else {
                        return Ok(FormulaAst::Literal(CellValue::Text(out)));
                    }
                }
                Some(c) => out.push(c),
            }
        }
    }

    fn parse_number(&mut self) -> Result<FormulaAst, ParseError> {
        let start = self.pos;
        // Integer part.
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.advance();
            } else {
                break;
            }
        }
        // Fractional part.
        if self.peek() == Some('.') {
            self.advance();
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        // Exponent.
        if matches!(self.peek(), Some('e') | Some('E')) {
            self.advance();
            if matches!(self.peek(), Some('+') | Some('-')) {
                self.advance();
            }
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        let text = std::str::from_utf8(&self.input[start..self.pos])
            .unwrap()
            .to_string();
        let n: f64 = text.parse().map_err(|_| ParseError::BadNumber {
            text: text.clone(),
        })?;
        Ok(FormulaAst::Literal(CellValue::Number(n)))
    }

    fn parse_error_literal(&mut self) -> Result<FormulaAst, ParseError> {
        // Each error has a fixed text. Find the longest match.
        let rest = std::str::from_utf8(&self.input[self.pos..]).unwrap();
        for (text, err) in [
            ("#REF!", SpreadsheetError::Ref),
            ("#NAME?", SpreadsheetError::Name),
            ("#DIV/0!", SpreadsheetError::DivZero),
            ("#VALUE!", SpreadsheetError::Value),
            ("#N/A", SpreadsheetError::NotAvailable),
            ("#NUM!", SpreadsheetError::Num),
            ("#NULL!", SpreadsheetError::Null),
            ("#CALC!", SpreadsheetError::Calc),
            ("#SPILL!", SpreadsheetError::Spill),
        ] {
            if rest.starts_with(text) {
                self.pos += text.len();
                return Ok(FormulaAst::Literal(CellValue::Error(err)));
            }
        }
        Err(ParseError::UnexpectedChar {
            position: self.pos,
            ch: '#',
        })
    }

    fn parse_ident_or_ref(&mut self) -> Result<FormulaAst, ParseError> {
        let start = self.pos;
        // Accept `$`, letters, digits, `.`, `_`.
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == '$' || c == '.' || c == '_' || c == '@' {
                self.advance();
            } else {
                break;
            }
        }
        let text = std::str::from_utf8(&self.input[start..self.pos]).unwrap();
        if text.is_empty() {
            return Err(ParseError::UnexpectedEof);
        }
        // Lotus-style `@SUM` — strip the leading `@`.
        let core_text = text.strip_prefix('@').unwrap_or(text);

        // Sheet-qualified reference: `Summary!A1` / `Summary!A1:B2`. A `!`
        // immediately after a bareword makes that word a **sheet name** (never a
        // cell), per the disambiguation rule — so `A1!B2` reads as "cell B2 on a
        // sheet literally named A1". The name is kept as written and resolved to a
        // `SheetId` later (by a workbook); an unknown sheet becomes `#REF!` at
        // evaluation, not a parse error.
        if self.peek() == Some('!') {
            self.advance(); // consume '!'
            let token = self.read_ref_token();
            return self.finish_ref_or_range(Some(core_text.to_string()), &token);
        }

        // Boolean literals.
        match core_text.to_ascii_uppercase().as_str() {
            "TRUE" => {
                // Distinguish TRUE() function-call form.
                if self.peek() == Some('(') {
                    return self.finish_function_call(core_text.to_string());
                }
                return Ok(FormulaAst::Literal(CellValue::Boolean(true)));
            }
            "FALSE" => {
                if self.peek() == Some('(') {
                    return self.finish_function_call(core_text.to_string());
                }
                return Ok(FormulaAst::Literal(CellValue::Boolean(false)));
            }
            _ => {}
        }

        // Function call?
        self.skip_whitespace();
        if self.peek() == Some('(') {
            return self.finish_function_call(core_text.to_string());
        }

        // Cell or range reference (unqualified — resolves against the formula's
        // own sheet).
        if CellAddress::parse(core_text).is_ok() {
            return self.finish_ref_or_range(None, core_text);
        }

        // Otherwise: bare name — treat as a zero-arg function call
        // (matches Excel where named ranges like `MY_NAME` resolve at
        // dispatch time).
        Err(ParseError::BadReference {
            text: core_text.to_string(),
        })
    }

    /// Read a bare cell/range token (`A1`, `$B$5`) from the cursor — the
    /// letters/digits/`$` run — and return it as an owned string. Stops at the
    /// first character that can't be part of an A1 address.
    fn read_ref_token(&mut self) -> String {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == '$' {
                self.advance();
            } else {
                break;
            }
        }
        std::str::from_utf8(&self.input[start..self.pos])
            .unwrap()
            .to_string()
    }

    /// Build a [`FormulaAst::Ref`] or [`FormulaAst::Range`] from a first A1 token
    /// (already read) and an optional sheet qualifier, consuming a `:end` token if
    /// the reference is a range. A token that doesn't parse as an address is a
    /// `BadReference`.
    fn finish_ref_or_range(
        &mut self,
        sheet: Option<String>,
        first_token: &str,
    ) -> Result<FormulaAst, ParseError> {
        let addr = CellAddress::parse(first_token).map_err(|_| ParseError::BadReference {
            text: first_token.to_string(),
        })?;
        if self.peek() == Some(':') {
            self.advance();
            let next_text = self.read_ref_token();
            let end_addr =
                CellAddress::parse(&next_text).map_err(|_| ParseError::BadReference {
                    text: next_text.clone(),
                })?;
            return Ok(FormulaAst::Range {
                sheet,
                range: CellRange::new(addr, end_addr),
            });
        }
        Ok(FormulaAst::Ref { sheet, addr })
    }

    /// Parse a single-quoted sheet name and the reference it qualifies:
    /// `'Q1 Budget'!A1` / `'O''Brien'!A1:B2`. A doubled `''` inside the quotes is
    /// a literal apostrophe (the Excel rule). The `!` is required.
    fn parse_quoted_sheet_ref(&mut self) -> Result<FormulaAst, ParseError> {
        self.advance(); // opening quote
        let mut name = String::new();
        loop {
            match self.advance() {
                None => return Err(ParseError::UnexpectedEof),
                Some('\'') => {
                    if self.peek() == Some('\'') {
                        self.advance(); // doubled '' → a literal apostrophe
                        name.push('\'');
                    } else {
                        break; // closing quote
                    }
                }
                Some(c) => name.push(c),
            }
        }
        if self.advance() != Some('!') {
            return Err(ParseError::ExpectedToken {
                expected: "!",
                found: "quoted sheet name without '!'".into(),
            });
        }
        let token = self.read_ref_token();
        self.finish_ref_or_range(Some(name), &token)
    }

    fn finish_function_call(&mut self, name: String) -> Result<FormulaAst, ParseError> {
        self.skip_whitespace();
        // Expect `(`.
        if self.advance() != Some('(') {
            return Err(ParseError::ExpectedToken {
                expected: "(",
                found: format!("near {name}"),
            });
        }
        let mut args = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(')') {
            self.advance();
            return Ok(FormulaAst::Call { name, args });
        }
        loop {
            let arg = self.parse_expr(0)?;
            args.push(arg);
            self.skip_whitespace();
            match self.peek() {
                Some(',') => {
                    self.advance();
                    self.skip_whitespace();
                }
                Some(')') => {
                    self.advance();
                    break;
                }
                _ => {
                    return Err(ParseError::ExpectedToken {
                        expected: ", or )",
                        found: format!("at position {}", self.pos),
                    });
                }
            }
        }
        Ok(FormulaAst::Call { name, args })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_integer() {
        let r = parse("42").unwrap();
        assert_eq!(r, FormulaAst::Literal(CellValue::Number(42.0)));
    }

    #[test]
    // 3.14 here is arbitrary numeric test data, not an approximation of PI.
    #[allow(clippy::approx_constant)]
    fn parse_decimal_with_leading_equals() {
        let r = parse("=3.14").unwrap();
        assert_eq!(r, FormulaAst::Literal(CellValue::Number(3.14)));
    }

    #[test]
    fn parse_scientific_notation() {
        let r = parse("1.5e2").unwrap();
        assert_eq!(r, FormulaAst::Literal(CellValue::Number(150.0)));
        let r = parse("2e-3").unwrap();
        assert!(matches!(r, FormulaAst::Literal(CellValue::Number(_))));
    }

    #[test]
    fn parse_string_with_escaped_quote() {
        let r = parse("\"hello\"").unwrap();
        assert_eq!(r, FormulaAst::Literal(CellValue::Text("hello".into())));
        let r = parse("\"say \"\"hi\"\"\"").unwrap();
        assert_eq!(r, FormulaAst::Literal(CellValue::Text("say \"hi\"".into())));
    }

    #[test]
    fn parse_booleans_case_insensitive() {
        for s in ["TRUE", "true", "True"] {
            let r = parse(s).unwrap();
            assert_eq!(r, FormulaAst::Literal(CellValue::Boolean(true)));
        }
    }

    #[test]
    fn parse_error_literals() {
        for s in ["#REF!", "#NAME?", "#DIV/0!", "#N/A", "#NUM!"] {
            let r = parse(s).unwrap();
            assert!(matches!(r, FormulaAst::Literal(CellValue::Error(_))));
        }
    }

    #[test]
    fn parse_cell_ref() {
        let r = parse("A1").unwrap();
        assert_eq!(r, FormulaAst::cell(CellAddress::new(1, 1)));
        let r = parse("$B$5").unwrap();
        assert_eq!(r, FormulaAst::cell(CellAddress::absolute(5, 2)));
    }

    #[test]
    fn parse_range() {
        let r = parse("A1:B10").unwrap();
        if let FormulaAst::Range { range: r, .. } = r {
            assert_eq!(r.start, CellAddress::new(1, 1));
            assert_eq!(r.end, CellAddress::new(10, 2));
        } else {
            panic!("expected Range, got {r:?}");
        }
    }

    #[test]
    fn parse_cross_sheet_ref() {
        // An unqualified ref carries no sheet.
        assert_eq!(
            parse("A1").unwrap(),
            FormulaAst::Ref {
                sheet: None,
                addr: CellAddress::new(1, 1)
            }
        );
        // `Summary!A1` qualifies the ref with the sheet name (as written).
        assert_eq!(
            parse("Summary!A1").unwrap(),
            FormulaAst::sheet_cell("Summary", CellAddress::new(1, 1))
        );
        // `Summary!A1:B2` qualifies a range; both endpoints share the qualifier.
        assert_eq!(
            parse("Summary!A1:B2").unwrap(),
            FormulaAst::sheet_range("Summary", CellRange::new(CellAddress::new(1, 1), CellAddress::new(2, 2)))
        );
    }

    #[test]
    fn parse_quoted_cross_sheet_ref() {
        // A single-quoted name allows spaces/punctuation.
        assert_eq!(
            parse("'Q1 Budget'!A1").unwrap(),
            FormulaAst::sheet_cell("Q1 Budget", CellAddress::new(1, 1))
        );
        // Doubled '' inside the quotes is a literal apostrophe.
        assert_eq!(
            parse("'O''Brien'!A1").unwrap(),
            FormulaAst::sheet_cell("O'Brien", CellAddress::new(1, 1))
        );
    }

    #[test]
    fn cross_sheet_ref_disambiguates_a1_named_sheet() {
        // A `!` makes the token before it a sheet name, never a cell — so
        // `'A1'!B2` is "cell B2 on the sheet named A1", not a range.
        assert_eq!(
            parse("'A1'!B2").unwrap(),
            FormulaAst::sheet_cell("A1", CellAddress::new(2, 2))
        );
    }

    #[test]
    fn parse_arithmetic_with_precedence() {
        let r = parse("=1 + 2 * 3").unwrap();
        if let FormulaAst::Binary { op, lhs, rhs } = r {
            assert_eq!(op, BinaryOp::Add);
            assert_eq!(*lhs, FormulaAst::Literal(CellValue::Number(1.0)));
            if let FormulaAst::Binary { op: inner_op, .. } = &*rhs {
                assert_eq!(*inner_op, BinaryOp::Mul);
            } else {
                panic!("rhs should be a multiplication");
            }
        } else {
            panic!("expected addition at top");
        }
    }

    #[test]
    fn parse_exponentiation_is_right_associative() {
        // 2^3^2 should parse as 2^(3^2) = 2^9 = 512.
        let r = parse("=2^3^2").unwrap();
        if let FormulaAst::Binary { op, lhs, rhs } = r {
            assert_eq!(op, BinaryOp::Pow);
            assert_eq!(*lhs, FormulaAst::Literal(CellValue::Number(2.0)));
            // rhs is 3^2.
            assert!(matches!(*rhs, FormulaAst::Binary { op: BinaryOp::Pow, .. }));
        } else {
            panic!("expected top-level Pow");
        }
    }

    #[test]
    fn parse_function_call() {
        let r = parse("=SUM(A1:A10)").unwrap();
        if let FormulaAst::Call { name, args } = r {
            assert_eq!(name, "SUM");
            assert_eq!(args.len(), 1);
            assert!(matches!(args[0], FormulaAst::Range { .. }));
        } else {
            panic!("expected Call");
        }
    }

    #[test]
    fn parse_nested_function_call() {
        let r = parse("=IF(A1>0, SUM(B1:B5), 0)").unwrap();
        if let FormulaAst::Call { name, args } = r {
            assert_eq!(name, "IF");
            assert_eq!(args.len(), 3);
        } else {
            panic!("expected IF call");
        }
    }

    #[test]
    fn parse_visicalc_at_function_prefix() {
        let r = parse("@SUM(A1:A5)").unwrap();
        if let FormulaAst::Call { name, .. } = r {
            assert_eq!(name, "SUM");
        } else {
            panic!("expected Call (stripped @)");
        }
    }

    #[test]
    fn parse_percent_postfix() {
        // 50% should be Percent(50).
        let r = parse("50%").unwrap();
        if let FormulaAst::Percent(inner) = r {
            assert_eq!(*inner, FormulaAst::Literal(CellValue::Number(50.0)));
        } else {
            panic!("expected Percent");
        }
    }

    #[test]
    fn parse_unary_negate() {
        let r = parse("-A1").unwrap();
        if let FormulaAst::Unary { op, operand } = r {
            assert_eq!(op, UnaryOp::Negate);
            assert_eq!(*operand, FormulaAst::cell(CellAddress::new(1, 1)));
        } else {
            panic!("expected Unary");
        }
    }

    #[test]
    fn parse_parenthesised_expression() {
        let r = parse("=(1 + 2) * 3").unwrap();
        if let FormulaAst::Binary { op, .. } = r {
            assert_eq!(op, BinaryOp::Mul);
        } else {
            panic!("expected multiplication at top");
        }
    }

    #[test]
    fn parse_comparisons() {
        for s in ["A1=5", "A1<>5", "A1<5", "A1<=5", "A1>=5"] {
            let r = parse(s).unwrap();
            assert!(matches!(r, FormulaAst::Binary { .. }));
        }
    }

    #[test]
    fn parse_concatenation() {
        let r = parse("=\"a\"&\"b\"").unwrap();
        if let FormulaAst::Binary { op, .. } = r {
            assert_eq!(op, BinaryOp::Concat);
        } else {
            panic!("expected Concat");
        }
    }

    #[test]
    fn parse_rejects_trailing_garbage() {
        assert!(parse("1 + 2 garbage").is_err());
    }

    #[test]
    fn parse_rejects_unclosed_paren() {
        assert!(parse("(1 + 2").is_err());
    }

    #[test]
    fn deeply_nested_parens_error_instead_of_overflowing() {
        // Far past MAX_PARSE_DEPTH. A recursive-descent parser without
        // a depth guard would blow the native stack here; we must get a
        // clean `TooDeep` error back instead of aborting the process.
        let deep = format!("{}1{}", "(".repeat(5000), ")".repeat(5000));
        assert_eq!(parse(&deep), Err(ParseError::TooDeep));
    }

    #[test]
    fn deeply_stacked_unary_errors_instead_of_overflowing() {
        let deep = format!("{}1", "-".repeat(5000));
        assert_eq!(parse(&deep), Err(ParseError::TooDeep));
    }

    #[test]
    fn moderate_nesting_still_parses() {
        // Comfortably under the cap — must still succeed.
        let ok = format!("{}1{}", "(".repeat(100), ")".repeat(100));
        assert!(parse(&ok).is_ok());
    }
}
