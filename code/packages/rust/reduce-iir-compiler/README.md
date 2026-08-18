# reduce-iir-compiler

Reduce CST → `interpreter_ir::IIRModule`, **v0.1.0**.

The third real-language bridge from a math language in this repo onto
`interpreter_ir` (IIR) — the shared IR the AOT/lang-vm chain lowers to 7
real backends (NativeAOT, LLVM, WASM, JVM, CLR, VM interpreter, JIT) —
rather than only the Semantic IR source-to-source pipeline
[`reduce-to-semantic-ir`](../reduce-to-semantic-ir) already targets. See
[`reduce-iir-vm.md`](../../../specs/reduce-iir-vm.md) for the
per-language deltas from
[`macsyma-iir-vm.md`](../../../specs/macsyma-iir-vm.md)'s original
design.

## Scope (v0.1.0)

**Accepted:** integer literals; `+ - * /` (binary chains and unary `-`
only — Reduce has no unary-plus); assignment (`x := expr`, plain-name
target only); free-symbol references; any other operator/head — or any
operand involving a free symbol — represented as an *unevaluated*
symbolic expression (an inert `cons`-chain).

**Rejected, with an explicit error, never a silent mis-lowering:** `Float`
literals; `h(l, m) := body` (procedure definition); `if`/`then`/`else`
(Reduce, unlike Derive, has a dedicated expression-shaped `if`);
`<< s1; s2; ... >>` group statements; comparisons (`=`/`neq`/`<`/`>`/
`<=`/`>=`); `and`/`or`/`not`; `^`/`**` (power); `a . b` cons; `{...}`
list literals; any postfix function call/array subscript `f(x)`.

See `src/lower.rs`'s module doc comment for the full rationale, matching
`macsyma-iir-compiler`'s and `derive-iir-compiler`'s identical design.

## Usage

```rust
use reduce_iir_compiler::compile_source;

let module = compile_source("2 + 3;\n", "demo")?;
let value = reduce_vm::run(&module)?;
assert_eq!(value.as_int(), Some(5));
```

## Verification

`cargo test -p reduce-iir-compiler` covers every accepted construct (via
`reduce-vm`, as a dev-dependency) and every rejected construct's
explicit-error path. `tests/oracle.rs` additionally cross-checks every
accepted construct against `reduce-runtime`'s own evaluator, both sides
rendered through the same `print_reduce` function.
