# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-25

### Added

- `Bitset` class: compact boolean array packed into 64-bit `Long` words.
- LSB-first bit ordering: bit 0 is the least significant bit of word 0.
- **Clean-trailing-bits invariant**: bits beyond `length()` in the last word
  are always zero, ensuring correctness without special-casing partial words.
- Constructor: `Bitset(size: Int)` — zero-filled bitset of given length.
- Factory: `Bitset.fromInteger(Long)` — from unsigned 64-bit value; uses
  `countLeadingZeroBits()` to compute logical length.
- Factory: `Bitset.fromBinaryStr(String)` — from binary string (MSB left,
  LSB right); throws `IllegalArgumentException` on invalid characters.
- `set(i)` — sets bit; auto-grows using doubling strategy (amortised O(1)).
- `clear(i)` — clears bit; no-op if `i >= length()`.
- `test(i)` — returns `false` if `i >= length()`.
- `toggle(i)` — flips bit; auto-grows; cleans trailing bits.
- `and`, `or`, `xor`, `not`, `andNot` — bulk bitwise; each returns a new
  `Bitset` with length = `max(a.length(), b.length())`.
- `popcount()` — uses `Long.countOneBits()` (hardware POPCNT on modern JVMs).
- `length()`, `capacity()` — logical and allocated sizes.
- `any()`, `all()`, `none()` — query methods; `all()` uses vacuous truth for
  empty bitsets.
- `iterSetBits()` — returns `List<Int>` of set bit indices in ascending order
  using the trailing-zero-count / lowest-bit-clear trick.
- `toInteger()` — returns `Long` (unsigned); throws `ArithmeticException` if
  bits beyond position 63 are set.
- `toBinaryStr()` — conventional binary string (MSB on left).
- `equals()`, `hashCode()`, `toString()` overrides.
- **Implementation note**: internal fields are named `_words` and `_size` to
  avoid Kotlin naming conflicts between `var length: Int` and `fun length(): Int`
  in the same class.
- 43 Kotlin/JUnit 5 tests covering: constructors, single-bit ops, auto-growth,
  bulk bitwise ops, clean-trailing-bits invariant, counting/query, iteration,
  conversion roundtrips, equality, and edge cases.
