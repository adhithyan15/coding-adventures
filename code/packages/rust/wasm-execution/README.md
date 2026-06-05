# wasm-execution

WebAssembly 1.0 wasm-execution.

As of 0.2.0 it also executes the first **WasmGC** opcodes (LANG77 / McCarthy
L3b-3a): the `0xFB`-prefixed `i31.new` / `i31.get_s` (integer boxing), with an
`i31ref` represented as its `i32` payload on the value stack. This lets the
McCarthy-Lisp → wasm value model be *run* in-repo, not just emitted. The
`struct.*` / `ref.*` GC opcodes (which need a GC object heap) are a follow-up.

## Dependencies

- wasm-leb128
- wasm-types
- wasm-opcodes
- wasm-module-parser
- virtual-machine

## Development

```bash
# Run tests
bash BUILD
```
