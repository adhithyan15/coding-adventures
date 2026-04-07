# Changelog — coding-adventures-correlation-vector (Lua)

## [0.1.0] — 2026-04-06

### Added

- Initial implementation of the Correlation Vector (CV) library for Lua,
  implementing spec CV00-correlation-vector.md.
- `M.new(opts)` — factory that creates a new CVLog object with optional
  `enabled` flag for zero-overhead production mode.
- `cvlog:create(opts)` — creates a root CV with SHA-256-based or synthetic
  (00000000) base ID.
- `cvlog:contribute(cv_id, opts)` — appends a contribution (source, tag, meta,
  timestamp) to an existing CV entry.
- `cvlog:derive(parent_cv_id, opts)` — creates a child CV whose ID is the
  parent ID with a new numeric suffix.
- `cvlog:merge(cv_ids, opts)` — creates a CV with multiple parents; base is
  SHA-256 of the sorted parent IDs for commutativity.
- `cvlog:delete(cv_id, opts)` — marks a CV as deleted; subsequent contributions
  raise an error.
- `cvlog:passthrough(cv_id, opts)` — records that a stage examined but did not
  change an entity; returns the cv_id unchanged.
- `cvlog:get(cv_id)` — returns the raw entry table or nil.
- `cvlog:ancestors(cv_id)` — BFS walk returning ancestor IDs nearest-first.
- `cvlog:descendants(cv_id)` — returns all CVs that trace back to the given ID.
- `cvlog:history(cv_id)` — returns the contributions array.
- `cvlog:lineage(cv_id)` — returns entries for all ancestors oldest-first,
  ending with the requested entity.
- `cvlog:serialize()` — converts the CVLog to a JSON string via
  `coding-adventures-json-serializer`.
- `M.deserialize(json_str)` — reconstructs a CVLog from a JSON string, restoring
  the counter to avoid ID collisions with previously allocated IDs.
- Full busted test suite covering: root lifecycle, derivation, merging, deep
  ancestry chains, disabled-log mode, serialization roundtrip, and ID uniqueness
  (1000 CVs, no collisions).
- Knuth-style literate comments throughout: every function explains WHY, not
  just what, with analogies and examples for learners.
