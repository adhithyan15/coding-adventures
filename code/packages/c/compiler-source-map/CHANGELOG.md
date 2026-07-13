# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C17 port of the Rust `compiler-source-map` crate: the
  compiler-pipeline source-mapping sidecar.
- Four segment handles — `SmapSourceToAst`, `SmapAstToIr`, `SmapIrToIr`,
  `SmapIrToMc` — each with `*_new` / `*_free`, `add`, and forward/reverse
  lookups; plus `smap_position_to_string`.
- `SmapChain` bundling the segments (owning its SourceToAst/AstToIr, taking
  ownership of passes via `add_optimizer_pass` and the backend via
  `set_machine_code`), with the composite `source_to_mc` (forward) and
  `mc_to_source` (reverse) queries that follow IR IDs through the optimiser
  passes and skip deleted instructions.
- `Option` outcomes become status codes / NULL; every dynamic array guards its
  doubling against `size_t` overflow; pure-ISO string helpers replace POSIX
  `strdup`.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC): each segment's
  add/lookup, and the chain's end-to-end round-trips including an optimiser pass
  and a deletion — mirroring the Rust crate's tests.
