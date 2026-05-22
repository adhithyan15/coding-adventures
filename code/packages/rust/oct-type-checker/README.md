# `oct-type-checker`

Rust type checker for the Oct language (OCT02 phase 2).  Reads the AST
produced by `coding-adventures-oct-parser` and verifies Oct's
language-level type invariants.

## Usage

```rust
use oct_type_checker::check_source;

let result = check_source("fn main() { let x: u8 = 42; }");
assert!(result.ok);

let bad = check_source("fn main() { let x: u8 = 1; if x { } }");
assert!(!bad.ok);
for err in &bad.errors {
    eprintln!("{}:{}: {}", err.line, err.column, err.message);
}
```

## Where it fits in the pipeline

```
Oct source
   │
   ▼ oct-lexer
tokens
   │
   ▼ oct-parser
AST  ← walked by THIS CRATE
   │
   ▼ oct-iir-compiler (OCT02 phase 3)
IIRModule
   │
   ▼ lang-aot
native executable
```

## V1 scope

- Two Oct types: `u8` (0..=255) and `bool`.
- `bool` coerces implicitly to `u8`; the reverse direction is rejected.
- `if` / `while` conditions must be `bool`.
- Static and function signatures collected in pass 1, so calls work
  before the callee appears in source order.
- 8008 intrinsics (`in`, `out`, `adc`, …) pass through type-check with
  a best-effort return type; the iir-compiler rejects them.

See [code/specs/OCT02-oct-rust-frontend.md](../../../specs/OCT02-oct-rust-frontend.md).
