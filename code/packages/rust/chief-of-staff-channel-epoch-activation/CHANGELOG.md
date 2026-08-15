# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - Unreleased

### Added

- Add strict D18S version 2 and D18T version 1 canonical codecs.
- Add an injected atomic originator-custody contract with redacted handles,
  deterministic non-durable test custody, conflict-safe preparation, and
  channel destruction.
- Compose production D18C membership, D18P storage, D18Q grants, and D18F
  publication into crash-safe prepare, replay, activate, reserve, commit, and
  recovery operations.
- Add deterministic shared fixtures with generator Git blob provenance and
  clearly isolated test-only secrets.
- Cover migration, every public replay crash boundary, concurrent candidate
  selection, pending publication races, corruption, custody loss, channel
  destruction, redacted diagnostics, and sixteen-attempt CAS exhaustion.
