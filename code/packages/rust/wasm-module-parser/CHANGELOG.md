# Changelog

All notable changes to this package will be documented in this file.

## [0.2.4] — 2026-08-15 — reject malformed mutability bytes + data count cross-check (tasks #82, #84)

### Fixed

- A global's (or WasmGC struct field's) mutability byte is a spec-mandated
  one-bit flag -- `0x00` (immutable) or `0x01` (mutable) only, NOT "any
  nonzero byte means mutable". All three decode sites (global imports,
  global definitions, struct fields) used `byte != 0`, silently accepting
  e.g. `0x04` as mutable instead of rejecting the module as malformed.
  New shared `decode_mutability` helper enforces the two legal encodings;
  new tests at each of the three call sites. Found via a
  `wasm-conformance` prioritization scan after task #80 (PR #11844) --
  the real testsuite's `global.wast` `assert_malformed` "malformed
  mutability" cases were wrongly parsing as valid modules.
- The Data Count section (id 12, bulk-memory-operations proposal) used to
  fall into the generic "unknown section, skip it" arm -- its declared
  segment count was never even read, let alone cross-checked against the
  data section's actual segment count. New `SECTION_DATA_COUNT` handling
  reads the count and, after the section-parsing loop, rejects the module
  if it disagrees with `module.data.len()`. Found via the same
  prioritization scan -- `custom.wast`'s "data count and data section
  have inconsistent lengths" `assert_malformed` case.
- Baseline impact (regenerated in `wasm-conformance` 0.1.22): `align.wast`
  0/2 -> 2/2, `custom.wast` 6/8 -> 8/8, `global.wast` 3/7 -> 7/7 (all
  `assert_malformed`), zero regressions anywhere else in the 61-file
  vendored corpus.

## [0.2.3] — 2026-08-15 — decode the `v128` value-type byte (task #75, SIMD PR1b-3 follow-up)

### Fixed

- `decode_value_type` gains a `0x7B => Ok(ValueType::V128)` arm.
  Previously the ONLY value-type byte this crate's binary decoder didn't
  recognize at all — `wasm-types` (0.1.3) and `wasm-wast-parser`'s
  TEXT-format decoder (SIMD PR1b-2) already handled `v128`, but a
  `(module binary ...)` whose type section declared `(result v128)` (or
  a v128 param/local) failed to decode with "unknown value type byte:
  0x7B". Confirmed needed by the real, pinned-commit `simd_const.wast`'s
  own `(module binary ...)` directives (e.g. `parse_i32x4`/`parse_f64x2`
  etc.). The code SECTION never needed a fix — function bodies are read
  as raw, undecoded bytes (`parse_code_section`), so `0xFD`-prefixed SIMD
  instructions inside one were already opaque to this crate either way;
  only the TYPE-section value-type byte needed recognizing.
- 2 new tests: a `(result v128)` function type decoding from the type
  section, and a `(v128) -> (v128)` function with a declared v128 LOCAL
  decoding from a real function body (exercises all 3 `decode_value_type`
  call sites the fix touches, not just the narrower results-only case).

## [0.2.2] — 2026-08-15 — decode the `shared` memory bit (WASM18)

### Added

- `parse_limits` now decodes flags-byte bit 1 ("shared", threads
  proposal, memory only) and returns `(Limits, bool)` instead of just
  `Limits`. All 4 call sites updated; table import/table-section call
  sites discard the bool (tables aren't part of the threads proposal),
  memory import/memory-section call sites thread it into the new
  `MemoryType.shared` field (see `wasm-types` 0.1.2).

## [0.2.1] — 2026-08-13 — element-section allocation cap (WASM15)

### Fixed — `parse_element_section`'s `func_count` could trigger a multi-gigabyte allocation

`parse_element_section` read a per-entry `func_count: u32leb` directly from
the (untrusted) byte stream and pre-allocated a `Vec<u32>` with it —
`Vec::with_capacity(func_count)`, with no cap. A crafted count like
`0xFFFFFFFF` (four bytes) requested capacity for ~4.29 billion `u32`s
(~17 GiB) before the loop ever reached its first, failing per-index read
on a truncated stream — a real, if minor, DoS vector purely from parsing
a `.wasm` module's element section header.

Found by a security review of an unrelated PR (WASM12), which noticed
this call site didn't follow the same `MAX_PREALLOC` cap the type
section's `params`/`results` vectors already use a few lines away (added
back in `0.2.0`'s predecessor work). Fixed identically:
`Vec::with_capacity(func_count.min(MAX_PREALLOC))` — the `Vec` still
grows correctly for a real, larger element list; only the up-front
reservation is bounded. New regression test constructs a truncated
element section with `func_count = u32::MAX` and confirms parsing
returns a clean `Err` (missing-byte) rather than attempting the
allocation.

## [0.2.0] — 2026-06-04

### Added — WasmGC struct types in the type section (LANG77 / McCarthy L3b-3a-3c)

The type section now parses **WasmGC struct types**, not just function types, so
a module that defines a `$LispyPair` cons cell (emitted by `iir-to-wasm` for
McCarthy Lisp) round-trips through the parser instead of being rejected.

- `parse_type_section` branches on the entry tag: `0x60` → function type (as
  before), `0x50` → a WasmGC **sub-type** entry, parsed as a struct type into
  the existing `WasmModule.struct_types`. Any other tag is a clean error.
- New `parse_struct_type` mirrors `wasm-module-encoder`'s `encode_struct_type`:
  `0x50 <supertype_count> 0x5F <field_count> [<val_type> <mutability>]*`. We
  require `supertype_count == 0` (explicit supertypes unsupported) and the
  struct composite marker `0x5F`.
- `decode_value_type` learned the single-byte GC reference types `anyref`
  (`0x6E`) and `i31ref` (`0x6C`); a new `read_value_type` streaming helper
  decodes the multi-byte concrete `structref` (`0x63 <typeidx>`) for struct
  field types.
- Function and struct types share one type-index space; since the encoder emits
  all function types first, a function's `module.types` index still equals its
  wasm type index (documented assumption).

Hardening: a vector length read from the (untrusted) byte stream is no longer
used directly as the pre-allocation capacity — `with_capacity` is capped at a
small bound and the `Vec` grows as elements actually arrive, so a crafted huge
`field_count`/`param_count` can't force a giant allocation. Truncated or
malformed struct types are clean errors, never panics.

8 new tests: struct-type recovery, func+struct mixed (index alignment),
immutable/`i31ref`/`structref` fields, and clean errors for a bad marker,
declared supertypes, truncation, and an unknown type tag.

## [0.1.0] - 2026-03-23

### Added

- Initial implementation of `WasmModuleParser` — parses raw `.wasm` binary bytes into a
  structured `WasmModule` with no execution.
- `WasmParseError` type with `message` and `offset` fields; implements `Display` and
  `std::error::Error`.
- Full header validation: magic `\0asm` (0x00 0x61 0x73 0x6D) and version 1
  (0x01 0x00 0x00 0x00).
- Section parsers for all 12 WASM 1.0 section types:
  - §0 Custom: name + raw data
  - §1 Type: function signatures (FuncType)
  - §2 Import: function/table/memory/global imports
  - §3 Function: type index array
  - §4 Table: funcref tables with limits
  - §5 Memory: linear memory with limits
  - §6 Global: globals with init_expr (constant expression)
  - §7 Export: named exports (function/table/memory/global)
  - §8 Start: optional function index
  - §9 Element: table initialisation segments
  - §10 Code: function bodies with expanded local declarations
  - §11 Data: memory initialisation segments
- Internal `Parser` struct with cursor-tracked position for precise error offsets.
- `read_expr` helper that reads constant expressions (init_expr / offset_expr) byte-by-byte
  until the `end` opcode (0x0B), with correct immediate parsing for i32.const, i64.const,
  f32.const, f64.const, and global.get.
- 28 unit tests covering all sections, all error cases, and a round-trip test.
- Literate programming style: ASCII format diagrams, per-section explanations, and
  Knuth-style inline documentation throughout.
