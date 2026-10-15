### Fixed — bounded Windows curriculum-owner checks

- Reuse the guarded ledger reader's exact source bytes for canonical owner
  validation, avoiding a second read of each direct owner and its read/compare
  race while retaining per-file shard and symlink checks.
- Compare lesson and owner filenames with sets instead of repeated linear scans.
- Check real-corpus deterministic assembly and unique ownership in timed
  per-track tests, with an explicit all-track coverage check and a regression
  for a shard directory beside a direct owner.
