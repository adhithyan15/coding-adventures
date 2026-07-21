//! # Q Lexer — tokenizing kdb+'s scripting language.
//!
//! Q (Kx Systems, 2003) is the readable second surface syntax over the
//! K/kdb+ execution engine (`code/specs/MA11-q-language.md` §1) — the same
//! noun/verb split and right-to-left, no-precedence evaluation as APL/J
//! (MA05/MA06), plus keyword-free symbolic primitives, an explicit
//! function-literal syntax, and (deferred past this cut) a first-class
//! table type. Like every language frontend in this repo, this crate does
//! not hand-write the bulk of tokenization — it loads the compiled
//! `code/grammars/q/q.tokens` grammar and feeds it to the generic
//! [`GrammarLexer`]. `q-lexer` only tokenizes; there is no `q-parser` or
//! `q.grammar` here (that is MA-11c, a separate follow-on task) and no
//! recursion-depth cap (that is a parser-level concern for MA-11c, the
//! same "cap belongs where recursive descent happens" precedent
//! `apl-parser`/`j-parser` already set — `apl-lexer`/`j-lexer` have none
//! either).
//!
//! ## Design decision: two whitespace-sensitive rules live in Rust, not in `q.tokens`
//!
//! Every other `.tokens` file in this repo's array-language family treats
//! whitespace as pure separator noise between otherwise unambiguous tokens.
//! Q's real lexer breaks that assumption in exactly two places
//! (MA11 §3 bullet 2), both driven by *whether whitespace is adjacent to a
//! character*, not by which characters are present:
//!
//! 1. **Negative-literal vs. subtraction.** `2 -1` (space before `-`, none
//!    after) tokenizes as the two-element strand `NUMBER(2) NUMBER(-1)`;
//!    `2 - 1` and `2-1` both tokenize as `NUMBER(2) MINUS NUMBER(1)`
//!    (ordinary subtraction — with or without surrounding spaces).
//! 2. **`/` comment marker vs. REDUCE adverb.** A `/` preceded by
//!    whitespace (or at the start of a line) opens a comment to end of
//!    line; a `/` glued directly to a preceding verb/noun character with no
//!    intervening space is the REDUCE adverb (`+/x`).
//!
//! Rust's `regex` crate — what [`GrammarLexer`] compiles every `.tokens`
//! pattern with — has no lookaround support (no `(?<=...)`, no `(?=...)`),
//! so "was there whitespace immediately before this position" cannot be
//! written as a token pattern no matter how it's phrased. This is a real
//! limit of the shared engine, confirmed by reading
//! `code/packages/rust/grammar-tools/src/token_grammar.rs`'s own
//! `validate_token_grammar` (it compiles every pattern with
//! `regex::Regex::new` directly) — not an assumption.
//!
//! This is not a new problem in this repo: `scilab-lexer` already solves
//! the same *shape* of problem (`'` is transpose-or-string depending on
//! whether a value immediately precedes it) with a **pre-tokenize hook**
//! (rewrites the raw source text before `GrammarLexer` sees it) and a
//! **post-tokenize hook** (patches the resulting `Vec<Token>` afterward) —
//! see `code/packages/rust/scilab-lexer/src/lib.rs`'s `protect_quotes` /
//! `restore_placeholders`. This crate reuses that exact strategy,
//! independently implemented for Q's two different disambiguations:
//!
//! - [`strip_slash_comments`] is a **pre-tokenize** hook. Comment text is
//!   arbitrary and need not lex successfully as Q code at all — if this
//!   ran as a *post*-tokenize pass instead, a single unrecognized character
//!   inside a comment (anything not in `q.tokens`'s alphabet) would make
//!   the whole file fail to tokenize before the pass ever got a token list
//!   to patch. Blanking comment text out of the *source string* first
//!   sidesteps that entirely: by the time `GrammarLexer` runs, a comment
//!   region is nothing but spaces, which the grammar's own `WHITESPACE`
//!   skip pattern consumes exactly like any other run of whitespace.
//! - [`fold_negative_number_literals`] is a **post-tokenize** hook.
//!   Deciding whether a `-`/digit pair folds into one signed literal
//!   requires knowing the *type* of the previously emitted token (is it a
//!   NUMBER/NAME/closing bracket — does it complete a noun, the way `2`
//!   does in `2-1`?) — exactly the kind of already-classified information
//!   only the token stream carries. There is no way to decide this from
//!   raw characters alone without re-implementing token classification by
//!   hand, which is precisely the "no hand-written lexer" line this
//!   design stays on the right side of: the hook consumes tokens
//!   `GrammarLexer` already produced, it does not re-derive them.
//!
//! Neither hook is a general-purpose mechanism bolted onto `GrammarLexer`
//! itself — the shared engine already exposes `add_pre_tokenize` /
//! `add_post_tokenize` for exactly this kind of narrow, documented,
//! per-language adjustment (see `code/packages/rust/lexer/src/grammar_lexer.rs`),
//! so no changes to the shared crate were needed or made.
//!
//! ### A gap noticed in the shared engine (documented, not fixed here)
//!
//! While investigating whether `GrammarLexer` already had a *declarative*
//! way to express "no preceding whitespace" (the F10 pattern-group /
//! mode-transition system), the transition table only fires *after* a
//! token is emitted, keyed on the emitted token's type/value — it has no
//! notion of "was trivia consumed immediately before this token", so it
//! cannot see the signal this crate needs. That is a real, general gap in
//! the shared engine (any future language needing the same kind of
//! whitespace-position signal will hit it too), not something specific to
//! Q — per this repo's own "fix what's local, defer what's shared"
//! discipline, it is left unfixed here and simply recorded in this comment
//! for whoever picks up a language that needs it declaratively.

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

mod _grammar;

// ===========================================================================
// Pre-tokenize hook: `/`-comment stripping (MA11 §3 bullet 2, second item)
// ===========================================================================

/// Blank out `/`-to-end-of-line comments before `GrammarLexer` ever sees the
/// source text.
///
/// A `/` opens a comment when it is preceded by whitespace or starts a line
/// (including the very start of the file); it is left untouched — becoming
/// the ordinary `REDUCE` token — when it directly follows a non-whitespace
/// character.
///
/// # Algorithm
///
/// A single left-to-right scan over the characters, tracking one boolean:
/// whether the *immediately preceding* character (in the source as written,
/// not the rewritten output) was "whitespace-like" — a space, a tab, a
/// carriage return, a newline, or nothing at all (start of input, which is
/// trivially the start of line 1).
///
/// When a `/` is reached while that flag is set, everything from the `/`
/// through (but not including) the next `\n` — or end of input, if the
/// comment is on the last line — is replaced character-for-character with a
/// single space. Replacing rather than deleting is what keeps this a
/// *pre*-tokenize hook instead of a hand-rolled lexer: every other
/// character's line/column position is completely unaffected, because
/// nothing is removed, only turned into whitespace the grammar's own
/// `WHITESPACE` skip pattern already knows how to consume. The terminating
/// newline itself is left alone (not blanked) because `q.tokens` gives it
/// its own significant `NEWLINE` token, exactly like `NB.` comments do in
/// `j.tokens` — the comment ends just before it, never swallowing it.
///
/// Because Q's in-scope value model has no string or symbol literals yet
/// (MA11 §4 defers both), there is no quoted region this scan needs to
/// avoid stepping into — unlike `scilab-lexer`'s `protect_quotes`, which
/// must dodge `'...'`/`"..."` contents. That asymmetry is exactly why this
/// hook is a single pass with one boolean instead of a full character-class
/// state machine.
fn strip_slash_comments(source: String) -> String {
    let chars: Vec<char> = source.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(n);
    let mut i = 0;
    // Start of input counts as "whitespace-like" — trivially the start of
    // line 1, per the comment-opening rule.
    let mut prev_is_whitespace_like = true;

    while i < n {
        let c = chars[i];
        if c == '/' && prev_is_whitespace_like {
            // Comment: blank through to (but not including) the next '\n',
            // or to end of input if this is the last line.
            while i < n && chars[i] != '\n' {
                out.push(' ');
                i += 1;
            }
            // The blanked run is itself whitespace, so the flag stays set —
            // relevant if a lone '/' were somehow immediately followed by
            // another '/' at end-of-input with no newline between (it
            // can't be, since the inner loop above already consumed every
            // non-newline character up to EOF in that case, but keeping the
            // flag correct here costs nothing and avoids relying on that).
            prev_is_whitespace_like = true;
            continue;
        }
        out.push(c);
        prev_is_whitespace_like = matches!(c, ' ' | '\t' | '\r' | '\n');
        i += 1;
    }

    out
}

// ===========================================================================
// Post-tokenize hook: negative-literal folding (MA11 §3 bullet 2, first item)
// ===========================================================================

/// Token type-names that "complete a noun" — a value a following `-`,
/// glued directly against it with no intervening space, could plausibly be
/// subtracting from. `NUMBER` and `NAME` are the obvious cases (`2-1`,
/// `x-1`); `RPAREN` covers a parenthesised expression's result (`(2+3)-1`);
/// `RBRACE` covers a function-literal value (MA11 §3 bullet 1: a lambda is
/// itself an ordinary noun); `RBRACKET` is included defensively for the
/// same "closing delimiter ends a value" principle even though this cut's
/// grammar has no bracket-terminated noun expression yet (indexing is
/// deferred, MA11 §4) — future-proofing at zero cost today.
fn ends_a_noun(effective_type_name: &str) -> bool {
    matches!(
        effective_type_name,
        "NUMBER" | "NAME" | "RPAREN" | "RBRACKET" | "RBRACE"
    )
}

/// The column immediately after `tok`, i.e. the column a following
/// character would occupy if it were glued on with zero gap.
///
/// Valid because every token this grammar can produce is single-line: Q's
/// in-scope literal set (MA11 §4) has no multi-line strings, so `column +
/// (char count of value)` is always the correct end position. `MINUS`'s own
/// value is always exactly one character (`"-"`), so this also gives the
/// position where a following digit would need to start to be "glued".
fn end_column(tok: &Token) -> usize {
    tok.column + tok.value.chars().count()
}

/// Fold a `MINUS` token immediately followed by a `NUMBER` token into a
/// single signed `NUMBER` token, when doing so is unambiguous per MA11 §3
/// bullet 2's rule — restated precisely:
///
/// > a `-` immediately followed by a digit with no intervening space, at a
/// > position where a new list-stranding element may start, is folded into
/// > a signed-numeric-literal token rather than emitted as the standalone
/// > subtract verb.
///
/// "a position where a new list-stranding element may start" is exactly
/// "NOT immediately continuing an already-completed noun with zero gap" —
/// i.e. the previous emitted token, if any, is either not present, not a
/// noun-ending token at all (see [`ends_a_noun`]), or *is* noun-ending but
/// separated from this `-` by at least one space. Concretely:
///
/// | Input       | Prev tok / gap before `-`      | Fold? | Result                 |
/// |-------------|--------------------------------|-------|------------------------|
/// | `2 -1`      | `NUMBER(2)`, space before `-`  | yes   | `NUMBER(2) NUMBER(-1)` |
/// | `2 - 1`     | space before AND after `-`     | no*   | `NUMBER(2) MINUS NUMBER(1)` |
/// | `2-1`       | `NUMBER(2)`, glued, no gap     | no    | `NUMBER(2) MINUS NUMBER(1)` |
/// | `x:-1`      | `COLON`, glued, not noun-ending| yes   | `NAME COLON NUMBER(-1)`|
/// | `(2+3) -1`  | `RPAREN`, space before `-`     | yes   | `... RPAREN NUMBER(-1)`|
/// | `(2+3)-1`   | `RPAREN`, glued, no gap        | no    | `... RPAREN MINUS NUMBER(1)` |
///
/// (*`2 - 1` never even reaches the noun-adjacency check: the space
/// *after* the `-` alone already disqualifies it, since the rule requires
/// no gap between `-` and the digit.)
///
/// # Algorithm
///
/// A single left-to-right pass building a new `Vec<Token>`. At each `MINUS`
/// token, look at the very next token: if it is a `NUMBER` glued to the
/// `MINUS` with zero gap (same line, adjacent columns), and the token most
/// recently pushed to the *output* is either absent, not noun-ending, or
/// noun-ending but NOT glued to this `MINUS` with zero gap, then splice the
/// two input tokens into one `NUMBER` token carrying `"-"` + the original
/// digits, positioned at the `MINUS`'s own line/column. Otherwise the
/// `MINUS` is pushed through unchanged and the loop continues normally —
/// crucially, an un-folded `MINUS` still becomes the "previous token" seen
/// by the next iteration, so a chain like `1 - -1` resolves each `-`
/// independently and correctly (see the tests).
fn fold_negative_number_literals(tokens: Vec<Token>) -> Vec<Token> {
    let mut out: Vec<Token> = Vec::with_capacity(tokens.len());
    let mut i = 0;

    while i < tokens.len() {
        let tok = &tokens[i];

        if tok.effective_type_name() == "MINUS" {
            if let Some(next) = tokens.get(i + 1) {
                let glued_to_digit = next.effective_type_name() == "NUMBER"
                    && next.line == tok.line
                    && next.column == end_column(tok);

                let glued_to_prev_noun = out.last().is_some_and(|prev| {
                    ends_a_noun(prev.effective_type_name())
                        && prev.line == tok.line
                        && end_column(prev) == tok.column
                });

                if glued_to_digit && !glued_to_prev_noun {
                    let mut folded = next.clone();
                    folded.value = format!("-{}", next.value);
                    folded.line = tok.line;
                    folded.column = tok.column;
                    out.push(folded);
                    i += 2;
                    continue;
                }
            }
        }

        out.push(tok.clone());
        i += 1;
    }

    out
}

// ===========================================================================
// Public API
// ===========================================================================

/// Create a [`GrammarLexer`] configured for Q source, with the
/// comment-stripping and negative-literal-folding hooks installed.
pub fn create_q_lexer(source: &str) -> GrammarLexer<'_> {
    let grammar = _grammar::token_grammar();
    let mut lexer = GrammarLexer::new(source, &grammar);
    lexer.add_pre_tokenize(Box::new(strip_slash_comments));
    lexer.add_post_tokenize(Box::new(fold_negative_number_literals));
    lexer
}

/// Tokenize Q source text into a vector of tokens (ending in `EOF`).
///
/// # Panics
///
/// Panics on an unrecognized character. Use [`try_tokenize_q`] for a
/// `Result`-returning version instead.
///
/// # Example
///
/// ```
/// use coding_adventures_q_lexer::tokenize_q;
///
/// let tokens = tokenize_q("x:2 -1\n+/x");
/// ```
pub fn tokenize_q(source: &str) -> Vec<Token> {
    create_q_lexer(source)
        .tokenize()
        .unwrap_or_else(|err| panic!("Q tokenization failed: {err}"))
}

/// Tokenize Q source text, returning a `Result` instead of panicking.
pub fn try_tokenize_q(source: &str) -> Result<Vec<Token>, String> {
    create_q_lexer(source).tokenize().map_err(|e| e.to_string())
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use lexer::token::TokenType;

    fn types(source: &str) -> Vec<String> {
        tokenize_q(source)
            .into_iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| t.effective_type_name().to_string())
            .collect()
    }

    fn values(source: &str) -> Vec<String> {
        tokenize_q(source)
            .into_iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| t.value)
            .collect()
    }

    // These are the same disambiguation cases documented above, exercised
    // directly against the hooks; the crate's `tests/test_tokenizer.rs`
    // covers the full grammar plus a much larger set of edge cases.

    #[test]
    fn space_before_minus_only_folds_to_negative_literal() {
        assert_eq!(types("2 -1"), vec!["NUMBER", "NUMBER"]);
        assert_eq!(values("2 -1"), vec!["2", "-1"]);
    }

    #[test]
    fn space_on_both_sides_of_minus_stays_subtraction() {
        assert_eq!(types("2 - 1"), vec!["NUMBER", "MINUS", "NUMBER"]);
    }

    #[test]
    fn no_space_at_all_stays_subtraction() {
        assert_eq!(types("2-1"), vec!["NUMBER", "MINUS", "NUMBER"]);
    }

    #[test]
    fn comment_preceded_by_space_is_stripped() {
        assert_eq!(types("1 / a comment"), vec!["NUMBER"]);
    }

    #[test]
    fn reduce_glued_to_a_verb_is_not_a_comment() {
        assert_eq!(types("+/x"), vec!["PLUS", "REDUCE", "NAME"]);
    }
}
