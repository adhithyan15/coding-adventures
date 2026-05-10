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

## 0.1.1 — 2026-05-05

### Added — `try_specialize_builtin` in `specialise`

`aot-core::specialise::aot_specialise` now lowers `call_builtin "<op>"
arg1 arg2` to typed CIR ops (`add_<ty>`, `sub_<ty>`, `mul_<ty>`,
`div_<ty>`, `cmp_<rel>_<ty>`, `mov_<ty>`) when the operands have
known types.  Maps the eleven Twig primitive names (`+`, `-`, `*`,
`/`, `=`, `==`, `!=`, `<`, `<=`, `>`, `>=`, `_move`) to typed
mnemonics.

This is what unlocks the user-visible promise of the LANG-runtime
pipeline: write a statically-typed program in IIR's interpreter
flavour, and the AOT compiler resolves all primitive operations to
native CPU instructions instead of runtime calls.

### Test coverage

- 112 existing tests still pass (no regressions).
- New end-to-end coverage in `twig-aot/tests/macos_arm64_smoke.rs`
  exercises the full lowering chain on real Twig source.
