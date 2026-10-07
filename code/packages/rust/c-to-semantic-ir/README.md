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
includes, `#ifdef`, `defined`, bounded decimal comparisons, one-operator
arithmetic and bounded logical conditions in `#if` through the generic engine.
`compile_preprocessed_file` uses that token stream with declared include roots
before parsing and lowering.
The pathless `compile_source` API retains its legacy behavior. Full C `#if`
expressions, stringize, paste and default frontend routing remain pending.

## API

For an on-disk translation unit, pass a relative entry name and trusted include
roots. The entry spelling is bounded before cloning; `RootedFs` resolves and
reads files under the tightened preprocessor bounds. The result is parsed from
the preprocessed token stream, without source reconstruction or re-lexing.

```rust
use c_to_semantic_ir::compile_preprocessed_file;
use coding_adventures_source_preprocessor::Bounds;
let module = compile_preprocessed_file(
    "main.c", [std::path::PathBuf::from("src")], "demo", Bounds::default()
).unwrap();
```

Quoted includes search beside the verified including file, then under the
declared roots. The primary file and system includes search declared roots
only. Every resolved candidate must remain inside a declared root; an
unresolved include fails explicitly.
`#if` currently handles single decimal comparisons, one checked `+`, `-`, or
`*` within signed 32-bit range, `!` on one operand and `&&`/`||` chains. It
rejects unsupported C expressions. Stringize and paste in macro bodies also
fail explicitly.

The pathless `compile_source` API below still uses the legacy C source parser
path and does not run the generic preprocessor.

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
