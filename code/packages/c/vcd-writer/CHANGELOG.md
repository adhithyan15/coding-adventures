# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C17 port of the Rust `vcd-writer` crate: a streaming Value
  Change Dump (VCD, IEEE 1364-2005) text writer.
- Header API (`vcd_open_scope` / `vcd_open_scope_kind` / `vcd_declare` /
  `vcd_close_scope` / `vcd_end_definitions`) with bijective base-94 identifier
  allocation; body API (`vcd_time` / `vcd_value_change` / `vcd_value_change_at` /
  `vcd_dump_initial`) with skip-if-unchanged; `vcd_text` / `vcd_ok` / `vcd_free`.
- Value formatting: single bit for scalars, `b<binary>` for vectors, `r<n>` for
  reals; overflow-guarded growable output buffer and dynamic def/last-value
  tables.
- Iterates `dump_initial` in declaration order (deterministic; the Rust HashMap
  order is unspecified). The Rust `attach` closure glue is out of scope.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC): an exact match of the
  Rust crate's documented example plus scalar / real / skip / dumpvars / two-char
  identifier cases.
