# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Added

- Added closed `workflow.lastPublication` metadata for exact authoring,
  canonical manifest, and reviewed target identities.
- Added the semantic `record-publication` command and serialized
  `dispatchAtRevision` transaction so stale queued edits cannot acknowledge an
  older deployment or mark newer documents published.
- Added one exact opaque revision validator shared by storage, project workflow
  metadata, and publication composition without trimming or normalization.

## [0.1.0] - 2026-10-02

### Added

- Added the closed, bounded `AuthoringProject` v1 codec over Content IR.
- Added deterministic canonical JSON, canonical UUIDv7 identities, portable
  slugs/theme identifiers, safe URL handling, and complete recursive limits.
- Added semantic immutable project and document commands.
- Added compare-and-swap autosave with serialized concurrent dispatch.
- Added bounded persistent undo/redo with exact restart recovery.
- Added fail-closed handling for malformed, unsupported, oversized,
  non-canonical, conflicting, cancelled, and failed storage operations.
- Added allocation-bounded canonical byte preflight, descriptor-only hostile
  input snapshots, strict command scalars, and an explicit storage commit-point
  contract with indeterminate-result recovery.
- Added 95%+ statement/line and 90%+ branch coverage across hostile inputs,
  every command, persistence failures, history truncation, and recovery.
