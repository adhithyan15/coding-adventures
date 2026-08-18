# reduce-vm

A tiny interpreter for the Reduce v0 arithmetic/assignment IIR subset,
built directly on `dynval-runtime`. Executes the `interpreter_ir::IIRModule`
produced by [`reduce-iir-compiler`](../reduce-iir-compiler) against the
`dynval-runtime` tagged-`i64` value model, returning a `LispyValue`.

Reduce's own VM — deliberately independent of every sibling VM in this
rollout (see [`macsyma-iir-vm.md`](../../../specs/macsyma-iir-vm.md) §6
for the explicit VM-sharing decision). All share only the
`dynval-runtime` foundation, not any instruction set or opcodes.

## Instruction set

Only three opcodes: `const`, `call_builtin`, `ret` — v0 has no branches,
calls, or closures.

## Usage

```rust
use reduce_vm::run;

let value = run(&module)?; // module: interpreter_ir::IIRModule
```

## Verification

`cargo test -p reduce-vm` — 15 unit tests over hand-built `IIRModule`s
covering every opcode, every error path, and the instruction budget.
