# Changelog

All notable changes to the C++ `lisp-parser` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Added

- `SExpr` child accessors `child_count` / `child` / `dotted_last` /
  `quoted_inner`, so tree walkers (e.g. the `lisp-compiler`) can traverse the
  AST without reaching into the `detail` namespace.

## [0.1.0] - 2026-07-13

### Added

- Initial header-only pure-ISO C++17 port of the Rust `lisp-parser` crate
  (namespace `ca::lisp_parser`) — a recursive-descent parser producing an
  S-expression AST. Depends on the sibling header-only `cpp/lisp-lexer`.
- `parse(source)` and `parse_tokens(tokens)` → `std::vector<SExpr>`; throws
  `ParseError` (a `std::runtime_error`) on a lexer or syntax error.
- `SExpr` as a move-only `std::variant<Atom, List, DottedPair, Quoted>` (owning
  single children via `std::unique_ptr`), with `kind()`, `atom_kind()` /
  `atom_value()`, `find_atoms()`, `count_lists()`, `count_quoted()`.
- 29 checks mirroring the Rust crate's own unit tests, run under every available
  C++ compiler via the shared `iso-harness`; the suite also passes clean under
  AddressSanitizer + UndefinedBehaviorSanitizer.
