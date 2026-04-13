# Changelog — coding_adventures_gf256_native

All notable changes to this package are documented here.

## [0.1.0] — 2026-04-03

### Added

- Initial release: Ruby native extension wrapping the `gf256` Rust crate.
- Module `CodingAdventures::GF256Native` with six module functions:
  - `add(a, b)` — GF(256) addition (= XOR, characteristic 2)
  - `subtract(a, b)` — GF(256) subtraction (= XOR, same as add)
  - `multiply(a, b)` — GF(256) multiplication via log/antilog tables (O(1))
  - `divide(a, b)` — GF(256) division; raises `ArgumentError` if b == 0
  - `power(base, exp)` — GF(256) exponentiation; handles 0^0 = 1 by convention
  - `inverse(a)` — multiplicative inverse; raises `ArgumentError` if a == 0
- Three module constants:
  - `ZERO = 0` — the additive identity
  - `ONE = 1` — the multiplicative identity
  - `PRIMITIVE_POLYNOMIAL = 285` (= 0x11D = x^8 + x^4 + x^3 + x^2 + 1)
- All inputs validated as Ruby Integers in range 0..=255 with informative error messages.
- `divide` and `inverse` raise `ArgumentError` (not process abort) via `std::panic::catch_unwind`.
- Built via `cargo build --release` with zero dependencies beyond libruby.
- 40+ test cases covering all operations, algebraic identities (distributive law,
  Fermat's little theorem), error conditions, and known test vectors.
