# Changelog — coding_adventures_correlation_vector

All notable changes to this package are documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — 2026-04-05

### Added

- Initial implementation of `CodingAdventures.CorrelationVector` per spec CV00.
- `CVLog` struct with `entries`, `pass_order`, `enabled`, `base_counters`, `child_counters`.
- Nested struct modules: `Origin`, `Contribution`, `DeletionRecord`, `Entry`.
- `new/1` — create a fresh CVLog with optional `enabled` flag.
- `create/2` — born a new root CV; ID format `base.N` (SHA-256 of origin or `00000000`).
- `contribute/5` — append a contribution; raises on deleted entries.
- `derive/3` — create a child CV; ID format `parent_id.M`.
- `merge/3` — create a CV from multiple parents; uses `00000000` base for no-origin merges.
- `delete/5` — record intentional deletion; entry remains in log.
- `passthrough/3` — record a stage observed but did not change the entity.
- `get/2` — retrieve a CV entry by ID.
- `ancestors/2` — walk parent_ids chain; returns nearest-first.
- `descendants/2` — scan log for all children of a CV.
- `history/2` — contributions in order; deletion appended as synthetic final contribution.
- `lineage/2` — full provenance chain, oldest ancestor first.
- `serialize/1` — convert CVLog to plain Elixir map (CV00 interchange format).
- `to_json_string/1` — serialize to JSON using `coding_adventures_json_serializer`.
- `from_json_string/1` — reconstruct CVLog from JSON using `coding_adventures_json_value`.
- Counter reconstruction on deserialization so subsequent operations produce unique IDs.
- Comprehensive ExUnit test suite covering all 7 spec groups at >95% coverage.
- Literate programming comments throughout the implementation.
- README and CHANGELOG.
