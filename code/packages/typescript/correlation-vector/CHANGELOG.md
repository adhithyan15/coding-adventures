# Changelog — @coding-adventures/correlation-vector

## [0.1.0] — 2026-04-05

### Added

- Initial TypeScript implementation of the Correlation Vector (CV) specification (CV00).
- `Origin`, `Contribution`, `DeletionRecord`, `CVEntry` interfaces as specified in CV00.
- `CVLog` class with full API:
  - `create(origin?)` — born a new root CV, returns a deterministic `base.N` ID.
  - `contribute(cvId, source, tag, meta?)` — append a contribution; throws if deleted.
  - `derive(parentCvId, origin?)` — create a child CV with ID `parent.M`.
  - `merge(parentCvIds, origin?)` — create a CV descended from multiple parents.
  - `delete(cvId, source, reason, meta?)` — record a deletion (entry remains in log).
  - `passthrough(cvId, source)` — record identity contribution (visited, no change).
  - `get(cvId)` — retrieve the full CVEntry.
  - `ancestors(cvId)` — all ancestor IDs, nearest parent first.
  - `descendants(cvId)` — all descendant IDs.
  - `history(cvId)` — contributions in order (not including deletion record).
  - `lineage(cvId)` — full entry chain from oldest ancestor to entity itself.
  - `serialize()` — plain JS object in the canonical CV00 JSON schema.
  - `static deserialize(data)` — reconstruct CVLog from plain object.
  - `toJsonString()` — compact JSON string using `@coding-adventures/json-serializer`.
  - `static fromJsonString(s)` — reconstruct from JSON string.
- Inlined pure TypeScript SHA-256 implementation (no external sha256 dep) for ID base generation.
- ID base computed as first 8 hex characters of `SHA-256(source + ":" + location)`.
- Counter state lives on CVLog instances; no global state.
- `enabled = false` makes all write operations no-ops; `create`/`derive`/`merge` still return valid IDs.
- Serialized format uses `snake_case` keys (`parent_ids`, `pass_order`) for cross-language interoperability.
- Counter reconstruction on `deserialize` ensures no ID collisions after roundtrip.
- Test suite with ≥95% coverage across 8 test groups (root lifecycle, derivation, merging, deep ancestry, disabled log, serialization roundtrip, ID uniqueness, edge cases).

### Notes

- This is PR 1/10: the TypeScript reference implementation that all other language implementations will follow.
- Implementation follows the spec exactly; no divergences.
