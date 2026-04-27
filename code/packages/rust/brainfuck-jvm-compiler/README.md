# brainfuck-jvm-compiler

`brainfuck-jvm-compiler` is the thin Rust orchestration layer that turns
Brainfuck source into JVM `.class` bytes.

It mirrors the repo's WASM compiler packages:

```text
Brainfuck source
  -> brainfuck
  -> brainfuck-ir-compiler
  -> ir-optimizer
  -> ir-to-jvm-class-file
  -> jvm-class-file parser
  -> .class bytes
```

The package stays intentionally thin. It does not know JVM bytecode details; it
just wires the existing frontend to the generic JVM backend and then parses the
result immediately as a structural self-check.
