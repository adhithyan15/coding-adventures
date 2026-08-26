# Changelog

- 0.2.5 (W25 — memory64 proposal, first slice): `encode_memory_type`
  recognizes `MemoryType::is64` (`wasm-types` 0.1.10), emitting binary
  `limits` flags bit `0x04` and `u64leb`-encoded `min`/`max` for a 64-bit
  memory (verified live against the real spec's binary grammar, matching
  `wasm-module-parser`'s decode side). New `encode_u64` helper.
  `encode_limits` (the TABLE-only encoder — memory needs its own, since
  tables never carry `is64`) narrows `Limits.min`/`max` back to `u32` at
  the call site, safe because `table64` is a separate, out-of-scope
  proposal. Round-trip test added
  (`encodes_memory64_flag_and_wide_limits_round_trip`) using a `min`/`max`
  value genuinely past `u32::MAX`. See `code/specs/
  W25-wasm-memory64-first-slice.md`.

- 0.2.4 (W21 — exceptions proposal, tag/throw first slice): `encode_import`'s
  exhaustive `(ExternalKind, ImportTypeInfo)` match gained a
  `(ExternalKind::Tag, ImportTypeInfo::Tag(type_index))` arm (encodes the
  tag's type index the same way a function import's type index is
  encoded) plus its own mismatch-fallback arm, purely so the workspace
  keeps compiling now that both enums gained a `Tag` variant (`wasm-types`
  0.1.7). This crate's own text-to-binary path isn't exercised by
  `wasm-wast-parser`'s real corpus pipeline (that crate builds
  `WasmModule` directly, never through this encoder), matching the GC
  epic's W20 precedent of not touching this crate for a proposal that
  otherwise only needed real coverage in the text-parser/execution/
  validator layers. See `code/specs/W21-wasm-exceptions-tag-throw-slice.md`.
- 0.2.3 (task #97): `encode_element` rewritten to write the real binary
  flags byte and dispatch across the 4 element-segment modes this repo
  now represents (0/1/2/5), the encoder-side counterpart to
  `wasm-module-parser`'s decode fix -- previously only ever wrote mode
  0 (active, implicit table 0, funcidx-list) regardless of
  `Element.is_passive`/the new exprs-list shape. Uses `unwrap_or(0)` as
  a defensive-but-non-panicking fallback for the structurally-
  unreachable case of an ACTIVE segment containing a `None` (`ref.null`)
  entry -- this crate's only callers construct `Element` from Rust
  code, never from parsed external/attacker bytes, so this can't
  actually happen, but a silent wrong-but-valid encoding is safer here
  than a panic either way.

- 0.2.2 (task #95): `encode_data_segment` now writes the real segment-
  mode flag (`1` for a passive segment, `0` + the offset expression for
  active) instead of unconditionally writing `memory_index` then an
  offset expression -- the encoder-side counterpart to
  `wasm-module-parser`'s decode fix. A passive segment round-tripped
  through this encoder before this fix would have silently become a
  bogus active segment with an empty offset expression.

- 0.2.1 (WASM18): Fix `encode_memory_type` -- it only ever called
  `encode_limits`, which has no way to express flags-byte bit 1
  ("shared", threads proposal). Any `MemoryType { shared: true, .. }`
  encoded to binary silently lost the shared flag on the wire, even
  though `wasm-module-parser` 0.2.2 correctly *decodes* that same bit.
  `encode_memory_type` now writes its own flags byte
  (`has_max | shared << 1`), mirroring `parse_limits`'s decode exactly.
  Caught by writing a `shared: true` round-trip test -- the existing
  memory round-trip test only ever used `shared: false`, which can't
  distinguish "bit never written" from "bit correctly written as 0". 2
  new tests (bare memory section + memory import), both verified to
  fail without the fix via a temporary revert.
- 0.2.0 (LANG77 / McCarthy L3b-3a-4): Add `GcInstruction::RefTest(typeidx)` /
  `RefTestNull(typeidx)` — the WasmGC `ref.test (ref $t)` (`0xFB 0x14 <typeidx>`)
  and its nullable variant (`0xFB 0x15 <typeidx>`), which McCarthy `pair?`
  emits to test whether a lisp value is a `$LispyPair` cons cell. 1 new test.
- 0.1.0: Initial release.
