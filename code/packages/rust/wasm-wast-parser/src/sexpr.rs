//! # Generic S-expression tree — the shared shape both `module.rs` and
//! `script.rs` walk.
//!
//! The tokenizer only knows about parens/atoms/strings; this module groups
//! a flat token stream into a nested tree, so downstream code walks
//! `(a (b c) d)` as a real tree instead of hand-tracking paren depth
//! everywhere. WAT's own **folded instruction syntax** — `(i32.add
//! (i32.const 1) (local.get 0))` — is exactly this tree shape too, so
//! `module.rs`'s instruction encoder walks the same [`SExpr::List`] nodes
//! module forms do; there is no separate "folded vs. flat" code path.

use crate::tokenizer::{tokenize, SpannedToken, Token};
use crate::WastParseError;

#[derive(Debug, Clone, PartialEq)]
pub enum SExpr {
    Atom(String, usize),
    Str(Vec<u8>, usize),
    List(Vec<SExpr>, usize),
}

impl SExpr {
    pub fn pos(&self) -> usize {
        match self {
            SExpr::Atom(_, p) | SExpr::Str(_, p) | SExpr::List(_, p) => *p,
        }
    }

    pub fn as_atom(&self) -> Option<&str> {
        match self {
            SExpr::Atom(s, _) => Some(s),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[SExpr]> {
        match self {
            SExpr::List(items, _) => Some(items),
            _ => None,
        }
    }

    /// True when this is a `(head ...)` list whose first item is the atom
    /// `head` — the common "is this form a `func`/`import`/`i32.add`
    /// keyword form" check.
    pub fn is_keyword_list(&self, head: &str) -> bool {
        matches!(self, SExpr::List(items, _) if items.first().and_then(|i| i.as_atom()) == Some(head))
    }
}

/// `items.get(idx)`, turned into a [`WastParseError`] instead of a caller
/// having to index-panic or `.unwrap()`. Shared by `module.rs` and
/// `script.rs` — both walk positional S-expression lists (`(export "e"
/// (func $x))`, `(assert_trap (invoke "f") "msg")`) where a required
/// trailing field can simply be *missing* in malformed-but-syntactically-
/// parseable input (`(module (export "e"))`, `(register)`), and that must
/// produce a clean parse error, not a crash — exactly the shape of input
/// the real testsuite's own `assert_malformed` fixtures are designed to
/// throw at a parser.
pub fn expect_get(items: &[SExpr], idx: usize) -> Result<&SExpr, WastParseError> {
    items.get(idx).ok_or(WastParseError::UnexpectedEof)
}

pub fn parse_source(src: &str) -> Result<Vec<SExpr>, WastParseError> {
    let tokens = tokenize(src)?;
    parse_sexprs(&tokens)
}

/// Nesting depth ceiling for `(...)` forms. Chosen well above anything a
/// real, hand-written `.wat`/`.wast` file plausibly needs (a few dozen
/// levels at most, even for a deeply nested `if`/`block` tower), but far
/// below what would exhaust a normal thread's call stack -- an
/// adversarially deep input (`((((((...))))))`, thousands of parens) hits
/// [`WastParseError::TooDeeplyNested`] instead of a hard, uncatchable stack
/// overflow.
pub const MAX_NESTING_DEPTH: usize = 512;

pub fn parse_sexprs(tokens: &[SpannedToken]) -> Result<Vec<SExpr>, WastParseError> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let (expr, next) = parse_one(tokens, i, 0)?;
        out.push(expr);
        i = next;
    }
    Ok(out)
}

fn parse_one(tokens: &[SpannedToken], i: usize, depth: usize) -> Result<(SExpr, usize), WastParseError> {
    let tok = tokens.get(i).ok_or(WastParseError::UnexpectedEof)?;
    match &tok.token {
        Token::LParen => {
            if depth >= MAX_NESTING_DEPTH {
                return Err(WastParseError::TooDeeplyNested { pos: tok.pos });
            }
            let pos = tok.pos;
            let mut items = Vec::new();
            let mut j = i + 1;
            loop {
                match tokens.get(j) {
                    None => return Err(WastParseError::UnexpectedEof),
                    Some(t) if t.token == Token::RParen => {
                        j += 1;
                        break;
                    }
                    _ => {
                        let (child, next) = parse_one(tokens, j, depth + 1)?;
                        items.push(child);
                        j = next;
                    }
                }
            }
            Ok((SExpr::List(items, pos), j))
        }
        Token::RParen => Err(WastParseError::UnexpectedToken {
            pos: tok.pos,
            found: ")".to_string(),
            expected: "an atom, string, or '('",
        }),
        Token::Atom(s) => Ok((SExpr::Atom(s.clone(), tok.pos), i + 1)),
        Token::Str(b) => Ok((SExpr::Str(b.clone(), tok.pos), i + 1)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_lists() {
        let exprs = parse_source("(module (func $f (param i32)))").unwrap();
        assert_eq!(exprs.len(), 1);
        let module = exprs[0].as_list().unwrap();
        assert_eq!(module[0].as_atom(), Some("module"));
        let func = module[1].as_list().unwrap();
        assert_eq!(func[0].as_atom(), Some("func"));
        assert_eq!(func[1].as_atom(), Some("$f"));
    }

    #[test]
    fn folded_instruction_is_just_a_list() {
        // (i32.add (i32.const 1) (local.get 0)) is structurally identical
        // to any other nested list -- no special-casing needed here.
        let exprs = parse_source("(i32.add (i32.const 1) (local.get 0))").unwrap();
        let list = exprs[0].as_list().unwrap();
        assert_eq!(list[0].as_atom(), Some("i32.add"));
        assert_eq!(list.len(), 3);
    }

    #[test]
    fn unclosed_list_is_an_error() {
        assert!(parse_source("(module (func").is_err());
    }

    #[test]
    fn stray_close_paren_is_an_error() {
        assert!(parse_source(")").is_err());
    }

    #[test]
    fn is_keyword_list_matches_head_atom() {
        let exprs = parse_source("(func $f)").unwrap();
        assert!(exprs[0].is_keyword_list("func"));
        assert!(!exprs[0].is_keyword_list("import"));
    }
}
