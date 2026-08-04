# coding-adventures-q-lexer

Q (kdb+'s scripting language) tokenizer backed by
`code/grammars/q/q.tokens`, compiled to Rust and statically linked into the
crate.

The runtime path does not read grammar files from disk, which keeps it
suitable for a future WASM facade.

## Scope

Covers the historical-core subset fixed by
[MA11 §4](../../../specs/MA11-q-language.md): dense numeric arrays, the
primitive verb glyphs `+ - * % ! , # _ & | ~`, the six comparison glyphs
`= < > <= >= <>`, the three adverbs `'` (each) `/` (over/reduce) `\` (scan),
`name:expr` assignment, parenthesised grouping, explicit `(a;b;c)` list
literals, `{[x;y] ...}` function-literal delimiters, and `/`-to-end-of-line
comments.

This crate only tokenizes. There is no `q-parser`/`q.grammar` here (that is
a separate follow-on task, MA-11c) and no recursion-depth cap (that is a
parser-level concern for MA-11c, the same split `apl-lexer`/`j-lexer` vs.
`apl-parser`/`j-parser` already establish in this repo).

## Design decision: two rules are whitespace-sensitive, and live in Rust

Every other lexer in this repo's array-language family (`apl-lexer`,
`j-lexer`, `scilab-lexer`, ...) treats whitespace as pure separator noise:
whether two tokens touch or not never changes which token either one is.
Q's real lexer breaks that assumption in exactly two places
([MA11 §3 bullet 2](../../../specs/MA11-q-language.md)):

1. **Negative-literal vs. subtraction.** Q spells a negative number with an
   ordinary leading `-` (unlike J's leading-underscore `_5`), and
   disambiguates purely by adjacent whitespace: `2 -1` (space before `-`,
   none after) tokenizes as the two-element strand `NUMBER(2) NUMBER(-1)`;
   `2 - 1` and `2-1` both tokenize as `NUMBER(2) MINUS NUMBER(1)` (ordinary
   subtraction).
2. **`/` comment marker vs. REDUCE adverb.** A `/` preceded by whitespace
   (or starting a line) opens a comment to end of line; a `/` glued
   directly to a preceding verb/noun with no space is the REDUCE adverb
   (`+/x`).

Rust's `regex` crate — what the shared
[`GrammarLexer`](../lexer/src/grammar_lexer.rs) compiles every `.tokens`
pattern with — has **no lookaround support** (no `(?<=...)`, no `(?=...)`),
so "is there whitespace immediately before this character" cannot be
expressed as a token pattern, full stop. This was confirmed by reading
`grammar-tools`' own token-grammar validator, which compiles every pattern
with a plain `regex::Regex::new` call.

Rather than inventing a new mechanism, this crate reuses a strategy already
established in this repo: `scilab-lexer` resolves an analogous
same-character-two-meanings problem (`'` is transpose or a string delimiter
depending on whether a value immediately precedes it) with a
[**pre-tokenize hook**](../lexer/src/grammar_lexer.rs) (rewrites the raw
source text before `GrammarLexer` runs) and a **post-tokenize hook**
(patches the resulting token list afterward). `q-lexer` uses the exact same
two extension points — `GrammarLexer::add_pre_tokenize` /
`add_post_tokenize` — for its own two, independently-implemented
disambiguations:

- **`strip_slash_comments`** (pre-tokenize): blanks `/`-to-end-of-line
  comment text into spaces before the grammar ever sees it. This has to run
  *before* tokenization, not after, because comment text is arbitrary and
  need not lex successfully as Q code — a post-tokenize pass would require
  the whole file to already lex cleanly, including inside comments, which
  is not a safe assumption.
- **`fold_negative_number_literals`** (post-tokenize): merges a `MINUS`
  token immediately followed by a `NUMBER` token into one signed `NUMBER`
  token, but only when the previously emitted token does not "complete a
  noun" glued directly against the `-` with zero gap (see the doc comment
  on `fold_negative_number_literals` in `src/lib.rs` for the exact rule and
  a worked table of examples). This needs to run *after* tokenization
  because the decision depends on the *type* of the previous token
  (NUMBER/NAME/closing bracket vs. anything else), which only the token
  stream — not raw characters — carries.

`q.tokens` itself documents this split prominently in its own header
comment, and states explicitly that no other rule in this crate needs
special-casing: every other token (every primitive verb, every adverb,
assignment, grouping, function-literal delimiters) is declarative,
first-match-wins, exactly like every sibling `.tokens` file in this repo.

While researching this, no genuine bug was found in the shared
`GrammarLexer` engine — but one real limitation was confirmed and is
recorded in `src/lib.rs`'s own doc comment: the engine's declarative
mode-transition table (F10) only fires *after* a token is emitted, keyed on
that token's type/value, with no way to see "was trivia consumed
immediately before this token" — so it cannot express either of Q's two
rules declaratively today. That is a shared-engine gap, not a Q-specific
one, and is left unfixed here per this repo's "fix what's local, defer what's
shared" convention.

## The one other thing to get right: `<>` is not-equal, not `~=` or `#`

[MA11 §4](../../../specs/MA11-q-language.md) calls this out explicitly: Q
spells not-equal `<>` — never MATLAB/Scilab's `~=` spelling, and never `#`
(which in Q is the unrelated count/take primitive). A frontend built by
pattern-matching against a sibling array-language grammar file, rather than
reading MA11 §4's own primitive-verb table directly, is the easiest way to
get this specific glyph backwards.

## Usage

```rust
use coding_adventures_q_lexer::tokenize_q;

let tokens = tokenize_q("x:2 -1\nsum:{[a] +/a}\nsum x");
```

`tokenize_q` panics on a malformed source string; use `create_q_lexer`
directly (or `try_tokenize_q`) if you need the `Result`-returning form
instead.

## Where this fits

`q-lexer` is the first of Q's frontend crates
([MA-11b](../../../specs/MA11-q-language.md#6-crate-layout-and-rollout-one-item--one-pr)),
following MA-11a's design spec. The sibling `q-parser` crate (MA-11c) will
consume this crate's token stream against `code/grammars/q/q.grammar` —
including the one genuinely new grammar production this language needs,
user-defined function literals (MA11 §2/§3) — to build the `GrammarASTNode`
CST that a future `q-runtime` (MA-11d) will evaluate, alongside `q-repl`
and `q-to-semantic-ir` (MA-11e), per
[HML00](../../../specs/HML00-historical-math-languages-roadmap.md) Wave 6.
