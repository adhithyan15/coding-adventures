# derive-vm

A tiny interpreter for the Derive v0 arithmetic/assignment IIR subset,
built directly on `dynval-runtime`. Executes the `interpreter_ir::IIRModule`
produced by [`derive-iir-compiler`](../derive-iir-compiler) against the
`dynval-runtime` tagged-`i64` value model (symbols, cons cells, nil,
booleans), returning a `LispyValue`.

Derive's own VM — deliberately independent of `twig-vm`/`mccarthy-lisp-vm`/
`macsyma-vm` (each language in this rollout gets its own; see
[`macsyma-iir-vm.md`](../../../specs/macsyma-iir-vm.md) §6 for the
explicit VM-sharing decision). All four share only the `dynval-runtime`
foundation, not any instruction set or opcodes.

## Instruction set

Only three opcodes: `const`, `call_builtin`, `ret` — v0 has no branches,
calls, or closures. See the crate's top-level doc comment for the full
table.

## Usage

```rust
use derive_vm::run;

let value = run(&module)?; // module: interpreter_ir::IIRModule
```

## Verification

`cargo test -p derive-vm` — 16 unit tests over hand-built `IIRModule`s
covering every opcode, every error path, and the instruction budget. The
source-level (compiler → VM) path is covered by
`derive-iir-compiler`'s own tests and oracle test.
