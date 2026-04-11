# Changelog — coding_adventures_correlation_vector

All notable changes to this package will be documented in this file.

## [0.1.0] — 2026-04-05

### Added

- Initial implementation of the Correlation Vector (CV00) spec in Rust.
- `CVLog` — the central log struct with `entries`, `pass_order`, and `enabled` fields.
- `CVEntry` — full record for one tracked entity: id, parent_ids, origin, contributions, deleted.
- `Origin` — where/when an entity was born (source, location, timestamp, meta).
- `Contribution` — a stage's record of processing an entity (source, tag, meta).
- `DeletionRecord` — permanent record of an intentional entity removal.
- `CVLog::new(enabled)` — create an empty log, optionally disabled.
- `CVLog::create(origin)` — create a root CV with SHA-256-based ID.
- `CVLog::contribute(cv_id, source, tag, meta)` — append a contribution; errors on deleted entities.
- `CVLog::derive(parent_cv_id, origin)` — create a child CV with dot-extended ID.
- `CVLog::merge(parent_cv_ids, origin)` — create a CV from multiple parents.
- `CVLog::delete(cv_id, source, reason, meta)` — mark an entity as deleted.
- `CVLog::passthrough(cv_id, source)` — record a no-change stage visit.
- `CVLog::get(cv_id)` — retrieve a CVEntry by ID.
- `CVLog::ancestors(cv_id)` — BFS walk of parent chain, nearest first.
- `CVLog::descendants(cv_id)` — reverse index scan for all children/grandchildren.
- `CVLog::history(cv_id)` — ordered list of contributions.
- `CVLog::lineage(cv_id)` — full ancestor chain + entity, oldest first.
- `CVLog::to_json_string()` — serialize to compact JSON.
- `CVLog::from_json_string(s)` — deserialize from JSON, reconstructing counters.
- 30+ unit tests covering all 7 spec groups (>95% coverage).
- Full literate-programming documentation with inline diagrams and examples.
