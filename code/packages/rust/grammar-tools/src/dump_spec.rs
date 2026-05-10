//! # `dump_spec` — emit a single-source-of-truth language spec as JSON.
//!
//! ## Why this module exists
//!
//! Every language built on the LANG-VM pipeline declares its lexical and
//! syntactic structure in a `.tokens` and `.grammar` file pair under
//! `code/grammars/`.  Several downstream tools want to consume parts of
//! that declaration:
//!
//! - The Rust lexer generator (`compile_token_grammar`).
//! - The Rust parser generator (`compile_parser_grammar`).
//! - The LSP server framework (`grammar-lsp-bridge::LanguageSpec`).
//! - The VS Code extension generator (`vscode-lang-extension-generator`).
//! - Any future editor integration (Neovim, Emacs, Zed, …).
//!
//! Without a shared serialisation format these tools either re-parse the
//! files independently — duplicating work and risking drift — or hard-code
//! values like keyword lists into per-language Rust crates and manually
//! synchronise them.  The `twig-lsp-bridge` source comment says it
//! plainly:
//!
//! > "This list MUST stay in sync with the `keywords:` section of
//! > `twig.tokens`."
//!
//! That's a smell.  `dump_spec` is the antidote: emit the language's
//! structural facts as a stable JSON document, and have every downstream
//! tool consume that document instead of redeclaring.
//!
//! ## What this module produces
//!
//! Given a parsed `.tokens` file, an optional parsed `.grammar` file, and
//! a small bag of caller-supplied metadata (language id, display name,
//! file extensions, etc.), [`dump_language_spec`] returns a JSON document
//! describing the language.  The schema lives in this file's doc-comment
//! and is checked by tests; bumping `$schemaVersion` is required when
//! breaking changes ship.
//!
//! ## Schema (v1)
//!
//! ```json
//! {
//!   "$schemaVersion": 1,
//!   "languageId":    "twig",
//!   "languageName":  "Twig",
//!   "fileExtensions": ["twig", "tw"],
//!   "keywords":         ["define", "if", ...],
//!   "reservedKeywords": [],
//!   "contextKeywords":  [],
//!   "lineComment":      ";",
//!   "blockComment":     null,                // or ["/*", "*/"]
//!   "brackets":         [["(", ")"]],
//!   "rules":            ["program", "expression", ...],
//!   "declarationRules": ["define", "module_form"],
//!   "caseSensitive":    true
//! }
//! ```
//!
//! Fields without a natural source (file extensions, declaration rule
//! names, line/block comment markers) are passed in via `SpecMetadata`.
//! Everything else is derived directly from the parsed token/grammar
//! structures, so re-running the dumper on an evolved `.tokens` file
//! automatically picks up new keywords and bracket pairs.

use crate::parser_grammar::ParserGrammar;
use crate::token_grammar::TokenGrammar;
use serde_json::{json, Value};

/// Caller-supplied metadata that isn't intrinsic to the `.tokens` /
/// `.grammar` files themselves.  These exist so the dumper can produce
/// a complete `LanguageSpec` JSON document without per-language Rust
/// glue code.
#[derive(Clone, Debug, Default)]
pub struct SpecMetadata {
    /// Slug used internally (`"twig"`).  Required.
    pub language_id: String,

    /// Human-readable display name (`"Twig"`).  Required.
    pub language_name: String,

    /// File extensions *without* a leading dot (`["twig", "tw"]`).
    /// Required.
    pub file_extensions: Vec<String>,

    /// Single-line comment prefix, e.g. `";"` or `"//"`.  Empty means
    /// "no line-comment markers in the emitted spec".
    pub line_comment: String,

    /// Block-comment delimiters, e.g. `("/*", "*/")`.  Both empty means
    /// "no block-comment in the emitted spec".  Both populated means
    /// emit them; supplying only one is invalid and treated as none.
    pub block_comment_start: String,

    /// See [`SpecMetadata::block_comment_start`].
    pub block_comment_end: String,

    /// Names of grammar rules that represent top-level declarations the
    /// LSP "document symbols" feature should surface (e.g. `["define",
    /// "module_form"]` for Twig).  Empty is fine.
    pub declaration_rules: Vec<String>,
}

/// Bracket pair token-name conventions used by `infer_brackets`.  We
/// recognise the standard pairs; languages that use unusual brackets
/// can add their own pairs by listing additional tokens with these
/// names.
const BRACKET_PAIRS: &[(&str, &str, &str, &str)] = &[
    ("LPAREN", "RPAREN", "(", ")"),
    ("LBRACE", "RBRACE", "{", "}"),
    ("LBRACKET", "RBRACKET", "[", "]"),
    ("LANGLE", "RANGLE", "<", ">"),
];

/// Return the bracket pairs declared in the token grammar.
///
/// We look for the conventional names `LPAREN`/`RPAREN`,
/// `LBRACE`/`RBRACE`, `LBRACKET`/`RBRACKET`, `LANGLE`/`RANGLE`.  For
/// each pair where *both* tokens are defined, we emit the corresponding
/// literal pair.  The order in `BRACKET_PAIRS` is preserved.
///
/// This is deliberately convention-driven rather than parsing the token
/// patterns — using regex on the `pattern` field would conflate tokens
/// that happen to start with `(` (like comment delimiters) with actual
/// brackets.  The naming convention is the cleaner signal.
pub fn infer_brackets(tg: &TokenGrammar) -> Vec<(String, String)> {
    let names: std::collections::HashSet<&str> =
        tg.definitions.iter().map(|d| d.name.as_str()).collect();
    BRACKET_PAIRS
        .iter()
        .filter_map(|(open_name, close_name, open_lit, close_lit)| {
            if names.contains(open_name) && names.contains(close_name) {
                Some((open_lit.to_string(), close_lit.to_string()))
            } else {
                None
            }
        })
        .collect()
}

/// Build the JSON `LanguageSpec` document for a parsed (`.tokens`,
/// `.grammar`) pair plus caller-supplied metadata.
///
/// Returns a `serde_json::Value` rather than a string so callers can
/// pretty-print it with their own indentation preference, embed it in a
/// larger document, or further inspect it programmatically.
///
/// The grammar argument is optional — passing `None` produces a spec
/// with `"rules"` empty.  This is useful when only the lexical level is
/// needed (some editor integrations don't care about parser rules).
pub fn dump_language_spec(
    tg: &TokenGrammar,
    pg: Option<&ParserGrammar>,
    meta: &SpecMetadata,
) -> Value {
    // Rule names from the grammar, if supplied.  We preserve the source
    // order rather than sorting so consumers can spot which rule the
    // grammar treats as the entry point (typically the first rule).
    let rules: Vec<String> = pg
        .map(|g| g.rules.iter().map(|r| r.name.clone()).collect())
        .unwrap_or_default();

    let brackets: Vec<Value> = infer_brackets(tg)
        .into_iter()
        .map(|(o, c)| json!([o, c]))
        .collect();

    // Block comment is emitted only when both delimiters are non-empty.
    // If either is missing the field is null, signalling "no block
    // comment configured".  Half-configured input is treated as none.
    let block_comment: Value =
        if !meta.block_comment_start.is_empty() && !meta.block_comment_end.is_empty() {
            json!([meta.block_comment_start, meta.block_comment_end])
        } else {
            Value::Null
        };

    let line_comment: Value = if meta.line_comment.is_empty() {
        Value::Null
    } else {
        Value::String(meta.line_comment.clone())
    };

    json!({
        "$schemaVersion":  1,
        "languageId":      meta.language_id,
        "languageName":    meta.language_name,
        "fileExtensions":  meta.file_extensions,
        "keywords":        tg.keywords,
        "reservedKeywords": tg.reserved_keywords,
        "contextKeywords":  tg.context_keywords,
        "lineComment":     line_comment,
        "blockComment":    block_comment,
        "brackets":        brackets,
        "rules":           rules,
        "declarationRules": meta.declaration_rules,
        "caseSensitive":   tg.case_sensitive,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser_grammar::parse_parser_grammar;
    use crate::token_grammar::parse_token_grammar;

    /// Tiny tokens fixture covering brackets, keywords, and a comment.
    const SAMPLE_TOKENS: &str = r#"
LPAREN     = "("
RPAREN     = ")"
NAME       = /[a-z][a-z0-9_]*/
NUMBER     = /[0-9]+/
keywords:
    define
    let
    if
skip:
    WHITESPACE = /\s+/
    COMMENT    = /;[^\n]*/
"#;

    const SAMPLE_GRAMMAR: &str = r#"
program     = { expression } ;
expression  = LPAREN NAME { expression } RPAREN | NAME | NUMBER ;
"#;

    fn meta() -> SpecMetadata {
        SpecMetadata {
            language_id: "twig".into(),
            language_name: "Twig".into(),
            file_extensions: vec!["twig".into(), "tw".into()],
            line_comment: ";".into(),
            block_comment_start: "".into(),
            block_comment_end: "".into(),
            declaration_rules: vec![],
        }
    }

    #[test]
    fn dump_spec_emits_keywords_in_source_order() {
        let tg = parse_token_grammar(SAMPLE_TOKENS).expect("tokens parse");
        let v = dump_language_spec(&tg, None, &meta());
        assert_eq!(v["keywords"], json!(["define", "let", "if"]));
    }

    #[test]
    fn dump_spec_includes_metadata_fields() {
        let tg = parse_token_grammar(SAMPLE_TOKENS).expect("tokens parse");
        let v = dump_language_spec(&tg, None, &meta());
        assert_eq!(v["languageId"], "twig");
        assert_eq!(v["languageName"], "Twig");
        assert_eq!(v["fileExtensions"], json!(["twig", "tw"]));
        assert_eq!(v["lineComment"], ";");
        assert_eq!(v["caseSensitive"], true);
    }

    #[test]
    fn dump_spec_infers_brackets_from_token_names() {
        let tg = parse_token_grammar(SAMPLE_TOKENS).expect("tokens parse");
        let v = dump_language_spec(&tg, None, &meta());
        assert_eq!(v["brackets"], json!([["(", ")"]]));
    }

    #[test]
    fn dump_spec_block_comment_is_null_when_absent() {
        let tg = parse_token_grammar(SAMPLE_TOKENS).expect("tokens parse");
        let v = dump_language_spec(&tg, None, &meta());
        assert!(v["blockComment"].is_null());
    }

    #[test]
    fn dump_spec_block_comment_present_when_both_supplied() {
        let tg = parse_token_grammar(SAMPLE_TOKENS).expect("tokens parse");
        let mut m = meta();
        m.block_comment_start = "/*".into();
        m.block_comment_end = "*/".into();
        let v = dump_language_spec(&tg, None, &m);
        assert_eq!(v["blockComment"], json!(["/*", "*/"]));
    }

    #[test]
    fn dump_spec_block_comment_null_when_only_one_supplied() {
        let tg = parse_token_grammar(SAMPLE_TOKENS).expect("tokens parse");
        let mut m = meta();
        m.block_comment_start = "/*".into();
        // missing end
        let v = dump_language_spec(&tg, None, &m);
        assert!(v["blockComment"].is_null());
    }

    #[test]
    fn dump_spec_includes_grammar_rules_when_provided() {
        let tg = parse_token_grammar(SAMPLE_TOKENS).expect("tokens parse");
        let pg = parse_parser_grammar(SAMPLE_GRAMMAR).expect("grammar parse");
        let v = dump_language_spec(&tg, Some(&pg), &meta());
        assert_eq!(v["rules"], json!(["program", "expression"]));
    }

    #[test]
    fn dump_spec_rules_empty_when_grammar_omitted() {
        let tg = parse_token_grammar(SAMPLE_TOKENS).expect("tokens parse");
        let v = dump_language_spec(&tg, None, &meta());
        assert_eq!(v["rules"], json!([]));
    }

    #[test]
    fn dump_spec_declaration_rules_passthrough() {
        let tg = parse_token_grammar(SAMPLE_TOKENS).expect("tokens parse");
        let mut m = meta();
        m.declaration_rules = vec!["define".into(), "module_form".into()];
        let v = dump_language_spec(&tg, None, &m);
        assert_eq!(v["declarationRules"], json!(["define", "module_form"]));
    }

    #[test]
    fn dump_spec_schema_version_is_1() {
        let tg = parse_token_grammar(SAMPLE_TOKENS).expect("tokens parse");
        let v = dump_language_spec(&tg, None, &meta());
        assert_eq!(v["$schemaVersion"], 1);
    }

    #[test]
    fn infer_brackets_returns_empty_when_no_pairs() {
        let tokens = "FOO = \"foo\"\nBAR = \"bar\"\n";
        let tg = parse_token_grammar(tokens).expect("tokens parse");
        assert!(infer_brackets(&tg).is_empty());
    }

    #[test]
    fn infer_brackets_returns_only_complete_pairs() {
        // LPAREN without RPAREN should not produce a bracket pair.
        let tokens = "LPAREN = \"(\"\nNAME = /[a-z]+/\n";
        let tg = parse_token_grammar(tokens).expect("tokens parse");
        assert!(infer_brackets(&tg).is_empty());
    }

    #[test]
    fn infer_brackets_recognises_multiple_pairs() {
        let tokens = "
LPAREN   = \"(\"
RPAREN   = \")\"
LBRACE   = \"{\"
RBRACE   = \"}\"
LBRACKET = \"[\"
RBRACKET = \"]\"
NAME     = /[a-z]+/
";
        let tg = parse_token_grammar(tokens).expect("tokens parse");
        let brackets = infer_brackets(&tg);
        assert_eq!(
            brackets,
            vec![
                ("(".into(), ")".into()),
                ("{".into(), "}".into()),
                ("[".into(), "]".into()),
            ]
        );
    }
}
