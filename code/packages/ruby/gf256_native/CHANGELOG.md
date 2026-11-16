# Changelog — coding_adventures_gf256_native

All notable changes to this package are documented here.

## Unreleased - complete the Windows import-library symbol list

- `ext/**/build.rs` generates the MSVC import library for Ruby's DLL from a hand-written list of
  `rb_*` symbols. The list is now a superset of every symbol `ruby-bridge` declares (28 functions
  and 4 data symbols), so a symbol the bridge uses can never be missing at link time. Before, a
  crate whose list lagged the bridge failed to link on Windows with `rust-lld: undefined symbol:
  rb_str_new` / `rb_hash_new` / ... (`ruby/graph_native` in the scheduled full build; the same cause
  as the earlier `bitset_native`, `tree_set_native` and `board_vm_native` fixes). Every added name
  is exported by libruby. Validated by `cargo check` on Linux and by checking each name against
  libruby's exports; the Windows link itself is proven by the next daily Windows full build.

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
