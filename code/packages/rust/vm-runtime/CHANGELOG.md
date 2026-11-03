# Changelog — vm-runtime

## Unreleased — #18470 review — a malformed symbol argument is refused

- The in-process path used to turn a `Symbol` result whose payload does not
  fit a `u32` id into `Value::Null`. Null is nil to a Lisp module, so a
  malformed argument became a real value. `vmresult_to_value` now answers
  `None` for it, and `vm_execute` returns the new `VmResult::trap(4)`
  ("malformed argument") before running anything. Tests:
  `symbol_result_round_trips_and_rejects_a_wide_payload`,
  `a_malformed_symbol_argument_traps_before_execution`.

## Unreleased

### Fixed (`Value::Symbol` exhaustiveness)

- `VmResult::from_value` handles `vm-core`'s new `Value::Symbol(id)` (#18470:
  a symbol is an interned atom, no longer an integer in a reserved range).
  It gets a tag of its own, **`VmResultTag::Symbol = 8`**, with the symbol id
  as payload — never a `U64` that would make a symbol a number again, nor a
  `Str`, whose payload means a string-pool index. `from_u8(8)` and `Display`
  know it, and the in-process path maps it back to `Value::Symbol`. Tests:
  `from_value_symbol_is_not_an_integer`, `tag_from_u8_roundtrip` (now with 8).

## [0.1.2] — 2026-08-02

### Fixed (`Value::HeapRef` exhaustiveness)

- `VmResult::from_value` now handles `vm-core` 0.20.0's new
  `Value::HeapRef` variant, mapping it to `VmResult::from_ref(r.addr() as
  u64)` — the `VmResultTag::Ref` case this type already anticipated (see
  the `from_ref` constructor's original "LANG16" doc comment), just never
  had a producer wired up until now.

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
