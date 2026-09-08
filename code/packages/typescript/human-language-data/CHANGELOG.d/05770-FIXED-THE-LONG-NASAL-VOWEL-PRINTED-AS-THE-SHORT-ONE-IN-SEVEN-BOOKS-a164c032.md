### Fixed — the long nasal vowel printed as the short one in seven books

- Bengali, Gujarati, Hindi, Marathi, Marwadi, Punjabi and Urdu romanize a LONG
  NASAL vowel as a macron vowel plus `U+0303` COMBINING TILDE — `gā̃v`, `nahī̃`,
  `tū̃`. Latin Modern set that as the SHORT nasal vowel: the macron was dropped,
  so `gā̃v` printed `gãv`. A reader of the PDF could not tell a long nasal vowel
  from a short one anywhere in seven books — 346 occurrences in the generated
  books (Punjabi 180, Gujarati 47, Hindi 47, Marwadi 30, Urdu 25, Bengali 16,
  Marathi 1), out of 1,837 across the whole corpus. The larger figure describes
  the convention; the smaller one is what a reader actually sees, because the
  rest lives in lesson frontmatter and narration scripts that are never typeset.
- **Nothing in this repository could see it.** Every gate reads bytes and the
  bytes were right: the lesson wrote `U+0101 U+0303`, the generator emitted
  `U+0101 U+0303`, the hash ledgers agreed, `glyph-coverage` found both
  characters in the font's cmap, and XeLaTeX reported no missing character —
  because nothing was missing. The failure is MARK STACKING, and the only
  witness is the rendered page.
- Diagnosed by rendering rather than by reading. Latin Modern drops the first
  mark; Times New Roman and Georgia both stack the identical byte sequence
  correctly, which places the cause in the font and not in XeTeX or the source.
  The decomposed form, the reversed mark order, `\~{\=a}` and every renderer
  option fail the same way, because they all funnel into the same shaping.
- There is no precomposed character to reach for: Unicode has no "a with macron
  and tilde", nor one for i or u. And `\newunicodechar` maps ONE character while
  what fails is a two-character SEQUENCE — by the time TeX reads the tilde the
  vowel is set, and TeX cannot look backwards. So the lookahead goes on the
  VOWEL, which can see what follows: `_shared/nasal-macron.tex` makes `ā`, `ī`
  and `ū` active, peeks with `\futurelet`, and on a following combining tilde
  swallows it and sets the mark raised clear of the macron.
- **No lesson changes and no generated `.tex` changes by one byte.** The
  romanization was correct; the rendering was wrong. Respelling the words across
  seven tracks to work around a font would have moved the workaround into the
  curriculum. A paragraph of plain, non-nasal `ā`/`ī`/`ū` renders pixel-identical
  with and without the fix, which matters because plain vowels vastly outnumber
  nasal ones.
- Two earlier drafts of the shared file failed in ways the page did not show,
  and both are now pinned by `nasal-macron.test.ts`. A scaled `\textasciitilde`
  looked right and put an ASCII tilde in the PDF's text layer, so `pdftotext`
  read `sāp` and the nasalization vanished from every copy, search and screen
  reader. Centring that tilde in a box the width of the base then displaced it
  by a whole letter in the Punjabi table of contents, because a combining mark's
  ink sits LEFT of its origin. Setting `U+0303` itself directly after the vowel
  fixes both: zero advance width, the font's own placement in upright and
  italic, and `U+0101 U+0303` in the text layer.
- Verified by compiling all seven books with XeLaTeX — 0 errors, 0 missing
  characters, 0 overfull boxes — and by rasterising pages and reading them as
  images, including the Punjabi table of contents, which is where the
  displacement bug showed up and where the moving-argument and PDF-bookmark
  paths are exercised.
- `nasal-macron.test.ts` derives which tracks need the fix from the generated
  book text rather than from a list, so a track that adopts the convention
  tomorrow is required to carry the fix tomorrow. Falsified by removing the
  input from Urdu's preamble, which names Urdu and fails.
- The security review of this change found the one way the fix can be undone
  with no evidence anywhere: `newunicodechar`'s Unicode branch has NO
  redefinition warning — "Redefining Unicode character" exists only in its 8-bit
  branch — so a later `\newunicodechar{ā}` silently wins and the page reverts to
  the unfixed bug byte for byte. Marwadi's preamble already declares
  `\newunicodechar{ā}{\={a}}` before the input, so the tripwire is real and one
  reordering away. The test now asserts that nothing redefines `ā`, `ī`, `ū` or
  `U+0303` after the input, and it is falsified by moving Marwadi's line down.
- The same review corrected two claims that read better than they were true. The
  fragment does not depend on graphicx — `\raisebox` is a kernel command — so
  the ordering assertion now names hyperref, the dependency it actually has,
  instead of `visual.tex` as a proxy that a preamble could satisfy while
  breaking. And `\pdfstringdefDisableCommands` is INERT here, because
  `newunicodechar` declares its characters `\protected`: bookmarks were already
  carrying `U+0101 U+0303` in full. The block is kept as a guard and the comment
  now says so rather than claiming a collapse that does not happen.
