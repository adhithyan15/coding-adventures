//! # IDL Lexer — tokenizing IDL (Interactive Data Language).
//!
//! IDL (Research Systems Inc., 1977; today NV5 Geospatial) is the
//! science/astronomy array language whose lineage runs RSI -> ITT Visual
//! Information Solutions -> Exelis VIS -> L3Harris Geospatial -> NV5
//! Geospatial (`code/specs/MA12-idl-language.md` §1). Unlike every other
//! array-family frontend already in this repo (APL/J/Q/Scilab), IDL's
//! *surface* is an Algol/Fortran-family imperative grammar — statements,
//! `PRO`/`FUNCTION` definitions, `IF`/`FOR`/`WHILE`/`REPEAT` blocks, an
//! infix operator-precedence cascade with word operators (`EQ`/`AND`/...)
//! — closer in shape to this repo's `algol-lexer`/`dartmouth-basic-lexer`
//! than to any array-family lexer (MA12 §5). This crate does not
//! hand-write tokenization — it loads the compiled
//! `code/grammars/idl/idl.tokens` grammar and feeds it to the generic
//! [`GrammarLexer`]. `idl-lexer` only tokenizes; there is no `idl-parser`
//! or `idl.grammar` here (that is MA-12c, a separate follow-on task) and
//! no recursion-depth cap (that begins with recursive-descent parsing,
//! MA-12c — the same split every sibling `*-lexer`/`*-parser` pair in this
//! repo already follows).
//!
//! ## No pre/post-tokenize hooks — and why that is a deliberate finding, not an omission
//!
//! Every prior array-family lexer that had a same-character/two-meaning
//! ambiguity to resolve (Scilab's `'` transpose-vs-string-open, Q's `/`
//! comment-vs-REDUCE) needed a `GrammarLexer::add_pre_tokenize` /
//! `add_post_tokenize` hook, because the shared engine's declarative
//! `.tokens` layer has no lookaround and cannot express "does this
//! character have whitespace/a particular token type next to it." IDL has
//! an analogous-*looking* ambiguity — MA12 §3 item 3's `/KEYWORD` boolean
//! shorthand (`PLOT, x, /YLOG`) versus `/` as ordinary division — but,
//! unlike Q's `/`, it genuinely cannot be resolved by any hook operating on
//! raw characters or a flat token list, at any adjacency: `PLOT, x, /YLOG`
//! and `x = a/YLOG` both have `/` glued to an identifier with zero
//! intervening whitespace, yet mean different things, because the
//! distinguishing fact is *which grammar production is currently being
//! parsed* (is this position inside a call's argument list?) — genuine
//! parse-context information no lexer-level hook has access to. MA12 §3
//! item 3 says this explicitly: the signal is "grammatical position, not
//! whitespace." So this crate emits `SLASH` as one ordinary, unconditional
//! division-operator token in every position, full stop, and installs no
//! hooks at all — the `/KEYWORD` production is `idl-parser`'s problem
//! (MA-12c), not tokenized differently here. See `code/grammars/idl/idl.tokens`'s
//! own header comment for the fuller argument.
//!
//! The same absence of ambiguity applies to IDL's two quote characters (no
//! transpose operator exists to collide with `'`) and to `;` (no other
//! token in this cut's grammar shares that character the way Q's `/` does)
//! — so `idl.tokens` is entirely declarative, and this crate is a thin
//! wrapper with zero custom lexer code, simpler than every sibling array
//! frontend's own `*-lexer` crate.
//!
//! ## Case-insensitivity
//!
//! IDL is documented as not case sensitive for its language surface
//! (keywords, procedure/function/variable names), except for the contents
//! of a quoted string. `idl.tokens` sets `# @case_insensitive true` (no
//! `case_sensitive: false`), so only `keywords:`-block lookup folds case —
//! `if`/`If`/`IF` all promote to `KEYWORD("IF")` — while ordinary `NAME`
//! tokens and `STRING` contents keep the exact case the source text used.
//! See `idl.tokens`'s own header comment for the full reasoning and the
//! precedent (`dot.tokens`/`excel.tokens`/`spice/berkeley.tokens` already
//! use the identical combination in this repo).

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

mod _grammar;

// ===========================================================================
// Public API
// ===========================================================================

/// Create a [`GrammarLexer`] configured for IDL source.
///
/// No pre/post-tokenize hooks are installed — see this module's own doc
/// comment for why `idl.tokens` needs none (unlike `q-lexer`/`scilab-lexer`).
pub fn create_idl_lexer(source: &str) -> GrammarLexer<'_> {
    let grammar = _grammar::token_grammar();
    GrammarLexer::new(source, &grammar)
}

/// Tokenize IDL source text into a vector of tokens (ending in `EOF`).
///
/// # Panics
///
/// Panics on an unrecognized character. Use [`try_tokenize_idl`] for a
/// `Result`-returning version instead.
///
/// # Example
///
/// ```
/// use coding_adventures_idl_lexer::tokenize_idl;
///
/// let tokens = tokenize_idl("PRINT, 'hello'\n");
/// ```
pub fn tokenize_idl(source: &str) -> Vec<Token> {
    create_idl_lexer(source)
        .tokenize()
        .unwrap_or_else(|err| panic!("IDL tokenization failed: {err}"))
}

/// Tokenize IDL source text, returning a `Result` instead of panicking.
pub fn try_tokenize_idl(source: &str) -> Result<Vec<Token>, String> {
    create_idl_lexer(source)
        .tokenize()
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use lexer::token::TokenType;

    fn types(source: &str) -> Vec<String> {
        tokenize_idl(source)
            .into_iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| t.effective_type_name().to_string())
            .collect()
    }

    fn values(source: &str) -> Vec<String> {
        tokenize_idl(source)
            .into_iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| t.value)
            .collect()
    }

    // A handful of smoke tests directly against the public API; the crate's
    // `tests/test_tokenizer.rs` covers the full grammar plus a much larger
    // set of edge cases (mirroring q-lexer's/scilab-lexer's own split
    // between a small in-module smoke suite and the exhaustive integration
    // test file).

    #[test]
    fn tokenizes_a_print_statement_with_a_single_quoted_string() {
        // PRINT is an ordinary library-routine NAME, not a reserved
        // keyword -- MA12 Â§4 lists it alongside PLOT/SIN/TOTAL/etc. as an
        // intrinsic procedure/function, resolved by name at a later layer,
        // not part of the closed keyword set Â§3/Â§6 fix (IF/FOR/.../EQ/
        // AND/...). Only THOSE are promoted to KEYWORD here.
        assert_eq!(types("PRINT, 'hello'"), vec!["NAME", "COMMA", "STRING"]);
        assert_eq!(values("PRINT, 'hello'"), vec!["PRINT", ",", "hello"]);
    }

    #[test]
    fn keyword_lookup_is_case_insensitive_but_names_preserve_case() {
        assert_eq!(types("if"), vec!["KEYWORD"]);
        assert_eq!(values("if"), vec!["IF"]);
        assert_eq!(values("MyVar"), vec!["MyVar"]);
    }

    #[test]
    fn slash_is_always_plain_division_at_the_lexer_layer() {
        // The /KEYWORD-vs-division question is idl-parser's (MA-12c), not
        // this crate's -- see this module's own doc comment. PLOT is an
        // ordinary library-routine NAME (see the test above), not a
        // keyword.
        assert_eq!(types("a/YLOG"), vec!["NAME", "SLASH", "NAME"]);
        assert_eq!(
            types("PLOT, x, /YLOG"),
            vec!["NAME", "COMMA", "NAME", "COMMA", "SLASH", "NAME",]
        );
    }

    #[test]
    fn tokenize_idl_panics_on_malformed_source() {
        let result = std::panic::catch_unwind(|| tokenize_idl("@"));
        assert!(result.is_err());
    }

    #[test]
    fn create_idl_lexer_exposes_the_result_returning_api() {
        assert!(create_idl_lexer("x = 1").tokenize().is_ok());
        assert!(create_idl_lexer("@").tokenize().is_err());
    }
}
