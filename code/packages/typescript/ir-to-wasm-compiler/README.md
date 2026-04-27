# ir-to-wasm-compiler

`ir-to-wasm-compiler` lowers the repo's generic compiler IR into a typed
`WasmModule`.

The current backend is intentionally small and focused on the Brainfuck proof
of concept: structured loops, byte-addressed linear memory, and WASI imports
for byte I/O and exit.
