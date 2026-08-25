# Changelog

## Unreleased

### Added

- A pure tracked-artifact snapshot validator now independently consumes the
  five language-neutral fixtures, including redacted portable-path failures,
  Unicode-scalar boundaries and ordering, inert link metadata, and exact,
  case, nested, and Unicode-compatible `node_modules` aliases.
- Generated source-embedded Unicode 17.0.0 normalization, full default-fold,
  and full-uppercase tables keep validation independent of host Lua tables;
  the generator verifies the emitted Lua module against every official vector.

### Fixed

- The emitted-Lua Unicode verifier now requires an explicit pinned Lua 5.4.7
  executable, ignores Lua initialization and module-path environment state,
  bounds retained child output, and terminates the isolated process tree after
  every exit. Windows starts the verifier suspended inside a kill-on-close Job
  Object; POSIX uses an isolated process group. Timeout, early-root-exit, and
  cleanup-error regressions cover descendant containment.
- Lua `.rockspec` dependency metadata now follows the shared strict UTF-8
  contract. Invalid bytes fail closed with the stable
  `METADATA_INVALID_UTF8` diagnostic and CLI exit code 2 without leaking host
  checkout paths.

## 0.1.0 — 2026-03-23

### Added
- Initial implementation of the Lua build tool.
- Package discovery via recursive BUILD file walk.
- Dependency resolution for all 7 supported languages:
  Python (pyproject.toml), Ruby (.gemspec), Go (go.mod),
  TypeScript (package.json), Rust (Cargo.toml), Elixir (mix.exs),
  Lua (.rockspec).
- Directed graph with Kahn's algorithm for topological sort.
- Sequential build execution with pass/fail tracking.
- Build report with summary statistics.
- Platform-specific BUILD file support (BUILD_mac, BUILD_linux, BUILD_windows).
- CLI with --root, --dry-run, --language, --force options.
