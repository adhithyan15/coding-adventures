# Changelog

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
