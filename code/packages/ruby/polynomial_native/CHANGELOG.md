# Changelog — coding_adventures_polynomial_native

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

- Initial release: Ruby native extension wrapping the `polynomial` Rust crate.
- Module `CodingAdventures::PolynomialNative` with twelve module functions:
  - `normalize(poly)` — strip trailing near-zero coefficients
  - `degree(poly)` — return the degree (index of highest non-zero term)
  - `zero` — return the zero polynomial `[0.0]`
  - `one` — return the unit polynomial `[1.0]`
  - `add(a, b)` — term-by-term polynomial addition
  - `subtract(a, b)` — term-by-term polynomial subtraction
  - `multiply(a, b)` — polynomial multiplication via convolution
  - `divmod_poly(a, b)` — polynomial long division, returns `[quotient, remainder]`
  - `divide(a, b)` — returns the quotient of polynomial division
  - `modulo(a, b)` — returns the remainder of polynomial division
  - `evaluate(poly, x)` — evaluate using Horner's method (O(n), no exponentiation)
  - `gcd(a, b)` — greatest common divisor via the Euclidean algorithm
- Polynomial representation: Ruby `Array<Float>` where index equals degree.
- `divmod_poly`, `divide`, and `modulo` raise `ArgumentError` (not process abort)
  when the divisor is the zero polynomial, via `std::panic::catch_unwind`.
- Built via `cargo build --release` with zero dependencies beyond libruby.
- 40+ test cases covering fundamentals, arithmetic, division, evaluation, GCD,
  algebraic identities, and error conditions.
