//! # MacroOct lexer — PREP01 slice 1.
//!
//! Tokenizes MacroOct source text using the grammar-driven Rust lexer.  Thin
//! wrapper around the generic `GrammarLexer` over the auto-generated token
//! grammar in `_grammar.rs` (compiled from `code/grammars/macrooct/macrooct.tokens`
//! via the `grammar-tools` CLI).
//!
//! ## What MacroOct is
//!
//! **Oct, plus a preprocessor, and nothing else.** MacroOct is the first
//! proving ground for the generic preprocessor engine in
//! `code/specs/PREP01-generic-source-preprocessor.md`: a preprocessor runs
//! *before* the parser, so a dialect that gains one needs no new parser, no
//! new type checker and no new backend — only a lexical grammar that can see
//! its own directives.  That is what this crate provides, and it is the whole
//! of MacroOct's frontend divergence from Oct.
//!
//! ```text
//!     MacroOct source
//!         │
//!         ▼  THIS CRATE  (macrooct.tokens = oct.tokens + @directives)
//!     [Token]  including AT_INCLUDE / AT_IF / AT_ELSE / AT_END / AT_DEFINE
//!         │
//!         ▼  source-preprocessor engine + MacroOctDialect
//!     [Token]  pure Oct — every directive token consumed
//!         │
//!         ▼  Oct's own parser grammar, type checker and compile_ast, UNCHANGED
//!     IIRModule
//! ```
//!
//! ## Why this crate mirrors `oct-lexer` so closely
//!
//! It is the same lexer over a superset grammar, so any divergence in
//! *structure* would be a divergence with no cause — and a reader comparing
//! the two should be able to see at a glance that the only difference is the
//! grammar being compiled.  The keyword-promotion loop below is copied
//! deliberately rather than shared: `oct-lexer` is part of the reference this
//! dialect is checked against, and PREP01 forbids modifying it (a reference
//! you are free to edit is not a reference).  Extracting a shared helper would
//! mean editing Oct.
//!
//! The risk that buys — `macrooct.tokens` silently forking away from
//! `oct.tokens` — is not left to discipline.  See
//! [`tests::macrooct_agrees_with_oct_on_every_non_directive_rule`], which
//! parses **both** grammar files and asserts they agree rule for rule.
//!
//! ## Usage
//!
//! ```
//! use coding_adventures_macrooct_lexer::tokenize_macrooct;
//!
//! let tokens = tokenize_macrooct("@if 1\nfn main() { out(1, 42); }\n@end\n");
//! assert_eq!(tokens[0].value, "@if");
//! // The directive and the code it guards are on different source lines, which
//! // is how the engine tells them apart -- see `line`, below.
//! assert_eq!(tokens[0].line, 1);
//! assert!(tokens.iter().any(|t| t.value == "fn" && t.line == 2));
//! ```

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use grammar_tools::token_grammar::TokenGrammar;
use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

mod _grammar;

fn grammar() -> TokenGrammar {
    _grammar::token_grammar()
}

/// Create a `GrammarLexer` over a MacroOct source string.  Most callers want
/// [`tokenize_macrooct`] instead; this is the lower-level entry point.
pub fn create_macrooct_lexer(source: &str) -> GrammarLexer<'_> {
    let grammar = grammar();
    GrammarLexer::new(source, &grammar)
}

/// Tokenize a MacroOct source string, reporting a lex failure as an error.
///
/// `oct-lexer`'s equivalent panics; this one does not, because its caller is
/// the preprocessor engine, whose entire contract is that **no input, however
/// malformed, may panic** — including the text of an `@include`d file, which
/// is chosen by the program being compiled rather than by the person running
/// the compiler.  `MacroOctDialect::lex` converts this `Err` into a located
/// `PpError`, so a bad included file produces a diagnostic naming it.
pub fn try_tokenize_macrooct(source: &str) -> Result<Vec<Token>, String> {
    let grammar = grammar();
    let mut lexer = GrammarLexer::new(source, &grammar);
    let mut tokens = lexer.tokenize().map_err(|err| format!("{err}"))?;
    promote_keywords(&mut tokens);
    Ok(tokens)
}

/// Tokenize a MacroOct source string.  Panics on lex errors — mirrors
/// `oct_lexer::tokenize_oct` for callers (tests, mostly) that want the simple
/// shape.  Anything reachable from untrusted input should use
/// [`try_tokenize_macrooct`].
pub fn tokenize_macrooct(source: &str) -> Vec<Token> {
    try_tokenize_macrooct(source)
        .unwrap_or_else(|err| panic!("MacroOct tokenization failed: {err}"))
}

/// Promote `KEYWORD` tokens so their `type_name` matches the keyword value.
///
/// Identical to `oct-lexer`'s loop, and it has to be: Oct's *parser* grammar
/// matches keywords by type name (`"fn"`, `"let"`, …), and MacroOct hands its
/// preprocessed stream to that same grammar.  Skip this step and every
/// MacroOct program fails to parse with a message about `fn` — a confusing
/// distance from the actual cause.
fn promote_keywords(tokens: &mut [Token]) {
    for token in tokens.iter_mut() {
        if token.type_name.as_deref() == Some("KEYWORD") {
            token.type_name = Some(token.value.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_plain_oct_source_unchanged() {
        let tokens = tokenize_macrooct("fn main() { let x: u8 = 5; }");
        assert!(tokens.iter().any(|t| t.value == "fn"));
        assert!(tokens.iter().any(|t| t.value == "main"));
        assert!(tokens.iter().any(|t| t.value == "5"));
        assert!(tokens.iter().any(|t| t.value == "u8"));
    }

    #[test]
    fn keyword_type_names_are_promoted() {
        let tokens = tokenize_macrooct("fn main() {}");
        let fn_tok = tokens.iter().find(|t| t.value == "fn").unwrap();
        assert_eq!(fn_tok.type_name.as_deref(), Some("fn"));
    }

    #[test]
    fn each_directive_lexes_as_one_token_of_its_own_type() {
        // One token, not `@` followed by a word: the dialect classifies a line
        // by its first token's value, so a split spelling would make every
        // directive unrecognisable.
        //
        // The trailing `Eof` is the `GrammarLexer`'s end sentinel, present on
        // every stream it produces. It is not incidental here — see
        // `the_stream_ends_with_an_eof_sentinel` below.
        for (src, name) in [
            ("@include", "AT_INCLUDE"),
            ("@define", "AT_DEFINE"),
            ("@if", "AT_IF"),
            ("@else", "AT_ELSE"),
            ("@end", "AT_END"),
        ] {
            let tokens = tokenize_macrooct(src);
            assert_eq!(tokens.len(), 2, "{src} must lex as one token plus Eof");
            assert_eq!(tokens[0].value, src);
            assert_eq!(tokens[0].type_name.as_deref(), Some(name));
            assert_eq!(tokens[1].type_, lexer::token::TokenType::Eof);
        }
    }

    /// The end sentinel exists, and `macrooct-iir-compiler` has to know it.
    ///
    /// The preprocessor engine groups tokens into logical lines by
    /// `Token::line` and hands each run to the dialect. An `Eof` sentinel
    /// sitting on the *same line as a closing `@end`* would be swallowed as
    /// part of that directive and never emitted; an `Eof` arriving from an
    /// `@include`d file would be spliced into the **middle** of the stream.
    /// Both are silent corruptions, so the compiler strips the sentinel before
    /// preprocessing and re-appends it after, and the dialect's `lex` strips it
    /// from every included file. This test pins the fact those two mitigations
    /// depend on.
    #[test]
    fn the_stream_ends_with_an_eof_sentinel() {
        for src in ["", "fn main() {}", "@end", "@end\n"] {
            let tokens = tokenize_macrooct(src);
            assert_eq!(
                tokens.last().map(|t| t.type_),
                Some(lexer::token::TokenType::Eof),
                "{src:?} must end with the lexer's Eof sentinel"
            );
            assert!(
                tokens[..tokens.len() - 1]
                    .iter()
                    .all(|t| t.type_ != lexer::token::TokenType::Eof),
                "{src:?} must carry exactly one Eof, at the end"
            );
        }
        // And on a source with no trailing newline, the sentinel really does
        // share the last line -- the case that would silently eat it.
        let tokens = tokenize_macrooct("@if 1\nfn main() {}\n@end");
        let end = tokens.iter().find(|t| t.value == "@end").unwrap();
        assert_eq!(tokens.last().unwrap().line, end.line);
    }

    #[test]
    fn at_if_does_not_shadow_at_include() {
        // `@if` is listed after `@include`, but the two only diverge at their
        // third character, so this is the ordering hazard worth an explicit
        // test: `@include` must not lex as AT_IF followed by garbage.
        let tokens = tokenize_macrooct("@include \"ports.oct\"");
        assert_eq!(tokens[0].type_name.as_deref(), Some("AT_INCLUDE"));
        assert_eq!(tokens[1].type_name.as_deref(), Some("STR_LIT"));
        assert_eq!(tokens[1].value, "\"ports.oct\"");
    }

    #[test]
    fn the_keyword_if_and_the_directive_at_if_are_different_tokens() {
        // The practical dividend of the `@` prefix: MacroOct never has to
        // decide whether `if` at the start of a line means Oct's conditional
        // statement or the preprocessor's conditional directive.
        let tokens = tokenize_macrooct("@if 1\nif x == 1 { }\n@end");
        assert_eq!(tokens[0].type_name.as_deref(), Some("AT_IF"));
        let oct_if = tokens.iter().find(|t| t.value == "if").unwrap();
        assert_eq!(oct_if.type_name.as_deref(), Some("if"));
    }

    #[test]
    fn a_string_literal_stops_at_the_end_of_its_line() {
        // An unterminated quote must not swallow the rest of the file into one
        // token. `STR_LIT` excludes `\n`, so this input has no STR_LIT at all
        // and fails to lex -- reported here, on this line, rather than as a
        // baffling error thousands of lines later.
        assert!(try_tokenize_macrooct("@include \"oops\nfn main() {}").is_err());
    }

    #[test]
    fn tokens_carry_the_line_the_engine_groups_them_by() {
        // MacroOct's grammar skips newlines (Oct's does), so nothing marks an
        // end of line in the token stream. The preprocessor groups by
        // `Token::line` instead, which makes this field load-bearing rather
        // than merely diagnostic.
        let tokens = tokenize_macrooct("@if 1\nfn main() { }\n@end\n");
        assert_eq!(tokens[0].line, 1);
        assert_eq!(tokens[1].line, 1);
        assert!(tokens.iter().filter(|t| t.line == 2).count() >= 5);
        let end = tokens.iter().find(|t| t.value == "@end").unwrap();
        assert_eq!(end.line, 3);
        // `@end` is alone on its line — nothing else shares it, which is what
        // makes "the run of tokens on an AT_* token's line" a sound definition
        // of "the directive".
        assert_eq!(tokens.iter().filter(|t| t.line == 3).count(), 1);
    }

    #[test]
    fn a_lex_failure_is_an_error_not_a_panic() {
        // The engine's no-panic contract reaches through `Dialect::lex` into
        // this crate, because an included file's text is chosen by the program
        // being compiled.
        assert!(try_tokenize_macrooct("fn main() { let x: u8 = $; }").is_err());
    }

    // ---------------------------------------------------------------------
    // The drift guard
    // ---------------------------------------------------------------------

    /// Path to a grammar file, relative to this crate.
    ///
    /// Reading the `.tokens` sources at test time is deliberate, and is the
    /// only place this repo's compiled-grammar convention is stepped around.
    /// The property under test is a statement about the two **source
    /// grammars**, not about two compiled artefacts: comparing the generated
    /// `_grammar.rs` files instead would pass happily if someone edited
    /// `oct.tokens` and did not regenerate, which is the single most likely
    /// way this drift actually happens.
    fn grammar_path(language: &str, file: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../grammars")
            .join(language)
            .join(file)
    }

    fn parse(language: &str, file: &str) -> TokenGrammar {
        let path = grammar_path(language, file);
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        grammar_tools::token_grammar::parse_token_grammar(&text)
            .unwrap_or_else(|e| panic!("cannot parse {}: {e}", path.display()))
    }

    /// The names MacroOct adds. Everything else must match Oct exactly.
    const MACROOCT_ONLY: &[&str] =
        &["AT_INCLUDE", "AT_DEFINE", "AT_IF", "AT_ELSE", "AT_END", "STR_LIT"];

    /// `macrooct.tokens` restates Oct's lexical rules, so it can fork the
    /// language by accident. This is the test PREP01 §7 requires: delete a
    /// rule, reorder two, or change one pattern, and it fails naming the rule.
    ///
    /// Order is compared, not just membership, because the lexer is
    /// first-match-wins: moving `INT_LIT` above `BIN_LIT` changes what `0b101`
    /// means while leaving the *set* of rules identical.
    #[test]
    fn macrooct_agrees_with_oct_on_every_non_directive_rule() {
        let oct = parse("oct", "oct.tokens");
        let macrooct = parse("macrooct", "macrooct.tokens");

        // Compare by (name, pattern, is_regex). `line_number` is deliberately
        // excluded: the two files have different comment prose, so their line
        // numbers differ by construction and comparing them would make the
        // test fail on every edit to either file's documentation.
        let shape = |d: &grammar_tools::token_grammar::TokenDefinition| {
            (d.name.clone(), d.pattern.clone(), d.is_regex)
        };

        let oct_rules: Vec<_> = oct.definitions.iter().map(shape).collect();
        let macrooct_rules: Vec<_> = macrooct
            .definitions
            .iter()
            .filter(|d| !MACROOCT_ONLY.contains(&d.name.as_str()))
            .map(shape)
            .collect();

        assert_eq!(
            macrooct_rules, oct_rules,
            "macrooct.tokens has drifted from oct.tokens on a non-directive rule. \
             MacroOct is a preprocessor dialect of Oct, so every rule except \
             {MACROOCT_ONLY:?} must match Oct's exactly, in Oct's order (the lexer \
             is first-match-wins, so order is semantics). Update macrooct.tokens \
             to match; do NOT change oct.tokens, which PREP01 holds fixed as the \
             reference."
        );

        // The additions must actually be present — otherwise deleting all five
        // directive rules would make the filter vacuous and this test pass.
        let names: Vec<&str> = macrooct.definitions.iter().map(|d| d.name.as_str()).collect();
        for extra in MACROOCT_ONLY {
            assert!(names.contains(extra), "macrooct.tokens lost its {extra} rule");
        }

        // Keywords and skip rules are part of the lexical language too. A
        // MacroOct that promoted an extra keyword, or stopped skipping `//`
        // comments, would accept a different language while every definition
        // above still matched.
        assert_eq!(
            macrooct.keywords, oct.keywords,
            "MacroOct must promote exactly Oct's keywords — a directive is spelled \
             with a leading `@`, so MacroOct needs no keyword of its own"
        );
        assert_eq!(
            macrooct.skip_definitions.iter().map(shape).collect::<Vec<_>>(),
            oct.skip_definitions.iter().map(shape).collect::<Vec<_>>(),
            "MacroOct must skip exactly what Oct skips"
        );
        assert_eq!(macrooct.case_sensitive, oct.case_sensitive);
        assert_eq!(macrooct.case_insensitive, oct.case_insensitive);
    }

    /// The compiled grammar must actually be the compilation of the file the
    /// test above checked.
    ///
    /// Without this, `macrooct.tokens` could agree with `oct.tokens` perfectly
    /// while `_grammar.rs` — the thing the lexer really uses — was generated
    /// from an older revision. Checking the source file and using a compiled
    /// artefact leaves exactly that gap between them.
    #[test]
    fn the_compiled_grammar_matches_the_macrooct_tokens_file() {
        let from_file = parse("macrooct", "macrooct.tokens");
        let compiled = grammar();

        let shape = |d: &grammar_tools::token_grammar::TokenDefinition| {
            (d.name.clone(), d.pattern.clone(), d.is_regex)
        };
        assert_eq!(
            compiled.definitions.iter().map(shape).collect::<Vec<_>>(),
            from_file.definitions.iter().map(shape).collect::<Vec<_>>(),
            "src/_grammar.rs is stale — regenerate it with \
             `grammar-tools compile-tokens code/grammars/macrooct/macrooct.tokens \
             -o code/packages/rust/macrooct-lexer/src/_grammar.rs`"
        );
        assert_eq!(compiled.keywords, from_file.keywords);
        assert_eq!(
            compiled.skip_definitions.iter().map(shape).collect::<Vec<_>>(),
            from_file.skip_definitions.iter().map(shape).collect::<Vec<_>>()
        );
    }
}
