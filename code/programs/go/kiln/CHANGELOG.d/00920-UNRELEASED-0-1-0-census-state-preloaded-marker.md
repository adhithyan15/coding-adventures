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
- A marker name is skipped only when the entry is a regular file: a directory or a symlink
  with a marker name is counted as `stray_traces_entry`
  (`TestCensusMarkerNameAsDirectoryIsStray`).
- The summary JSON has a new plain fact `state_preloaded_marker` (and the Markdown a
  "traces/ markers" section), reported with or without `--records`, so "skipped" is not
  "unseen". With a collect header, a marker whose header void list lacks `state-preloaded`
  raises the named problem `state_preloaded_marker_without_void`
  (`TestCensusStatePreloadedMarkerConsistency`).
- `globalSeqCheck` uses `isTracesMarker` instead of its own list of the four names (same
  behaviour).
