# ir-to-wasm-validator

Checks whether a generic `IrProgram` can be lowered into the current WASM
backend subset.

It is intentionally a small pipeline stage:

```
IrProgram -> ir-to-wasm-validator -> OK/errors
```
