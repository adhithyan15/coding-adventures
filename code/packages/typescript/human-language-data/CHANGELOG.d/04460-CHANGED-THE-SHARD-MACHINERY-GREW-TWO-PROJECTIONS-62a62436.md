### Changed - the shard machinery grew two projections

- `ShardPlan.listKey`/`idOf`/`ordinalOf` become `ShardPlan.sections`, a list of
  `ShardSection`. A section names one top-level key, an optional subdirectory,
  and whether it holds an `"array"` or an `"object"`.
- **`_keys`**: `_meta.json` now records the monolith's top-level key order, but
  **only when it is needed** — when the sharded keys are not already a suffix of
  the document. §2.5 deliberately did not invent this and left the decision to
  "whoever migrates" a ledger whose array is not last; `curriculum.json` is that
  ledger, with three sharded keys in the middle. In practice only `spanish`
  needs it (its `conceptAliases` follows `extensions`); the other 21 tracks and
  all 21 previously-committed shard sets are untouched.
- The "array must be the last top-level key" refusal is gone, replaced by a
  check that a ledger has no top-level `_keys` of its own to collide with.
- `mergeSectionedShards` lives in `shard.ts` and is used by **both**
  `--unshard` and `loadLanguageCurricula`. Two definitions of what these files
  mean is precisely the drift `--check` exists to catch — and `--check` only
  compares the monolith against `unshardContents`, so a divergent loader would
  go unreported.
- `listShardNames` descends exactly one level into subdirectories, refusing a
  symlinked subdirectory as it already refused a symlinked shard. One level is a
  constant, so it cannot be walked into a cycle by a committed symlink.

