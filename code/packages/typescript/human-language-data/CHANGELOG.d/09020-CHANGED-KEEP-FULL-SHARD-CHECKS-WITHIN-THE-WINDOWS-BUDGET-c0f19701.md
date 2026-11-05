### Changed — keep full shard checks within the Windows budget

- Read generated book and narration hash owners once with bounded,
  pinned-directory I/O in the complete shard check. Parse and validate every
  owner with the existing strict validators; a missing, extra, linked,
  reparse-point, or malformed owner still fails.
- Check each authored chapter and curriculum plan before its shared
  completeness consumer, then reuse those validated identities. Standalone
  plan checks retain their direct independent readers, and unplanned authored
  tracks fall back to those readers.
- Keep the all-plan test, its 30-second timeout, and the package's 85% line
  coverage threshold unchanged. Full Windows coverage and exact-head CI are
  still required before acceptance.
