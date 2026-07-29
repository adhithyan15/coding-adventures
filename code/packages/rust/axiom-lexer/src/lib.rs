//! # Axiom Lexer — tokenizing Axiom, the strongly-typed CAS.
//!
//! Axiom (Scratchpad II, IBM Research, 1977; commercialized 1992; today
//! continued by OpenAxiom/FriCAS/the independent Axiom project —
//! `code/specs/MA13-axiom-language.md` §1) is this repo's first
//! symbolic-family (CAS) language whose value model needs a per-value
//! domain/category type tag at all (MA13 §2) — every prior CAS sibling here
//! (Macsyma, Wolfram, Derive, Reduce, Maple) is, underneath its own surface
//! syntax, a single flat universe of untyped symbolic expressions. This
//! crate covers only MA13 §3's scoped first cut: the **consumer view** of
//! Axiom's category/domain system (`:` declare, `::` coerce, `has` query)
//! over ordinary CAS-family arithmetic/comparison/assignment/definition
//! surface — never the *producer* side (user-defined categories, domains,
//! `Join`, packages), which MA13 §3/§4 defer whole to a future item.
//!
//! Like every language frontend in this repo, this crate does not
//! hand-write tokenization — it loads the compiled
//! `code/grammars/axiom/axiom.tokens` grammar and feeds it to the generic
//! [`GrammarLexer`]. `axiom-lexer` only tokenizes; there is no
//! `axiom-parser` or `axiom.grammar` here (that is MA-13c, a separate
//! follow-on task).
//!
//! ## No recursion-depth cap — and why that is the right call here, not an omission
//!
//! `axiom-lexer` performs no recursive descent at all: [`GrammarLexer`]
//! tokenizes with a single left-to-right scan over the source text, one
//! token at a time, with no call stack that grows with input structure.
//! Every sibling `*-lexer` crate in this repo that shares this shape says so
//! explicitly and adds no depth cap of its own — `apl-lexer`/`j-lexer`
//! (per `q-lexer`'s own doc comment), `q-lexer` itself, `scilab-lexer`, and
//! most directly `idl-lexer` ("no recursion-depth cap ... the same split
//! every sibling `*-lexer`/`*-parser` pair in this repo already follows").
//! A recursion/rule-depth cap (`MAX_RULE_DEPTH`-style, per this repo's own
//! `lessons.md` entries on parser recursion) is a **recursive-descent
//! parser** concern — it belongs to `axiom-parser` (MA-13c), the first
//! layer in Axiom's frontend that actually recurses (nested expressions,
//! nested parenthesised blocks), exactly as it does for every sibling
//! lexer/parser pair already in this repo. Adding one here, ahead of that
//! layer existing, would not guard against anything this crate's own
//! tokenization loop can do — it processes O(input length) tokens with O(1)
//! stack depth regardless of how deeply nested the *source* is, since
//! nesting structure is invisible to a flat token scan.
//!
//! ## No pre/post-tokenize hooks — `axiom.tokens` is entirely declarative
//!
//! Unlike `q-lexer` (whitespace-adjacency hooks for `-`-vs-negative-literal
//! and `/`-vs-comment) and `scilab-lexer` (a hook for `'`
//! transpose-vs-string), Axiom's MA-13b-scoped surface has no character
//! that means two different things depending on adjacent whitespace or a
//! preceding value. Every multi-character operator (`:=`, `::`, `==`, `~=`,
//! `<=`, `>=`, `**`) is resolved by ordinary longest-match-first ordering in
//! `axiom.tokens` itself, and `--` line comments never collide with `-`
//! (MINUS) because [`GrammarLexer`]'s own skip-pattern pass always runs
//! *before* ordinary token matching at every position — the same
//! declarative shape `sql.tokens`/`vhdl*.tokens`/`haskell*.tokens` already
//! rely on for their own coexisting MINUS/`--`-comment pair. So
//! `create_axiom_lexer` installs nothing beyond the compiled grammar,
//! mirroring `idl-lexer`'s equally hook-free shape.
//!
//! ## What this crate does *not* tokenize (MA13 §4's deferred list)
//!
//! No token exists anywhere in `axiom.tokens` for: `Record`/`Union`/`Any`,
//! `macro`, package-calling `$`, target-type `@`, the anonymous
//! "maps-to" function operator `+->`, block early-exit `=>`,
//! piecewise/multi-clause function definitions, list comprehensions, or
//! `for`/`while` iteration — all explicitly out of MA13's first-cut scope.
//! A source string using one of these constructs lexes every character it
//! *can* recognize and then fails honestly on the first character it
//! cannot (e.g. `@`, `$`), rather than silently accepting it as something
//! else.
//!
//! Built-in domain names (`Integer`, `Boolean`, `Fraction`, `Polynomial`,
//! `List`, `PositiveInteger`, `NonNegativeInteger`, `Float`, `String`) and
//! built-in category names (`Ring`, `OrderedSet`) are **not** lexer-level
//! keywords — MA13 §3/§4 fixes them as a small, non-extensible lookup table
//! entirely internal to a future `axiom-runtime` (MA-13d); this crate
//! tokenizes every one of them as an ordinary `NAME`, exactly like
//! `idl-lexer` resolves IDL's intrinsic procedure/function names (`PLOT`,
//! `SIN`, `TOTAL`, ...) as plain `NAME`s rather than reserved words. Only
//! `if`/`then`/`else`/`has` are real keywords in this cut (MA13 §4).

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

mod _grammar;

// ===========================================================================
// Public API
// ===========================================================================

/// Create a [`GrammarLexer`] configured for Axiom source.
///
/// No pre/post-tokenize hooks are installed — see this module's own doc
/// comment for why `axiom.tokens` needs none (unlike `q-lexer`/
/// `scilab-lexer`).
pub fn create_axiom_lexer(source: &str) -> GrammarLexer<'_> {
    let grammar = _grammar::token_grammar();
    GrammarLexer::new(source, &grammar)
}

/// Tokenize Axiom source text into a vector of tokens (ending in `EOF`).
///
/// # Panics
///
/// Panics on an unrecognized character. Use [`try_tokenize_axiom`] for a
/// `Result`-returning version instead.
///
/// # Example
///
/// ```
/// use coding_adventures_axiom_lexer::tokenize_axiom;
///
/// let tokens = tokenize_axiom("a : PositiveInteger\na := 3\n");
/// ```
pub fn tokenize_axiom(source: &str) -> Vec<Token> {
    create_axiom_lexer(source)
        .tokenize()
        .unwrap_or_else(|err| panic!("Axiom tokenization failed: {err}"))
}

/// Tokenize Axiom source text, returning a `Result` instead of panicking.
pub fn try_tokenize_axiom(source: &str) -> Result<Vec<Token>, String> {
    create_axiom_lexer(source)
        .tokenize()
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use lexer::token::TokenType;

    fn types(source: &str) -> Vec<String> {
        tokenize_axiom(source)
            .into_iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| t.effective_type_name().to_string())
            .collect()
    }

    fn values(source: &str) -> Vec<String> {
        tokenize_axiom(source)
            .into_iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| t.value)
            .collect()
    }

    // A handful of smoke tests directly against the public API; the crate's
    // `tests/test_tokenizer.rs` covers the full grammar plus a much larger
    // set of edge cases (mirroring idl-lexer's/q-lexer's own split between
    // a small in-module smoke suite and the exhaustive integration test
    // file).

    #[test]
    fn tokenizes_a_declaration_and_immediate_assignment() {
        assert_eq!(
            types("a : PositiveInteger\na := 3"),
            vec!["NAME", "COLON", "NAME", "NAME", "ASSIGN", "NUMBER"]
        );
    }

    #[test]
    fn tokenizes_a_coercion() {
        assert_eq!(
            types("3 :: Fraction Integer"),
            vec!["NUMBER", "COERCE", "NAME", "NAME"]
        );
    }

    #[test]
    fn tokenizes_a_has_query() {
        assert_eq!(
            types("Polynomial(Integer) has Ring"),
            vec!["NAME", "LPAREN", "NAME", "RPAREN", "KEYWORD", "NAME"]
        );
        assert_eq!(values("Polynomial(Integer) has Ring")[4], "has");
    }

    #[test]
    fn rational_division_is_ordinary_number_slash_number() {
        // MA13 §4: `1/3` is NOT a dedicated rational-literal token -- it is
        // ordinary integer division, three tokens, lowered to a packed
        // rational representation entirely at a later layer.
        assert_eq!(types("1/3"), vec!["NUMBER", "SLASH", "NUMBER"]);
    }

    #[test]
    fn tokenize_axiom_panics_on_malformed_source() {
        let result = std::panic::catch_unwind(|| tokenize_axiom("@"));
        assert!(result.is_err());
    }

    #[test]
    fn create_axiom_lexer_exposes_the_result_returning_api() {
        assert!(create_axiom_lexer("x := 1").tokenize().is_ok());
        assert!(create_axiom_lexer("@").tokenize().is_err());
    }
}
