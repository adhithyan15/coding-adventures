# brainfuck-wasm-compiler

`brainfuck-wasm-compiler` is the end-to-end TypeScript pipeline for compiling
Brainfuck source into `.wasm` bytes.

Pipeline:

1. Parse Brainfuck source into an AST
2. Lower the AST into generic IR
3. Run the standard IR optimizer
4. Validate that the IR can lower into the current WASM backend
5. Lower into a typed `WasmModule`
6. Validate the module
7. Encode the final `.wasm` bytes
