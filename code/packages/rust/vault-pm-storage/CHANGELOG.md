# Changelog

All notable changes to this package are documented here.

## [0.1.0] - 2026-08-09

### Added

- The bounded, provider-neutral `VaultObjectStore` V1 contract.
- Redacted identifiers, bodies, cursors, provider revisions, and typed errors.
- Capability reporting for consistency, conditional operations, change feeds,
  checksums, upload/read optimizations, deletion, sharing, and provider limits.
- A thread-safe deterministic in-memory backend with immutable writes, exact
  reads/stats, ordered cursor pagination, deletion, and change hints.
- A one-shot deterministic fault wrapper for provider errors, corrupt reads,
  stale or duplicate listings, and ambiguous committed writes.
- A reusable conformance runner and embedded language-neutral fixture.

### Security

- Conflicting bytes under one logical object ID fail as corruption.
- Store instances bind idempotently to exactly one vault locator.
- Debug and display output omit bodies, identifiers, cursors, revisions, and
  attacker-controlled provider messages.
- All V1 body, cursor, revision, list, and change-page bounds are explicit.
