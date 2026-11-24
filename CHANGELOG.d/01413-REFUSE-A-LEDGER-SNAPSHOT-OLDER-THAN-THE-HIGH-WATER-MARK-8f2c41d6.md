### Refuse a ledger snapshot older than the high-water mark

- `internal/ledger` gains `MergeOptions` and `MergeWithOptions`, which take a
  `HighWater` sequence. A merge refuses a snapshot whose newest segment
  sequence is lower than the mark, before reading the delta, so a restored
  older snapshot cannot resurrect the revoked passes it still contains
  (KILN00 §18 Q9).
- `ledger.NewestSequence` reads the mark a merge job records after a
  successful merge. `Merge` keeps its signature and passes zero, so every
  existing caller is unchanged.
- The mark must be stored outside the snapshot — a separate cache key, a
  separate artifact, or a committed file. A mark inside the snapshot
  directory is rolled back with it and defends nothing.
- KILN00 §10.5 documents the mechanism and §18 Q9 is marked answered, with
  only the mark's storage location left to the merge job's wiring.
