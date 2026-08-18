# axiom-vm

A tiny interpreter for the Axiom v0 arithmetic/assignment IIR subset,
built directly on `dynval-runtime`. Executes the `interpreter_ir::IIRModule`
produced by [`axiom-iir-compiler`](../axiom-iir-compiler), returning a
`LispyValue`.

Axiom's own VM — deliberately independent of every sibling VM in this
rollout (see [`macsyma-iir-vm.md`](../../../specs/macsyma-iir-vm.md) §6).

## Instruction set

Only three opcodes: `const`, `call_builtin`, `ret`.

## Usage

```rust
use axiom_vm::run;

let value = run(&module)?; // module: interpreter_ir::IIRModule
```

## Verification

`cargo test -p axiom-vm` — 15 unit tests over hand-built `IIRModule`s.
