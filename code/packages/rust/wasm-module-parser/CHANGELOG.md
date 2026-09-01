# Changelog

All notable changes to this package will be documented in this file.

## [0.2.15] — 2026-09-01 — track whether a binary module declared a data count section

Real corpus bug found during a fresh prioritization pass (`code/specs/
W07-wasm-post-mvp-epics.md`'s "Addendum (2026-09-01)" item 2):
`binary.wast` has two `assert_malformed` cases — "memory.init requires a
data count section" and "data.drop requires a data count section" — that
this crate's own parse always accepted, since nothing anywhere enforced
that rule.

### Added

- `WasmModule::missing_data_count_section` (new field, defined in
  `wasm-types` — see that crate's own CHANGELOG for the field's full
  design rationale). This crate sets it to `data_count.is_none()` right
  after the existing data-count/data-section-length cross-check, once
  parsing is otherwise complete.

### Notes

- The actual "reject `memory.init`/`data.drop` without one" enforcement
  does NOT live in this crate — this crate never walks function-body
  instructions byte-by-byte (`parse_code_section` stores `code` raw,
  deliberately, to avoid a byte-pattern scan for `0xFC 0x08`/`0xFC 0x09`
  that could false-positive on some OTHER instruction's raw immediate
  bytes coincidentally matching that pair, e.g. an `f64.const`'s literal
  8 bytes). `wasm-validator`'s type-checker already walks every
  instruction precisely for real type-checking and already has dedicated
  `0x08`/`0x09` arms — see its own CHANGELOG for where the actual gate
  lives.
- 2 new regression tests (`src/lib.rs`): the flag is `true` when §12 is
  absent, `false` when present (and agreeing with the data section, so
  the module parses at all).

## [0.2.14] — 2026-09-01 — W33 fourth slice: `FieldType.val_type` → `FieldType.storage` rename fallout

No functional change in this crate — `wasm-types` 0.1.17 renamed
`FieldType.val_type: ValueType` to `FieldType.storage: StorageType`
(adding packed `i8`/`i16` field storage support); `parse_struct_type`'s
one `FieldType` construction site updated to `FieldType::plain(...)`.
Binary-format struct field decoding still only ever produces
`StorageType::Val(_)` (packed `i8`/`i16` binary tags are not decoded by
this crate — out of scope for this slice, see the accompanying spec
addendum). All 67 tests in this crate pass.

## [0.2.13] — 2026-08-31 — security review follow-up: `read_expr` byte-stream desync fix

### Fixed

- `read_expr` (used for every `global`/`data`/`element` constant/offset
  expression) had no immediate-skipping arm for `ref.func` (`0xD2`) or
  `ref.null` (`0xD0`) — its catch-all just kept scanning for the next
  `0x0B` byte, so either instruction's own immediate bytes were
  misinterpreted as the START of the next "instruction" instead of being
  consumed as part of the current one. Found by a security review of the
  W32 second slice (which is what first made `ref.func`/`ref.null`
  reachable inside a constant expression at all — before that slice they
  simply trapped at instantiation with "illegal opcode", so the bug was
  real but unreachable). Demonstrated two concrete failure modes:
  - A funcidx or concrete type-index whose LEB128 encoding happens to
    include the byte `0x0B` (e.g. funcidx 11) is misread as an early
    `end`, so a spec-valid module like `(global funcref (ref.func 11))`
    is rejected with a bogus "section size mismatch" error instead of
    parsing.
  - Worse: the genuinely leftover trailing byte(s) can be silently
    absorbed into whatever data follows the expression in the same
    section (e.g. a data segment's payload), producing a **different**,
    still-successfully-parsed result than a spec-conformant parser would
    read from the identical input bytes — a differential-parsing
    vulnerability, not merely a spurious rejection.

  Fixed by adding real immediate-consuming arms for both opcodes: `0xD2`
  reuses `read_u32leb` (bounded, overlong/out-of-range-rejecting, same as
  `global.get`'s existing `0x23` arm); `0xD0` reuses `read_value_type`
  (already correctly handles both the 1-byte abstract-heap-type form and
  the 2-byte `0x63`/`0x64 <typeidx>` concrete-type form). New regression
  tests reproduce both the funcidx and the concrete-typeidx variant of
  the byte-`0x0B`-collision case directly against `parse_global_section`.
- `evaluate_const_expr`'s `ref.func` (`0xD2`) arm (`wasm-execution`) used
  the raw, unbounded `decode_unsigned` (a `u64` decoder) then narrowed
  with `idx as u32`, which silently truncates rather than rejecting: a
  crafted funcidx like `0x1_0000_0003` decoded to `Ref(Some(3))` — an
  out-of-range reference silently *aliasing* a real, different,
  legitimate function 3, instead of being cleanly rejected as malformed.
  The arm's own comment also incorrectly claimed `wasm-validator`
  already bounds-checks every `ref.func` funcidx inside global init
  expressions — independently verified false (zero production reads of
  `globals[..].init_expr`/`elements[..].offset_expr` exist in
  `wasm-validator` outside test fixtures) and corrected. Fixed by
  switching to `decode_unsigned_bounded(.., 32)`, matching every other
  32-bit index space this crate decodes; a downstream `.get()`-shaped
  bounds check at the point of use (`call_ref`, a table read, …) still
  safely rejects any resulting out-of-range-but-32-bit-representable
  index — this fix closes the *silent truncation*, not a missing
  bounds check that was never this function's job to begin with.
- `call_ref`'s (`0x14`) callee-existence check (`wasm-execution`) lived
  inside the `if let Some(expected) = ctx.types.get(type_idx)` block, so
  an unresolvable `type_idx` (out of range, or an engine built with no
  type section — exactly what this crate's own `engine_from_wat` test
  helper constructs) silently skipped it entirely, falling through to
  `call_function` with an unvalidated function index. The sibling
  `return_call_ref` (`0x15`) already ran the equivalent check
  unconditionally; `call_ref` is now consistent with it. No panic
  either way (`call_function_inner` independently re-checks), but the
  skip meant `call_ref` was silently weaker than its own tail-call
  twin for no reason.

## [0.2.12] — 2026-08-31 — W32 second slice: non-null concrete ref decoding

### Added

- `read_value_type` (struct FIELD context) now recognizes `0x64
  <typeidx>` — the non-null counterpart to the existing `0x63` nullable
  tag, decoding to `ValueType::NonNullStructRef` — independently verified
  against the real reference interpreter's own `interpreter/binary/
  decode.ml` (`ref_type`'s `-0x1c -> (NoNull, heap_type s)` arm: `-28 mod
  128 = 0x64`). Like `0x63`, only reachable through this struct-field
  path in this crate — function params/results/locals/globals still
  decode through the single-byte `decode_value_type`, a pre-existing
  limitation unrelated to this slice (see `REF_NON_NULL_CONCRETE_TAG`'s
  own doc comment).

## [0.2.11] — 2026-08-31 — W32 first slice: bottom reference-type decoding

### Added

- `decode_value_type` recognizes the four W32-first-slice bottom
  reference-type bytes (`0x73`/`0x72`/`0x74`/`0x71` →
  `NullFuncref`/`NullExternref`/`NullExnref`/`NullRef`), independently
  verified against the real reference interpreter's own
  `interpreter/binary/decode.ml`. Also closes a small pre-existing gap
  alongside them: `funcref`/`externref`/`exnref` (`0x70`/`0x6F`/`0x69`)
  were previously only recognized in table-element-type/heap-type-
  immediate contexts, never as a plain value-type byte, so a
  `(module binary ...)` type section with a `(result funcref)`-shaped
  signature couldn't decode — now every `ValueType::encode()` output
  round-trips through this decoder byte-for-byte.

## [0.2.10] — 2026-08-31 — LEB128/malformed-binary hardening pass

Vendored the official testsuite's `binary.wast`/`binary-leb128.wast`/
`binary_leb128_64.wast` — dense `assert_malformed` corpora specifically
targeting the binary format's LEB128 encoding rules and other structural
malformed-binary shapes. Found and fixed several distinct, genuine
input-validation gaps: a WASM interpreter that doesn't correctly reject a
malformed binary isn't just failing a conformance test, it's a real
robustness gap (a malformed-but-accepted module could smuggle unintended
behavior past parsing).

### Fixed — LEB128 malformed-encoding classes

- **`read_u32leb` silently truncated out-of-range values.** Every `u32`
  field in the format (section sizes, every vector count, every index
  space, string/name lengths, …) went through `decode_unsigned` (no
  byte-count cap beyond the generic 10-byte/`u64` limit) followed by
  `val as u32`, which wraps instead of erroring. A value like `2^32`
  (one bit too many for `u32`, comfortably inside `decode_unsigned`'s own
  range) silently became `0`. Now routed through `wasm-leb128`
  0.2.0's new `decode_unsigned_bounded(.., 32)`, which rejects both this
  ("integer too large") and an overlong (>5-byte) encoding ("integer
  representation too long") — see that crate's own changelog for the
  shared fix underneath.
- **`read_expr`'s `i32.const`/`i64.const` immediates were decoded
  UNSIGNED** (`decode_unsigned`) even though both are spec-signed (`s32`/
  `s64`) immediates. Byte-consumption happened to match either way (the
  two decoders share the same continuation-bit loop), which is why this
  was invisible for any well-formed input — but it meant a deliberately
  badly-padded encoding (e.g. `i32.const -1` with its high padding bits
  left unset instead of properly sign-extended) parsed as valid instead
  of being rejected. `i32.const` now goes through
  `decode_signed_bounded(.., 32)`; `i64.const` through `decode_signed`
  (native 64-bit, now itself overflow-checked). `global.get`'s index
  immediate (always unsigned) now goes through
  `decode_unsigned_bounded(.., 32)` for the same overlong/out-of-range
  protection every other index space already needed.

### Fixed — structural malformed-binary classes

- **Unrecognized section ids were silently skipped.** The old code's own
  doc comment claimed this was deliberate spec forward-compatibility;
  it isn't — the binary format defines exactly ids 0–12, and anything
  else (`0x0E`, `0x7F`, `0x80`, …) is malformed, not a future extension
  to tolerate.
- **No section-ordering enforcement at all.** Numbered sections (1–11,
  plus the bulk-memory proposal's DataCount at byte id 12) must appear
  at most once, in a fixed canonical order — `canonical_section_order`
  maps each id to its position in that order (`DataCount`'s byte id, 12,
  is numerically LARGER than `Code`'s, 10, but must appear BEFORE it —
  see that function's own doc comment for why comparing raw id bytes
  would be wrong). A repeated or out-of-order section is now rejected;
  Custom (id 0) is exempt, as always (any number, anywhere).
- **No section-size-mismatch check.** A section's declared `size` was
  taken on faith — bytes left unconsumed inside a section's own declared
  boundary after its parser ran were silently dropped instead of
  flagged. (Surfaced, while fixing this, two PRE-EXISTING and genuinely
  invisible-until-now decode bugs elsewhere in this file: the element
  section's mode-2 [explicit-table-index active segment] entries were
  missing their `elemkind` byte read entirely, silently desyncing every
  subsequent field by one byte; and a table entry whose leading byte is
  `0x40` [function-references proposal's "table with an explicit init
  expression" form] was being misread as an ordinary two-field table
  entry instead of the differently-shaped one it actually is. The first
  is now fixed for real; the second now fails loudly with a clear
  "not yet supported" message instead of silently misparsing — full
  support needs a `wasm-types`/`wasm-runtime` init-expression feature
  this crate doesn't have yet.)
- **Function/code section length mismatch was never checked** — the
  function section declares one type index per function, the code
  section one body per function; the same index space, so they must
  agree, the same shape as the existing data-count-vs-data-section
  cross-check.
- **A function body wasn't required to end in the `end` (0x0B) opcode.**
  `body_size`, taken at face value, let a body whose declared last byte
  is something else (e.g. `drop`) parse as an ordinary, if truncated,
  function.
- **`limits` flags accepted any byte value** — only bits 0 (has-max), 1
  (shared), 2 (64-bit index) are defined by any proposal this crate
  supports; a stray bit elsewhere (`0x08`, `0x10`, `0x81`, …) silently
  parsed identically to whatever the recognized bits happened to say.
- **No cap on a function body's total local count** — same
  attacker-controlled-tiny-file-claims-enormous-allocation shape
  `MAX_PREALLOC` already guards against for the element section's
  `func_count`, just summed across a body's run-length-encoded local
  groups instead of a single field. New `MAX_LOCALS` (1,000,000) cap,
  checked incrementally so the huge allocation itself never happens.

### Added

- 24 new unit tests, one per malformed-encoding/structural class above,
  living directly in this crate's own test suite (not just implicitly
  covered by the vendored corpus).

### Deliberately not fixed here

- A `memarg`'s `align`/`offset` LEB128 immediates, and an extended
  (`0xFC`/`0xFD`-prefixed) opcode's own LEB128 sub-number, live INSIDE a
  function body — which this crate still reads as an opaque raw byte
  blob after its locals (`parse_code_section`), deferring all
  instruction-level decoding to a future `wasm-validator` walker (see
  that crate's "no instruction-level type-checker yet" doc comment).
  Fixing these specific LEB128 fields correctly would require building
  that walker (a per-opcode immediate-shape table for every instruction,
  not just memory ops, since skipping past every OTHER instruction to
  find the next one requires knowing its shape too) — a much larger,
  separate architectural undertaking than LEB128 boundary validation.
  `binary_leb128_64.wast`'s one `assert_malformed` case and 7 of
  `binary-leb128.wast`'s live here; everything else in both files (and
  in `binary.wast`) is fixed.
- A handful of `binary.wast` cases that need real per-opcode
  instruction-level decoding for the same reason (an illegal top-level
  opcode byte, unbalanced block/loop/if nesting, detecting a
  `memory.init`/`data.drop` opcode's presence to require a DataCount
  section) are likewise deferred to that same future work.

## [0.2.9] — 2026-08-26 — W26: table64 proposal, first slice

### Changed

- The table section's and table import's `parse_limits` call sites no
  longer reject binary `limits` flags bit `0x04` (64-bit index) — it's now
  wired into `TableType::is64` (`wasm-types` 0.1.11), mirroring the memory
  arm exactly. Previously a deliberate, named W25 rejection ("table64 is
  not supported by this parser").

See `code/specs/W26-wasm-table64-first-slice.md`.

## [0.2.8] — 2026-08-26 — W25: memory64 proposal, first slice

### Added

- `read_u64leb`: decodes an unsigned LEB128 `u64`, needed for a 64-bit
  memory's `min`/`max` limits (up to `2^48` pages, past `u32`'s range).

### Changed

- `parse_limits` now recognizes binary `limits` flags bit `0x04` (64-bit
  index, memory64 proposal) alongside the existing bit 0 ("has max") and
  bit 1 ("shared", W18) — `min`/`max` decode as `u64leb` instead of
  `u32leb` when set. Returns `(Limits, shared, is64)` (was `(Limits,
  shared)`). Verified live against the real spec's binary grammar
  (`https://webassembly.github.io/spec/core/binary/types.html`) rather
  than assumed. A table's limits flags byte with bit `0x04` set is
  rejected with a clear parse error ("table64 ... is not supported by
  this parser") instead of silently misinterpreted — `table64` (the
  analogous widening for tables) is a separate, out-of-scope proposal.
- `parse_memory_section`/import-section's memory arm thread the new
  `is64` bool into `MemoryType::is64` (`wasm-types` 0.1.10).

See `code/specs/W25-wasm-memory64-first-slice.md`.

## [0.2.7] — 2026-08-25 — W21: exceptions proposal, tag/throw first slice

### Changed

- `parse_import_section`'s exhaustive `match kind { ... }` gained an
  `ExternalKind::Tag` arm (a clean parse error, "tag imports are not
  supported by the binary module parser") — `decode_external_kind` still
  only recognizes bytes `0x00`-`0x03`, so this arm is unreachable in
  practice; it exists only so the match stays exhaustive now that
  `ExternalKind` (`wasm-types` 0.1.7) has a 5th variant. Real BINARY
  tag-section/tag-import decoding stays out of scope for this slice —
  `wasm-wast-parser`'s text pipeline (this repo's real corpus entry
  point) never round-trips through this crate at all, matching the GC
  epic's W20 precedent. See `code/specs/
  W21-wasm-exceptions-tag-throw-slice.md`.

## [0.2.6] — 2026-08-17 — decode all 4 real element-segment modes (task #97)

### Changed

- `parse_element_section` rewritten to decode the real binary flags
  byte and dispatch across the 4 element-segment modes this repo now
  represents (0/1/2/5 -- see `wasm-types`' own CHANGELOG entry for the
  full mode list and the real-corpus census that scoped it), instead
  of only ever reading mode 0 (active, implicit table 0, funcidx-list).
  Modes 1/2/5 previously had no decode path at all.
- New `read_elem_expr_entry` helper decodes a single exprs-list entry
  (`ref.func`/`ref.null`) directly -- NOT via the generic `read_expr`,
  which doesn't know to stop at these two opcodes' own immediates and
  would misparse the entry.



### Fixed

- `parse_data_section` used to read a leading LEB128 unconditionally as
  `memory_index` and always read an offset expression next -- decoding
  only mode 0 (active, implicit memory 0) correctly. It happened to
  "work" for every module this repo ever produced only because mode 0's
  flag byte (`0x00`) is bit-identical to `memory_index = 0` encoded as a
  LEB128. It could not have decoded a real mode 1 (passive -- no offset
  expression present at all, so the following data bytes would have
  been misread as one) or mode 2 (active with an explicit nonzero
  memory index -- the flag isn't the memory index at all) segment
  correctly. Now decodes the real segment-mode flag and branches on all
  3 real encodings; any other flag value is a clean `WasmParseError`
  (a real spec violation), not silently misparsed.

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
