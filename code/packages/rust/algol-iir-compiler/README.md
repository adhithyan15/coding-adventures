# algol-iir-compiler

Rust frontend for compiling a conservative ALGOL 60 scalar subset into the
shared LANG VM `interpreter_ir::IIRModule`.

This crate intentionally lives on the Rust LANG VM chain:

```text
ALGOL source -> algol-lexer/parser -> algol-iir-compiler -> IIRModule
  -> vm-core / jit-core / aot-core / iir-to-wasm / iir-to-jvm / iir-to-cil
  -> iir-to-beam / iir-to-llvm
```

The first slice supports scalar `integer` and `boolean` programs with
assignments, integer arithmetic (`+`, `-`, `*`, `div`, `mod`), comparisons,
`if`/`else`, compound statements, labels, `goto`, and
`for i := a step k until b do ...` where `k` is a constant integer.
Unsupported ALGOL 60 features, including arrays, procedures, strings, reals,
switches, nested declaration scopes, and by-name calls, return explicit
compiler errors.
