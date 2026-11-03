# Changelog

All notable changes to this program will be documented in this file.

## [Unreleased]

### Added

- D21's nondeterminism categories (VER00 §6.3). The closed taxonomy grows
  from 19 to 25 pairs. `rand:read` generates `op.Rand.Read`, backed by an
  aliased `crypto/rand` import, and `thread:spawn` generates
  `op.Thread.Spawn(role, body)`, which checks the role against the declared
  targets and refuses a nil body. `thread:message`, `ui:read`, `ui:write` and
  `ui:event` are recognized but fail with the explicit unsupported-pair
  error.
- Tests now type-check generated files against the standard library
  instead of only matching text.

### Fixed

- `env:read` with an exact target generated `return nil, ...` from a
  method that returns a string, which did not compile. The scoped check now
  takes the method's own zero value.
- Generated imports are sorted by import path, ignoring any alias, which is
  the order gofmt keeps. Output for every existing Go manifest is
  byte-identical.

### Security

- Validate every manifest capability against Spec 13's closed 19-pair
  category/action taxonomy before generating source.
- Fail explicitly for recognized pairs whose enforcing Go wrapper is not yet
  implemented. Unsupported pairs no longer produce TODO namespaces or replace
  an existing generated cage.
- Conform the embedded pair table to the shared language-neutral taxonomy
  fixture.
- Preflight and render every `--all` input before writing, so one invalid or
  unsupported manifest cannot leave the repository partially regenerated.

## [1.0.0] - 2026-03-25

### Added

- Initial implementation of `capability-cage-generator`
- Reads `required_capabilities.json` and emits `gen_capabilities.go` with capabilities compiled in as Go constants
- `--manifest=<path>` flag for processing a single manifest file
- `--all` flag for sweeping all `code/packages/go/*/required_capabilities.json` files in the repo
- `--dry-run` flag for printing generated output without writing files
- Correct Go constant names (`cage.CategoryFS`, `cage.ActionRead`, etc.) for all 8 categories and 14 actions
- Package name derivation: reads `package` declaration from existing `.go` files, falls back to stripping hyphens from the manifest's `package` field
- Non-Go packages (python/, ruby/, etc.) are silently skipped with a stderr message
- Comprehensive test suite covering all flags, error conditions, and generated output format
