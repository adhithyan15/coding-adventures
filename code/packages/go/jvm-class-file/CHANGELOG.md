# Changelog

## [0.1.0] - Unreleased

### Added

- Created the Go `jvm-class-file` package as the reusable class-file layer for
  future Go JVM backends.
- Added parsing for a conservative subset of the JVM constant pool together
  with method and `Code` attribute decoding.
- Added a minimal class-file builder for one-class, one-method artifacts.
- Added round-trip and real-fixture tests covering helper lookups and malformed
  input handling.
