# maple-iir-compiler

Maple CST → `interpreter_ir::IIRModule`, **v0.1.0**.

The fourth real-language bridge from a math language in this repo onto
`interpreter_ir` (IIR). See
[`maple-iir-vm.md`](../../../specs/maple-iir-vm.md) for the per-language
deltas from
[`macsyma-iir-vm.md`](../../../specs/macsyma-iir-vm.md)'s original
design.

## Scope (v0.1.0)

**Accepted:** integer literals; `+ - * /` (binary chains and unary `-`
only); assignment (`x := expr`); free-symbol references; any other
operator/head — or any operand involving a free symbol — represented as
an *unevaluated* symbolic expression.

**Rejected, with an explicit error:** `Float` literals; `true`/`false`
boolean literal keywords; `f := x -> body` (arrow function definition);
`if`/`then`/`elif`/`else`/`end if`; comparisons; `and`/`or`/`not`; `^`
(power); `[...]` list literals; `{...}` set literals; any postfix
function call `f(x)`.

Unlike Derive/Reduce, Maple's grammar makes `f(x) := body` a genuine
*parse error* — the assignment LHS is always a bare `NAME` by
construction, so this crate never needs Derive's/Reduce's bare-name
disambiguation check. See `src/lower.rs`'s module doc comment for the
full rationale.

## Usage

```rust
use maple_iir_compiler::compile_source;

let module = compile_source("2 + 3;\n", "demo")?;
let value = maple_vm::run(&module)?;
assert_eq!(value.as_int(), Some(5));
```

## Verification

`cargo test -p maple-iir-compiler` covers every accepted construct (via
`maple-vm`, as a dev-dependency) and every rejected construct's
explicit-error path. `tests/oracle.rs` additionally cross-checks every
accepted construct against `maple-runtime`'s own evaluator, both sides
rendered through the same `print_maple` function.
