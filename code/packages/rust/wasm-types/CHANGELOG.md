# Changelog

All notable changes to this package will be documented in this file.

## [0.1.14] - 2026-08-31 (W32 second slice — non-null concrete reference types)

### Added

- **`ValueType::NonNullStructRef(u32)`/`NonNullConcreteFuncRef(u32)`**: the
  **non-null concrete reference types** from the GC/function-references
  proposals — `(ref $T)`/`(ref $t)`, no `null` keyword — per
  `code/specs/W32-wasm-non-null-concrete-reference-types.md`'s addendum
  section 1. Binary tag `0x64 <LEB128(idx)>`, independently verified
  against the real reference interpreter's `interpreter/binary/decode.ml`
  (`ref_type`'s `-0x1c -> (NoNull, heap_type s)` arm: `-28 mod 128 =
  0x64`) — one more than `StructRef`/`ConcreteFuncRef`'s existing `0x63`
  ("nullable"), matching the spec document's own claimed value exactly
  (no discrepancy to fix here, unlike W24's `exnref` bug or this same
  spec's own first-slice keyword-spelling correction).
- **`ValueType::is_non_null_subtype_of`**: the non-null subtyping lattice
  from the spec's section 2 — `NonNullStructRef(i) <: StructRef(i) <:
  Anyref` and `NonNullConcreteFuncRef(i) <: ConcreteFuncRef(i) <: Funcref`
  (both hops of each chain are direct rules, not composed, matching how
  `ConcreteFuncRef <: Funcref` (W11-B) and the four W32-first-slice bottom
  types were each direct rules too). The reverse direction never holds —
  a nullable type is never accepted where non-null is required, and
  NEITHER bottom type (`NullRef`/`NullFuncref`) satisfies a non-null slot
  either.

### Scope note

This is the **second slice** of W32: the two non-null concrete-ref
variants and their subtyping rules only. Structural subtyping for
`call_indirect`/`ref.cast` against concrete function/struct types (needed
by `type-subtyping.wast`), real recursive type groups' own nominal
identity rules, and array types remain a later slice — see this
package's own addendum to the spec document.

## [0.1.13] - 2026-08-31 (W32 first slice — bottom reference types)

### Added

- **`ValueType::NullFuncref`/`NullExternref`/`NullExnref`/`NullRef`**: the
  four **bottom reference types** from the GC/function-references/
  exceptions proposals (`nullfuncref`/`nullexternref`/`nullexnref`/
  `nullref`, a.k.a. `none`) — each a genuine strict subtype of every
  compatible nullable reference type in its own hierarchy (func/extern/
  exn/any), per `code/specs/W32-wasm-non-null-concrete-reference-types.md`
  section 1. Single-byte encodings `0x73`/`0x72`/`0x74`/`0x71`,
  independently verified against the real reference interpreter's
  `interpreter/binary/decode.ml` (`NoFuncHT = -0x0d`, `NoExternHT =
  -0x0e`, `NoExnHT = -0x0c`, `NoneHT = -0x0f`; SLEB128 single-byte
  encoding of a small negative value is `value mod 128`) rather than
  re-asserted from this crate's own prior doc comments — the same
  discipline W24's `exnref` tag-byte bug established. Replaces an earlier
  pass's "lossy aliasing" of these four keywords straight onto their
  nullable supertypes (`nullfuncref` == `funcref`, etc., in
  `wasm-wast-parser`), which made the bottom types indistinguishable from
  their supertypes and unable to express the asymmetric subtyping the
  spec (and the real corpus) requires.
- **`ValueType::is_bottom_subtype_of`**: the bottom-type subtyping lattice
  from the spec's section 2 — e.g. `NullFuncref <: Funcref` and
  `NullFuncref <: ConcreteFuncRef(_)` for every index (bottom of the
  WHOLE func hierarchy), `NullRef <: Anyref`/`I31ref`/`StructRef(_)`, etc.
  Deliberately NOT reflexive (`T <: T` is the caller's own `==` check) and
  deliberately does NOT encode non-null concrete refs or structural
  subtyping — both explicitly out of scope for this first slice; see the
  spec's own "Explicitly out of scope" section.

### Scope note

This is the **first slice** of W32: only the four bottom types and their
subtyping rules. `NonNullStructRef`/`NonNullConcreteFuncRef` (non-null
concrete refs) and structural subtyping for `call_indirect`/`ref.cast`
against concrete function/struct types are a separate, later slice — see
the spec's own addendum.

## [0.1.12] - 2026-08-26 (W11 addendum — concrete function-type refs)

### Added

- **`ValueType::ConcreteFuncRef(u32)`**: a nullable reference to a
  specific concrete FUNCTION type (`(ref null $t)` where `$t` names a
  `func` type) — the function-references-proposal analogue of the
  pre-existing `ValueType::StructRef(u32)`, but indexing
  `WasmModule::types` (the func-type array) directly instead of the
  struct-type array's `+ types.len()`-offset space. Needed for exactly
  one real-corpus construct: `return_call.wast`/
  `return_call_indirect.wast`'s "Result subtyping" test, which declares a
  helper function `(func $f (result (ref null $t)) (ref.null $t))` and
  then uses its result where `funcref` is expected. Encodes identically
  to `StructRef` (`0x63` followed by `LEB128(idx)`) — see the variant's
  own doc comment for why the two never collide despite sharing that tag
  byte. Deliberately does NOT add a non-null `(ref $t)` variant (the real
  "typed function references" wall, tracked separately, much larger in
  scope).

## [0.1.11] - 2026-08-26 (W26 — table64 proposal, first slice)

### Changed

- **`TableType` gains `pub is64: bool`** (table64 proposal): whether this
  table uses 64-bit addressing, mirroring `MemoryType::is64` (W25) exactly.
  Defaults to `false` at every existing call site — no behavior change for
  any 32-bit table. `Limits` (already widened to `u64` in W25) is reused
  as-is — table64's own real spec ceiling is `u64::MAX`, verified live
  against the reference interpreter's `check_tabletype`, not the smaller
  `2^48`-page bound memory64 uses.

See `code/specs/W26-wasm-table64-first-slice.md` for the full slice
(binary/text encoding, validator/executor address-width plumbing, vendored
`table64.wast` and `memory64-imports.wast`).

## [0.1.10] - 2026-08-26 (W25 — memory64 proposal, first slice)

### Changed

- **`Limits.min`/`max` widened from `u32`/`Option<u32>` to
  `u64`/`Option<u64>`**: a real, spec-valid 64-bit memory's limits can
  reach `2^48` pages (the memory64 proposal's own ceiling), which doesn't
  fit `u32`. `TableType` shares this same struct and stays well within
  `u32`'s range for every value this repo has ever built — a pure,
  numerically non-breaking widening for every existing table/32-bit-
  memory caller.
- **`MemoryType` gains `pub is64: bool`** (memory64 proposal): whether
  this memory uses 64-bit addressing. Defaults to `false` at every
  existing call site — no behavior change for any 32-bit memory.

See `code/specs/W25-wasm-memory64-first-slice.md` for the full slice
(binary/text encoding, validator/executor address-width plumbing,
vendored `memory64.wast`).

## [0.1.9] - 2026-08-26 (W24 — exceptions proposal, fourth slice: real exnref)

### Fixed

- **Security review finding**: `ValueType::Exnref::byte_tag`/`encode`
  changed from `0xE9` to `0x69`. `0xE9` was this variant's real spec type
  opcode `-0x17` mis-encoded as its two's-complement-mod-256 byte
  (`-23 + 256 = 233 = 0xE9`) rather than its correct single-byte SLEB128
  encoding (`-23 & 0x7F = 0x69`) — every OTHER abstract reference type
  here (`funcref` `-0x10`→`0x70`, `externref` `-0x11`→`0x6F`, `anyref`
  `-0x12`→`0x6E`, `i31ref`→`0x6C`) happens to have its raw byte value
  ALSO be its correct SLEB128 encoding, which is what let this go
  unnoticed since W22 first added `Exnref` — `exn`'s value (`-0x17`)
  is simply the first one where the two representations diverge. `0xE9`
  has its LEB128 continuation bit SET (`>= 0x80`), making it
  indistinguishable, in a blocktype decoder, from the leading byte of a
  genuine multi-byte type index — a real, attacker-reachable bug
  (surfaced and fixed while adding `exnref` blocktype-shorthand support
  for W24; see `wasm-execution`/`wasm-validator`'s own changelogs).
  `0x69` has its continuation bit clear, matching every other
  special-cased blocktype byte's own safe invariant. No test anywhere in
  this repo hard-coded the old `0xE9` value (confirmed via a repo-wide
  grep before changing it), so this is a clean, non-breaking fix.

### Changed

- `ValueType::Exnref`'s doc comment updated: no longer "deliberately
  inert" as of `wasm-execution` 0.9.68 — a `catch_ref`/`catch_all_ref`
  clause that matches now pushes a real, reified `exnref` value (a handle
  into `wasm-execution`'s new `exception_heap`), and `throw_ref` consumes
  one to re-raise the exception it names.

## [0.1.8] - 2026-08-25 (W22 — exceptions proposal: real catch/catch_all matching)

### Added

- `ValueType::Exnref` — the exceptions proposal's `exnref` type (real
  spec byte `-0x17`, i.e. unsigned `0xE9`). Deliberately inert: recognized
  purely so a module MENTIONING it (e.g. a `catch_ref`/`catch_all_ref`
  target block's declared result type) still parses/validates as a
  whole, since W14's per-module build isolation means one unrecognized
  value type anywhere in a module fails the ENTIRE module — and the real
  testsuite's own `try_table.wast` mixes `exnref`-typed functions
  alongside ordinary `catch`/`catch_all`-only ones in the SAME module.
  Never a real runtime value in this repo (no `catch_ref`/`catch_all_ref`
  clause is ever selected as a match — see `wasm-execution`'s own
  changelog). See `code/specs/W22-wasm-exceptions-catch-clause-matching.md`.

## [0.1.7] - 2026-08-25 (W21 — exceptions proposal: tag/throw first slice)

### Added

- `ExternalKind::Tag = 0x04` — matches the real exception-handling
  proposal's binary encoding exactly (live-fetched and confirmed against
  `WebAssembly/exception-handling`'s own `Exceptions.md`).
- `ImportTypeInfo::Tag(u32)` — a tag import's function-type index (the
  tag's underlying signature; its `results` must be empty, a rule
  `wasm-validator` enforces, not this crate).
- `WasmModule.tags: Vec<u32>` — module-defined tags' type indices, same
  "imports live in `imports`, this Vec is only the module-defined ones"
  convention `functions: Vec<u32>` already uses.

See `code/specs/W21-wasm-exceptions-tag-throw-slice.md`.

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
