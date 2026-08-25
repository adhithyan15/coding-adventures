### Added - book.tex is generated, and a compile gate (HL21 section 6)

- `<track>/book/book.tex` is now emitted by `book-cli`, so it gets `--write` and
  `--check` like the chapters, the narration and the hashes already do. It was
  the last hand-maintained link in the lesson -> chapter -> book chain, and the
  only one that could be forgotten without anything failing.
- Split by ORIGIN rather than by size: authored `frontmatter.tex` and
  `backmatter.tex` sit beside it, and only the `\input` list between them is
  derived.
- The chapter list merges `targets[]` **and** `handwritten[]` by chapter number.
  Using `targets` alone would silently drop 16 chapters from French and German.
- Verified against all 23 tracks: the generated file reproduces the committed
  one byte for byte for 8 tracks, and differs only by blank lines for the other
  15. No track differs by anything else. `spanish/book/book.tex` is one of the
  byte-identical ones and is unchanged.
- Add `code/scripts/check-book-compile.sh` and `npm run check:compile`: nothing
  previously checked that the LaTeX *compiles*, only that bytes matched. Opt-in
  and not wired into `vitest run`, because 23 books take ~100 seconds. Tracks
  whose figures need an absent SVG-to-PDF converter are skipped with a message
  rather than failed.

