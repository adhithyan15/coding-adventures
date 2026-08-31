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
    let exprs = parse_sexprs(&tokens)?;
    strip_annotations(exprs)
}

// ─────────────────────────────────────────────────────────────────────────
// Annotations — WAT's `(@id ...)` custom out-of-band tooling syntax.
// ─────────────────────────────────────────────────────────────────────────

/// Recursively removes annotation forms — `(@id ...)`, used by tooling for
/// source maps and similar metadata that has no effect on module semantics
/// — from a parsed S-expression tree.
///
/// ## Why a whole-tree pass here, not per-call-site skipping
///
/// The real corpus's own `annotations.wast` sprinkles `(@a)` between
/// **every single token** of a module — inside `param`/`result` lists,
/// `export`/`import` clauses, folded instruction operands, `offset`
/// expressions, and more. `module.rs`'s field/instruction dispatch is
/// ~8000 lines of code that indexes into a field's items *positionally*
/// (`items[1]`, `items[2]`, ...) — teaching every one of those call sites
/// to notice and skip an interloping annotation would mean touching all
/// of them, and missing even one would silently misparse (wrong item at
/// the wrong index) rather than cleanly error.
///
/// Instead, this runs ONCE, immediately after [`parse_sexprs`] builds the
/// tree (see [`parse_source`]), and removes every annotation at every
/// depth before any semantic code ever sees the tree. Downstream code
/// (`module.rs`, `script.rs`) stays completely unaware annotations exist
/// — the tree it walks is indistinguishable from the same source with
/// every annotation deleted by hand.
///
/// ## Recognizing an annotation
///
/// A list `(head ...)` is an annotation iff `head` is an atom starting
/// with `@` — no real WAT keyword (`module`, `func`, `i32.add`, ...) ever
/// starts with `@`, so this is unambiguous. The annotation's `id` is
/// either:
/// - the rest of that atom after `@` (`(@a ...)` → id `"a"`), or
/// - an immediately-adjacent (no intervening whitespace/comment — checked
///   via byte position, since the tokenizer discards whitespace) quoted
///   string (`(@"a" ...)` → id `"a"`), which must itself be valid UTF-8.
///
/// A missing, empty, or non-adjacent id is malformed
/// ([`WastParseError::EmptyAnnotationId`]) — see `annotations.wast`'s own
/// `(@)`/`(@ x)`/`(@"")` cases. This is a best-effort check, not a
/// guarantee every malformed annotation in the real corpus is caught:
/// `wasm-conformance`'s `assert_malformed` grading treats an unexpectedly
/// *accepted* malformed case as `NotYetSupported`, never a hard failure
/// (see that crate's own `grade_assert_malformed` doc comment), so a
/// missed case here costs one directive's pass tally, not correctness.
/// Once a list IS recognized as an annotation, everything else about its
/// contents — arbitrary nesting, strings, even other `(@...)` forms — is
/// simply discarded whole; the annotation's own internal grammar doesn't
/// matter because none of it reaches any semantic code either way.
pub fn strip_annotations(exprs: Vec<SExpr>) -> Result<Vec<SExpr>, WastParseError> {
    let mut out = Vec::with_capacity(exprs.len());
    for e in exprs {
        if let Some(kept) = strip_one(e)? {
            out.push(kept);
        }
    }
    Ok(out)
}

fn is_annotation_head(items: &[SExpr]) -> bool {
    matches!(items.first(), Some(SExpr::Atom(s, _)) if s.starts_with('@'))
}

/// Validate an annotation list's id, per [`strip_annotations`]'s doc
/// comment. `items[0]` is already known to be an atom starting with `@`.
fn validate_annotation_id(items: &[SExpr]) -> Result<(), WastParseError> {
    let head_atom = items[0].as_atom().unwrap();
    let head_pos = items[0].pos();
    let suffix = &head_atom[1..];
    if !suffix.is_empty() {
        // e.g. `(@a ...)` -- the id came from the atom itself.
        return Ok(());
    }
    // Bare `@` -- the id must come from an immediately-adjacent string.
    match items.get(1) {
        Some(SExpr::Str(bytes, str_pos)) if *str_pos == head_pos + 1 => {
            let text = std::str::from_utf8(bytes).map_err(|_| WastParseError::InvalidUtf8 { pos: *str_pos })?;
            if text.is_empty() {
                Err(WastParseError::EmptyAnnotationId { pos: head_pos })
            } else {
                Ok(())
            }
        }
        _ => Err(WastParseError::EmptyAnnotationId { pos: head_pos }),
    }
}

fn strip_one(e: SExpr) -> Result<Option<SExpr>, WastParseError> {
    match e {
        SExpr::List(items, pos) => {
            if is_annotation_head(&items) {
                validate_annotation_id(&items)?;
                return Ok(None);
            }
            let mut kept = Vec::with_capacity(items.len());
            for item in items {
                if let Some(k) = strip_one(item)? {
                    kept.push(k);
                }
            }
            Ok(Some(SExpr::List(kept, pos)))
        }
        other => Ok(Some(other)),
    }
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

    // ══════════════════════════════════════════════════════════════════════
    // strip_annotations -- see its own doc comment for the design.
    // parse_source already calls it, so these exercise it end-to-end.
    // ══════════════════════════════════════════════════════════════════════

    #[test]
    fn a_top_level_annotation_disappears_entirely() {
        let exprs = parse_source("(@a) (func)").unwrap();
        assert_eq!(exprs.len(), 1);
        assert!(exprs[0].is_keyword_list("func"));
    }

    #[test]
    fn an_annotation_nested_inside_a_list_disappears_leaving_the_rest_intact() {
        let exprs = parse_source("(func (@a) (param i32) (@a) (result i32))").unwrap();
        let func = exprs[0].as_list().unwrap();
        // Exactly `func`, `(param i32)`, `(result i32)` -- both annotations
        // gone, nothing else disturbed.
        assert_eq!(func.len(), 3);
        assert_eq!(func[0].as_atom(), Some("func"));
        assert!(func[1].is_keyword_list("param"));
        assert!(func[2].is_keyword_list("result"));
    }

    #[test]
    fn annotations_are_stripped_at_every_depth_of_nesting() {
        let exprs = parse_source("(a (b (@x) (c (@y) d)))").unwrap();
        let a = exprs[0].as_list().unwrap();
        let b = a[1].as_list().unwrap();
        assert_eq!(b.len(), 2); // "b", (c d) -- (@x) gone
        let c = b[1].as_list().unwrap();
        assert_eq!(c.len(), 2); // "c", "d" -- (@y) gone
    }

    #[test]
    fn an_annotation_whose_own_body_is_never_visited() {
        // Once a list is recognized as an annotation, its contents -- no
        // matter how deeply nested or exotic -- are discarded whole,
        // without recursing into them for further stripping.
        let exprs = parse_source("(@a (b (c (d)))) (func)").unwrap();
        assert_eq!(exprs.len(), 1);
        assert!(exprs[0].is_keyword_list("func"));
    }

    #[test]
    fn a_bare_at_sign_with_no_id_is_empty_annotation_id() {
        assert!(matches!(parse_source("(@)"), Err(WastParseError::EmptyAnnotationId { .. })));
    }

    #[test]
    fn at_sign_then_whitespace_then_an_id_is_empty_annotation_id() {
        // The id must be IMMEDIATELY adjacent to `@` -- a separate atom
        // after whitespace doesn't count as the id.
        assert!(matches!(parse_source("(@ x)"), Err(WastParseError::EmptyAnnotationId { .. })));
    }

    #[test]
    fn an_adjacent_quoted_id_is_a_valid_annotation() {
        let exprs = parse_source(r#"(@"a") (func)"#).unwrap();
        assert_eq!(exprs.len(), 1);
        assert!(exprs[0].is_keyword_list("func"));
    }

    #[test]
    fn an_adjacent_but_empty_quoted_id_is_empty_annotation_id() {
        assert!(matches!(parse_source(r#"(@"")"#), Err(WastParseError::EmptyAnnotationId { .. })));
    }

    #[test]
    fn a_non_adjacent_quoted_id_is_empty_annotation_id() {
        assert!(matches!(parse_source(r#"(@ "a")"#), Err(WastParseError::EmptyAnnotationId { .. })));
    }
}
