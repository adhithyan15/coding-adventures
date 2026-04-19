# brainfuck-wasm-compiler

Native Kotlin Brainfuck-to-Wasm package. It parses Brainfuck source, emits a
small Wasm module exporting `_start`, and provides `compileSource`,
`packSource`, and `writeWasmFile` helpers for the convergence pipeline.
