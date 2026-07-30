# Changelog

All notable changes to this program will be documented in this file.

## [Unreleased]

### Added

- **C and C++ scaffold templates** (`--language c` / `--language cpp`). Generates
  a pure-ISO package wired to the shared iso-harness: `BUILD`
  (`# build-tool: deps=c/iso-harness` + `sh tools/run.sh`), `BUILD_windows`
  (PowerShell/MSVC), `tools/run.sh` + `tools/run.ps1` that locate the harness by
  walking up the tree and compile under every present compiler, an
  `include/` header (C: `.h` + `src/.c`; C++: header-only `.hpp`), a
  `tests/…_test.{c,cpp}` using the header-only `iso_test.h`, and a `.gitignore`
  for `_build/`. C/C++ have no package manifest, so `readDeps` returns an empty
  set (deps go in the BUILD comment). See
  `code/specs/CCPP01-c-cpp-iso-multicompiler-lane.md`.

### Fixed

- Haskell capability metadata now uses the complete shared schema-v1 document,
  with golden output and shared-schema regression coverage instead of the
  legacy one-field JSON object.
- Description validation now rejects Unicode controls, line separators, and
  block-comment termination before any generated source is written.
- Haskell scaffolds now follow the repository's current plain Cabal package
  naming, `CodingAdventures.*` module layout, Hspec wiring, `-Wall`, explicit
  Windows skip, empty capability metadata, and plain `cabal test` convention.
- Haskell dependency discovery now reads the transitive sibling paths from
  the `packages` field in `cabal.project`, ignores comments and unrelated
  fields, validates sibling directories, and retains a Cabal-manifest fallback
  for older packages.
- Cabal manifests now distinguish the short synopsis from a publishable
  description and include a category, README, and changelog metadata.
- Cabal generation now treats descriptions containing percent signs as plain
  text instead of accidentally reusing generated content as a format string.

## [1.1.0] - 2026-03-25

### Added

- Language templates generate `required_capabilities.json` alongside their
  package-specific metadata.
- New schema-supported packages scaffold with empty capabilities and a "pure
  computation" default justification.
- Template tests verify generated capability JSON includes the required
  package identity and fields.

## [1.0.0] - 2026-03-21

### Added

- Initial implementation of scaffold-generator in Go
- CLI parsing via cli-builder with scaffold-generator.json spec
- Name normalization (kebab-case to snake_case, CamelCase, joinedlower)
- Dependency resolution: transitive closure via BFS, topological sort via Kahn's algorithm
- File generation for all 6 languages: Python, Go, Ruby, TypeScript, Rust, Elixir
- Automatic Rust workspace Cargo.toml member list updates
- Dry-run mode for previewing generated files
- Input validation (kebab-case names, known languages, existing dependencies)
- Comprehensive test suite covering name normalization, dep resolution, file generation
