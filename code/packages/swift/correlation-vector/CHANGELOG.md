# CHANGELOG — CorrelationVector (Swift)

## [0.1.0] — 2026-04-06

### Added

- Initial implementation of `CorrelationVector` Swift package per spec CV00.
- `Origin` struct — records where an entity was born (origin string + synthetic flag).
- `Contribution` struct — records one stage's transformation (source, tag, JsonValue meta, ISO 8601 timestamp).
- `DeletionRecord` struct — soft-delete marker (by, at timestamp).
- `CVEntry` struct — full provenance record (cvId, origin, parentCvId, mergedFrom, contributions, deleted, passOrder).
- `CorrelationVectorError` struct — typed error for programming mistakes (entry not found, contributing to deleted entry). Named to avoid collision with `CoreVideo.CVError`.
- `CVLog` class — central registry with six core operations:
  - `create(originString:synthetic:meta:)` — born a new root CV with SHA-256 base segment.
  - `contribute(cvId:source:tag:meta:)` — append a stage contribution; throws if entry missing or deleted.
  - `derive(parentCvId:source:tag:meta:)` — create a child CV with dot-extended ID.
  - `merge(cvIds:source:tag:meta:)` — create a CV from multiple parents.
  - `delete(cvId:by:)` — soft-delete (entry remains in log permanently).
  - `passthrough(cvId:source:)` — record a stage examined but did not transform.
- Five query methods: `get`, `ancestors` (BFS, nearest-first), `descendants`, `history`, `lineage` (oldest-first).
- `serialize()` — encodes the full log to compact JSON via `JsonSerializer`.
- `deserialize(_:)` — reconstructs a log from a JSON string.
- `enabled` flag: when `false`, all write operations are no-ops; IDs are still generated and returned.
- 46 tests across 7 test groups covering all spec requirements:
  1. Root lifecycle (create, contribute, passthrough, delete, error conditions)
  2. Derivation (child ID format, ancestors, descendants)
  3. Merging (3-way merge, mergedFrom, ancestors)
  4. Deep ancestry chain (4 levels, nearest-first / oldest-first)
  5. Disabled log (IDs generated, entries not stored)
  6. Serialization roundtrip (all fields preserved including metadata)
  7. ID uniqueness (1000 creates, no collisions)
- Dependencies: `sha256` (base segment generation), `json-value` (typed metadata), `json-serializer` (encode/decode).
- Knuth-style literate programming throughout the implementation source.
