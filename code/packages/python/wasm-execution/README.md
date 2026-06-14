# wasm-execution

WebAssembly 1.0 wasm-execution

## Dependencies

- wasm-leb128
- wasm-types
- wasm-opcodes
- wasm-module-parser
- virtual-machine

## Execution Limits

`WasmExecutionEngine` accepts `WasmExecutionLimits` so embedders can cap
runtime work for untrusted modules. The first limit is `max_instructions`,
which counts interpreted WASM instructions across module-defined calls and
raises a `TrapError` once the budget is exhausted.

## Development

```bash
# Run tests
bash BUILD
```
