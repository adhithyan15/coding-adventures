### Fixed — the main-font charset was a curated allow-list with 407 holes

- `core/main-font-charset.json` is the sole authority for whether a character in
  main-font text reaches the reader. It was CURATED — hand-added one character at
  a time — and held 123 of the 530 non-ASCII characters Latin Modern actually
  sets. A curated allow-list fails in exactly one direction: silently, against the
  author. `1.º` was unwritable in every book in the corpus until the ordinal
  tranche added `U+00AA` and `U+00BA` by hand, and `german/CHANGELOG.md` records
  capital `Ü` being avoided for the same reason while the font has had it all
  along.
- The file is regenerated from the font now: the Unicode cmap subtables of the
  four Latin Modern Roman text faces, minus ASCII, minus the Private Use Area, and
  minus the two invisible characters. 123 entries become 530, and no entry is
  removed — the escape-hatch `\newunicodechar` mappings live in the preambles and
  are untouched.
- **Verified by rendering, not by reading the cmap.** Two XeLaTeX passes with
  `\setmainfont{Latin Modern Roman}`: a listing document reported zero
  `Missing character` lines across all 407 additions, and a one-character-per-page
  document was rasterised and each page measured for ink, so a cmap entry pointing
  at a blank glyph could not pass as support. Four measured zero ink at 72dpi and
  were re-rendered at 600dpi — `U+00AF`, `U+0331`, `U+0332`, `U+2423`, all thin
  single strokes — and all four set. `U+00A0` is the one glyph in the cmap with no
  outline at all, and it is excluded.
- A trap found on the way: unioning every cmap subtable inflates the answer from
  794 codepoints to 828, because each face carries a `(1,0)` Mac Roman format-6
  subtable whose keys are BYTE VALUES, not codepoints — which makes `0x80..0x9F`
  look like mapped characters when they are the C1 control range. Only the Unicode
  subtables are read, which is also what `loader.ts`'s own font reader does for
  every vendored font.
- The small-caps faces carry 787 rather than 794. The six they drop — long s and
  the five f-ligatures — are recorded in the file rather than lost, with the fact
  that `\textsc` and `\scshape` appear zero times across every `book/` directory.
- Nothing checked the file itself: `loadMainFontCharset` reads one field and
  throws only on an empty array. `glyph-coverage.test.ts` now refuses a file that
  contradicts itself — a `cp` that disagrees with its `char`, a duplicate, an
  unsorted list — and refuses the three kinds of entry the regeneration excludes,
  which is where a hand-edit would land. Falsified: a wrong `cp`, a duplicate, a
  planted Private Use entry and a deleted small-caps entry each break it.
- The corpus gate stays non-vacuous. `ɔ` (Bengali, HL-C223), `ǣ` (Latin, HL-C214)
  and `ẞ` are genuinely absent from Latin Modern and are asserted absent from the
  list, so the planted-character test that proves the gate still fires.
- The security review of this change caught the new self-consistency test closing
  four instances rather than the class: it listed ASCII, the Private Use Area, the
  C1 range and the two space-like characters by hand, so a later hand-edit adding
  `U+202E` RIGHT-TO-LEFT OVERRIDE, `U+200B` or `U+FEFF` would have passed a gate
  whose whole stated purpose is catching that. It asserts the Unicode categories
  `\p{C}` and `\p{Z}` now — the property "renders nothing" the exclusion was
  always about — with a control proving the regex discriminates.

