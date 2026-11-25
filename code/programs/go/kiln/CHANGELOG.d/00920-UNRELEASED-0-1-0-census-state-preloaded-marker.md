## [0.1.0] - UNRELEASED

### Fixed (census: `traces/.state-preloaded` no longer counted as `stray_traces_entry`)

- The census walk over `traces/` skipped `.seq`, `.serial` and `.collected` but not
  `StatePreloadedMarker`, which `collect` writes into every shard that is void for
  state-preloaded (every runner-home census shard, by design). Each such shard falsely
  reported `stray_traces_entry=1` (census run 45466205573, all 6 shards). The skip list is
  now `isTracesMarker`, built from named constants (`SeqFile`, `SerialFile`,
  `CollectedMarker`, `StatePreloadedMarker`); the other writers of top-level `traces/` files
  (`prepare`, `LockSerial`, `collect`) were audited and no other marker was unhandled.
  A genuinely stray file is still counted. Test: `TestCensusStateMarkerNotStray`.
