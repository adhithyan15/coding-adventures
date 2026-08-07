# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C++17 header-only port of the Rust `compiler-source-map`
  crate, in namespace `ca::csm`.
- `SourcePosition` (with `to_string` and equality) and the four segment structs
  — `SourceToAst`, `AstToIr`, `IrToIr`, `IrToMachineCode` — with public-field
  value semantics, `add`, and forward/reverse lookups (`std::optional` / borrowed
  pointer results).
- `SourceMapChain` composing the segments, with `add_optimizer_pass` and the
  `source_to_mc` (forward) / `mc_to_source` (reverse) composite queries that
  follow IR IDs through the passes and skip deleted instructions.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC): each segment's
  add/lookup, and the chain's end-to-end round-trips including an optimiser pass
  and a deletion — mirroring the Rust crate's tests.
