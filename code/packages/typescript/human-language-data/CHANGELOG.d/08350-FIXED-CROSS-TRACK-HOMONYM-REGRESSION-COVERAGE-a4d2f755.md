### Fixed — cross-track homonym regression coverage

- Pin representative real-corpus root slugs corrected by #17565, including
  Sanskrit nine/new, garland/dirt, yellow, black/art, and Latin free/book.
- Assert that the cousins join keeps genuine same-etymon pairs while excluding
  distinct homonyms. The test reads only named lessons and does not change
  curriculum content, lesson durations, or validation gates.
