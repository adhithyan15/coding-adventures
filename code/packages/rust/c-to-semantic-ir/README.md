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
It also accepts `#elif` with that same bounded condition subset and rejects a
second `#else` or `#elif` after `#else`.
`compile_preprocessed_file` uses that token stream with declared include roots
before parsing and lowering. Pathless `compile_source` uses the same bounded
preprocessor with an in-memory primary source. Every active include fails
closed because that API has no include roots. Full C `#if` expressions,
stringize, and paste remain pending.

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
`#if` currently handles single decimal comparisons, one checked `+`, `-`,
`*`, `/`, `%`, `<<`, `>>`, `&`, `|`, or `^` within the bounded signed 32-bit subset, `!` on
one operand and `&&`/`||` chains. Every clause must match the bounded grammar
and operand ranges after expansion. Logical `&&` and `||` skip value computation
once the result is determined, so an unneeded zero divisor, arithmetic overflow,
or invalid shift does not fail the directive. Those operations still fail when
their clause is needed. Longer or mixed arithmetic and other unsupported C
expressions fail explicitly. Shift
counts must be 0–31, the left operand must be nonnegative, and left-shift
results must fit signed 32-bit.
Bitwise conditions accept only nonnegative signed 32-bit operands; longer or
mixed expressions remain unsupported.
One outer parenthesis pair may surround a single operand, negated operand,
or comparison clause. Nested parentheses and grouping a logical chain or
arithmetic, shift, or bitwise operation remain unsupported.
One `!(left comparison right)` clause is also accepted. Its two operands and
comparison operator use the existing bounded rules; nested parentheses and
negated arithmetic or logical chains remain unsupported.
One `!(operand)` clause is accepted for a plain-decimal literal or undefined
identifier after macro expansion or `defined()` preparation. It preserves
the earlier negated-comparison form and rejects nested, arithmetic, and
longer mixed expressions.
Stringize and paste in macro bodies also fail explicitly.
`#undef NAME` removes the current object-like or function-like macro by its
unexpanded name. It is inert in a skipped conditional group; malformed
operands fail with the directive location on the rooted file-input path.

The pathless `compile_source` API below preprocesses directives before the
token-input C parser. It supports the bounded directive forms above but cannot
read host files or resolve active includes. Use `compile_preprocessed_file`
when the translation unit needs headers.
The three-way conformance harness keeps standard headers for its native C
oracle and passes the same program body without those headers to this pathless
API; fixed-width type names and `printf` are part of this frontend's bounded
subset.

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
