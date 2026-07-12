# Changelog

All notable changes to the C++ `symbolic-ir` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.2.0] - 2026-07-12

### Added

- Initial header-only pure-ISO C++17 port of the Rust `symbolic-ir` crate
  (namespace `ca::symbolic_ir`; version tracks the Rust crate at 0.2.0).
- Value-semantic `Node` with six variants (Symbol, Integer, Rational, Float,
  Str, Apply); the recursive Apply payload is shared via `std::shared_ptr`.
- Factories `Node::symbol/integer/rational/floating/str/apply` and the free
  helpers `sym` / `integer` / `rat` / `flt` / `str_node` / `apply`.
- `Node::rational`: GCD reduction, sign moved to the numerator, collapse to
  Integer when the denominator reduces to 1; throws `std::invalid_argument` in
  place of the Rust panic; two's-complement magnitude arithmetic avoids
  `INT64_MIN` signed-overflow UB.
- Structural `operator==` (floats by bit pattern), a consistent FNV-1a `hash()`,
  and a recursive `to_string()` with shortest round-tripping float formatting.
- The standard head-name vocabulary as `constexpr const char*` constants.
- 46 checks against the Rust crate's reference behavior, run under every
  available C++ compiler via the shared `iso-harness`.
