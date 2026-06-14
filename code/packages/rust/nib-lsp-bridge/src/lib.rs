//! # `nib-lsp-bridge` — Nib LSP bridge.
//!
//! Provides [`nib_language_spec()`] and the `nib-lsp-server` binary
//! that wires the Nib grammar into
//! [`grammar_lsp_bridge::GrammarLanguageBridge`].

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use grammar_lsp_bridge::{LanguageSpec, LspSemanticTokenType};

// ===========================================================================
// Grammar sources
// ===========================================================================

/// Embedded contents of `code/grammars/nib.tokens`.
const NIB_TOKENS_SOURCE: &str =
    include_str!("../../../../grammars/nib.tokens");

/// Embedded contents of `code/grammars/nib.grammar`.
const NIB_GRAMMAR_SOURCE: &str =
    include_str!("../../../../grammars/nib.grammar");

// ===========================================================================
// Token kind map
// ===========================================================================

/// Token kind map for Nib.
///
/// | Nib token  | LSP semantic type |
/// |------------|-------------------|
/// | `KEYWORD`  | `Keyword`         |
/// | `INT_LIT`  | `Number`          |
/// | `HEX_LIT`  | `Number`          |
/// | `NAME`     | `Variable`        |
/// | `PLUS`/`MINUS`/`STAR`/`SLASH`/`EQ`/`EQ_EQ`/`NEQ`/`LEQ`/`GEQ`/`LT`/`GT`/`LAND`/`LOR`/`AMP`/`PIPE`/`CARET`/`TILDE`/`BANG` | `Operator` |
static NIB_TOKEN_KIND_MAP: &[(&str, LspSemanticTokenType)] = &[
    ("KEYWORD",   LspSemanticTokenType::Keyword),
    ("INT_LIT",   LspSemanticTokenType::Number),
    ("HEX_LIT",   LspSemanticTokenType::Number),
    ("NAME",      LspSemanticTokenType::Variable),
    ("EQ_EQ",     LspSemanticTokenType::Operator),
    ("NEQ",       LspSemanticTokenType::Operator),
    ("LEQ",       LspSemanticTokenType::Operator),
    ("GEQ",       LspSemanticTokenType::Operator),
    ("LT",        LspSemanticTokenType::Operator),
    ("GT",        LspSemanticTokenType::Operator),
    ("LAND",      LspSemanticTokenType::Operator),
    ("LOR",       LspSemanticTokenType::Operator),
    ("EQ",        LspSemanticTokenType::Operator),
    ("PLUS",      LspSemanticTokenType::Operator),
    ("MINUS",     LspSemanticTokenType::Operator),
    ("STAR",      LspSemanticTokenType::Operator),
    ("SLASH",     LspSemanticTokenType::Operator),
    ("AMP",       LspSemanticTokenType::Operator),
    ("PIPE",      LspSemanticTokenType::Operator),
    ("CARET",     LspSemanticTokenType::Operator),
    ("TILDE",     LspSemanticTokenType::Operator),
    ("BANG",      LspSemanticTokenType::Operator),
];

// ===========================================================================
// Keyword names — reserved words for completion + hover
// ===========================================================================

/// Nib keyword names — exactly mirrors the `keywords:` section of
/// `nib.tokens`.
static NIB_KEYWORD_NAMES: &[&str] = &[
    "fn", "let", "static", "const", "return",
    "for", "while", "in",
    "if", "else",
    "true", "false",
];

// ===========================================================================
// Declaration rules
// ===========================================================================

// ===========================================================================
// Format wrapper
// ===========================================================================

/// Adapter turning [`nib_formatter::format`] into the
/// `fn(&str) -> Result<String, String>` shape required by
/// [`LanguageSpec::format_fn`].
fn nib_format_wrapper(source: &str) -> Result<String, String> {
    nib_formatter::format(source).map_err(|e| e.to_string())
}

/// Grammar rule names that represent a top-level declaration.
///
/// `fn_decl` is Nib's function declaration form (`fn name(…) -> ty { … }`).
/// Top-level `static` and `const` bindings would also live here once Nib's
/// grammar grows distinct rule names for them.
static NIB_DECLARATION_RULES: &[&str] = &["fn_decl"];

// ===========================================================================
// The static LanguageSpec
// ===========================================================================

/// The static `LanguageSpec` for Nib.
static NIB_LANGUAGE_SPEC: LanguageSpec = LanguageSpec {
    name:              "nib",
    file_extensions:   &["nib"],
    tokens_source:     NIB_TOKENS_SOURCE,
    grammar_source:    NIB_GRAMMAR_SOURCE,
    token_kind_map:    NIB_TOKEN_KIND_MAP,
    declaration_rules: NIB_DECLARATION_RULES,
    keyword_names:     NIB_KEYWORD_NAMES,
    // NIB-FMT01: nib-formatter wires textDocument/formatting.
    format_fn:         Some(nib_format_wrapper),
    symbol_table_fn:   None,
};

/// Return the Nib language spec.
pub fn nib_language_spec() -> &'static LanguageSpec {
    &NIB_LANGUAGE_SPEC
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_ls00::language_bridge::LanguageBridge;
    use grammar_lsp_bridge::GrammarLanguageBridge;

    fn bridge() -> GrammarLanguageBridge {
        GrammarLanguageBridge::new(nib_language_spec())
    }

    #[test]
    fn spec_name_is_nib() {
        assert_eq!(nib_language_spec().name, "nib");
    }

    #[test]
    fn spec_file_extensions() {
        let exts = nib_language_spec().file_extensions;
        assert!(exts.contains(&"nib"));
    }

    #[test]
    fn tokens_source_loaded() {
        let src = nib_language_spec().tokens_source;
        assert!(src.contains("NAME"));
        assert!(src.contains("keywords:"));
    }

    #[test]
    fn grammar_source_loaded() {
        let src = nib_language_spec().grammar_source;
        assert!(src.contains("fn_decl"));
    }

    #[test]
    fn declaration_rules_includes_fn_decl() {
        let rules = nib_language_spec().declaration_rules;
        assert!(rules.contains(&"fn_decl"));
    }

    #[test]
    fn keyword_names_cover_nib_reserved_words() {
        let kws = nib_language_spec().keyword_names;
        for must in &["fn", "let", "return", "if", "else",
                      "while", "true", "false"] {
            assert!(kws.contains(must), "missing Nib keyword: {must}");
        }
    }

    #[test]
    fn format_fn_is_set() {
        // nib-formatter (NIB-FMT01) is now wired in.
        assert!(nib_language_spec().format_fn.is_some());
    }

    #[test]
    fn format_fn_produces_canonical_output() {
        // End-to-end sanity: the wired-in formatter runs.
        let fmt = nib_language_spec().format_fn.expect("format_fn set");
        let out = fmt("fn main() { return 42; }").expect("ok");
        assert!(out.contains("return 42"), "expected return 42; got {out:?}");
    }

    #[test]
    fn bridge_constructs_without_panic() {
        let _ = bridge();
    }

    #[test]
    fn tokenize_simple_function() {
        let b = bridge();
        let tokens = b.tokenize("fn main() -> u8 { return 42; }").expect("tokenize ok");
        let types: Vec<_> = tokens.iter().map(|t| t.token_type.as_str()).collect();
        assert!(types.contains(&"KEYWORD"));
        assert!(types.contains(&"NAME"));
        assert!(types.contains(&"INT_LIT"));
    }
}
