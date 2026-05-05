//! # `twig-lsp-bridge` — Twig instantiation of the grammar-driven LSP bridge.
//!
//! **LS02 PR B** — Provides [`twig_language_spec()`] and the `twig-lsp-server`
//! binary that wires the Twig grammar into [`grammar_lsp_bridge::GrammarLanguageBridge`].
//!
//! ## What this crate provides
//!
//! - A `LanguageSpec` for Twig (token kind map, declaration rules, keyword list).
//! - [`twig_language_spec()`] — accessor for the static spec.
//! - The `twig-lsp-server` binary (`bin/twig_lsp_server.rs`).
//!
//! ## Status — SKELETON (LS02 PR B, depends on LS02 PR A)
//!
//! Implement LS02 PR A first (`grammar-lsp-bridge`), then fill in this file:
//!
//! 1. Add `twig-formatter` to Cargo.toml deps.
//! 2. Set `format_fn: Some(twig_format_wrapper)` where:
//!    ```rust,ignore
//!    fn twig_format_wrapper(source: &str) -> Result<String, String> {
//!        twig_formatter::format(source).map_err(|e| e.to_string())
//!    }
//!    ```
//! 3. Fill in the token_kind_map from the table in LS02 spec §"Token kind map for Twig".
//! 4. Verify keyword_names matches `twig.tokens` `keywords:` section.
//! 5. Update twig.tokens / twig.grammar paths (currently using include_str! placeholders).
//!
//! ## Grammar file locations
//!
//! twig.tokens: code/grammars/twig.tokens
//! twig.grammar: code/grammars/twig.grammar
//!
//! Use `include_str!("../../../grammars/twig.tokens")` from this crate's src/.
//! Verify the relative path before PR B.

use grammar_lsp_bridge::{LanguageSpec, LspSemanticTokenType};

// TODO (LS02 PR B): replace these with include_str! pointing at the real grammar files.
// Paths relative to this src/ directory:
//   twig.tokens  → "../../../grammars/twig.tokens"  (3 levels up from src/ to code/)
//   twig.grammar → "../../../grammars/twig.grammar"
const TWIG_TOKENS_SOURCE: &str = "# TODO: replace with include_str!(\"../../../grammars/twig.tokens\")";
const TWIG_GRAMMAR_SOURCE: &str = "# TODO: replace with include_str!(\"../../../grammars/twig.grammar\")";

/// Token kind map for Twig — derived from twig.tokens.
///
/// See LS02 spec §"Token kind map for Twig" for the full table.
///
/// TODO (LS02 PR B): fill in from twig.tokens token names.
static TWIG_TOKEN_KIND_MAP: &[(&str, LspSemanticTokenType)] = &[
    ("KEYWORD",     LspSemanticTokenType::Keyword),
    ("NAME",        LspSemanticTokenType::Variable),
    ("INTEGER",     LspSemanticTokenType::Number),
    ("BOOL_TRUE",   LspSemanticTokenType::Keyword),
    ("BOOL_FALSE",  LspSemanticTokenType::Keyword),
    ("QUOTE",       LspSemanticTokenType::Operator),
    ("COLON",       LspSemanticTokenType::Operator),
    ("ARROW",       LspSemanticTokenType::Operator),
    // LPAREN, RPAREN intentionally omitted (no semantic colour for parens)
];

/// Twig keyword names — from the `keywords:` section of twig.tokens.
///
/// TODO (LS02 PR B): verify this matches twig.tokens exactly.
static TWIG_KEYWORD_NAMES: &[&str] = &[
    "define", "lambda", "let", "if", "begin", "quote",
    "nil", "module", "export", "import",
];

/// The static LanguageSpec for Twig.
///
/// Constructed once; lives for the process lifetime.
/// See [`twig_language_spec()`] for access.
static TWIG_LANGUAGE_SPEC: LanguageSpec = LanguageSpec {
    name: "twig",
    file_extensions: &["twig", "tw"],
    tokens_source: TWIG_TOKENS_SOURCE,
    grammar_source: TWIG_GRAMMAR_SOURCE,
    token_kind_map: TWIG_TOKEN_KIND_MAP,
    declaration_rules: &["define"],
    keyword_names: TWIG_KEYWORD_NAMES,
    format_fn: None, // TODO (LS02 PR B): set to Some(twig_format_wrapper) after adding twig-formatter dep
    symbol_table_fn: None,
};

/// Return the Twig language spec.
///
/// Pass the result to `GrammarLanguageBridge::new()` to build the bridge.
pub fn twig_language_spec() -> &'static LanguageSpec {
    &TWIG_LANGUAGE_SPEC
}
