# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C17 port of the Rust `clr-simulator` crate: a type-inferring,
  stack-based virtual machine for a subset of Microsoft's CIL (.NET CLR
  bytecode) — integer/reference values, an `object[]` heap, boxing, object
  arrays, method calls with frames, and conditional branches.
- Status-returning API (`ClrStatus`) in place of the Rust panics: `clr_new` /
  `clr_free`, `clr_load` / `clr_load_program`, `clr_step` / `clr_run`, stack /
  local inspection accessors, and the `clr_encode_ldc_i4` / `_stloc` / `_ldloc`
  compact instruction encoders.
- **Bounds safety for untrusted bytecode**: every operand read and heap/array
  index is checked (returning `CLR_ERR_BYTECODE_OVERRUN` /
  `CLR_ERR_INDEX_OUT_OF_RANGE` instead of an out-of-bounds read), all growable
  buffers guard `size_t` overflow, and arithmetic wraps through `uint32_t` to
  avoid signed-overflow UB. Verified clean under ASan + UBSan.
- 60 checks mirroring the crate's unit tests plus extra bounds-safety cases, run
  under every ISO C compiler via the shared `iso-harness`.
