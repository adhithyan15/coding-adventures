### Changed - shard-only curriculum ledgers

- Removed the tracked `core/spine.json`, `core/book-generation.json`, and all
  23 per-track `chapters.json` / `curriculum.json` compatibility aggregates.
  Their canonical `.d/` directories preserve the exact parsed documents, and
  `check:shards` now rejects a bad merge that resurrects an ignored monolith.
- Exported the shard-aware book-generation config reader and moved the remaining
  Python authoring and handwritten-parity tools onto shard-native reads and
  track-owned writes. A tranche updating one language now changes that
  language's book-generation shard rather than a file shared by all tracks.
- The Python shard boundary now confines every operation beneath the curriculum
  root, refuses symlinked ancestors and files, replaces shards atomically, and
  validates chapter/curriculum identities before any mutation.
