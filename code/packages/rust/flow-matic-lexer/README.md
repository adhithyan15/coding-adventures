# flow-matic-lexer (coding-adventures-flow-matic-lexer)

Lexer for **FLOW-MATIC** (originally *B-0*, built by Grace Hopper's team at
Remington Rand for the UNIVAC I, 1955–1959) — the first English-like
data-processing language and the direct ancestor of COBOL. It loads the compiled
`flow_matic.tokens` grammar (`code/grammars/flow_matic/flow_matic.tokens`) and
feeds it to the generic `lexer::GrammarLexer`; no tokenization is hand-written.

Implements the lexical layer of [PL06](../../../specs/PL06-flow-matic.md).

## API

```rust
use coding_adventures_flow_matic_lexer::{tokenize_flow_matic, try_tokenize_flow_matic};

let tokens = tokenize_flow_matic("(1) COMPARE PRODUCT-NO (A) WITH PRODUCT-NO (B) .");
```

## What it tokenizes

- **Unsigned integers** (`NUMBER`) — operation labels and branch targets.
- **Hyphenated data names** (`NAME`) — `PRODUCT-NO`, `UNIT-PRICE`, `FILE-A`, the
  COBOL-family trait. Case is insignificant for matching (so lowercase input
  works too), but a NAME's value preserves the case it was typed in — canonical
  all-caps source keeps `PRODUCT-NO` intact.
- **English verbs / clause words** as `KEYWORD` (uppercased): `COMPARE`, `WITH`,
  `IF`, `GREATER`, `WRITE-ITEM`, `GO`, `TO`, `OPERATION`, … The hyphenated verbs
  (`WRITE-ITEM`, `READ-ITEM`, `CLOSE-OUT`) ride the keyword-promotion step.
- **Punctuation** — `(` `)` for operation labels and field qualifiers, `.` to
  end an operation, `;` to separate clauses.

## Why no hooks

Unlike COBOL, whose fixed 80-column card layout needs a `pre_tokenize`
column-strip hook, FLOW-MATIC listings are free-form: whitespace (including
newlines) separates tokens and a period ends each operation. So this crate is a
pure grammar wrapper with **no** pre/post-tokenize hooks — the simplest kind of
frontend in the repo.

## Regenerating the grammar

`src/_grammar.rs` is generated from `code/grammars/flow_matic/flow_matic.tokens`:

```
cargo run --release --manifest-path code/programs/rust/grammar-tools/Cargo.toml \
  -- generate-rust-compiled-grammars flow_matic
```
