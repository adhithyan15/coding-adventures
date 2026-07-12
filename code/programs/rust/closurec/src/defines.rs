//! `defines` — `--define NAME=value` (and short `-D`) substitution.
//!
//! # What CC does
//!
//! The upstream Java Closure Compiler treats `--define NAME=value`
//! as a *constant override*. When CC sees a `goog.define` call or
//! a JSDoc `@define`-annotated variable named `NAME`, it replaces
//! the right-hand side of the assignment with the supplied value.
//! Subsequent passes (constant-fold, DCE, etc.) then act on that
//! value, so `if (DEBUG) { … }` collapses to dead code when
//! `--define DEBUG=false` is passed.
//!
//! # What we do today (CLOC11.19)
//!
//! We don't yet parse JSDoc `@define` annotations, and we don't
//! yet have an AST to surgically modify the RHS of an assignment.
//! What we *do* have is the lexer from CLOC11.06 and the typed
//! [`DefineValue`](crate::config::DefineValue) from CLOC11.01.
//!
//! The simplest token-level slice that mimics CC's behavior:
//!
//! - Walk every token.
//! - If the token is an identifier (`NAME`) whose value matches
//!   a `--define` key exactly, replace it with a synthetic
//!   token carrying the formatted value (number/string/bool/null).
//! - Pass everything else through untouched.
//!
//! This is **looser than CC**: CC only substitutes references to
//! variables tagged `@define`. We substitute *every* reference to
//! the name. In practice this matches what users expect when they
//! pass `--define FLAG_DEBUG=false` for a flag they own — they
//! want that name replaced anywhere it appears. The few edge
//! cases where CC would *not* substitute (e.g. an undeclared
//! shadowing variable) are not common in real builds, and the
//! CLOC11.21 follow-up that adds `@define`-aware substitution
//! will tighten the rule.
//!
//! # Composition with WHITESPACE_ONLY
//!
//! `transform_source` calls `apply_defines` *after* the
//! compilation-level dispatch. So:
//!
//! - `--compilation_level WHITESPACE_ONLY --define DEBUG=false`
//!   first whitespace-minifies, then substitutes `DEBUG` →
//!   `false` in the resulting compact source.
//! - `--compilation_level SIMPLE --define DEBUG=false`
//!   (still identity for the SIMPLE arm in CLOC11.19) substitutes
//!   directly.
//!
//! Each call re-tokenizes — that's `O(n)` per pass, fine for our
//! input sizes. CLOC11.07+ may consolidate the tokenize step
//! when both WHITESPACE_ONLY-ish trimming and define substitution
//! need to run; for now keeping them separate keeps each module
//! testable in isolation.

use crate::config::DefineValue;
use coding_adventures_javascript_tokens::EsVersion;
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Reasons define substitution can fail. Like
/// [`whitespace_only::MinifyError`](crate::whitespace_only::MinifyError),
/// the only failure path today is the tokenizer rejecting the
/// source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefineError {
    /// The tokenizer rejected the source.
    LexError(String),
}

impl std::fmt::Display for DefineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DefineError::LexError(s) => {
                write!(f, "define substitution: tokenizer failed: {s}")
            }
        }
    }
}

impl std::error::Error for DefineError {}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Apply `--define NAME=value` substitutions to `source`.
///
/// Returns the source with every identifier-token whose value
/// matches a key in `defines` replaced by the formatted form of
/// the corresponding [`DefineValue`].
///
/// If `defines` is empty, returns `source` unchanged (and skips
/// tokenization entirely — cheap fast path for the common case).
pub fn apply_defines(
    source: &str,
    defines: &BTreeMap<String, DefineValue>,
    version: EsVersion,
) -> Result<String, DefineError> {
    // Fast path: no defines means no work.
    if defines.is_empty() {
        return Ok(source.to_string());
    }

    let tokens =
        coding_adventures_javascript_lexer::tokenize_javascript_typed(source, version)
            .map_err(DefineError::LexError)?;

    // Walk tokens. For each one, decide whether to substitute.
    // Output is rebuilt incrementally — we can't just rewrite
    // bytes in `source` because the lexer's `value` is unescaped
    // for some token kinds (strings) and the source-byte ranges
    // aren't stable through escape handling. Reconstructing from
    // tokens lets us be precise.
    //
    // We use the same "word-like" gap rule as `whitespace_only`
    // so the output stays well-tokenized.
    let mut out = String::with_capacity(source.len());
    let mut last_emitted_was_word_like = false;
    for tok in &tokens {
        if is_eof(tok) {
            continue;
        }

        if is_trivia(tok) {
            // Pass trivia through verbatim — define substitution
            // is supposed to be invisible to comments/whitespace.
            out.push_str(&tok.value);
            // Trivia doesn't count for adjacency; whatever was
            // last emitted before the trivia still governs.
            continue;
        }

        if is_identifier(tok) {
            if let Some(value) = defines.get(&tok.value) {
                // Substitution! Emit the formatted value, and
                // book-keep the adjacency. Most replacements are
                // word-like (numbers, bools, null) so we set the
                // flag accordingly. Strings are not word-like
                // (quoted) so they don't need the separator.
                let (rendered, replacement_is_word_like) = render_define_value(value);
                if last_emitted_was_word_like && replacement_is_word_like {
                    out.push(' ');
                }
                out.push_str(&rendered);
                last_emitted_was_word_like = replacement_is_word_like;
                continue;
            }
        }

        // Non-substituted token: emit as-is, with a separator
        // when needed.
        let word_like = is_word_like(tok);
        if last_emitted_was_word_like && word_like {
            out.push(' ');
        }
        if is_string_literal(tok) {
            // gap-090: tok.value is the raw string interior (escapes not
            // processed) because es2025.tokens uses `escapes: none`. Use
            // the same normalising emitter as whitespace_only.rs so the
            // defines pass does not double-escape backslashes.
            crate::whitespace_only::emit_quoted_string(&mut out, &tok.value);
            last_emitted_was_word_like = false;
        } else {
            out.push_str(&tok.value);
            last_emitted_was_word_like = word_like;
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Value rendering
// ---------------------------------------------------------------------------

/// Render a [`DefineValue`] into JavaScript source plus a flag
/// indicating whether the result is a word-like token (for
/// adjacency-separator bookkeeping).
fn render_define_value(value: &DefineValue) -> (String, bool) {
    match value {
        DefineValue::Bool(true) => ("true".to_string(), true),
        DefineValue::Bool(false) => ("false".to_string(), true),
        DefineValue::Null => ("null".to_string(), true),
        DefineValue::Number(n) => {
            // f64::to_string handles integer-valued doubles
            // without a trailing `.0`, which matches CC's
            // output. Special-case NaN/Inf to JS sentinels.
            if n.is_nan() {
                ("NaN".to_string(), true)
            } else if n.is_infinite() {
                if *n > 0.0 {
                    ("Infinity".to_string(), true)
                } else {
                    ("-Infinity".to_string(), true)
                }
            } else {
                (format_number(*n), true)
            }
        }
        DefineValue::String(s) => {
            // `s` is an already-decoded Rust string (parsed from the
            // --define CLI flag), not a raw JS escape sequence. Re-encode
            // each char in Closure canonical form (double-quote delimited;
            // no quote-choice needed for synthesised values).
            let mut out = String::with_capacity(s.len() + 2);
            out.push('"');
            for c in s.chars() {
                crate::whitespace_only::encode_js_char(&mut out, c, '"');
            }
            out.push('"');
            // Quoted string is NOT word-like.
            (out, false)
        }
    }
}

/// Format an `f64` for embedding in JS source. Integer-valued
/// doubles render without `.0`, so `Number(42.0)` → `"42"`,
/// matching CC.
fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n.is_finite() && n.abs() < 1e16 {
        // Integer-valued in safe-integer range — emit as integer.
        format!("{}", n as i64)
    } else {
        format!("{}", n)
    }
}

// ---------------------------------------------------------------------------
// Token classification helpers
// ---------------------------------------------------------------------------
//
// These mirror the classifiers in `whitespace_only.rs`. We don't
// share them because each module's needs are subtly different —
// `whitespace_only` cares about "what to filter out"; `defines`
// cares about "what to substitute" and "what's word-like for
// adjacency tracking". Sharing would require passing around an
// option set. v1: just duplicate.

fn is_eof(tok: &lexer::token::Token) -> bool {
    matches!(tok.type_, lexer::token::TokenType::Eof)
}

fn is_trivia(tok: &lexer::token::Token) -> bool {
    if let Some(name) = &tok.type_name {
        let upper = name.to_ascii_uppercase();
        return matches!(
            upper.as_str(),
            "COMMENT"
                | "LINE_COMMENT"
                | "BLOCK_COMMENT"
                | "WHITESPACE"
                | "WS"
                | "NEWLINE"
                | "LINE_TERMINATOR"
                | "SKIP"
        );
    }
    matches!(
        tok.type_,
        lexer::token::TokenType::Newline
            | lexer::token::TokenType::Indent
            | lexer::token::TokenType::Dedent
    )
}

/// True iff this token is a plain identifier (eligible for
/// `--define` substitution). Keywords are explicitly NOT
/// eligible — CC won't substitute `--define if=foo`.
fn is_identifier(tok: &lexer::token::Token) -> bool {
    if let Some(name) = &tok.type_name {
        let upper = name.to_ascii_uppercase();
        return matches!(upper.as_str(), "NAME" | "IDENT" | "IDENTIFIER");
    }
    matches!(tok.type_, lexer::token::TokenType::Name)
}

fn is_word_like(tok: &lexer::token::Token) -> bool {
    if let Some(name) = &tok.type_name {
        let upper = name.to_ascii_uppercase();
        return matches!(
            upper.as_str(),
            "NAME"
                | "NUMBER"
                | "KEYWORD"
                | "REGEX"
                | "BIGINT"
                | "PRIVATE_NAME"
                | "TEMPLATE"
                | "TEMPLATE_NO_SUB"
                | "TEMPLATE_HEAD"
                | "TEMPLATE_MIDDLE"
                | "TEMPLATE_TAIL"
                | "IDENT"
                | "IDENTIFIER"
        );
    }
    matches!(
        tok.type_,
        lexer::token::TokenType::Name
            | lexer::token::TokenType::Number
            | lexer::token::TokenType::Keyword
    )
}

fn is_string_literal(tok: &lexer::token::Token) -> bool {
    if let Some(name) = &tok.type_name {
        let upper = name.to_ascii_uppercase();
        if upper == "STRING" || upper == "STRING_LITERAL" {
            return true;
        }
    }
    matches!(tok.type_, lexer::token::TokenType::String)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn defs(pairs: &[(&str, DefineValue)]) -> BTreeMap<String, DefineValue> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    fn run(source: &str, pairs: &[(&str, DefineValue)]) -> String {
        apply_defines(source, &defs(pairs), EsVersion::Es2025).expect("ok")
    }

    #[test]
    fn empty_defines_passes_through() {
        let src = "var x = DEBUG;";
        // No defines → exactly the source back.
        let out = apply_defines(src, &BTreeMap::new(), EsVersion::Es2025).unwrap();
        assert_eq!(out, src);
    }

    #[test]
    fn substitutes_bool_false() {
        let out = run("var x = DEBUG;", &[("DEBUG", DefineValue::Bool(false))]);
        assert!(out.contains("false"));
        assert!(!out.contains("DEBUG"), "got: {out:?}");
    }

    #[test]
    fn substitutes_bool_true() {
        let out = run("if (FLAG) { a; }", &[("FLAG", DefineValue::Bool(true))]);
        assert!(out.contains("true"));
        assert!(!out.contains("FLAG"));
    }

    #[test]
    fn substitutes_number_integer() {
        // Integer-valued doubles render without trailing `.0`.
        let out = run("var n = VERSION;", &[("VERSION", DefineValue::Number(42.0))]);
        assert!(out.contains("42"), "got: {out:?}");
        assert!(!out.contains("42.0"), "should not have trailing .0: {out:?}");
    }

    // `3.14` here is deliberate test *data* (a fractional define value that happens
    // to resemble PI); it is not an attempt to approximate std::f64::consts::PI.
    #[allow(clippy::approx_constant)]
    #[test]
    fn substitutes_number_fractional() {
        let out = run("var n = PI;", &[("PI", DefineValue::Number(3.14))]);
        assert!(out.contains("3.14"), "got: {out:?}");
    }

    #[test]
    fn substitutes_string() {
        let out = run(
            "var s = NAME;",
            &[("NAME", DefineValue::String("hello".to_string()))],
        );
        assert!(out.contains("\"hello\""), "got: {out:?}");
        assert!(!out.contains("NAME"));
    }

    #[test]
    fn substitutes_null() {
        let out = run("var x = EMPTY;", &[("EMPTY", DefineValue::Null)]);
        assert!(out.contains("null"));
    }

    #[test]
    fn does_not_substitute_unrelated_identifiers() {
        // Only the exact name match should fire.
        let out = run("var debug = DEBUG;", &[("DEBUG", DefineValue::Bool(false))]);
        assert!(out.contains("debug"), "lowercase debug must remain: {out:?}");
        assert!(out.contains("false"));
    }

    #[test]
    fn does_not_substitute_inside_string_literal() {
        // `"DEBUG"` is a string literal, not an identifier
        // token, so it must survive untouched.
        let out = run(
            "var s = \"DEBUG\";",
            &[("DEBUG", DefineValue::Bool(false))],
        );
        assert!(out.contains("\"DEBUG\""), "string content corrupted: {out:?}");
        assert!(!out.contains("false"), "must not substitute inside string: {out:?}");
    }

    #[test]
    fn keeps_space_between_keyword_and_substituted_value() {
        // `return DEBUG;` with DEBUG → false must produce
        // `return false;` (NOT `returnfalse;`).
        let out = run("return DEBUG;", &[("DEBUG", DefineValue::Bool(false))]);
        assert!(out.contains("return false"), "got: {out:?}");
        // And not the concatenated form.
        assert!(!out.contains("returnfalse"), "got: {out:?}");
    }

    #[test]
    fn no_space_around_punctuation_after_substitution() {
        // `x=DEBUG;` → `x=false;` (no extra space).
        let out = run("x=DEBUG;", &[("DEBUG", DefineValue::Bool(false))]);
        assert_eq!(out, "x=false;");
    }

    #[test]
    fn multiple_defines_all_apply() {
        let out = run(
            "var a = X, b = Y;",
            &[
                ("X", DefineValue::Number(1.0)),
                ("Y", DefineValue::String("hi".to_string())),
            ],
        );
        assert!(out.contains("1"));
        assert!(out.contains("\"hi\""));
    }

    #[test]
    fn keyword_not_eligible_for_substitution() {
        // CC won't substitute `--define if=...` because `if` is
        // a keyword. Our `is_identifier` rules keywords out.
        let out = run(
            "if (x) {}",
            &[("if", DefineValue::Bool(true))],
        );
        assert!(out.contains("if"), "keyword must survive: {out:?}");
    }

    #[test]
    fn nan_renders_as_nan_sentinel() {
        let out = run("var n = N;", &[("N", DefineValue::Number(f64::NAN))]);
        assert!(out.contains("NaN"), "got: {out:?}");
    }

    #[test]
    fn infinity_renders_as_infinity_sentinel() {
        let out_pos = run("var n = P;", &[("P", DefineValue::Number(f64::INFINITY))]);
        assert!(out_pos.contains("Infinity"));
        let out_neg = run(
            "var n = M;",
            &[("M", DefineValue::Number(f64::NEG_INFINITY))],
        );
        assert!(out_neg.contains("-Infinity"));
    }

    #[test]
    fn string_with_quotes_re_escapes_correctly() {
        let out = run(
            "var s = MSG;",
            &[(
                "MSG",
                DefineValue::String("she said \"hi\"".to_string()),
            )],
        );
        assert!(
            out.contains("\"she said \\\"hi\\\"\""),
            "got: {out:?}",
        );
    }

    #[test]
    fn error_display() {
        let e = DefineError::LexError("bad".into());
        assert!(e.to_string().contains("tokenizer failed"));
        let _: &dyn std::error::Error = &e;
    }
}
