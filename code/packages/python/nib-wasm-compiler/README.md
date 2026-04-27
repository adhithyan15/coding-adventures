# nib-wasm-compiler

End-to-end Nib to WASM compiler.

Pipeline:

```
Nib source
  -> nib-parser
  -> nib-type-checker
  -> nib-ir-compiler
  -> ir-optimizer
  -> ir-to-wasm-validator
  -> ir-to-wasm-assembly
  -> wasm-assembler
  -> .wasm bytes
```
