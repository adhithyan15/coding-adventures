# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-25

### Added

- `Bitset` class: compact boolean array packed into 64-bit `long` words.
- LSB-first bit ordering: bit 0 is the least significant bit of word 0.
- **Clean-trailing-bits invariant**: bits beyond `length` in the last word are
  always zero, ensuring correctness of `popcount`, `any`, `all`, `none`,
  `equals`, and `toInteger` without special-casing partial words.
- Constructor: `new Bitset(int size)` — zero-filled bitset of given length.
- Factory: `Bitset.fromInteger(long value)` — from unsigned 64-bit value;
  uses `Long.numberOfLeadingZeros` to compute logical length.
- Factory: `Bitset.fromBinaryStr(String s)` — from binary string (MSB left,
  LSB right); throws `IllegalArgumentException` on invalid characters.
- `set(int i)` — sets bit; auto-grows using doubling strategy (amortised O(1)).
- `clear(int i)` — clears bit; no-op if `i >= length`.
- `test(int i)` — returns `false` if `i >= length` (unallocated bits are zero).
- `toggle(int i)` — flips bit; auto-grows; cleans trailing bits afterwards.
- `and(Bitset)`, `or(Bitset)`, `xor(Bitset)`, `not()`, `andNot(Bitset)` —
  bulk bitwise operations; each returns a new `Bitset` with length =
  `max(a.length, b.length)`.
- `popcount()` — uses `Long.bitCount(long)` (compiles to hardware POPCNT).
- `length()`, `capacity()` — logical and allocated sizes.
- `any()`, `all()`, `none()` — query methods; `all()` uses vacuous truth for
  empty bitsets.
- `iterSetBits()` — returns `List<Integer>` of set bit indices in ascending
  order using the trailing-zero-count / lowest-bit-clear trick.
- `toInteger()` — returns `long` (unsigned); throws `ArithmeticException` if
  bits beyond position 63 are set.
- `toBinaryStr()` — conventional binary string (MSB on left).
- `equals(Object)`, `hashCode()`, `toString()` overrides.
- 35 JUnit 5 tests covering: constructors, single-bit ops, auto-growth, bulk
  bitwise ops, clean-trailing-bits invariant, counting/query, iteration,
  conversion roundtrips, equality, and edge cases.
