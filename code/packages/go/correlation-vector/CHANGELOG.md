# Changelog — correlation-vector (Go)

All notable changes to this project will be documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

---

## [0.1.0] — 2026-04-05

### Added

- Initial Go implementation of the Correlation Vector (CV00) spec.
- `Origin`, `Contribution`, `DeletionRecord`, `CVEntry` structs.
- `CVLog` struct with all six mutating operations:
  - `Create(origin *Origin) string` — generate a root CV ID and store the entry.
  - `Contribute(cvID, source, tag string, meta map[string]any)` — append a
    contribution; logs an error if the target entry is deleted.
  - `Derive(parentCVID string, origin *Origin) string` — create a child CV
    using the dot-extension ID scheme.
  - `Merge(parentCVIDs []string, origin *Origin) string` — create a multi-parent CV.
  - `Delete(cvID, source, reason string, meta map[string]any)` — tombstone an entry.
  - `Passthrough(cvID, source string)` — record an identity contribution.
- Five query operations: `Get`, `Ancestors`, `Descendants`, `History`,
  `Lineage`.
- JSON serialisation via `Serialize() (jsonvalue.Value, error)`,
  `ToJSONString() (string, error)`, and `FromJSONString(s string) (*CVLog, error)`.
- `enabled` flag for zero-overhead tracing in production (`NewCVLog(enabled bool)`).
- ID generation using `code/packages/go/sha256` (repo's own SHA-256) for
  deterministic, collision-resistant base segments.
- `PassOrder` slice tracking first-seen order of contributing sources.
- Counter reconstruction on deserialisation so new IDs never collide with
  restored ones.
- 41 tests covering all seven spec groups:
  root lifecycle, derivation, merging, deep ancestry, disabled log,
  serialisation roundtrip, and ID uniqueness (10,000 creates).
- Integration stress-test of the repo's `json-serializer` and `sha256` packages.
- Literate-programming style throughout — inline explanations, diagrams, and
  examples suitable for readers new to the pattern.
