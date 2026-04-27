# Changelog

## 0.1.0

- Add end-to-end Oct to WebAssembly compiler facade.
- Pipeline: oct-lexer → oct-parser → oct-type-checker → oct-ir-compiler (WASM_IO) → ir-to-wasm-validator → ir-to-wasm-assembly → wasm-assembler → wasm-validator.
- `OctWasmCompiler` class with `compile_source` and `write_wasm_file` methods.
- Module-level convenience functions: `compile_source`, `pack_source`, `write_wasm_file`.
- Uses `WASM_IO` config so `out()` → SYSCALL 1 (fd_write) and `in()` → SYSCALL 2 (fd_read).
- `PackageError` with `stage` attribute for precise failure diagnosis.
