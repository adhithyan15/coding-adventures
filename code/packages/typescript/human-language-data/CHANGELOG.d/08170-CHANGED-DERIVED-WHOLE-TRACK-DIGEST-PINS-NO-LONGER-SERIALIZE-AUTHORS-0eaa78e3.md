### Changed — derived whole-track digest pins no longer serialize authors (#17411)

- Remove the tracked `tests/curriculum-digests/<track>.json` snapshots. The
  canonical chapter, curriculum, and lesson-membership shards already own the
  graph; tests now assert deterministic assembly and unique lesson ownership
  directly while retaining the loader's fail-closed schema and graph checks.
- Remove the tracked `tests/doc-shard-digests/<track>--<document>.json`
  snapshots and their test-only reader. Roadmap and session-map validation now
  exercises every real plan and reconstructs it twice from its direct shards,
  alongside the existing filename, file-type, ordering, ignored-entry,
  append-only, and symlink guards.
- This removes the two hottest measured Punjabi serialization points: the
  session-map digest changed in 23 recent commits and the curriculum digest in
  22. Independent lesson owners no longer have to edit either aggregate.
