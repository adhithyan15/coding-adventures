# Changelog

## [0.3.0] — 2026-06-10 — McCarthy predicates: isinst + xor + ref-aware compares (W7)

- `isinst <typeTok>` (0x75): the `pair?` type test — keep a heap `object[]` ref,
  else push `null` (the CLR twin of JVM `instanceof` / wasm `ref.test`).
- `xor` (0x61): McCarthy logical `not` lowers to `x ^ 1`.
- Comparison opcodes (`ceq`/`cgt`/`clt`) are now **reference-aware** via a new
  `as_cmp_int` (null→0, heap ref→1) so `pair?`/`is_null` can `ceq` a reference
  against `ldnull` without the strict arithmetic guard firing.
- **Fix:** `ldnull` is `0x14` (was mis-defined as `0x01` = `break`); a latent bug
  the cons path never hit but `pair?`/`is_null` do. Existing consumers unaffected.

All notable changes to this project will be documented in this file.

## [0.2.0] - 2026-06-10 — object/reference value model (LANG77 / McCarthy W6b)

### Added

- **A `Value` stack model + object heap.** A stack/local slot is now
  `Option<Value>`, where `Value` is `Int(i32)` or `Ref(Option<usize>)`
  (`Ref(None)` = `null`, `Ref(Some(i))` = an index into the new object `heap`).
  This lets the simulator execute **reference types**, not just `i32`.
- **Reference-type opcodes** for the IIR→CIL `System.Object[]` cons cells:
  `newarr` (0x8D), `stelem.ref` (0xA4), `ldelem.ref` (0xA2), `dup` (0x25), and
  `box` (0x8C) / `unbox.any` (0xA5) as identity in this loose model (the boxed
  `Int` roundtrips through the array, like the wasm engine's `i31` box/unbox).
- 2 new tests: an `object[]` cons roundtrip (`[7,9]` → read back `7`) and
  `ldnull` → null-is-falsy.

### Changed

- `CLRSimulator.stack` / `.locals` / `CLRTrace` fields are `Vec<Option<Value>>`
  (were `Vec<Option<i32>>`); arithmetic/comparison/branch behaviour for integers
  is **unchanged** (`Value::Int` wraps the old payload). Consumers reading the
  stack now compare against `Some(Value::Int(n))`.

### Added

- `CLRSimulator` -- type-inferring stack-based virtual machine with nullable values
- Load/store: ldc.i4 (compact 0-8, short -128..127, full 32-bit), ldloc/stloc
- Arithmetic: add, sub, mul, div with DivideByZeroException detection
- Control flow: br.s, brfalse.s, brtrue.s
- Two-byte comparison opcodes: ceq, cgt, clt via 0xFE prefix
- Special: nop, ldnull (nullable stack support), ret
- Encoding helpers and assembler
