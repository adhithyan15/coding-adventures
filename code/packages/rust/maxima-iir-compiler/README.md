# maxima-iir-compiler

Maxima source → `interpreter_ir::IIRModule`, **v0.1.0**.

A direct re-export of [`macsyma-iir-compiler`](../macsyma-iir-compiler)'s
public API under Maxima's own name — no shim function, since Maxima and
Macsyma share the exact same algebraic surface (the same relationship
[`maxima-to-semantic-ir`](../maxima-to-semantic-ir) already established
on the Semantic-IR side). See
[`macsyma-iir-vm.md`](../../../specs/macsyma-iir-vm.md) for the full
design this rollout follows.

There is **no `maxima-vm` crate**. Every compiled `IIRModule` runs
unchanged on [`macsyma-vm`](../macsyma-vm) — nothing in that dispatch
loop is Macsyma-specific, and Maxima has never had any runtime code of
its own anywhere in this repo.

## Scope (v0.1.0)

Identical to `macsyma-iir-compiler`'s own v0 scope — see that crate's
README for the full accepted/rejected construct list.

## Usage

```rust
use maxima_iir_compiler::compile_source;

let module = compile_source("2 + 3$", "demo")?;
let value = macsyma_vm::run(&module)?;
assert_eq!(value.as_int(), Some(5));
```

## Verification

`cargo test -p maxima-iir-compiler` covers the re-export symmetry
(`compile`/`compile_source`), a representative accepted program running
end-to-end through `macsyma-vm`, and one rejected-construct check. No
separate oracle test — since this crate has no lowering logic of its own,
`macsyma-iir-compiler`'s own oracle test already covers every construct
this crate can produce.
