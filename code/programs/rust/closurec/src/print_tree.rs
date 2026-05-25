//! `print_tree` — `--print_tree` token-stream dump.
//!
//! # What CC does
//!
//! The upstream Java Closure Compiler's `--print_tree` flag dumps
//! the parsed AST to stdout and exits without emitting JS. It's a
//! diagnostic flag for pass authors and grammar developers.
//!
//! # What we do today
//!
//! Until the parser produces the typed AST (CLOC11.07's bridge
//! is still pending), we have tokens but not nodes. So
//! `--print_tree` today emits the **token stream** — one token
//! per line — which is the closest analogue CC's
//! debugging users actually find useful. The wire format:
//!
//! ```text
//! <TYPE_NAME>  <value>
//! ```
//!
//! Trivia (comments, whitespace, newlines) is filtered out, since
//! it would dominate the dump otherwise and isn't what users
//! want to see when they ask "what tokens did the lexer
//! produce?"
//!
//! When the AST bridge lands (CLOC11.07-ish), this module's
//! emission switches to a structural tree-print of
//! `javascript_ast::Program`. The CLI flag and its position in
//! `run_compiler` stay the same — only the body of
//! `format_token_dump` evolves.

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
}
