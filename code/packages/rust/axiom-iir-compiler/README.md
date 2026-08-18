# axiom-iir-compiler

Axiom CST → `interpreter_ir::IIRModule`, **v0.1.0**.

The fifth real-language bridge from a math language in this repo onto
`interpreter_ir` (IIR). See
[`axiom-iir-vm.md`](../../../specs/axiom-iir-vm.md) for the per-language
deltas from
[`macsyma-iir-vm.md`](../../../specs/macsyma-iir-vm.md)'s original
design.

**`program` is a SINGLE expression** — `axiom.grammar`'s own `program =
expr` (Axiom is modeled as a numbered, per-line interactive session),
unlike every sibling frontend's multi-statement worksheet loop. A
consequence: this crate has no `env`/binding-threading at all — a bound
variable can never be referenced after its own `x := e` statement, since
there is no second statement in the same compiled module.

## Scope (v0.1.0)

**Accepted:** integer literals; `+ - * /` (binary chains and unary `-`
only); assignment (`x := expr`, always a bare NAME target by grammar
construction — nothing to disambiguate); free-symbol references; any
other operator/head — or any operand involving a free symbol —
represented as an *unevaluated* symbolic expression.

**Rejected, with an explicit error:** `Float`/`String` literals; function
definitions; `a : T` (declaration), `e :: T` (coercion), `D has C`
(category-membership query) — Axiom's own genuinely new territory, with
no arithmetic analogue; `if`/`then`/`else`; comparisons; `^` (power);
`[...]` list literals; any postfix function call `f(x)`.

## Usage

```rust
use axiom_iir_compiler::compile_source;

let module = compile_source("2 + 3", "demo")?;
let value = axiom_vm::run(&module)?;
assert_eq!(value.as_int(), Some(5));
```

## Verification

`cargo test -p axiom-iir-compiler` covers every accepted construct (via
`axiom-vm`, as a dev-dependency) and every rejected construct's
explicit-error path. `tests/oracle.rs` additionally cross-checks every
accepted construct against `axiom-runtime`'s own evaluator, both sides
rendered through the same `print_axiom` function.
