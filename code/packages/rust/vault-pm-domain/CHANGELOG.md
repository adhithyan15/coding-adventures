# Changelog

All notable changes to this package are documented here.

## [0.1.0] - 2026-08-09

### Added

- Strict redacted product IDs with explicit Crockford Base32 user rendering.
- Validated content types and item documents over VLT02 records.
- Generic observed-remove sets and deterministic LWW registers.
- Live candidates, tombstones, no-loss conflicts, and pure merge decisions.
- Redacted record/item views that never copy plaintext secret fields.

### Security

- IDs, documents, conflicts, and views use custom redacted formatters.
- Item and view drop paths wipe secret-bearing or sensitive string values.
- Concurrent secret edits and delete/edit races are retained as conflicts.
- Observed sets enforce hard retained-value, add-operation, and tombstone
  limits during mutation, exact reconstruction, and merge.
- Operation-ID collisions and dangling removal tombstones are rejected.
- Tombstone compaction requires an explicit repository causal-stability
  predicate and preserves concurrent or later adds.
