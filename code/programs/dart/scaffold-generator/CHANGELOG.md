# Changelog

## [0.2.0] - 2026-07-31

### Added

- Schema-v1 `required_capabilities.json` emission for generated libraries and
  programs, with byte-exact language-neutral golden fixtures.
- An explicit reviewed runtime-authority manifest for repository reads,
  scaffold creation and writes, dated changelogs, and console reporting.
- Native Dart and shared Draft 2020-12 schema tests for both generated
  capability profiles.

### Changed

- Removed the callable public library surface so the checked-in fixed-root CLI
  entrypoint is the only supported path to filesystem authority.

## [0.1.0] - 2026-04-18

### Added

- Initial Dart scaffold-generator program for generating Dart libraries and
  programs with CI-ready package layouts.
