# Changelog — `coding-adventures-oct-lexer`

## 0.1.0 — 2026-05-20 (OCT02 phase 1)

Initial Rust port of the Oct lexer.  Thin wrapper around the
grammar-driven `GrammarLexer` over the embedded `oct.tokens` grammar.
First step of OCT02 — bringing Oct's frontend into Rust so the LANG VM
AOT chain can compile `.oct` files in V2 phases (3 + 4).

The Python `oct-lexer` package continues to exist for the Intel-8008
simulator backend and other Python consumers.

### Tests

5 unit tests covering:

- Simple function tokenization (`fn main() { let x: u8 = 5; }`).
- `KEYWORD` token type-name promotion (so the grammar's `"fn"` /
  `"let"` literal rules match by type name).
- 8008 intrinsic tokens (`out`, `in`, `carry`, …).
- Arithmetic and relop tokens (`+`, `-`, `==`).
- Loop / break keyword tokens.
