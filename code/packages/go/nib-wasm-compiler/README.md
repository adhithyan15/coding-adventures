# nib-wasm-compiler

`nib-wasm-compiler` is the Go end-to-end pipeline for compiling Nib source into
WebAssembly bytes.

It intentionally stays as orchestration glue:

```text
Nib source -> parser -> type checker -> compiler IR -> Wasm module -> bytes
```

## Usage

```go
result, err := nibwasmcompiler.CompileSource("fn answer() -> u4 { return 7; }")
if err != nil {
    panic(err)
}
_ = result.Binary
```

The result keeps the parsed AST, typed AST, raw IR, optimized IR, Wasm module,
validated module, and final binary so tests and educational tools can inspect
each stage.
