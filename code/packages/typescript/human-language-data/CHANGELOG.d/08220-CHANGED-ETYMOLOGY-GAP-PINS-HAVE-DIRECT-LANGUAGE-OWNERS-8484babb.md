### Changed — etymology gap pins have direct language owners

- Moved every track's exact `withoutHook` and `withoutVisible` ratchet plus its
  migration or source-audit rationale into one direct owner module.
- Kept the generic root-ledger behavior, one-corpus-load measurement, exact
  rise/fall semantics, and newest-offender diagnostics in the stable test.
- Added fail-closed discovery for missing, unknown, duplicate, malformed,
  unsafe, linked, case-colliding, indirect, or aggregate owners.
- Preserved every pre-migration pin value exactly.
