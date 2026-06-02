//! # Lexer — source text → token stream.
//!
//! Hand-written tokenizer. The token set is small enough (under 20
//! kinds) that a generated lexer would buy us little — and writing
//! it by hand lets the diagnostic surface (line + column on every
//! error) stay simple.
//!
//! ## Token kinds
//!
//! - **Keywords**: `prior`, `for`, `contributes`, `from`, `to`,
//!   `interacts`, `when`, `and`, `observe`, `source`, `trust`,
//!   `locator`, `consensus`, `authoritative`, `empirical`,
//!   `inferred`, `unattributed`.
//! - **Punctuation**: `(`, `)`, `,`, `?`.
//! - **Literals**: numbers (`0.10`, `1.5`, `2`), quoted strings
//!   (`"…"` with backslash escapes for `"` and `\`).
//! - **Identifiers**: lowercase letter or underscore followed by
//!   letters / digits / underscores. Identifiers shadow keywords
//!   only when not in keyword position — see the parser; the lexer
//!   does keyword recognition eagerly.
//! - **Trivia**: whitespace (skipped), `%` line comments (skipped to
//!   end of line), newlines (significant only for diagnostic spans;
//!   the parser is whitespace-insensitive otherwise).

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    KwPrior,
    KwFor,
    KwContributes,
    KwFrom,
    KwTo,
    KwInteracts,
    KwWhen,
    KwAnd,
    KwObserve,
    KwSource,
    KwTrust,
    KwLocator,
    KwConsensus,
    KwAuthoritative,
    KwEmpirical,
    KwInferred,
    KwUnattributed,
    // Punctuation
    LParen,
    RParen,
    Comma,
    Question,
    // Literals
    Number(f64),
    String(String),
    // Identifiers
    Ident(String),
    // End of input
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LexError {
    UnterminatedString { line: usize, col: usize },
    InvalidNumber { lexeme: String, line: usize, col: usize },
    UnknownCharacter { ch: char, line: usize, col: usize },
}

pub fn lex(src: &str) -> Result<Vec<Token>, LexError> {
    let mut tokens = Vec::new();
    let mut chars = src.chars().peekable();
    let mut line = 1usize;
    let mut col = 1usize;

    while let Some(&c) = chars.peek() {
        match c {
            ' ' | '\t' | '\r' => {
                chars.next();
                col += 1;
            }
            '\n' => {
                chars.next();
                line += 1;
                col = 1;
            }
            '%' => {
                // Line comment to EOL.
                while let Some(&nc) = chars.peek() {
                    if nc == '\n' {
                        break;
                    }
                    chars.next();
                    col += 1;
                }
            }
            '(' => {
                tokens.push(Token { kind: TokenKind::LParen, line, col });
                chars.next();
                col += 1;
            }
            ')' => {
                tokens.push(Token { kind: TokenKind::RParen, line, col });
                chars.next();
                col += 1;
            }
            ',' => {
                tokens.push(Token { kind: TokenKind::Comma, line, col });
                chars.next();
                col += 1;
            }
            '?' => {
                tokens.push(Token { kind: TokenKind::Question, line, col });
                chars.next();
                col += 1;
            }
            '"' => {
                let start_line = line;
                let start_col = col;
                chars.next();
                col += 1;
                let mut s = String::new();
                let mut terminated = false;
                while let Some(&nc) = chars.peek() {
                    match nc {
                        '"' => {
                            chars.next();
                            col += 1;
                            terminated = true;
                            break;
                        }
                        '\\' => {
                            chars.next();
                            col += 1;
                            if let Some(&esc) = chars.peek() {
                                match esc {
                                    '"' => { s.push('"'); chars.next(); col += 1; }
                                    '\\' => { s.push('\\'); chars.next(); col += 1; }
                                    'n' => { s.push('\n'); chars.next(); col += 1; }
                                    other => {
                                        // Unknown escape — keep verbatim.
                                        s.push('\\');
                                        s.push(other);
                                        chars.next();
                                        col += 1;
                                    }
                                }
                            }
                        }
                        '\n' => {
                            chars.next();
                            line += 1;
                            col = 1;
                            s.push('\n');
                        }
                        other => {
                            s.push(other);
                            chars.next();
                            col += 1;
                        }
                    }
                }
                if !terminated {
                    return Err(LexError::UnterminatedString {
                        line: start_line,
                        col: start_col,
                    });
                }
                tokens.push(Token {
                    kind: TokenKind::String(s),
                    line: start_line,
                    col: start_col,
                });
            }
            d if d.is_ascii_digit() || d == '-' => {
                let start_line = line;
                let start_col = col;
                let mut s = String::new();
                if d == '-' {
                    s.push('-');
                    chars.next();
                    col += 1;
                }
                while let Some(&nc) = chars.peek() {
                    if nc.is_ascii_digit() || nc == '.' {
                        s.push(nc);
                        chars.next();
                        col += 1;
                    } else {
                        break;
                    }
                }
                let n = s.parse::<f64>().map_err(|_| LexError::InvalidNumber {
                    lexeme: s.clone(),
                    line: start_line,
                    col: start_col,
                })?;
                tokens.push(Token {
                    kind: TokenKind::Number(n),
                    line: start_line,
                    col: start_col,
                });
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let start_line = line;
                let start_col = col;
                let mut s = String::new();
                while let Some(&nc) = chars.peek() {
                    if nc.is_ascii_alphanumeric() || nc == '_' {
                        s.push(nc);
                        chars.next();
                        col += 1;
                    } else {
                        break;
                    }
                }
                let kind = keyword_or_ident(&s);
                tokens.push(Token { kind, line: start_line, col: start_col });
            }
            other => {
                return Err(LexError::UnknownCharacter { ch: other, line, col });
            }
        }
    }
    tokens.push(Token { kind: TokenKind::Eof, line, col });
    Ok(tokens)
}

fn keyword_or_ident(s: &str) -> TokenKind {
    match s {
        "prior" => TokenKind::KwPrior,
        "for" => TokenKind::KwFor,
        "contributes" => TokenKind::KwContributes,
        "from" => TokenKind::KwFrom,
        "to" => TokenKind::KwTo,
        "interacts" => TokenKind::KwInteracts,
        "when" => TokenKind::KwWhen,
        "and" => TokenKind::KwAnd,
        "observe" => TokenKind::KwObserve,
        "source" => TokenKind::KwSource,
        "trust" => TokenKind::KwTrust,
        "locator" => TokenKind::KwLocator,
        "consensus" => TokenKind::KwConsensus,
        "authoritative" => TokenKind::KwAuthoritative,
        "empirical" => TokenKind::KwEmpirical,
        "inferred" => TokenKind::KwInferred,
        "unattributed" => TokenKind::KwUnattributed,
        other => TokenKind::Ident(other.to_string()),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(tokens: &[Token]) -> Vec<TokenKind> {
        tokens.iter().map(|t| t.kind.clone()).collect()
    }

    #[test]
    fn lexes_simple_prior_statement() {
        let src = "prior 0.10 for acs";
        let toks = lex(src).unwrap();
        assert_eq!(
            kinds(&toks),
            vec![
                TokenKind::KwPrior,
                TokenKind::Number(0.10),
                TokenKind::KwFor,
                TokenKind::Ident("acs".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexes_compound_term_with_args() {
        let src = "pmh(hypertension)";
        let toks = lex(src).unwrap();
        assert_eq!(
            kinds(&toks),
            vec![
                TokenKind::Ident("pmh".into()),
                TokenKind::LParen,
                TokenKind::Ident("hypertension".into()),
                TokenKind::RParen,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexes_string_with_escaped_quote() {
        // Use a heavier-quoted raw string so we can include both
        // `\"` escapes and a real closing quote in the source we
        // pass to the lexer.
        let src = r##"source "Pope JH \"et al.\"""##;
        let toks = lex(src).unwrap();
        match &toks[1].kind {
            TokenKind::String(s) => assert_eq!(s, r#"Pope JH "et al.""#),
            other => panic!("expected string, got {other:?}"),
        }
    }

    #[test]
    fn skips_line_comments() {
        let src = "% this is a comment\nprior 0.10 for acs";
        let toks = lex(src).unwrap();
        assert!(matches!(toks[0].kind, TokenKind::KwPrior));
        assert_eq!(toks[0].line, 2);
    }

    #[test]
    fn lexes_query_question_mark() {
        let src = "? acs";
        let toks = lex(src).unwrap();
        assert_eq!(
            kinds(&toks),
            vec![
                TokenKind::Question,
                TokenKind::Ident("acs".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn unterminated_string_is_an_error() {
        let src = r#"source "missing quote"#;
        assert!(matches!(lex(src), Err(LexError::UnterminatedString { .. })));
    }

    #[test]
    fn unknown_character_is_an_error() {
        let src = "prior $0.10 for acs";
        let err = lex(src).unwrap_err();
        assert!(matches!(err, LexError::UnknownCharacter { ch: '$', .. }));
    }

    #[test]
    fn lexes_all_trust_tier_keywords() {
        let src = "consensus authoritative empirical inferred unattributed";
        let toks = lex(src).unwrap();
        assert_eq!(
            kinds(&toks)[..5],
            [
                TokenKind::KwConsensus,
                TokenKind::KwAuthoritative,
                TokenKind::KwEmpirical,
                TokenKind::KwInferred,
                TokenKind::KwUnattributed,
            ]
        );
    }

    #[test]
    fn line_and_column_diagnostics_work_across_newlines() {
        let src = "prior 0.10 for acs\ncontributes 1.5";
        let toks = lex(src).unwrap();
        let contrib = toks.iter().find(|t| matches!(t.kind, TokenKind::KwContributes)).unwrap();
        assert_eq!(contrib.line, 2);
        assert_eq!(contrib.col, 1);
    }
}
