# Changelog

All notable changes to this package will be documented in this file.

## [0.1.6] - 2026-08-17 (task #97 — passive/exprs-list element segments)

### Changed

- `Element.function_indices` widened from `Vec<u32>` to
  `Vec<Option<u32>>` -- `None` represents a `ref.null` entry in a
  passive exprs-list segment (`(elem funcref (ref.func $f) (ref.null
  func))`), `Some(idx)` a real function reference, reusing the same
  `Option<u32>` shape `Table::elements`/`WasmValue::Ref` already use
  rather than inventing a new one.
- `Element.is_passive: bool` added, mirroring `DataSegment.is_passive`
  (task #95) exactly: `true` for a segment declared with no table index
  or offset expression at all, so `wasm-runtime::instantiate()` never
  applies it automatically -- it stays resident until an explicit
  `table.init` copies from it or `elem.drop` frees it.
- Binary encoding scope (see `code/specs/W17-wasm-bulk-table-ops.md`
  for the real-corpus census that justified this): only 4 of the
  spec's 8 element-segment modes are represented (0/1/2/5 -- active-
  implicit funcidx-list, passive funcidx-list, active-explicit
  funcidx-list, passive exprs-list restricted to `ref.func`/`ref.null`).
  Modes 3/7 (declarative) and 4/6 (active+exprs) are non-goals; no
  vendored corpus file this repo uses needs them.

### Migration

- Every existing construction site (`Element { function_indices: vec![1,
  2], .. }`) now needs `vec![Some(1), Some(2)]`; every read site
  (`for func_idx in &elem.function_indices`) now receives
  `Option<u32>` instead of a bare `u32`.

## [0.1.5] - 2026-08-16 (task #95 — passive data segments)

### Added

- `DataSegment.is_passive: bool` -- `true` for a passive segment (bulk-
  memory proposal): declared with no offset expression at all
  (`(data $d "bytes")`, or binary segment-mode flag `0x01`), so
  `wasm-runtime::instantiate()` never applies it automatically -- it
  stays resident until an explicit `memory.init` copies from it or
  `data.drop` frees it. `false` for an ordinary WASM 1.0 active segment,
  unchanged. Additive field on an existing struct -- every existing
  construction site across the workspace needed `is_passive: false`
  added, but no other field's meaning changed.

## [0.1.4] - 2026-08-16 (task #96 — multi-table, `EXTERNREF` constant)

### Added

- `pub const EXTERNREF: u8 = 0x6F`, alongside the existing `FUNCREF`. Used
  by `wasm-wast-parser` to fix a real bug where a table's declared
  `externref` reftype was silently discarded during parsing in favor of
  a hardcoded `FUNCREF` default.

## [0.1.3] - 2026-08-15 (SIMD PR1a — `ValueType::V128`)

### Added

- `ValueType::V128` — the SIMD proposal's 128-bit lane vector type,
  encoded as a single byte `0x7B` (verified against the SIMD proposal's
  own binary-encoding table). `byte_tag()`/`encode()` both updated.
  Unlike the numeric types, its 16 raw bytes don't fit in this repo's
  shared `virtual-machine::Value` typed-stack slot (max 64 bits) — see
  `wasm-execution` 0.8.0 for how the value level carries it (a heap
  handle, mirroring `Anyref`/`I31ref`'s own `WasmValue::Ref` handle
  shape) and `code/specs/W13-wasm-simd-v128-first-slice.md` for the full
  design.

## [0.1.2] - 2026-08-15 (WASM18 — `shared` bit on `MemoryType`)

### Added

- `MemoryType` gained a new `pub shared: bool` field (threads proposal,
  binary-format flags bit 1). A **breaking** struct-field addition — every
  `MemoryType { limits, .. }` construction site across the workspace
  needed a `shared: false`/real value added; see `wasm-module-parser`
  0.2.2 and `wasm-wast-parser` 0.1.9 for the two places that now decode
  a real value instead of always defaulting it.

### Corrected (implementation-time, vs. the merged W09 spec)

- The merged `code/specs/W09-wasm-atomics-plain.md` spec claimed atomic
  instructions require the target memory be declared `shared`. The real,
  pinned-commit WebAssembly threads-proposal testsuite (`atomic.wast`)
  directly contradicts this with its own `;; unshared memory is OK`
  module, exercising every atomic op against a plain, non-shared memory
  and expecting success. `wasm-validator` 0.2.3 does NOT gate atomic ops
  on `shared`; this field exists purely so `shared` round-trips
  correctly through parse/encode, not to drive a validation rule.

## [0.1.1] - 2026-08-15 (WASM17 — funcref/externref as first-class value types)

### Added

- Two new `ValueType` variants: `Funcref` (`byte_tag()` = `Some(0x70)`) and
  `Externref` (`byte_tag()` = `Some(0x6F)`), reusing this repo's own
  `funcref` = `0x70` convention already established by the pre-existing
  `FUNCREF` constant (`TableType::element_type`'s default). Both encode as
  single bytes via `ValueType::encode`, matching `Anyref`/`I31ref`.
- Part of the WASM17 slice (see `code/specs/W08-wasm-funcref-externref.md`)
  unblocking real conformance-testsuite files (`global.wast`, `select.wast`,
  `br_table.wast`, `call_indirect.wast`) that reference `funcref`/`externref`
  as real value types, not just the implicit table element type.

## [0.1.0] - 2026-03-23

### Added

- Initial package scaffolding generated by scaffold-generator

## [0.2.0] - 2026-03-23

### Added

- Full WASM 1.0 type system implementation in `src/lib.rs`
- `ValueType` enum (`I32`, `I64`, `F32`, `F64`) with `#[repr(u8)]` discriminants
  matching WASM binary encoding (0x7C–0x7F)
- `BlockType` enum (`Empty`, `Value`, `TypeIndex`) for structured control flow
- `BLOCK_TYPE_EMPTY` constant (0x40)
- `ExternalKind` enum (`Function`, `Table`, `Memory`, `Global`) with `#[repr(u8)]`
  discriminants matching WASM binary encoding (0x00–0x03)
- `FuncType` struct for function signatures (params + results)
- `Limits` struct for min/max size constraints on memories and tables
- `MemoryType`, `TableType`, `GlobalType` structs
- `FUNCREF` constant (0x70) for the table element reference type
- `Import`, `ImportTypeInfo`, `Export` structs
- `Global`, `Element`, `DataSegment`, `FunctionBody`, `CustomSection` structs
- `WasmModule` struct — top-level container for all decoded module sections,
  with `#[derive(Default)]` for ergonomic construction
- 26 unit tests covering all types, constants, construction, equality, and edge cases
- Literate programming style throughout: ASCII diagrams of binary encoding,
  explanations of WASM execution semantics, and inline examples
