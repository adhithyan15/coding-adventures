# Changelog — coding_adventures_gf256 (Elixir)

All notable changes to this package will be documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.2.0] — 2026-04-11

### Added

- `CodingAdventures.GF256Field` struct — parameterizable field for any primitive polynomial.
  Uses Russian peasant (shift-and-XOR) multiplication; no log/antilog tables stored.
  - `new_field/1` creates a `%GF256Field{polynomial: poly}` struct.
  - `GF256.new_field(0x11B)` creates the AES GF(2^8) field.
  - Field-aware 3-arity overloads: `multiply/3`, `divide/3`, `power/3`, `inverse/2`,
    `add/3`, `subtract/3` — accept `%GF256Field{}` as first argument.
- Tests: AES field `multiply(0x53, 0xCA) = 0x01`, FIPS 197 `multiply(0x57, 0x83) = 0xC1`,
  RS backward-compat verification, commutativity, error cases.

## [0.1.0] — 2026-04-03

### Added
- `CodingAdventures.GF256` module with full GF(2^8) field arithmetic
- Log and antilog tables built at compile time using `:array` and `@module` attributes
- Primitive polynomial `0x11D` (x^8 + x^4 + x^3 + x^2 + 1) — Reed-Solomon standard
- `add/2` and `subtract/2` — both implemented as XOR (characteristic-2 field)
- `multiply/2` — log/antilog table lookup, O(1)
- `divide/2` — log/antilog table lookup, raises `ArgumentError` on zero divisor
- `power/2` — integer exponentiation via log table
- `inverse/1` — multiplicative inverse, raises `ArgumentError` for zero
- `zero/0` and `one/0` — field identity constants
- `alog_table/0` and `log_table/0` — table accessor functions for testing
- 45+ ExUnit tests including full inverse correctness check for all 255 non-zero elements
- Knuth-style literate programming comments throughout
