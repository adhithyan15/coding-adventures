### Changed — hindi and marathi pronunciation references are generated

- Both `appendix-pronunciation.tex` files are rendered from
  `<track>/pronunciation-reference.md`. Both use the new `runningHead` field so
  the head over the page still reads "Devanagari".
- The Hindi Markdown nested a bullet list inside a numbered fact. A reference's
  numbered list reads an indented line as a continuation of the item, so the
  compiled page printed a literal hyphen where the bullet had been. The mātrā
  signs are authored as one middot-separated run instead — a generator limit
  worth knowing about, not a bug in this track.
- `marathi/book/preamble.tex` gains `\newunicodechar{ĕ}{\u{e}}`. Latin Modern has
  no U+0115, so the transliteration of the English-loanword vowel ॲ would have
  been a hole; the glyph-coverage gate caught it.
