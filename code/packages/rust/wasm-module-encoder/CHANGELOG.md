# Changelog

- 0.2.0 (LANG77 / McCarthy L3b-3a-4): Add `GcInstruction::RefTest(typeidx)` /
  `RefTestNull(typeidx)` — the WasmGC `ref.test (ref $t)` (`0xFB 0x14 <typeidx>`)
  and its nullable variant (`0xFB 0x15 <typeidx>`), which McCarthy `pair?`
  emits to test whether a lisp value is a `$LispyPair` cons cell. 1 new test.
- 0.1.0: Initial release.
