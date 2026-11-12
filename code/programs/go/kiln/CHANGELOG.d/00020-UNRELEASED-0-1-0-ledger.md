## [0.1.0] - UNRELEASED

### Added

- `internal/ledger`: the directory ledger of KILN00 section 10. Strict,
  canonical JSON-lines records (`pass` / `tombstone`) with documented size and
  count caps; an immutable-segment `Store` with `Lookup` (newest record per key,
  tombstone shadowing, deterministic tie-breaks) and trust-gated atomic
  `Record`; and `Merge`, the core of the `ledger-merge` job (strict delta
  validation, compaction, deterministic output, pruning that never drops a
  shadowing tombstone). Segment names are validated and symlinks / non-regular
  files are refused.
