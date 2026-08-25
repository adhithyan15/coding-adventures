### Added - core/spine.json sharded, and the shard/unshard CLI (HL21, step 2 of 4)

- Split `core/spine.json` into `core/spine.d/`: `_meta.json` for the
  document-level keys and one `NNNN-<NODE-ID>.json` per node. The shards are the
  source of truth.
- The ordinal prefix makes sorted filename order reproduce AUTHORED order. The
  spine runs pre-A1 to C2 and is not alphabetical, so naming shards by node id
  alone would have silently re-sorted the ladder. Ordinals are spaced by ten so
  a node can be inserted as `0015` without renaming its neighbours.
- Add `shard-cli` with `--shard`, `--unshard` and `--check`, plus the `shard`,
  `unshard` and `check:shards` scripts. Round-tripping is lossless and
  deterministic: `unshard(shard(core/spine.json))` reproduces the committed file
  byte for byte, proven by a test over the real ledger.
- `core/spine.json` is KEPT, as a generated artifact, because
  `language-ladder/src/curriculum.ts` statically imports it into a browser bundle
  and a browser cannot read a directory. `check:shards` runs in CI beside
  `check:books`, so a stale monolith fails the build.
- Node ids were verified against the safe-filename rule rather than assumed, and
  `--shard` refuses an unsafe id, a Windows reserved device name, a duplicate id,
  and a ledger whose array is not its last top-level key.

