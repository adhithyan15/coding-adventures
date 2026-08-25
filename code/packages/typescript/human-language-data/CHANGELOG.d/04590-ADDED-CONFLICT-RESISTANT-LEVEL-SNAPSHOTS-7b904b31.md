### Added - conflict-resistant level snapshots

- Replace the shared authored level-total hotspot with one exact generated snapshot
  per language under `core/level-snapshots/`.
- Reconstruct and verify the corpus-wide level histogram, unmapped count and mapped
  percentage from those shards, preserving exact regression coverage.
- Prove with two synthetic language tranches that independent tracks change disjoint
  files, so parallel curriculum work no longer conflicts on `levels.test.ts`.

