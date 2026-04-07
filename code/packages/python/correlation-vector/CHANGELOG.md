# Changelog — coding-adventures-correlation-vector

All notable changes to this project will be documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

---

## [0.1.0] — 2026-04-05

### Added

- Initial Python implementation of the Correlation Vector (CV00) spec.
- `Origin`, `Contribution`, `DeletionRecord`, `CVEntry` dataclasses.
- `CVLog` class with all six mutating operations:
  - `create(origin?)` — generate a root CV ID and store the entry.
  - `contribute(cv_id, source, tag, meta?)` — append a contribution; raises
    `ValueError` if the target entry is deleted.
  - `derive(parent_cv_id, origin?)` — create a child CV using dot-extension
    ID scheme.
  - `merge(parent_cv_ids, origin?)` — create a multi-parent CV.
  - `delete(cv_id, source, reason, meta?)` — tombstone an entry.
  - `passthrough(cv_id, source)` — record an identity contribution.
- Five query operations: `get`, `ancestors`, `descendants`, `history`,
  `lineage`.
- JSON serialisation via `serialize() → dict`, `to_json_string() → str`,
  `from_json_string(s) → CVLog`, and `deserialize(data) → CVLog`.
- `enabled` flag for zero-overhead tracing in production.
- ID generation using `coding-adventures-sha256` (repo's own SHA-256) for
  deterministic, collision-resistant base segments.
- `pass_order` list tracking first-seen order of contributing sources.
- Counter reconstruction on deserialisation so new IDs never collide with
  restored ones.
- Comprehensive test suite covering all seven groups from the spec:
  root lifecycle, derivation, merging, deep ancestry, disabled log,
  serialisation roundtrip, and ID uniqueness (10,000 creates).
- ≥ 95% test coverage enforced via `--cov-fail-under=95`.
- Ruff-clean, fully type-annotated, literate-programming style throughout.
