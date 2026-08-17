# Changelog

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
