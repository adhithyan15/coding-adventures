# Changelog

## [0.1.1] - 2026-07-11

### Fixed

- **DoS hardening**: `create_macsyma_parser` now opts the underlying
  `GrammarParser` into a recursion-depth cap (`MAX_RULE_DEPTH = 200`) via
  `.with_max_depth(...)`. Previously the parser recursed once per nested
  `(...)` layer with no limit; deeply-nested input (`((((…))))`, thousands of
  levels) could overflow the *native* thread stack — an uncatchable process
  abort — before ever reaching a `Result`-returning entry point. Now such
  input cleanly returns a `GrammarParseError` instead of crashing the host
  process.
- The `200` cap was derived empirically (not copied from
  `parser::grammar_parser::DEFAULT_MAX_RULE_DEPTH`, which is tuned for a much
  shallower ECMAScript-shaped grammar and would have allowed only ~8 real
  nesting levels here): a throwaway, isolated subprocess binary-searched, on
  a default ~2 MiB stack worker thread, for the largest `with_max_depth`
  value that still returns a clean error instead of overflowing. See the
  `MAX_RULE_DEPTH` doc comment in `src/lib.rs` for the full derivation
  (rule-chain analysis + measured crash floor).
- Added 3 regression tests exercising the guard on the real MACSYMA grammar:
  a big-stack deep-nesting test, a default-stack deep-nesting test (proving
  the cap trips before the native stack would overflow), and a boundary test
  proving legitimate nesting up to 14 levels still parses while 15 trips the
  cap.

## [0.1.0] - 2026-05-08

### Added

- Initial grammar-driven Rust MACSYMA parser.
- Statically linked compiled parser grammar.
