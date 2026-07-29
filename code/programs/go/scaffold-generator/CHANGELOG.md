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

- Haskell scaffolds now follow the repository's current plain Cabal package
  naming, `CodingAdventures.*` module layout, Hspec wiring, `-Wall`, explicit
  Windows skip, empty capability metadata, and plain `cabal test` convention.
- Haskell dependency discovery now reads the transitive sibling paths from
  `cabal.project` and retains a Cabal-manifest fallback for older packages.
- Cabal manifests now distinguish the short synopsis from a publishable
  description and include a category, README, and changelog metadata.

## [1.1.0] - 2026-03-25

### Added

- `generateCommonFiles` now generates `required_capabilities.json` alongside README.md and CHANGELOG.md
- New packages scaffold with empty capabilities and a "pure computation" default justification
- `TestGenerateCommonFiles` now verifies the generated `required_capabilities.json` is valid JSON with all required fields

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
