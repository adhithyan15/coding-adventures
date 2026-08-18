# maple-vm

A tiny interpreter for the Maple v0 arithmetic/assignment IIR subset,
built directly on `dynval-runtime`. Executes the `interpreter_ir::IIRModule`
produced by [`maple-iir-compiler`](../maple-iir-compiler), returning a
`LispyValue`.

Maple's own VM — deliberately independent of every sibling VM in this
rollout (see [`macsyma-iir-vm.md`](../../../specs/macsyma-iir-vm.md) §6).

## Instruction set

Only three opcodes: `const`, `call_builtin`, `ret`.

## Usage

```rust
use maple_vm::run;

let value = run(&module)?; // module: interpreter_ir::IIRModule
```

## Verification

`cargo test -p maple-vm` — 15 unit tests over hand-built `IIRModule`s.
