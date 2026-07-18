# CodingAdventures.BrainfuckWasmCompiler.FSharp

A pure F# Brainfuck-to-WebAssembly compiler. It ignores non-Brainfuck source
characters, validates bracket structure and nesting depth, and emits a typed
WebAssembly module with one memory and an exported `_start` function. Input and
output instructions use the WASI `fd_read` and `fd_write` imports only when the
program needs them.

```fsharp
open CodingAdventures.BrainfuckWasmCompiler.FSharp

let result = BrainfuckWasmCompiler.compileSource "++[>+<-]"
File.WriteAllBytes("program.wasm", result.WasmBytes)
```

The compiler builds the module with the sibling `wasm-types`, `wasm-leb128`,
and `wasm-module-encoder` packages. `writeWasmFile` is the only API that touches
the filesystem.
