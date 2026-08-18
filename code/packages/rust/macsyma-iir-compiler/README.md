# macsyma-iir-compiler

Macsyma CST → `interpreter_ir::IIRModule`, **v0.1.0**.

The first bridge from a math language in this repo onto `interpreter_ir`
(IIR) — the shared IR the AOT/lang-vm chain lowers to 7 real backends
(NativeAOT, LLVM, WASM, JVM, CLR, VM interpreter, JIT) — rather than only
the Semantic IR source-to-source pipeline
[`macsyma-to-semantic-ir`](../macsyma-to-semantic-ir) already targets. See
[`macsyma-iir-vm.md`](../../../specs/macsyma-iir-vm.md) for the full
design.

It consumes the same generic `GrammarASTNode` CST
[`coding-adventures-macsyma-parser`](../macsyma-parser) already produces —
this is the **third** independent consumer of that CST, after
`macsyma-compiler` (the REPL evaluator's own lowering, to
`symbolic_ir::IRNode`) and `macsyma-to-semantic-ir` (to `semantic_ir::Expr`
for JS/TS/etc. codegen). All three share no code beyond the parser.

## Scope (v0.1.0)

**Accepted:** integer literals; `+ - * /` (binary chains and unary
`-`/`+`); assignment (`x: expr`, plain-name target only); free-symbol
references; any other operator/head — or any operand involving a free
symbol — represented as an *unevaluated* symbolic expression (an inert
`cons`-chain, matching how `macsyma-runtime` itself leaves e.g. `x + y`
unevaluated when `x`/`y` are unbound).

**Rejected, with an explicit error, never a silent mis-lowering:**
`Rational`/`Float`/`Str` literals; `:=` (function definition);
`if`/`while`/`for`/`block`/`return`; `[...]` (list literals); comparisons;
`and`/`or`/`not`; `^`/`**` (power); any postfix function call `f(x)`.

See `src/lower.rs`'s module doc comment for exactly why comparisons/
`and`/`or`/`^` are rejected outright rather than given the same
inert-data fallback `+`/`-`/`*`/`/` get (short version: `macsyma-runtime`'s
real evaluator *does* numerically evaluate a concrete `2^3` or `3<5`,
unlike `+`/`-`/`*`, so building inert data for those would silently
disagree with ground truth) — and for the `/` **exactness rule** (why
`7/2` and `x:6$ x/2` are both rejected, while `20/4` and `-4/2` are
accepted).

## Usage

```rust
use macsyma_iir_compiler::compile_source;

let module = compile_source("2 + 3$", "demo")?;
let value = macsyma_vm::run(&module)?;
assert_eq!(value.as_int(), Some(5));
```

## Verification

`cargo test -p macsyma-iir-compiler` covers every accepted construct (via
`macsyma-vm`, as a dev-dependency) and every rejected construct's
explicit-error path. A cross-checked oracle test against
`macsyma-runtime`'s own evaluator is a follow-up (see the spec's PR
sequencing, §8).
