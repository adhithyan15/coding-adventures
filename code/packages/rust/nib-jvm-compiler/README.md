# nib-jvm-compiler

`nib-jvm-compiler` is the thin Rust orchestration layer for Nib's JVM path.

It keeps the repo's package boundaries intact:

```text
Nib source
  -> coding-adventures-nib-parser
  -> nib-type-checker
  -> nib-ir-compiler
  -> ir-optimizer
  -> ir-to-jvm-class-file
  -> jvm-class-file parser
  -> .class bytes
```

The crate preserves the important intermediate artifacts so tests and future
tools can inspect the AST, typed AST, IR, and generated class structure.
