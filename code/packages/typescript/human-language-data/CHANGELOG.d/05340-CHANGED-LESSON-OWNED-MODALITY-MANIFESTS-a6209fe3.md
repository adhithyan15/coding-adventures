### Changed — lesson-owned modality manifests

- Replace 23 generated per-language modality aggregates with 4,485 direct owners:
  23 stable metadata files and one owner for each of 4,462 lessons.
- Reconstruct the unchanged public manifest from canonical lesson owners while
  rejecting missing, stale, unsafe, nested, symlinked, or resurrected aggregates.
- Require exact identities from the language registry, parsed lessons, and narration
  hashes, and stage a complete validated replacement before removing legacy files.
