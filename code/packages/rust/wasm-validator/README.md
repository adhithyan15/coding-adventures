# wasm-validator

WebAssembly 1.0 module validator. `validate(&WasmModule) -> Result<ValidatedModule, ValidationError>`
checks everything the parser can't: type/function/global/table/memory index
bounds, export uniqueness, memory/table count limits, data/element segment
validity, and (since 0.2.0) a full instruction-level type check of every
function body — no stack underflow, no type mismatches, correct
local/global indices and mutability, memory instructions only when a
memory exists. See `src/type_check.rs` for the abstract-interpretation
algorithm (`W02-wasm-validator.md`'s own design, Phase 2).

## Dependencies

- wasm-leb128
- wasm-types
- wasm-opcodes
- wasm-module-parser
- virtual-machine
- wasm-wast-parser (dev-only, for readable WAT-based tests)

## Development

```bash
# Run tests
bash BUILD
```
