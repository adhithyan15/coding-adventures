### Changed - Hindi track changelog authoring is sharded (HL-C327)

- Register `hindi/CHANGELOG.md` as a newest-first level-2 document-shard plan
  after issue #14245 measured 23 recent same-track commits touching the file.
  The committed `CHANGELOG.d/` fragments reconstruct the measured 24-section
  baseline byte-for-byte, while the concurrently landed child-form entry owns
  an independent rank. A dedicated 85,282-byte SHA-256 pin guards the baseline
  while ordinary document-shard checks enforce ordering, naming, and structure.
- Ignore the generated Hindi aggregate and make the first post-migration entry
  a directly authored fragment. Independent Hindi lesson, script, assessment,
  and repair tranches no longer share one top-of-file insertion point.
