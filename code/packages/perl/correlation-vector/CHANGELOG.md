# Changelog

All notable changes to `CodingAdventures::CorrelationVector` will be documented here.

## [0.1.0] - 2026-04-06

### Added

- Initial implementation of `CodingAdventures::CorrelationVector`
- `new(%opts)` constructor with `enabled` flag for zero-overhead production mode
- `create(%opts)` — born a new root CV with optional origin string or synthetic flag
- `contribute($cv_id, %opts)` — record a stage contribution (source, tag, meta, timestamp)
- `derive($parent_cv_id, %opts)` — create a child CV descended from an existing one
- `merge(\@cv_ids, %opts)` — create a CV descended from multiple parents
- `delete($cv_id, %opts)` — tombstone an entity with deletion metadata
- `passthrough($cv_id, %opts)` — record that a stage passed without modifying
- `get($cv_id)` — retrieve the full entry hashref for a CV ID
- `ancestors($cv_id)` — return all ancestor CV IDs, nearest-first (BFS)
- `descendants($cv_id)` — return all CV IDs that descend from this one
- `history($cv_id)` — return the contributions list for a CV ID
- `lineage($cv_id)` — return full entries for entity and all ancestors, oldest-first
- `serialize()` — serialize the CVLog to a JSON string using `CodingAdventures::JsonSerializer`
- `deserialize($class, $json_str)` — class method to reconstruct a CVLog from JSON
- Per-entry `pass_order` tracking (deduplicated list of sources that touched each entity)
- Global monotonic sequence counter for unique ID generation
- ISO 8601 UTC timestamps on all contributions and deletion records
- 9 test groups covering root lifecycle, derivation, merging, deep ancestry,
  disabled mode, serialization roundtrip, ID uniqueness, error handling,
  and per-entity pass_order

### Dependencies

- `CodingAdventures::SHA256` — for SHA-256 based ID base computation
- `CodingAdventures::JsonSerializer` — for JSON encode/decode
- `Test2::V0` — test framework
