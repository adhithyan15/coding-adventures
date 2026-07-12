# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C++17 header-only port of the Rust `vcd-writer` crate, in
  namespace `ca`: a streaming Value Change Dump (VCD) text writer.
- `VcdWriter` with header API (`open_scope` / `open_scope_kind` / `declare` /
  `close_scope` / `end_definitions`) and body API (`time` / `value_change` /
  `value_change_at` / `dump_initial`); `finish` / `text`.
- Bijective base-94 identifier allocation; scalar / vector (`b<binary>`) /
  `real` (`r<n>`) value formatting; skip-if-unchanged.
- `dump_initial` iterates declaration order (deterministic). The Rust `attach`
  closure glue is out of scope.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC): an exact match of the
  Rust crate's documented example plus scalar / real / skip / dumpvars cases.
