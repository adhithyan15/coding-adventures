# wolfram-iir-compiler

Wolfram CST → `interpreter_ir::IIRModule`, **v0.1.0**.

The sixth and final real-language bridge in this rollout (after Macsyma,
Derive, Reduce, Maple, and Axiom) from a math language onto
`interpreter_ir` (IIR) — closing out Wave 5. See
[`wolfram-iir-vm.md`](../../../specs/wolfram-iir-vm.md) for the
per-language deltas from
[`macsyma-iir-vm.md`](../../../specs/macsyma-iir-vm.md)'s original
design.

**Deliberately narrower than [`wolfram-to-semantic-ir`](../wolfram-to-semantic-ir)**,
which covers Wolfram's *full* grammar (pattern blanks, replacement
rules, pure functions, `/@`/`@@` sugar) since SIR23's "everything is
data" design has no scope pressure forcing a narrower cut there. This
crate holds v0's arithmetic/assignment/unevaluated-Apply scope constant
across all six Wave 5 languages instead.

## Scope (v0.1.0)

**Accepted:** integer literals; `+ - * /` (binary chains) and **both**
unary `-`/`+` (Wolfram, unlike Derive/Reduce/Maple/Axiom, has a real
unary-plus no-op); assignment (`x = expr`, `SET` only, always a bare
NAME target — `SETDELAYED`/`:=` is rejected outright); free-symbol
references; any other operator/head — or any operand involving a free
symbol — represented as an *unevaluated* symbolic expression.

**Rejected, with an explicit error:** `Float`/`String` literals; pattern
blanks (`_`/`_h`), rules (`->`/`:>`), replacement (`/.`/`//.`),
conditions (`/;`), alternatives (`|`), pattern tests (`?`) — Wolfram's
own genuinely new territory, with no arithmetic analogue; comparisons;
logic (`&&`/`||`/`!`); `^` (power); pure functions (`#`/`#n`/`##`/`expr
&`); `/@`/`@@` (map/apply sugar); `{...}` list literals; any postfix
suffix at all (`f[x]`, `x[[i]]`, …).

## Usage

```rust
use wolfram_iir_compiler::compile_source;

let module = compile_source("2 + 3\n", "demo")?;
let value = wolfram_vm::run(&module)?;
assert_eq!(value.as_int(), Some(5));
```

## Verification

`cargo test -p wolfram-iir-compiler` covers every accepted construct
(via `wolfram-vm`, as a dev-dependency) and every rejected construct's
explicit-error path. `tests/oracle.rs` additionally cross-checks every
accepted construct against `wolfram-runtime`'s own evaluator, both sides
rendered through the same `print_wolfram` function.
