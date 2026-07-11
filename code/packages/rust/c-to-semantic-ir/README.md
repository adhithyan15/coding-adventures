# c-to-semantic-ir

C (integer-core subset) → **Semantic IR** — the mirror of the Ruby/Python
frontends for a *strict, typed* source language, and the last piece of the
C→SIR→Ruby initiative.  It parses C with `coding-adventures-c-parser`, then
walks the CST assigning a concrete `IntSpec` to every expression and inserting
`Expr::Convert` nodes per C's integer promotions / usual-arithmetic-conversions
— so a C program's width/wrap/truncate semantics survive the narrow waist and
every backend reproduces its results.

Implements [SIR27](../../../specs/SIR27-c-to-semantic-ir.md).

## API

```rust
use c_to_semantic_ir::compile_source;
let module = compile_source(
    "int main(void) { uint8_t c = 200 + 100; printf(\"%d\n\", c); return 0; }",
    "demo",
).unwrap();
```

## Milestone 1

Functions (with typed params), local declarations & assignments, typed `+`/`-`/
`*` arithmetic with `Convert`-per-operation, `(T)e` casts, `printf`, and
`return`.  Verified end-to-end: the emitted **Ruby** (via `ruby`) and **C** (via
`cc`) agree byte-for-byte with the C source's semantics — `(uint8_t)(200+100)==44`,
`(int32_t)(2e9+2e9)==-294967296`.  Control flow, comparisons (C-vs-SIR
truthiness), `/`/`%`/bitwise, pointers/structs are later milestones.
