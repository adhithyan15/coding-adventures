### Changed — one authority for chapter titles and labels

- Derive generated and handwritten book chapter titles and labels from each
  track's canonical `chapters.json` capability ledger. The generation manifest
  now owns only chapter coordinates, output paths, and rendering options.
- Reject legacy duplicate metadata and fail closed when a book declaration has
  no capability entry, while retaining the corpus-wide title-drift gate against
  the committed LaTeX chapters.

