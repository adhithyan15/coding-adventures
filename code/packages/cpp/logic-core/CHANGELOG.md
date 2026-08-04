# Changelog

All notable changes to the C++ `logic-core` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-13

### Added

- Initial header-only pure-ISO C++17 port of the Rust `logic-core` crate
  (namespace `ca::logic_core`) — terms, substitutions, and first-order
  unification.
- `Term` as a `std::variant` tree (Atom / Number / Str / LogicVar / Compound),
  `Number` as `std::variant<std::int64_t, double>` (so `1` ≠ `1.0`), with
  constructors `atom`, `integer`, `real`, `string`, `var` / `var_term`,
  `compound`, `logic_list`, plus `to_string` and value equality.
- `Substitution` with `empty`, `extend` (copy-on-extend), `walk` / `walk_var`,
  `size`, `occurs`, and equality; `unify(a, b, s)` →
  `std::optional<Substitution>` with the occurs-check.
- Faithful divergences: `integer`/`real` builders (Rust `int`/`float` are
  keywords), `static` variable-id counter (vs `AtomicU64`), `%g` float display.
- 24 checks mirroring the Rust crate's own unit tests, run under every available
  C++ compiler via the shared `iso-harness`; the suite also passes clean under
  AddressSanitizer + UndefinedBehaviorSanitizer.
