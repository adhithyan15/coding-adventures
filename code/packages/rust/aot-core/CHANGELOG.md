# Changelog — aot-core

## 0.1.0 — 2026-04-28

### Added

- **`AOTCore`** — ahead-of-time compilation controller that compiles an entire
  `IIRModule` to a `.aot` binary; configurable optimization level (0/1/2).
- **`infer_types()`** — flow-insensitive static type inference pass over
  `IIRFunction` instructions; seeds from declared parameter types and propagates
  through arithmetic, bitwise, comparison, and unary ops with numeric promotion.
- **`aot_specialise()`** — AOT analog of `jit-core`'s `specialise()`, producing
  typed `Vec<CIRInstr>` from an `IIRFunction` and a pre-computed type environment.
  Identical structure to the JIT pass; only the type-resolution step differs
  (env lookup vs. observed_type from the profiler).
- **`link()`** + **`entry_point_offset()`** — concatenate per-function binary
  blobs into a single code section with byte-offset table.
- **`snapshot::write()`** + **`snapshot::read()`** — 26-byte little-endian
  `.aot` binary format: magic `b"AOT\0"` + version + flags + entry_point_offset
  + IIR-table offset/size + native-code size, followed by code section and
  optional IIR-table section.
- **`VmRuntime`** — wraps a pre-compiled vm-runtime library; provides
  `serialise_iir_table()` (compact JSON via `serde_json`) and
  `deserialise_iir_table()` for inspection and testing.
- **`AOTStats`** — cumulative compilation statistics (functions compiled/untyped,
  time, binary size, optimization level).
- **`AOTError`** — `Backend` and `Snapshot` error variants.
- 110 unit tests + 12 doc-tests.
