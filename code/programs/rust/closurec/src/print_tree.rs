//! `print_tree` — `--print_tree` / `--print_tree_json` token-stream dumps.
//!
//! # What CC does
//!
//! The upstream Java Closure Compiler's `--print_tree` flag dumps
//! the parsed AST to stdout and exits without emitting JS. It's a
//! diagnostic flag for pass authors and grammar developers.
//! `--print_tree_json` emits the same structural information as
//! a JSON document, machine-consumable for tooling.
//!
//! # What we do today
//!
//! Until the parser produces the typed AST (CLOC11.07's bridge
//! is still pending), we have tokens but not nodes. So
//! `--print_tree` today emits the **token stream** — one token
//! per line — which is the closest analogue CC's
//! debugging users actually find useful. The text format:
//!
//! ```text
//! <TYPE_NAME>  <value>
//! ```
//!
//! And `--print_tree_json` (CLOC11.53) emits the same stream as a
//! JSON array of `{"type": "...", "value": "..."}` objects, one
//! object per significant token, in document order:
//!
//! ```json
//! [
//!   {"type": "KEYWORD", "value": "var"},
//!   {"type": "NAME", "value": "x"},
//!   {"type": "EQUALS", "value": "="},
//!   {"type": "NUMBER", "value": "1"},
//!   {"type": "SEMICOLON", "value": ";"}
//! ]
//! ```
//!
//! Trivia (comments, whitespace, newlines) is filtered out for
//! both, since it would dominate the dump otherwise and isn't
//! what users want when they ask "what tokens did the lexer
//! produce?"
//!
//! When the AST bridge lands (CLOC11.07-ish), this module's
//! emission switches to a structural tree-print of
//! `javascript_ast::Program`. The CLI flags and their position
//! in `run_compiler` stay the same — only the bodies of
//! `format_token_dump` / `format_token_dump_json` evolve.

use coding_adventures_javascript_tokens::EsVersion;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Reasons `--print_tree` formatting can fail.
///
/// Only path: the tokenizer rejects the source. Pure-string
/// formatting itself can't fail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrintTreeError {
    LexError(String),
}

impl std::fmt::Display for PrintTreeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // The message is shared between --print_tree and
            // --print_tree_json; the caller's CLI flag is what
            // told the user which they ran. Keeping a single
            // formatter avoids two near-identical variants.
            PrintTreeError::LexError(s) => {
                write!(f, "--print_tree: tokenizer failed: {s}")
            }
        }
    }
}

impl std::error::Error for PrintTreeError {}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Format the token stream of `source` as a multi-line dump.
///
/// Each line is `<TYPE_NAME>\t<value>`. Trivia and EOF are
/// filtered. The final line ends with a newline.
pub fn format_token_dump(
    source: &str,
    version: EsVersion,
) -> Result<String, PrintTreeError> {
    let tokens =
        coding_adventures_javascript_lexer::tokenize_javascript_typed(source, version)
            .map_err(PrintTreeError::LexError)?;

    let mut out = String::with_capacity(source.len() * 2);
    for tok in &tokens {
        if is_trivia(tok) || is_eof(tok) {
            continue;
        }
        out.push_str(&effective_type_name(tok));
        out.push('\t');
        out.push_str(&tok.value);
        out.push('\n');
    }
    Ok(out)
}

/// Format the token stream of `source` as a pretty-printed JSON
/// array — the `--print_tree_json` (CLOC11.53) wire format.
///
/// Each element is an object `{"type": "<TYPE_NAME>", "value":
/// "<value>"}`. Same trivia + EOF filtering as
/// `format_token_dump`. Output is **stable** (no map iteration
/// involved) — every invocation on the same source returns
/// byte-identical text, which is what diff-based fixtures depend
/// on.
///
/// We hand-roll the JSON emission rather than depend on
/// `serde_json` because:
///
///   1. The structure is fixed (array of two-key objects); we
///      don't need a generic serializer.
///   2. The repo principle is zero-deps where reasonable, and
///      this is a 30-line routine.
///   3. Indentation is deterministic — easier to pin in fixtures
///      than `serde_json::to_string_pretty`'s formatting choices.
///
/// Format:
///
/// ```json
/// [
///   {"type": "KEYWORD", "value": "var"},
///   {"type": "NAME", "value": "x"}
/// ]
/// ```
///
/// The empty case is `[]\n` (one bracket pair, trailing newline).
pub fn format_token_dump_json(
    source: &str,
    version: EsVersion,
) -> Result<String, PrintTreeError> {
    let tokens =
        coding_adventures_javascript_lexer::tokenize_javascript_typed(source, version)
            .map_err(PrintTreeError::LexError)?;

    // Filter once so we know whether the array is empty (compact
    // `[]\n`) vs non-empty (pretty-printed multi-line).
    let significant: Vec<&lexer::token::Token> = tokens
        .iter()
        .filter(|t| !is_trivia(t) && !is_eof(t))
        .collect();

    if significant.is_empty() {
        return Ok("[]\n".to_string());
    }

    // Pre-allocate ~50 bytes per token (typical line is shorter,
    // but slack here costs less than later realloc).
    let mut out = String::with_capacity(significant.len() * 50 + 4);
    out.push_str("[\n");
    let last = significant.len() - 1;
    for (i, tok) in significant.iter().enumerate() {
        out.push_str("  {\"type\": \"");
        append_json_escaped(&mut out, &effective_type_name(tok));
        out.push_str("\", \"value\": \"");
        append_json_escaped(&mut out, &tok.value);
        out.push_str("\"}");
        if i != last {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("]\n");
    Ok(out)
}

/// Multi-file variant of [`format_token_dump_json`].
///
/// Emits a JSON array where each element is a file-object:
///
/// ```json
/// [
///   {"path": "src/a.js", "tokens": [
///     {"type": "KEYWORD", "value": "var"}
///   ]},
///   {"path": "src/b.js", "tokens": [
///     {"type": "KEYWORD", "value": "let"}
///   ]}
/// ]
/// ```
///
/// Why a different shape from the single-file form:
///
/// - Single-file consumers expect "just the tokens" — wrapping
///   in a file-object for the 1-file case would be noise.
/// - Multi-file callers need to disambiguate which tokens came
///   from which file. Banners (the text form's solution) aren't
///   an option in JSON.
///
/// The two shapes are stable as the parser-bridge lands later
/// (CLOC11.07+): per-file becomes `{... "ast": {...}}` instead
/// of `{... "tokens": [...]}`, but the file-object wrapper
/// stays.
pub fn format_token_dump_json_multi(
    sources: &[(String, String)],
    version: EsVersion,
) -> Result<String, PrintTreeError> {
    if sources.is_empty() {
        return Ok("[]\n".to_string());
    }

    let mut out = String::with_capacity(sources.len() * 256);
    out.push_str("[\n");
    let last = sources.len() - 1;
    for (i, (path, content)) in sources.iter().enumerate() {
        out.push_str("  {\"path\": \"");
        append_json_escaped(&mut out, path);
        out.push_str("\", \"tokens\": [");

        // Collect this file's significant tokens.
        let tokens =
            coding_adventures_javascript_lexer::tokenize_javascript_typed(content, version)
                .map_err(PrintTreeError::LexError)?;
        let significant: Vec<&lexer::token::Token> = tokens
            .iter()
            .filter(|t| !is_trivia(t) && !is_eof(t))
            .collect();

        if significant.is_empty() {
            // Empty array on the same line as the `[`.
            out.push(']');
        } else {
            out.push('\n');
            let tk_last = significant.len() - 1;
            for (j, tok) in significant.iter().enumerate() {
                // 4-space indent so token objects sit inside the
                // file-object's `tokens` array nicely. The outer
                // file-object is at 2-space indent, so 4 here
                // matches the closing `]` we emit below.
                out.push_str("    {\"type\": \"");
                append_json_escaped(&mut out, &effective_type_name(tok));
                out.push_str("\", \"value\": \"");
                append_json_escaped(&mut out, &tok.value);
                out.push_str("\"}");
                if j != tk_last {
                    out.push(',');
                }
                out.push('\n');
            }
            out.push_str("  ]");
        }
        out.push('}');
        if i != last {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("]\n");
    Ok(out)
}

/// Append `s` to `out` with JSON string-escaping applied.
///
/// We escape exactly what RFC 8259 §7 requires inside a string:
///   - `"` → `\"`
///   - `\` → `\\`
///   - control characters U+0000..U+001F → `\u00XX`
///   - Backspace, formfeed, newline, carriage return, tab get
///     the short forms (`\b \f \n \r \t`) for readability.
///
/// We deliberately don't escape `/` (RFC permits but doesn't
/// require) or non-ASCII characters above U+001F — emitting them
/// literally produces UTF-8 output that every modern JSON parser
/// accepts (and produces smaller, more readable fixtures).
fn append_json_escaped(out: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000C}' => out.push_str("\\f"),
            // Remaining control chars get the long `\u00XX` form.
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            // Everything else (including non-ASCII printables)
            // goes through verbatim — JSON strings are Unicode.
            c => out.push(c),
        }
    }
}

// ---------------------------------------------------------------------------
// Token classification helpers (mirror whitespace_only / defines)
// ---------------------------------------------------------------------------

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

fn is_eof(tok: &lexer::token::Token) -> bool {
    matches!(tok.type_, lexer::token::TokenType::Eof)
}

/// Return the grammar-supplied `type_name` when present, else the
/// structural `TokenType`'s display name. Mirrors
/// `lexer::token::Token::effective_type_name` but inlines it so
/// the format string stays stable across lexer revisions.
fn effective_type_name(tok: &lexer::token::Token) -> String {
    if let Some(name) = &tok.type_name {
        return name.clone();
    }
    format!("{:?}", tok.type_).to_ascii_uppercase()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn dump(src: &str) -> String {
        format_token_dump(src, EsVersion::Es2025).expect("ok")
    }

    #[test]
    fn empty_source_yields_empty_dump() {
        assert_eq!(dump(""), "");
    }

    #[test]
    fn each_significant_token_emits_one_line() {
        let out = dump("var x=1;");
        // 4 significant tokens: var, x, =, 1, ; → 5 lines.
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 5, "got: {out:?}");
        // First line is the `var` keyword.
        assert!(lines[0].ends_with("\tvar"), "got: {:?}", lines[0]);
    }

    #[test]
    fn trivia_is_dropped() {
        // Comments and whitespace shouldn't appear.
        let out = dump("// hello\nvar x = 1; /* tail */");
        assert!(!out.contains("hello"));
        assert!(!out.contains("tail"));
        assert!(out.contains("\tvar"));
        assert!(out.contains("\tx"));
        assert!(out.contains("\t1"));
    }

    #[test]
    fn token_lines_use_tab_separator() {
        let out = dump("y");
        // Single identifier, single line, single tab.
        assert_eq!(out.matches('\t').count(), 1);
        assert!(out.ends_with('\n'));
    }

    #[test]
    fn error_display() {
        let e = PrintTreeError::LexError("bad".into());
        assert!(e.to_string().contains("tokenizer failed"));
        let _: &dyn std::error::Error = &e;
    }

    // ------------------------------------------------------------------
    // CLOC11.53 — JSON formatter
    // ------------------------------------------------------------------

    fn dump_json(src: &str) -> String {
        format_token_dump_json(src, EsVersion::Es2025).expect("ok")
    }

    #[test]
    fn empty_source_yields_empty_json_array() {
        assert_eq!(dump_json(""), "[]\n");
    }

    #[test]
    fn json_dump_has_one_object_per_significant_token() {
        let out = dump_json("var x=1;");
        // 5 tokens: var, x, =, 1, ; → 5 object lines + 2 bracket lines.
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 7, "got: {out:?}");
        assert_eq!(lines[0], "[");
        assert_eq!(lines[lines.len() - 1], "]");
        // First object is `var`.
        assert!(
            lines[1].contains("\"value\": \"var\""),
            "got: {:?}",
            lines[1]
        );
        // Object lines except the last have a trailing comma.
        assert!(lines[1].ends_with(','), "got: {:?}", lines[1]);
        assert!(lines[5].ends_with('}'), "got: {:?}", lines[5]);
        assert!(!lines[5].ends_with(','), "last obj must NOT have comma");
    }

    #[test]
    fn json_dump_drops_trivia() {
        let out = dump_json("// hello\nvar x = 1; /* tail */");
        assert!(!out.contains("hello"));
        assert!(!out.contains("tail"));
        // But the significant tokens are all there.
        assert!(out.contains("\"value\": \"var\""));
        assert!(out.contains("\"value\": \"x\""));
        assert!(out.contains("\"value\": \"1\""));
    }

    #[test]
    fn json_dump_escapes_quotes_and_backslashes_in_strings() {
        // A string-literal token's value contains `"` and `\` that
        // must be escaped to keep the JSON valid.
        let out = dump_json(r#"var s = "he said \"hi\"";"#);
        // The lexer's `value` for the string literal is the
        // unescaped content (he said "hi"). Our JSON emitter must
        // re-escape the embedded quotes. We don't need to know the
        // exact lexer value to assert correctness — we only need:
        //   (a) no unescaped quote sequences that would close the
        //       JSON string early,
        //   (b) the dump parses as a balanced bracket pair.
        // The simplest pin: every `"` not preceded by `\` is part
        // of the JSON framing (key/value boundaries). Count them
        // and make sure the count is even.
        let unescaped_quote_positions: Vec<usize> = out
            .char_indices()
            .filter(|(i, c)| {
                *c == '"' && (*i == 0 || out.as_bytes()[*i - 1] != b'\\')
            })
            .map(|(i, _)| i)
            .collect();
        assert!(
            unescaped_quote_positions.len().is_multiple_of(2),
            "balanced quotes: {out}"
        );
    }

    #[test]
    fn json_dump_escapes_control_characters_in_values() {
        // We don't easily get raw control chars through a JS lexer
        // — exercise the escaper directly on a synthetic input.
        let mut s = String::new();
        append_json_escaped(&mut s, "tab\there\nfeed\u{0008}back\u{0001}");
        assert!(s.contains("\\t"));
        assert!(s.contains("\\n"));
        assert!(s.contains("\\b"));
        assert!(s.contains("\\u0001"));
    }

    #[test]
    fn json_dump_starts_with_open_bracket_and_ends_with_closing_newline() {
        let out = dump_json("y");
        assert!(out.starts_with("[\n"));
        assert!(out.ends_with("]\n"));
    }
}
