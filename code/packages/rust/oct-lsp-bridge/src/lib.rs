//! # `oct-lsp-bridge` — Oct LSP bridge.
//!
//! Provides [`oct_language_spec()`] and the `oct-lsp-server` binary
//! that wires the Oct grammar into
//! [`grammar_lsp_bridge::GrammarLanguageBridge`].

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use grammar_lsp_bridge::{LanguageSpec, LspSemanticTokenType};

// ===========================================================================
// Grammar sources
// ===========================================================================

/// Embedded contents of `code/grammars/oct.tokens`.
const OCT_TOKENS_SOURCE: &str =
    include_str!("../../../../grammars/oct/oct.tokens");

/// Embedded contents of `code/grammars/oct.grammar`.
const OCT_GRAMMAR_SOURCE: &str =
    include_str!("../../../../grammars/oct/oct.grammar");

// ===========================================================================
// Token kind map
// ===========================================================================

/// Token kind map for Oct.
///
/// Oct's grammar attaches custom names to every operator, hence the
/// long list below.  Punctuation (LPAREN/RPAREN/LBRACE/RBRACE/COMMA/
/// SEMICOLON/COLON/ARROW) is intentionally absent — editor themes
/// don't colour it.
static OCT_TOKEN_KIND_MAP: &[(&str, LspSemanticTokenType)] = &[
    ("KEYWORD",      LspSemanticTokenType::Keyword),
    ("INT_LIT",      LspSemanticTokenType::Number),
    ("HEX_LIT",      LspSemanticTokenType::Number),
    ("BIN_LIT",      LspSemanticTokenType::Number),
    ("NAME",         LspSemanticTokenType::Variable),
    ("LINE_COMMENT", LspSemanticTokenType::Comment),
    ("EQ_EQ",        LspSemanticTokenType::Operator),
    ("NEQ",          LspSemanticTokenType::Operator),
    ("LEQ",          LspSemanticTokenType::Operator),
    ("GEQ",          LspSemanticTokenType::Operator),
    ("LT",           LspSemanticTokenType::Operator),
    ("GT",           LspSemanticTokenType::Operator),
    ("LAND",         LspSemanticTokenType::Operator),
    ("LOR",          LspSemanticTokenType::Operator),
    ("EQ",           LspSemanticTokenType::Operator),
    ("PLUS",         LspSemanticTokenType::Operator),
    ("MINUS",        LspSemanticTokenType::Operator),
    ("AMP",          LspSemanticTokenType::Operator),
    ("PIPE",         LspSemanticTokenType::Operator),
    ("CARET",        LspSemanticTokenType::Operator),
    ("TILDE",        LspSemanticTokenType::Operator),
    ("BANG",         LspSemanticTokenType::Operator),
];

// ===========================================================================
// Keyword names — reserved words for completion + hover
// ===========================================================================

/// Oct keyword names — exactly mirrors the `keywords:` section of
/// `oct.tokens`.  Includes both control-flow keywords and the 8008
/// hardware intrinsics (which are reserved at the lex level).
static OCT_KEYWORD_NAMES: &[&str] = &[
    // Control-flow / declaration
    "fn", "let", "static",
    "if", "else", "while", "loop", "break", "return",
    "true", "false",
    // 8008 hardware intrinsics
    "in", "out",
    "adc", "sbb",
    "rlc", "rrc", "ral", "rar",
    "carry", "parity",
];

// ===========================================================================
// Declaration rules
// ===========================================================================

// ===========================================================================
// Format wrapper
// ===========================================================================

/// Adapter turning [`oct_formatter::format`] into the
/// `fn(&str) -> Result<String, String>` shape required by
/// [`LanguageSpec::format_fn`].
fn oct_format_wrapper(source: &str) -> Result<String, String> {
    oct_formatter::format(source).map_err(|e| e.to_string())
}

/// Grammar rule names that represent a top-level declaration.
///
/// `fn_decl` is Oct's function declaration form (`fn name(…) -> ty { … }`).
static OCT_DECLARATION_RULES: &[&str] = &["fn_decl"];

// ===========================================================================
// The static LanguageSpec
// ===========================================================================

/// The static `LanguageSpec` for Oct.
static OCT_LANGUAGE_SPEC: LanguageSpec = LanguageSpec {
    name:              "oct",
    file_extensions:   &["oct"],
    tokens_source:     OCT_TOKENS_SOURCE,
    grammar_source:    OCT_GRAMMAR_SOURCE,
    token_kind_map:    OCT_TOKEN_KIND_MAP,
    declaration_rules: OCT_DECLARATION_RULES,
    keyword_names:     OCT_KEYWORD_NAMES,
    // OCT-FMT01: oct-formatter wires textDocument/formatting.
    format_fn:         Some(oct_format_wrapper),
    symbol_table_fn:   None,
};

/// Return the Oct language spec.
pub fn oct_language_spec() -> &'static LanguageSpec {
    &OCT_LANGUAGE_SPEC
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
        GrammarLanguageBridge::new(oct_language_spec())
    }

    #[test]
    fn spec_name_is_oct() {
        assert_eq!(oct_language_spec().name, "oct");
    }

    #[test]
    fn spec_file_extensions() {
        let exts = oct_language_spec().file_extensions;
        assert!(exts.contains(&"oct"));
    }

    #[test]
    fn tokens_source_loaded() {
        let src = oct_language_spec().tokens_source;
        assert!(src.contains("NAME"));
        assert!(src.contains("keywords:"));
    }

    #[test]
    fn grammar_source_loaded() {
        let src = oct_language_spec().grammar_source;
        assert!(src.contains("fn_decl"));
    }

    #[test]
    fn declaration_rules_includes_fn_decl() {
        let rules = oct_language_spec().declaration_rules;
        assert!(rules.contains(&"fn_decl"));
    }

    #[test]
    fn keyword_names_cover_oct_reserved_words() {
        let kws = oct_language_spec().keyword_names;
        // Control flow
        for must in &["fn", "let", "if", "else", "while", "return"] {
            assert!(kws.contains(must), "missing Oct keyword: {must}");
        }
        // Hardware intrinsics — Oct distinguishes these from
        // control-flow keywords.
        for must in &["in", "out", "carry", "parity"] {
            assert!(kws.contains(must), "missing Oct intrinsic: {must}");
        }
    }

    #[test]
    fn format_fn_is_set() {
        // oct-formatter (OCT-FMT01) is now wired in.
        assert!(oct_language_spec().format_fn.is_some());
    }

    #[test]
    fn format_fn_produces_canonical_output() {
        let fmt = oct_language_spec().format_fn.expect("format_fn set");
        let out = fmt("fn main() { let x: u8 = 5; }").expect("ok");
        // Statement should be indented by 2 spaces inside the body.
        assert!(out.contains("\n  let "), "expected indented body in {out:?}");
    }

    #[test]
    fn bridge_constructs_without_panic() {
        let _ = bridge();
    }

    #[test]
    fn tokenize_simple_function() {
        let b = bridge();
        let tokens = b.tokenize("fn main() { let x: u8 = 5; }").expect("tokenize ok");
        let types: Vec<_> = tokens.iter().map(|t| t.token_type.as_str()).collect();
        assert!(types.contains(&"KEYWORD"));
        assert!(types.contains(&"NAME"));
        assert!(types.contains(&"INT_LIT"));
    }
}
