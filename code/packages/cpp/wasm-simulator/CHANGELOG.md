# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C++17 header-only port of the Rust `wasm-simulator` crate, in
  namespace `ca::wasm`: a stack-based WebAssembly virtual machine (i32 subset).
- `WasmDecoder`, `WasmExecutor`, and `WasmSimulator` (`load`/`step`/`run` with
  public stack/locals/pc/halted/cycle) producing a `WasmStepTrace` per
  instruction (`std::vector` snapshots, `std::string` description,
  `std::optional` operand), plus the `encode_*` / `assemble_wasm` helpers.
- Arithmetic wraps modulo 2^32; Rust panics become `std::runtime_error` /
  `std::out_of_range`.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC): the crate's full-program
  vector, decode, the throwing error paths, and 2^32 wrapping.
