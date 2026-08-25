### Added - Sharded ledger reads (HL21, step 1 of 4)

- Add `src/shard.ts`: for any ledger at `X.json`, read the sibling directory
  `X.d/` when it exists and fall back to `X.json` when it does not. Shards are
  merged in sorted filename order, compared by code unit rather than
  `localeCompare`, so the merged result is byte-identical on every machine.
- Add `mergeMetaAndList`, the `_meta.json` + one-file-per-element fold that most
  of these ledgers want, and which requires `_meta.json` rather than defaulting
  it to `{}`.
- An `X.d/` that exists but holds no shards is an error, not an empty ledger —
  the same fail-closed choice `loadModalityManifest` already makes.
- A malformed shard reports the offending filename, not a byte offset into a
  merged read of 33 files.
- Route `loadCurriculumSpine` through the helper. No data moves in this change:
  with no `core/spine.d/` in the tree this is the old monolith read, unchanged.

