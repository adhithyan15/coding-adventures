# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Added

- Added OCaml library and program scaffolds with the shared byte-exact OCAML02
  fixtures, exact direct opam/Dune metadata, Alcotest, ocamlformat, bisect_ppx
  coverage, resolved local dependency pins, and capability manifests.

### Fixed

- Haskell scaffolds now emit a `BUILD` that selects the Command Line Tools
  linker on macOS: the two-line explanation, then
  `if [ "$(uname -s)" = Darwin ]; then export DEVELOPER_DIR=/Library/Developer/CommandLineTools; fi; cabal test all`.
  GHC 9.4.8's macOS bindist hardcodes a CLT SDK sysroot that an older default
  Xcode linker cannot read (`arm64e.x1`), and `build-tool --validate-build-files`
  rejects a Haskell BUILD without the selection.
- Haskell scaffolds now also emit a `BUILD_windows` holding the plain
  `cabal test all`, because `cmd /C` cannot parse the POSIX prefix. Windows
  previously ran the plain `BUILD`, so its behaviour is unchanged. Tests pin
  both files exactly (spec section 5.8).
- OCaml dependency pins now scope checksum bypass to local worktree ingestion
  while subsequent registry dependency installs retain strict checksums.
- OCaml metadata rejects `*)` and safely quotes printable descriptions before
  any output is written.
- Dune `%{...}` interpolation openers are rejected before any scaffold output.
- Accepted Unicode descriptions now remain raw UTF-8 with only OCaml/opam
  quote and backslash escaping, matching the Go front door byte-for-byte.
- OCaml dependency discovery reads only the expected opam `depends` field,
  program scaffolds pin package-tree dependencies by resolved relative path,
  and direct/transitive dependency symlinks fail closed.
- Haskell scaffolds now emit the complete shared schema-v1 capability manifest
  and keep their starter library and test source pure.
- Haskell generation rejects control characters before descriptions are
  interpolated into Cabal metadata, and the CLI rejects structural
  block-comment delimiters before any template is written.
- Refreshed the lockfile to use patched PostCSS and its aligned Nano ID
  resolution.

## [1.0.0] - 2026-03-21

### Added

- Initial TypeScript implementation of scaffold-generator
- Name normalization (toSnakeCase, toCamelCase, toJoinedLower)
- Dependency reading for all 6 languages (Python, Go, Ruby, TypeScript, Rust, Elixir)
- Transitive closure computation via BFS
- Topological sort via Kahn's algorithm for correct install ordering
- File generation for all 6 languages matching Go/Python reference implementations
- CLI builder integration for argument parsing
- Comprehensive test suite covering all major features
