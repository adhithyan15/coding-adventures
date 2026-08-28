### Changed — chapter-owned generated hash ledgers

- Store generated book and narration fingerprints as stable per-chapter owners
  beneath each language directory instead of one shared language array.
- Make generation, drift checks, and progress discovery consume the canonical
  owner directories directly and reject unsafe or unexpected files without
  recreating a tracked aggregate.
