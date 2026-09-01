# wasm-validator

WebAssembly 1.0 module validator. `validate(&WasmModule) -> Result<ValidatedModule, ValidationError>`
checks everything the parser can't: type/function/global/table/memory index
bounds, export uniqueness, memory/table count limits, data/element segment
validity, and (since 0.2.0) a full instruction-level type check of every
function body — no stack underflow, no type mismatches, correct
local/global indices and mutability, memory instructions only when a
memory exists. See `src/type_check.rs` for the abstract-interpretation
algorithm (`W02-wasm-validator.md`'s own design, Phase 2).

Since 0.2.77, this also includes a real **constant-expression type-checker**:
every global initializer and active element-/data-segment offset expression
is checked against its declared/required type (respecting the same
non-null/bottom/nominal-subtype rules as everything else), and `global.get`
inside a constant expression is checked against the real "prior, immutable
global only" spec rule. See `src/type_check.rs`'s "Const-expression
type-checking" section and `W02-wasm-validator.md` §§1.10-1.11.

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
