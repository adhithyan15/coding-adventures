# Changelog

All notable changes to this package are documented here.

## [Unreleased]

### Fixed

- Security review round 1 (33 items, one regression test each in
  `tests/security.rs`): shebang lookalike `#! [attr]` no longer hides code;
  raw identifiers (`r#unwrap`) are normalised so every name rule fires; SH17
  wrappers are mandatory except for a closed list of exports; `#![no_std]` and
  `#![forbid(unsafe_code)]` must be crate-level and exact; `[lib] path`, binaries,
  other targets and `[project]` are pinned; every index form is an index unless
  an array-literal position; shift, panic-path, iterator and closure evasions
  closed; `for` loops are judged by `impl` header context; turbofish and
  unnamed-type impls in SH5; every `cfg` other than the ghost gate is SH18;
  dev-dependencies are an allowlist; SH14 applies to class K with no kind and to
  tuple structs and unions; case-insensitive `build.rs`, `.cargo`,
  `rust-toolchain*`; `1.` and `1_f64` are floats.
- Quadratic scans (type extents, header scans, attributes, `#[cfg(test)]` mask,
  interior-mutability fixpoint, call-graph BFS) are linear or bounded, and fail
  closed with VT03 where a bound is hit; call graph uses iterative Tarjan.
- Loader: `O_NOFOLLOW | O_NONBLOCK` open with an open-handle `is_file` and
  `dev`/`ino` check, total byte, per-directory and total entry caps enforced
  during the walk, case-fold traps refused.

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
