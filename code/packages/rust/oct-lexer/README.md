# `oct-lexer` (Rust port — OCT02 phase 1)

Tokenizes [Oct](../../../specs/OCT00-oct-language.md) source text using the
grammar-driven Rust lexer.  Thin wrapper around the generic
[`GrammarLexer`](../lexer) over an auto-generated token grammar
(`src/_grammar.rs`, compiled from
[`code/grammars/oct.tokens`](../../../grammars/oct.tokens) via
`grammar-tools`).

## Why a Rust port?

Oct already has a complete Python frontend (`code/packages/python/oct-*`).
OCT02 phase 1 brings the **lexer and parser** to Rust so the LANG VM
AOT chain — which is Rust-only — can compile `.oct` files in V2.  The
type-checker and IIR compiler follow in subsequent OCT02 phases.

## Usage

```rust
use coding_adventures_oct_lexer::tokenize_oct;

let tokens = tokenize_oct("fn main() { let x: u8 = 5; }");
assert!(tokens.iter().any(|t| t.value == "fn"));
```

## Spec

[OCT02](../../../specs/OCT02-oct-rust-frontend.md) — Oct Rust frontend for the
LANG VM AOT chain.
