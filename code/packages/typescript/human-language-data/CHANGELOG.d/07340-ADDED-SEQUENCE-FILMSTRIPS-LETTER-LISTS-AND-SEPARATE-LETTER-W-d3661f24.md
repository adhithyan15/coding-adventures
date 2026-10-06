### Added — Sequence filmstrips: letter lists and separate-letter words print a stroke-order strip

- **What prints now.** A writing lesson whose headword names several letters
  used to print no filmstrip, because a filmstrip was one ledger entry. Now
  two kinds of headword become one strip that draws each letter's own cited
  frames in turn, one labelled group per letter ("Letter 2 of 3 — 2 strokes
  · 1 pen lift · 2 movements"):
  - a LIST of two or more single letters, separated by spaces, commas
    (`,` `،` `、`), em dashes or middle dots, in any script: `வ, க`,
    `ક — ણ — શ`, `в, р`, `ع ي`;
  - a WORD in a script whose letters stand apart (`SEPARATE_LETTER_SCRIPTS`:
    chinese, japanese, tamil, gujarati, kannada, telugu, malayalam), when
    every grapheme is one base letter: `はい`, `こんにちは`, `さようなら`.
- **28 lessons gained a filmstrip** (writing lessons with a strip: 494 →
  522): Arabic 7, Gujarati 7, Russian 5, Tamil 4, Japanese 3, Persian 1,
  Urdu 1. The filmstrip target-count pins move to match (arabic 22, gujarati
  41, japanese 53, persian 27, russian 27, tamil 34, urdu 30).
- **What is refused, and why.** A word is not composed in Devanagari (the
  word has one continuous headline; every cited letter draws its own), in
  the Arabic family (letters join and take positional forms; the ductus is
  isolated forms) or in Cyrillic (the cited source is connected school
  cursive). A word with a vowel sign, virama, nasal or length mark is not
  composed in any script, because some marks are written before the
  consonant they follow in Unicode and code-point order would draw them in
  the wrong order. A sequence prints only when EVERY letter is cited.
- **What the strip does not claim.** Each letter keeps its own panels at its
  own scale, so the figure says nothing about relative size, spacing or
  joins. Its `<desc>` says so, and names every letter's source.
- **Credits.** The footer prints each letter's citation once. One shared
  source reads as on a one-letter strip; otherwise each line names the
  letters it covers ("Letters 1 and 3: stroke order after …").
- **Layout.** Short letters share a shelf (`shelveLetters`): consecutive
  letters sit side by side while their frames fit across six panels, so a
  four-letter list is not shrunk to fit the book's 0.45\textheight figure
  cap. Group labels name letters by number, not by glyph, because figure
  text is set in Latin Modern Sans.
- **API.** `ScriptFilmstripTarget.letters` (optional), `writingSequenceOf`,
  `filmstripLetters`, `SEPARATE_LETTER_SCRIPTS`,
  `renderScriptSequenceFilmstripFigure`, `scriptSequenceFilmstripFigureSource`,
  `shelveLetters` and `letterNumbers`. `assertKnownFigureTarget` refuses a
  declared `letters` that names fewer than two non-empty letters. The book's
  alt text reads "How はい is written, letter by letter, stroke by stroke" for
  a word and "How the letters வ, க are written, …" for a list.
- **Unchanged bytes.** The one-letter renderer's panel, heading and citation
  code moved into shared helpers without changing output: `check:figures`
  reports only the 28 new SVGs and their seven track hash owners.
- **Regenerated.** 28 filmstrip SVGs, `core/generated-figure-hashes.d/` for
  arabic, gujarati, japanese, persian, russian, tamil and urdu, and 17 book
  chapters (one `\hlblockfigure` line each per new figure). Narration and
  modality outputs do not change: filmstrips are book-only.
- **Tests.** `tests/figure-filmstrip-sequence.test.ts` (layout, shelves,
  credits, refusals, purity, `renderFigure` dispatch, declared-target
  validation); `tests/figure-targets/sequence-strips-5e9c41a7.case.ts` (which
  headwords qualify, candidates, all-or-nothing citation, alt text); the
  real-corpus case now separates Tamil's 30 one-letter strips from its four
  letter-list strips and pins the three Japanese word strips and three
  refused joined-script words.
