# nib-wasm-compiler

`nib-wasm-compiler` is the Rust orchestration package that compiles Nib source
into WebAssembly bytes.

The package composes existing Rust crates:

```text
Nib source -> parser -> type checker -> compiler IR -> Wasm module -> bytes
```

It returns the parsed AST, typed AST, raw IR, optimized IR, Wasm module,
validated module, and encoded binary for educational inspection.

## Usage

```rust
let result = nib_wasm_compiler::compile_source("fn answer() -> u4 { return 7; }")?;
assert!(!result.binary.is_empty());
# Ok::<(), nib_wasm_compiler::PackageError>(())
```
