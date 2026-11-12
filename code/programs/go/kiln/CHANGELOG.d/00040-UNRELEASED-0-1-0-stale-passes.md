## [0.1.0] - UNRELEASED

### Security

- `Merge` keeps a delta pass's own time (capped at now) and stamps delta
  tombstones now, so a stale pass cannot undo a newer tombstone; `outDir` is
  cleaned; `Store.Record` allows only `MaxRecordSkew` (1 minute) of future skew.
