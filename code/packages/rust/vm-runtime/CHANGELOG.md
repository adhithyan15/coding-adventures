# Changelog — vm-runtime

## [0.1.1] — 2026-05-11

### Fixed (LANG32 — `Operand::Str` exhaustiveness)

- `encode_function_json` in `iir_table.rs` now handles `Operand::Str`:
  emits `{"str":"..."}` using Rust's `{:?}` format, which produces a
  double-quoted, escape-safe JSON string.  This keeps the JSON-line body
  section of the IIRT blob well-formed when functions carry string-literal
  operands.

## [0.1.0] — 2026-05-04

### Added

- `level::RuntimeLevel` enum — `None` / `Minimal` / `Standard` / `Full` with
  `level_number()`, `artefact_name(target)`, `requires_gc()`, `includes_builtins()`
  predicates.
- `level::required_level(module)` — automatic level selection from IIR opcode mix.
- `result::VmResultTag` enum — 9 discriminants (`Void`, `U8`…`U64`, `Bool`,
  `Str`, `Ref`, `Trap`) with `from_u8()` and `Display`.
- `result::VmResult` — tagged result type with constructors (`void`, `from_u8`,
  `from_bool`, `from_i64`, `from_ref`, `trap`) and extractors (`as_u64`,
  `as_i64`, `as_bool`, `trap_code`).  Converts from `vm_core::value::Value`.
- `iir_table::IIRTableWriter` — builds `vm_iir_table` blobs with IIRT magic,
  version header, sorted index, body (JSON lines), and string pool.
- `iir_table::IIRTableReader` — parses IIRT blobs; `lookup(name)` for
  name→index, `name_at(idx)`, `get(idx)` for function retrieval.
- `reloc::RelocationKind` — 6 relocation kinds (`IirFnIndex`, `BuiltinIndex`,
  `RtEntryAbs`, `RtEntryPcRel`, `StringPool`, `GcRootTable`) with `from_u16`.
- `reloc::RelocationEntry` — 16-byte relocation record with `serialise()` and
  `deserialise(bytes, pool)`.
- `inprocess::InProcessVMRuntime` — development/test in-process runtime backed
  by `vm-core`'s `VMCore`; implements `vm_execute`, `vm_resume_at`, and
  `lookup_function`.
- 52 unit tests + 15 doc-tests, all passing.
