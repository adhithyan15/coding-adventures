//! # `twig-lsp-bridge` — Twig instantiation of the grammar-driven LSP bridge.
//!
//! **LS02 PR B** — Provides [`twig_language_spec()`] and the `twig-lsp-server`
//! binary that wires the Twig grammar into [`grammar_lsp_bridge::GrammarLanguageBridge`].
//!
//! This crate is intentionally tiny.  All LSP logic lives in
//! `grammar-lsp-bridge`.  Here we just supply:
//!
//! - The two grammar files (`twig.tokens` / `twig.grammar`) as `&'static str`
//!   compile-time constants via `include_str!`.
//! - A `token_kind_map` translating Twig's token names to LSP semantic types.
//! - The list of grammar rules that represent declarations (`define`,
//!   `module_form`).
//! - The list of reserved keywords for completion.
//! - A wrapper that exposes [`twig_formatter::format`] under the
//!   `format_fn(&str) -> Result<String, String>` shape required by
//!   `LanguageSpec`.
//!
//! ## Architecture
//!
//! ```text
//! Editor (VS Code / Neovim / …)
//!     │  LSP / JSON-RPC over stdio
//!     ▼
//! twig-lsp-server (bin/twig_lsp_server.rs in this crate)
//!     │  GrammarLanguageBridge::new(twig_language_spec())
//!     ▼
//! grammar-lsp-bridge   ← all 8 LSP features live here
//!     │
//!     ▼
//! lexer + parser       ← runtime tokenisation / parsing
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use grammar_lsp_bridge::GrammarLanguageBridge;
//! use twig_lsp_bridge::twig_language_spec;
//!
//! let bridge  = GrammarLanguageBridge::new(twig_language_spec());
//! let boxed: Box<dyn coding_adventures_ls00::language_bridge::LanguageBridge>
//!     = Box::new(bridge);
//! let stdin   = std::io::stdin();
//! let stdout  = std::io::stdout();
//! let mut srv = coding_adventures_ls00::server::LspServer::new(
//!     boxed,
//!     stdin.lock(),
//!     stdout.lock(),
//! );
//! srv.serve();
//! ```

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use grammar_lsp_bridge::{LanguageSpec, LspSemanticTokenType};

// ---------------------------------------------------------------------------
// Grammar sources
//
// Embedded at compile time from `code/grammars/twig.tokens` and
// `code/grammars/twig.grammar`.  The relative path goes up three levels:
//
//   src/lib.rs (this file)
//   ../        → twig-lsp-bridge/
//   ../../     → packages/rust/
//   ../../../  → packages/
//   ../../../../  → code/
//
// So the include path is "../../../../grammars/twig.tokens".
// ---------------------------------------------------------------------------

/// Embedded contents of `code/grammars/twig.tokens`.
const TWIG_TOKENS_SOURCE: &str =
    include_str!("../../../../grammars/twig.tokens");

/// Embedded contents of `code/grammars/twig.grammar`.
const TWIG_GRAMMAR_SOURCE: &str =
    include_str!("../../../../grammars/twig.grammar");

// ---------------------------------------------------------------------------
// Token kind map — Twig token names → LSP semantic token types
//
// Token names match the uppercase identifiers in `twig.tokens`.
// Tokens not listed here (LPAREN, RPAREN) are simply uncoloured.
// ---------------------------------------------------------------------------

/// Token kind map for Twig — derived from `twig.tokens`.
///
/// | Twig token   | LSP semantic type | Rationale                                 |
/// |--------------|-------------------|-------------------------------------------|
/// | `KEYWORD`    | `Keyword`         | promoted Lisp special forms               |
/// | `NAME`       | `Variable`        | identifiers (functions get reclassified)  |
/// | `INTEGER`    | `Number`          | numeric literals                          |
/// | `BOOL_TRUE`  | `Keyword`         | `#t` reads like a keyword to the eye      |
/// | `BOOL_FALSE` | `Keyword`         | `#f` reads like a keyword to the eye      |
/// | `QUOTE`      | `Operator`        | `'` quote prefix                          |
/// | `COLON`      | `Operator`        | `:` type annotation marker                |
/// | `ARROW`      | `Operator`        | `->` return-type marker                   |
///
/// `LPAREN` / `RPAREN` are intentionally absent — punctuation gets no colour.
static TWIG_TOKEN_KIND_MAP: &[(&str, LspSemanticTokenType)] = &[
    ("KEYWORD",    LspSemanticTokenType::Keyword),
    ("NAME",       LspSemanticTokenType::Variable),
    ("INTEGER",    LspSemanticTokenType::Number),
    ("BOOL_TRUE",  LspSemanticTokenType::Keyword),
    ("BOOL_FALSE", LspSemanticTokenType::Keyword),
    ("QUOTE",      LspSemanticTokenType::Operator),
    ("COLON",      LspSemanticTokenType::Operator),
    ("ARROW",      LspSemanticTokenType::Operator),
];

// ---------------------------------------------------------------------------
// Keyword names — reserved words for completion + hover
//
// This list MUST stay in sync with the `keywords:` section of `twig.tokens`.
// We duplicate it here (rather than re-parsing the tokens file at runtime)
// so the bridge never pays for parsing the spec twice.
// ---------------------------------------------------------------------------

/// Twig keyword names — exactly mirrors the `keywords:` section of
/// `twig.tokens`.
static TWIG_KEYWORD_NAMES: &[&str] = &[
    // Core Lisp special forms
    "define",
    "lambda",
    "let",
    "if",
    "begin",
    "quote",
    "nil",
    // Module-form keywords (TW04 Phase 4a)
    "module",
    "export",
    "import",
];

// ---------------------------------------------------------------------------
// Declaration rules — top-level binding forms
//
// `find_nodes(ast, rule_name)` is run for each entry to populate
// document_symbols and completion suggestions.
// ---------------------------------------------------------------------------

/// Grammar rule names that represent a top-level declaration.
///
/// - `define`      — function or value binding (`(define f …)`)
/// - `module_form` — module declaration (`(module mymod (export …) …)`)
static TWIG_DECLARATION_RULES: &[&str] = &["define", "module_form"];

// ---------------------------------------------------------------------------
// Format wrapper
//
// `LanguageSpec::format_fn` requires `fn(&str) -> Result<String, String>`.
// `twig_formatter::format` returns `Result<String, FormatError>`.
// We adapt by stringifying the error.
// ---------------------------------------------------------------------------

/// Adapter turning [`twig_formatter::format`] into the `fn(&str) -> Result<String, String>`
/// shape required by [`LanguageSpec::format_fn`].
fn twig_format_wrapper(source: &str) -> Result<String, String> {
    twig_formatter::format(source).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// The static LanguageSpec
// ---------------------------------------------------------------------------

/// The static `LanguageSpec` for Twig.
///
/// Constructed once at compile time; lives for the process lifetime.
/// See [`twig_language_spec`] for safe access.
static TWIG_LANGUAGE_SPEC: LanguageSpec = LanguageSpec {
    name:              "twig",
    file_extensions:   &["twig", "tw"],
    tokens_source:     TWIG_TOKENS_SOURCE,
    grammar_source:    TWIG_GRAMMAR_SOURCE,
    token_kind_map:    TWIG_TOKEN_KIND_MAP,
    declaration_rules: TWIG_DECLARATION_RULES,
    keyword_names:     TWIG_KEYWORD_NAMES,
    format_fn:         Some(twig_format_wrapper),
    symbol_table_fn:   None,
};

/// Return the Twig language spec.
///
/// Pass the result to [`grammar_lsp_bridge::GrammarLanguageBridge::new`]
/// to construct a fully-featured Twig LSP bridge.
///
/// ```rust,ignore
/// use grammar_lsp_bridge::GrammarLanguageBridge;
/// use twig_lsp_bridge::twig_language_spec;
///
/// let bridge = GrammarLanguageBridge::new(twig_language_spec());
/// ```
pub fn twig_language_spec() -> &'static LanguageSpec {
    &TWIG_LANGUAGE_SPEC
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_ls00::language_bridge::LanguageBridge;
    use coding_adventures_ls00::types::Position;
    use grammar_lsp_bridge::GrammarLanguageBridge;

    fn bridge() -> GrammarLanguageBridge {
        GrammarLanguageBridge::new(twig_language_spec())
    }

    // -----------------------------------------------------------------------
    // Spec sanity
    // -----------------------------------------------------------------------

    #[test]
    fn spec_name_is_twig() {
        assert_eq!(twig_language_spec().name, "twig");
    }

    #[test]
    fn spec_file_extensions() {
        let exts = twig_language_spec().file_extensions;
        assert!(exts.contains(&"twig"));
        assert!(exts.contains(&"tw"));
    }

    #[test]
    fn tokens_source_loaded() {
        // Sanity: the file is non-empty and contains at least one token name.
        let src = twig_language_spec().tokens_source;
        assert!(src.contains("LPAREN"));
        assert!(src.contains("INTEGER"));
        assert!(src.contains("keywords:"));
    }

    #[test]
    fn grammar_source_loaded() {
        let src = twig_language_spec().grammar_source;
        assert!(src.contains("define"));
        assert!(src.contains("program"));
    }

    #[test]
    fn declaration_rules_includes_define() {
        let rules = twig_language_spec().declaration_rules;
        assert!(rules.contains(&"define"));
    }

    #[test]
    fn keyword_names_match_tokens_file() {
        let kws = twig_language_spec().keyword_names;
        for must in &["define", "lambda", "let", "if", "begin",
                      "quote", "nil", "module", "export", "import"] {
            assert!(kws.contains(must), "missing keyword: {must}");
        }
    }

    #[test]
    fn format_fn_is_set() {
        assert!(twig_language_spec().format_fn.is_some());
    }

    // -----------------------------------------------------------------------
    // Bridge construction
    // -----------------------------------------------------------------------

    #[test]
    fn bridge_constructs_without_panic() {
        let _ = bridge();
    }

    // -----------------------------------------------------------------------
    // tokenize
    // -----------------------------------------------------------------------

    #[test]
    fn tokenize_simple_define() {
        let b = bridge();
        let tokens = b.tokenize("(define x 42)").expect("tokenize ok");
        // Expected: LPAREN, KEYWORD("define"), NAME("x"), INTEGER("42"), RPAREN.
        let types: Vec<_> = tokens.iter().map(|t| t.token_type.as_str()).collect();
        assert!(types.contains(&"LPAREN"));
        assert!(types.contains(&"KEYWORD"));
        assert!(types.contains(&"NAME"));
        assert!(types.contains(&"INTEGER"));
        assert!(types.contains(&"RPAREN"));
    }

    #[test]
    fn tokenize_skips_comments_and_whitespace() {
        let b = bridge();
        let src = "; a comment\n(define x 1) ; trailing\n";
        let tokens = b.tokenize(src).expect("tokenize ok");
        // Comments and whitespace must be invisible to the parser stream.
        for t in &tokens {
            assert_ne!(t.token_type, "COMMENT");
            assert_ne!(t.token_type, "WHITESPACE");
        }
    }

    // -----------------------------------------------------------------------
    // parse
    // -----------------------------------------------------------------------

    #[test]
    fn parse_valid_define() {
        let b = bridge();
        let (_, diags) = b.parse("(define x 42)").expect("parse ok");
        assert!(diags.is_empty(), "no diagnostics: {:?}", diags);
    }

    #[test]
    fn parse_invalid_unbalanced_paren() {
        let b = bridge();
        let (_, diags) = b.parse("(define").expect("no internal err");
        assert!(!diags.is_empty(), "diagnostic for unbalanced paren");
    }

    // -----------------------------------------------------------------------
    // semantic_tokens
    // -----------------------------------------------------------------------

    #[test]
    fn semantic_tokens_classify_keyword_name_number() {
        let b = bridge();
        let toks = b.tokenize("(define foo 7)").expect("tokenize");
        let sem = b.semantic_tokens("", &toks).expect("Some").expect("Ok");
        let types: Vec<&str> = sem.iter().map(|s| s.token_type.as_str()).collect();
        assert!(types.contains(&"keyword"),  "{:?}", types);
        assert!(types.contains(&"variable"), "{:?}", types);
        assert!(types.contains(&"number"),   "{:?}", types);
    }

    // -----------------------------------------------------------------------
    // document_symbols
    // -----------------------------------------------------------------------

    #[test]
    fn document_symbols_finds_top_level_define() {
        let b = bridge();
        let (ast, _) = b.parse("(define foo 1)\n(define bar 2)").expect("parse");
        let syms = b.document_symbols(ast.as_ref()).expect("Some").expect("Ok");
        let names: Vec<&str> = syms.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"foo"), "{:?}", names);
        assert!(names.contains(&"bar"), "{:?}", names);
    }

    // -----------------------------------------------------------------------
    // hover
    // -----------------------------------------------------------------------

    #[test]
    fn hover_does_not_crash() {
        let b = bridge();
        let (ast, _) = b.parse("(define foo 1)").expect("parse");
        let pos = Position { line: 0, character: 8 }; // somewhere in "foo"
        // Returning None or Some(Ok(_)) are both valid; we only assert no error.
        match b.hover(ast.as_ref(), pos) {
            Some(Ok(_)) | None => {}
            Some(Err(e)) => panic!("hover error: {e}"),
        }
    }

    // -----------------------------------------------------------------------
    // completion
    // -----------------------------------------------------------------------

    #[test]
    fn completion_includes_keywords() {
        let b = bridge();
        let (ast, _) = b.parse("(define x 1)").expect("parse");
        let pos = Position { line: 0, character: 0 };
        let items = b.completion(ast.as_ref(), pos).expect("Some").expect("Ok");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"define"));
        assert!(labels.contains(&"lambda"));
        assert!(labels.contains(&"if"));
    }

    #[test]
    fn completion_includes_user_define() {
        let b = bridge();
        let (ast, _) = b.parse("(define myfunc 1)").expect("parse");
        let pos = Position { line: 0, character: 0 };
        let items = b.completion(ast.as_ref(), pos).expect("Some").expect("Ok");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"myfunc"), "user-defined name in {:?}", labels);
    }

    // -----------------------------------------------------------------------
    // format
    // -----------------------------------------------------------------------

    #[test]
    fn format_supported() {
        let b = bridge();
        assert!(b.supports_format(), "twig spec has format_fn → must be supported");
    }

    #[test]
    fn format_round_trips_valid_source() {
        let b = bridge();
        let edits = b.format("(define x 42)").expect("Some").expect("Ok");
        assert_eq!(edits.len(), 1, "single whole-file edit");
        // The formatter should produce valid (re-parseable) output.
        let formatted = &edits[0].new_text;
        let (_, diags) = b.parse(formatted).expect("re-parse ok");
        assert!(diags.is_empty(), "formatted output reparses cleanly");
    }

    // -----------------------------------------------------------------------
    // Capability flags
    // -----------------------------------------------------------------------

    #[test]
    fn all_optional_features_supported() {
        let b = bridge();
        assert!(b.supports_semantic_tokens());
        assert!(b.supports_document_symbols());
        assert!(b.supports_folding_ranges());
        assert!(b.supports_hover());
        assert!(b.supports_completion());
        assert!(b.supports_format());
    }
}
