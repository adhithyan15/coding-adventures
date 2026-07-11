# coding-adventures-apl-lexer

APL tokenizer backed by `code/grammars/apl/apl.tokens`, compiled to Rust and
statically linked into the crate.

The runtime path does not read grammar files from disk, which keeps it
suitable for a future WASM facade.

## Scope

Covers the historical-core subset fixed by
[MA05 §4](../../../specs/MA05-apl-language.md): dense numeric arrays, the
primitive functions `+ - × ÷ ⌈ ⌊ ⍴ ⍳ , = ≠ < ≤ ≥ >`, the operators `/`
(reduce), `\` (scan), and `∘.` (outer product), assignment `←`, parenthesised
grouping, and `⍝` line comments.

Every APL primitive in this subset is a single dedicated Unicode code point
(see MA05 §3 bullet 4), so — unlike MATLAB's `'` or Wolfram's bracketed
newlines — there is no character-overloading disambiguation to do at the
lexer level. Which of a glyph's two readings (monadic/dyadic) applies is
decided by the *parser* production that matches it, not by this crate.

## Usage

```rust
use coding_adventures_apl_lexer::tokenize_apl;

let tokens = tokenize_apl("A←⍳5\nB←+/A");
```

`tokenize_apl` panics on a malformed source string (there is no recoverable
lexer error to report to a caller yet); use `create_apl_lexer` directly if
you need the `Result`-returning `GrammarLexer::tokenize` instead.

## Where this fits

`apl-lexer` is the first of the two frontend crates for APL (MA-4c); the
sibling `apl-parser` crate (MA-4d) consumes this crate's token stream against
`code/grammars/apl/apl.grammar` to build the `GrammarASTNode` CST that a
future `apl-runtime` will evaluate.
