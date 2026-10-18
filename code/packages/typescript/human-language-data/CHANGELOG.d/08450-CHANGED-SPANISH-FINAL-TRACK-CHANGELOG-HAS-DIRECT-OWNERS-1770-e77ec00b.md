### Changed - Spanish final track changelog has direct owners (#17703)

- Spanish's 213-entry track changelog is now a strict newest-first
  `CHANGELOG.d/` document shard. Its former `CHANGELOG.md` is an ignored local
  projection, so parallel Spanish authors add distinct release-note owners.
- The 52 older detailed Spanish change reports moved byte-for-byte from the
  case-folding lowercase `changelog.d/` path to `change-notes.d/`. Their exact
  names and contents are pinned separately from the release-note shards.
- The document-shard plan, append-only deletion gate, aggregate resurrection
  gate, ignore policy, whitespace policy, and byte-exact migration tests now
  cover Spanish. Every registered human-language track therefore has a direct
  changelog owner boundary.

