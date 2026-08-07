# coding-adventures-j-lexer

J tokenizer backed by `code/grammars/j/j.tokens`, compiled to Rust and
statically linked into the crate.

The runtime path does not read grammar files from disk, which keeps it
suitable for a future WASM facade.

## Scope

Covers the historical-core subset fixed by
[MA06 §4](../../../specs/MA06-j-language.md): dense numeric arrays, the
primitive verbs `+ - * % ^ <. >. $ i. , #`, the six comparison glyphs
`= ~: < > <: >:`, the two adverbs `/` (reduce) and `\` (scan), the one
conjunction `@` (compose/atop), assignment `=.`/`=:`, parenthesised
grouping, and `NB.` line comments.

## The one thing to get right: `/` is not division

APL spells division with its own dedicated glyph (`÷`), so J — which needs
`/` for the reduce adverb it inherits unchanged from APL — moves division
to `%` instead. A frontend built by transliterating APL's glyphs one-for-one
would get this specific primitive backwards; per
[MA06 §1 bullet 1](../../../specs/MA06-j-language.md), it is the single most
common APL-to-J transliteration mistake, and this crate's test suite has a
dedicated regression test for it
(`slash_is_reduce_not_divide_and_percent_is_divide`).

## ASCII digraphs need lexer lookahead, unlike APL's single code points

Every APL primitive in this repo's MA05 cut is one dedicated Unicode code
point, so `apl-lexer` never needs to look more than one character ahead. J
is ASCII-only, so an overloaded base character needs an explicit `.`- or
`:`-suffixed digraph to spell a related but distinct primitive — e.g. `<`
(less than) vs. `<.` (floor/min) vs. `<:` (less-or-equal). Since this
repo's `GrammarLexer` matches the *first* pattern (in declaration order)
that matches at the current position, not the longest, `j.tokens` declares
every digraph ahead of the bare single-character token it could otherwise
be swallowed by (see that file's own SECTION 1 for the full ordering, and
the `digraphs_are_not_swallowed_by_their_single_character_prefix` test
below for the regression coverage).

Negative number literals use a leading underscore (`_5`), matching J's own
historical convention, rather than APL's high-minus `¯` — MA06 §4 doesn't
spell out a literal syntax, so this crate makes that call itself and
documents it in `j.tokens`'s own header. A bare `_`/`__` (J's real spelling
for infinity) is out of scope for this cut and is structurally excluded:
the `NUMBER` pattern requires at least one digit after the underscore.

Which of a glyph's two readings (monadic vs. dyadic) applies is a
parser-production concern, not a lexer one, exactly as MA05 §3 bullet 3
already established for APL — unchanged here.

## Usage

```rust
use coding_adventures_j_lexer::tokenize_j;

let tokens = tokenize_j("A=.i.5\nB=.+/A");
```

`tokenize_j` panics on a malformed source string (there is no recoverable
lexer error to report to a caller yet); use `create_j_lexer` directly if
you need the `Result`-returning `GrammarLexer::tokenize` instead.

## Where this fits

`j-lexer` is the first of J's frontend crates (MA-6b); the sibling
`j-parser` crate (MA-6c) will consume this crate's token stream against
`code/grammars/j/j.grammar` — including the one genuinely new grammar
production this language needs, tacit hook/fork trains (MA06 §3) — to build
the `GrammarASTNode` CST a future `j-runtime` will evaluate.
