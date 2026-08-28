## Changed — bounded chapter-hash rollups

- Fold chapter-owned generated book hashes into one lazy virtual module per
  registered language instead of exposing one browser loader per chapter.
- Preserve the book-status fallback and development invalidation behavior while
  keeping the lazy loader table bounded at 23 current tracks.
