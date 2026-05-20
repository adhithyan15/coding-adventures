# `oct-parser` (Rust port — OCT02 phase 1)

Parses [Oct](../../../specs/OCT00-oct-language.md) source text into a
grammar AST using the generic [`GrammarParser`](../parser) over an
auto-generated parser grammar (`src/_grammar.rs`, compiled from
[`code/grammars/oct.grammar`](../../../grammars/oct.grammar) via
`grammar-tools`).

## Usage

```rust
use coding_adventures_oct_parser::parse_oct;

let ast = parse_oct("fn main() { let x: u8 = 5; }").unwrap();
assert_eq!(ast.rule_name, "program");
```

## What's in the AST

The parser produces a `GrammarASTNode` tree rooted at `program`.
Subsequent OCT02 phases consume this tree:

- **Phase 2** — `oct-type-checker`: validate types, detect register
  exhaustion, count locals, etc.
- **Phase 3** — `oct-iir-compiler`: lower to `interpreter_ir::IIRModule`.
- **Phase 4** — `lang-aot` wiring + end-to-end smoke test.

## Spec

[OCT02](../../../specs/OCT02-oct-rust-frontend.md) — Oct Rust frontend for the
LANG VM AOT chain.
