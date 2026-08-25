### Fixed - HL21 §5.2 was wrong about `spine`, and every track proves it

- The spec argued `spine` is keyed by node id and so "needs no ordinal: an
  object has no meaningful order". True of JSON, false of this ledger:
  `JSON.stringify` emits keys in **insertion** order; **no** track has its spine
  keys sorted; and all 23 list them in exactly `core/spine.d/`'s ladder order,
  pre-A1 → C2. `<NODE-ID>.json` shards merged in sorted order would have
  scrambled the shared ladder in 23 files at once, silently, while still
  "round-tripping successfully".
- `spine/` therefore carries zero-padded ordinals like every other section, and
  a test asserts both halves: sorted shard order reproduces the ladder, and
  plain `<ID>.json` names would not have.
- `path`/`extensions` ordinals confirmed needed. Spanish diverges at index 3 —
  authored `ES-PATH-004` against sorted `ES-PATH-003-CASA`. That claim is now
  pinned **corpus-wide** rather than per-track, because it is not true of every
  track: `japanese` and `urdu` happen to have both lists already sorted and
  would coincidentally survive losing their ordinals. 20 of 22 would not.

