# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C++17, header-only port of the Rust `clr-simulator` crate in
  namespace `ca::clr_simulator`: a type-inferring, stack-based virtual machine
  for a subset of Microsoft's CIL (.NET CLR bytecode) — integer/reference
  `Value`s (`std::optional`-based slots), an `object[]` heap, boxing, object
  arrays, method calls with frames, and conditional branches.
- Idiomatic C++ surface: exceptions (`Error : std::runtime_error` carrying an
  `ErrorKind`) where the Rust crate panics, `Simulator::load` /
  `load_program` / `step` / `run`, inspection accessors, and static
  `encode_ldc_i4` / `encode_stloc` / `encode_ldloc` / `assemble` encoder
  helpers returning `std::vector<std::uint8_t>`.
- **Bounds safety for untrusted bytecode**: every operand read and heap/array
  index is checked, and arithmetic wraps through `std::uint32_t` (no
  signed-overflow UB). Verified clean under ASan + UBSan.
- 32 checks mirroring the crate's unit tests plus extra bounds-safety cases, run
  under every ISO C++ compiler via the shared `iso-harness`.
