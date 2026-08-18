# derive-iir-compiler

Derive CST → `interpreter_ir::IIRModule`, **v0.1.0**.

The second real-language bridge from a math language in this repo onto
`interpreter_ir` (IIR) — the shared IR the AOT/lang-vm chain lowers to 7
real backends (NativeAOT, LLVM, WASM, JVM, CLR, VM interpreter, JIT) —
rather than only the Semantic IR source-to-source pipeline
[`derive-to-semantic-ir`](../derive-to-semantic-ir) already targets. See
[`derive-iir-vm.md`](../../../specs/derive-iir-vm.md) for the
per-language deltas from
[`macsyma-iir-vm.md`](../../../specs/macsyma-iir-vm.md)'s original
design.

It consumes the same generic `GrammarASTNode` CST
[`coding-adventures-derive-parser`](../derive-parser) already produces —
the third independent consumer of that CST, after `derive-runtime` (the
REPL evaluator's own lowering) and `derive-to-semantic-ir`.

## Scope (v0.1.0)

**Accepted:** integer literals; `+ - * /` (binary chains and unary `-`
only — Derive's grammar has no unary-plus at all); assignment (`x :=
expr`, plain-name target only); free-symbol references; any other
operator/head — or any operand involving a free symbol — represented as
an *unevaluated* symbolic expression (an inert `cons`-chain, matching how
`derive-runtime` itself leaves e.g. `x + y` unevaluated when `x`/`y` are
unbound).

**Rejected, with an explicit error, never a silent mis-lowering:** `Float`
literals; `F(x) := body` (function definition — Derive has only one
`:=` token for both plain assignment and definition, disambiguated at
lowering time by the LHS's shape); comparisons; `AND`/`OR`/`NOT`; `^`
(power); `[...]`/`[...;...]` (vector/matrix literals); any postfix
function call `F(x)`.

See `src/lower.rs`'s module doc comment for exactly why comparisons/
`AND`/`OR`/`^` are rejected outright (same rationale as
`macsyma-iir-compiler`: `derive-runtime`'s real evaluator numerically
evaluates a concrete instance of these, so inert data would disagree with
ground truth) and for the `/` exactness rule.

## Usage

```rust
use derive_iir_compiler::compile_source;

let module = compile_source("2 + 3\n", "demo")?;
let value = derive_vm::run(&module)?;
assert_eq!(value.as_int(), Some(5));
```

## Verification

`cargo test -p derive-iir-compiler` covers every accepted construct (via
`derive-vm`, as a dev-dependency) and every rejected construct's
explicit-error path. `tests/oracle.rs` additionally cross-checks every
accepted construct against `derive-runtime`'s own evaluator, both sides
rendered through the same `print_derive` function.
