### Changed — french, spanish, german and latin pronunciation references are generated

- Four more `appendix-pronunciation.tex` files are rendered from
  `<track>/pronunciation-reference.md` instead of being hand-written. Prose that
  existed only in the LaTeX was carried into the Markdown first; each track's
  changelog lists it claim by claim.
- Two defects were caught by compiling the pages rather than by any assertion: a
  U+025B in the French Markdown that Latin Modern has no glyph for (fifteen
  missing characters, holes where the nasal vowel is taught), and a stray Han
  character in the German Markdown that the German book has no font for. Both are
  fixed at the source.
- The German consonant-shift table is authored as bullets rather than a Markdown
  table. Reference tables render as one labelled record per row, which suits the
  wide Cyrillic and Urdu tables it was built for and not three narrow columns.
