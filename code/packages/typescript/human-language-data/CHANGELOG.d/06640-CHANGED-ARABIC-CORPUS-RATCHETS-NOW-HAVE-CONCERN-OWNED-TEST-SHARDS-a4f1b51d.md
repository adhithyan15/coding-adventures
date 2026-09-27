### Changed — Arabic corpus ratchets now have concern-owned test shards

Arabic continuity, lesson-shape, writing-ramp, and root-ledger contracts now
live in independently owned case modules behind the corpus-test shard loader.
The complete-corpus writing check and the real-corpus integration evidence gate
retain measured 60-second timeout budgets under full-suite load.
