//! Formula lexer — converts a formula string into a stream of tokens.
//!
//! A *lexer* (also called a *tokenizer* or *scanner*) reads raw text and
//! groups characters into meaningful units called *tokens*.  Think of it like
//! reading a sentence and recognising individual words and punctuation.
//!
//! For a formula like `=SUM(A1:B3) + 2.5`, the lexer produces:
//!
//! ```text
//! Ident("SUM")  LParen  CellRef("A1")  Colon  CellRef("B3")  RParen
//! Plus  Number(2.5)  Eof
//! ```
//!
//! The leading `=` is consumed before lexing begins.
//!
//! # Design
//!
//! The lexer is a single-pass character scanner.  It holds a slice of bytes
//! (since formula characters are all ASCII) and a current position index.  At
//! each step it peeks at the current character and dispatches to a small
//! handler that consumes the token and advances the cursor.
//!
//! We use `u8` byte indexing rather than `char` iteration because spreadsheet
//! formula syntax is entirely ASCII; this avoids the overhead of UTF-8
//! multi-byte handling.

use crate::FormulaError;

/// A single token from the formula string.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// A floating-point number literal, e.g. `3.14` or `42`.
    Number(f64),
    /// A string literal, e.g. `"hello"` (quotes stripped).
    Str(String),
    /// `TRUE` or `FALSE`.
    Bool(bool),
    /// An identifier (function name), e.g. `SUM` or `IF`.
    Ident(String),
    /// A cell reference, e.g. `A1` or `Z99`.
    CellRef(String),
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `,`
    Comma,
    /// `:`
    Colon,
    /// End of input.
    Eof,
}

/// Tokenise a formula string that has already had its leading `=` stripped.
///
/// Returns a `Vec<Token>` ending with `Token::Eof`, or
/// `Err(FormulaError::Parse)` if the input contains characters or sequences
/// that cannot be tokenised.
pub fn tokenize(input: &str) -> Result<Vec<Token>, FormulaError> {
    let bytes = input.as_bytes();
    let mut pos = 0usize;
    let mut tokens = Vec::new();

    // Helper: peek at the current byte without advancing.
    // Returns `None` at end of input.
    macro_rules! peek {
        () => {
            bytes.get(pos).copied()
        };
    }

    // Helper: consume one byte and advance the cursor.
    macro_rules! advance {
        () => {{
            let b = bytes[pos];
            pos += 1;
            b
        }};
    }

    loop {
        // Skip whitespace — spaces and tabs are allowed anywhere.
        while matches!(peek!(), Some(b' ' | b'\t')) {
            pos += 1;
        }

        match peek!() {
            None => {
                tokens.push(Token::Eof);
                break;
            }

            // ── Single-character punctuation ──────────────────────────────
            Some(b'+') => { advance!(); tokens.push(Token::Plus); }
            Some(b'-') => { advance!(); tokens.push(Token::Minus); }
            Some(b'*') => { advance!(); tokens.push(Token::Star); }
            Some(b'/') => { advance!(); tokens.push(Token::Slash); }
            Some(b'(') => { advance!(); tokens.push(Token::LParen); }
            Some(b')') => { advance!(); tokens.push(Token::RParen); }
            Some(b',') => { advance!(); tokens.push(Token::Comma); }
            Some(b':') => { advance!(); tokens.push(Token::Colon); }

            // ── String literals ───────────────────────────────────────────
            Some(b'"') => {
                advance!(); // consume opening quote
                let start = pos;
                // Scan for the closing quote. We don't support backslash
                // escapes to keep the implementation simple (spreadsheet
                // formula strings rarely need them).
                while peek!().map_or(false, |b| b != b'"') {
                    pos += 1;
                }
                if peek!().is_none() {
                    // Unterminated string literal.
                    return Err(FormulaError::Parse);
                }
                let s = std::str::from_utf8(&bytes[start..pos])
                    .map_err(|_| FormulaError::Parse)?
                    .to_string();
                advance!(); // consume closing quote
                tokens.push(Token::Str(s));
            }

            // ── Numbers ───────────────────────────────────────────────────
            Some(b) if b.is_ascii_digit() || b == b'.' => {
                // Consume digits and an optional decimal point.
                let start = pos;
                while matches!(peek!(), Some(c) if c.is_ascii_digit() || c == b'.') {
                    pos += 1;
                }
                let num_str = std::str::from_utf8(&bytes[start..pos])
                    .map_err(|_| FormulaError::Parse)?;
                let n: f64 = num_str.parse().map_err(|_| FormulaError::Parse)?;
                tokens.push(Token::Number(n));
            }

            // ── Identifiers: TRUE, FALSE, function names, or cell refs ────
            //
            // An identifier starts with a letter.  We then look at what
            // follows:
            //   - If it is a single letter followed immediately by digits,
            //     it could be a cell reference (e.g. "A1").
            //   - "TRUE" and "FALSE" become boolean literals.
            //   - Everything else is treated as a function name (Ident).
            Some(b) if b.is_ascii_alphabetic() => {
                let start = pos;
                // Consume all alphanumeric characters and underscores.
                while matches!(peek!(), Some(c) if c.is_ascii_alphanumeric() || c == b'_') {
                    pos += 1;
                }
                let word = std::str::from_utf8(&bytes[start..pos])
                    .map_err(|_| FormulaError::Parse)?;
                let upper = word.to_ascii_uppercase();

                // Boolean literals.
                if upper == "TRUE" {
                    tokens.push(Token::Bool(true));
                } else if upper == "FALSE" {
                    tokens.push(Token::Bool(false));
                } else if is_cell_ref(word) {
                    // Cell reference: single letter + 1-2 digits.
                    tokens.push(Token::CellRef(upper));
                } else {
                    // Function name or other identifier.
                    tokens.push(Token::Ident(upper));
                }
            }

            // ── Unknown character ─────────────────────────────────────────
            Some(_) => {
                return Err(FormulaError::Parse);
            }
        }
    }

    Ok(tokens)
}

/// Return true if `word` looks like a cell reference: exactly one letter
/// followed by one or two digits, where the row is 1–99.
///
/// Example: "A1" → true, "Z99" → true, "SUM" → false, "AA1" → false.
fn is_cell_ref(word: &str) -> bool {
    let bytes = word.as_bytes();
    if bytes.is_empty() || bytes.len() > 3 {
        return false;
    }
    // First char must be a letter.
    if !bytes[0].is_ascii_alphabetic() {
        return false;
    }
    // Remaining chars must all be digits.
    let rest = &bytes[1..];
    if rest.is_empty() || rest.len() > 2 {
        return false;
    }
    if !rest.iter().all(|b| b.is_ascii_digit()) {
        return false;
    }
    // Row must be 1..=99.
    let row_str = std::str::from_utf8(rest).unwrap_or("0");
    let row: u8 = row_str.parse().unwrap_or(0);
    (1..=99).contains(&row)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_arithmetic() {
        let toks = tokenize("1 + 2 * 3").unwrap();
        assert_eq!(toks[0], Token::Number(1.0));
        assert_eq!(toks[1], Token::Plus);
        assert_eq!(toks[2], Token::Number(2.0));
        assert_eq!(toks[3], Token::Star);
        assert_eq!(toks[4], Token::Number(3.0));
        assert_eq!(toks[5], Token::Eof);
    }

    #[test]
    fn test_tokenize_cell_ref() {
        let toks = tokenize("A1 + Z99").unwrap();
        assert_eq!(toks[0], Token::CellRef("A1".to_string()));
        assert_eq!(toks[1], Token::Plus);
        assert_eq!(toks[2], Token::CellRef("Z99".to_string()));
    }

    #[test]
    fn test_tokenize_function_call() {
        let toks = tokenize("SUM(A1:B2)").unwrap();
        assert_eq!(toks[0], Token::Ident("SUM".to_string()));
        assert_eq!(toks[1], Token::LParen);
        assert_eq!(toks[2], Token::CellRef("A1".to_string()));
        assert_eq!(toks[3], Token::Colon);
        assert_eq!(toks[4], Token::CellRef("B2".to_string()));
        assert_eq!(toks[5], Token::RParen);
    }

    #[test]
    fn test_tokenize_string_literal() {
        let toks = tokenize("\"hello\"").unwrap();
        assert_eq!(toks[0], Token::Str("hello".to_string()));
    }

    #[test]
    fn test_tokenize_booleans() {
        let toks = tokenize("TRUE").unwrap();
        assert_eq!(toks[0], Token::Bool(true));
        let toks = tokenize("FALSE").unwrap();
        assert_eq!(toks[0], Token::Bool(false));
    }

    #[test]
    fn test_is_cell_ref() {
        assert!(is_cell_ref("A1"));
        assert!(is_cell_ref("Z99"));
        assert!(is_cell_ref("B12"));
        assert!(!is_cell_ref("AA1"));
        assert!(!is_cell_ref("SUM"));
        assert!(!is_cell_ref("A0"));
        assert!(!is_cell_ref("A100"));
        assert!(!is_cell_ref("A"));
    }

    #[test]
    fn test_tokenize_unterminated_string() {
        assert!(tokenize("\"hello").is_err());
    }

    #[test]
    fn test_tokenize_unknown_char() {
        assert!(tokenize("@foo").is_err());
    }
}
