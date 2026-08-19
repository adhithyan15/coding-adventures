# Changelog

## [0.2.0] - 2026-08-18

### Added

- **`AttachmentManifestId`** and `ItemDocument::attachment_manifests`, the
  domain half of `VLT-PM47-cli-attachments.md` §4.7. An item that has
  attachments has to know *where* they are, and a 256-bit content address per
  attachment is what it knows.

- **`DomainError::AttachmentManifestMismatch`.** `validate` now requires the
  manifest map's key set to equal `attachments.retained_values()` in both
  directions: membership with no manifest names bytes nobody can find, and a
  manifest with no membership points at bytes nothing claims, and neither is a
  state with a meaning. The key set is the *retained* one rather than the
  present one, because a removal a later merge undoes must not have dropped
  its reference on the way.

### Changed

- `ItemDocument::new` takes the manifest map as a tenth argument. Concurrent
  auto-merge unions the two maps and treats a disagreement about one immutable
  attachment id as a fault rather than a conflict — two replicas that both
  know a random 128-bit identity necessarily know the same immutable manifest
  address, so one of them is simply wrong.

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
- Lossless persistence iterators expose retained add and removal observations;
  present-only projections no longer tempt codecs to discard tombstones.
