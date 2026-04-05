# @coding-adventures/wasm-validator

Semantic validation for WebAssembly 1.0 modules.

This package sits between `@coding-adventures/wasm-module-parser` and the future
execution engine. It checks that a parsed `WasmModule` is safe to execute:

- type indices resolve
- exports point at real definitions
- start functions have type `() -> ()`
- constant expressions stay within the MVP rules
- function bodies type-check under the abstract stack machine
- control-flow labels, locals, globals, memories, and tables are all in bounds

## Usage

```ts
import { WasmModuleParser } from "@coding-adventures/wasm-module-parser";
import { validate, ValidationError } from "@coding-adventures/wasm-validator";

const parser = new WasmModuleParser();
const module = parser.parse(wasmBytes);

try {
  const validated = validate(module);
  console.log(validated.funcTypes.length);
} catch (error) {
  if (error instanceof ValidationError) {
    console.error(error.kind, error.message);
  }
}
```

## API

- `validate(module)` validates the full module and returns a `ValidatedModule`
- `validateStructure(module)` runs the structural phase and returns index spaces
- `validateFunction(...)` validates a single local function body
- `validateConstExpr(...)` validates a constant-expression byte sequence

## Dependencies

- `@coding-adventures/wasm-module-parser`
- `@coding-adventures/wasm-types`
- `@coding-adventures/wasm-opcodes`
- `@coding-adventures/wasm-leb128`
