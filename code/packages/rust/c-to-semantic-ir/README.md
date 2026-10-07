# c-to-semantic-ir

C (integer-core subset) → **Semantic IR** — the mirror of the Ruby/Python
frontends for a *strict, typed* source language, and the last piece of the
C→SIR→Ruby initiative.  It parses C with `coding-adventures-c-parser`, then
walks the CST assigning a concrete `IntSpec` to every expression and inserting
`Expr::Convert` nodes per C's integer promotions / usual-arithmetic-conversions
— so a C program's width/wrap/truncate semantics survive the narrow waist and
every backend reproduces its results.

Implements [SIR27](../../../specs/SIR27-c-to-semantic-ir.md).

The PREP01 C adapter is being built in stages. `dialect::CDialect` can classify
the core directive shapes and run object and function-like macros, local
includes, `#ifdef`, `defined`, and bounded decimal comparisons in `#if` through
the generic engine. `compile_source` has not yet been switched to that token
stream; full C `#if` expressions, stringize, and paste remain pending. The
existing C source behavior is unchanged at this stage.

## API

The PREP01 C directive adapter is currently a staged component. The public
`compile_source` API below still uses the legacy C source parser path; it does
not run the generic preprocessor yet.

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
