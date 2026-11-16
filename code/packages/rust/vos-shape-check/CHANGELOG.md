# Changelog

All notable changes to this package are documented here.

## [0.1.0] - 2026-11-16

### Added

- Lexical component shape checker (VOS-D002, VOS03): SH1-SH15, SH17 and SH18,
  class tags and kinds from `vos.toml` (VOS01 section 1.6), the static bans
  (`build.rs`, `build` key of any value, `links`, proc-macro, relocated targets),
  the central evidence-feature list, and the `-any` name lint.
- Own zero-dependency lexer, bracket pairing, a fail-closed TOML-subset reader,
  a by-name call graph for SH5 and the ghost-face wrapper checks for SH17.
- Deterministic report and exit codes 0, 1, 2; bounded, symlink-refusing loader in
  the binary only; the library is pure.
- Tests: per-rule clean and violating cases, the VOS01 toy crates and broken
  variants, CLI and filesystem tests. Line coverage 99%.
