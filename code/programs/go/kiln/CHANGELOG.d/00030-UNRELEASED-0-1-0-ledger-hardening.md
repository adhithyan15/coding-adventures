## [0.1.0] - UNRELEASED

### Security

- Ledger: delta timestamps are re-stamped by `Merge` and future-dated records
  are refused, so a forged `at` cannot outrank tombstones; equal-`at` ties go to
  the tombstone; writes require an `Authority` and a `Scope` instead of a bool;
  `Merge` builds in a partial sibling directory; directory fsync failures are
  errors; symlinks and reparse points are refused before open and aliases of the
  snapshot directory are resolved; directory reads are chunked and capped
  (`MaxDirBytes`); format characters (bidi overrides) are rejected in labels.
