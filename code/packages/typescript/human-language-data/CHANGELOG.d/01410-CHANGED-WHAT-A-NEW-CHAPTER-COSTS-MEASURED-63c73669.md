### Changed - what a new chapter costs, measured

Four wiring points are needed, and the first alone is not enough:
`chapters.json` (the capability ledger), `book-generation.json` (the target),
`tamil/book/book.tex` (the `\input`), and `curriculum.json` (path segment,
extension, spine segments). Declaring only the ledger fails the book-cli gate
"puts every ledgered chapter into its book, not merely into a file", which is
exactly the check that exists to catch this.

Pins re-derived by set difference against `origin/main`:

- `atomsTaught` 2652 -> 2660; `pre-A1` 878 -> 879 and `A2` 409 -> 412;
  ramp-to-A1 1187 -> 1188 with `TA-W20-read-onru` the only joiner; manifest
  `totalLessons` 1690 -> 1694, `chapterCount` 513 -> 514, `pen` 68 -> 69,
  `sight` 508 -> 511, `unstartableChapters` 137 -> 138.
- `missedByWindow.R2` 1808 -> 1816, and all eight entrants are one mechanism:
  the track grew 128 -> 132, so a window becomes evaluable for exactly those
  atoms whose `introducedAt + window.from` falls in (127, 131]. They are four
  two-atom pairs — VIDAI at 126, SUGAM at 125, UDAMBU at 124 and
  IVAR-EN-NANBAR at 123 — and not one of their revisit counts changed.
  R4 243 -> 247 is the same arithmetic at 80. R3 does not move at all —
  1309 -> 1309, seven in and seven out — which is the whole argument for
  declaring what a sentence
  actually re-uses. `TA-C39-vendum` names **தெரியும்**, **புரிகிறது** and
  **பிடிக்கும்** in one clause and credits all six of their atoms: the two
  `PIDI` atoms (index 108) land a revisit at exactly distance 20, R3's first
  position, so neither enters, while `TA-LEX-PURI-01`, `TA-GRAMMAR-PURI-02`
  (index 100) and `TA-GRAMMAR-TERI-02` (index 98) leave R3 outright and drop off
  the defect list. The same clause with the verbs merely named would have read
  identically on the page and left R3 five windows worse.
- Against that, ten atoms LEAVE a window, and those are the chapter earning its
  keep: `TA-SCRIPT-EE-SIGN-01` 1 -> 2 revisits, `INDEPENDENT-VOWEL-E-01` 2 -> 3,
  `NGA-LLA-01` 2 -> 3, `TTA-01` 1 -> 2, the two `PURI` atoms and
  `TA-GRAMMAR-TERI-02` out of R3 (seven in all);
  `GRAMMAR-DATIVE-SUBJECT-02` 2 -> 3, `LEX-DATIVE-SUBJECT-01` 3 -> 4 and
  `LEX-NUMBERS-1-5-01` 2 -> 3 out of R4.
- `atomsNeverRevisited` 472 -> 474, five in and three out. IN are
  `TA-GRAMMAR-EVVALAVU-VS-ETHANAI-02`, `TA-LEX-ORU-01`,
  `TA-GRAMMAR-ORU-ATTRIBUTIVE-02` and TA-W20's own `TA-SCRIPT-O-VOWEL-01` and
  `TA-SCRIPT-READ-ONRU-02`; OUT are `TA-SCRIPT-READ-MUUNRU-02`,
  `TA-GRAMMAR-PIDI-02` and `TA-SCRIPT-UU-SIGN-01`, each 0 -> 1 revisits. The
  last is TA-W19's own sign, credited where TA-W20 contrasts **மூன்று** with
  **ஒன்று**. The 422-atom defect
  subset moves separately, 422 -> 424; the two counters are worth keeping apart.
  The `ORU` pair being among the entrants is structural, not
  an oversight: `TA-W20` genuinely re-reads **ஒரு**, but a writing lesson may
  only take other writing lessons as prerequisites — `TA-EXT-003-SCRIPT` is
  inlined at `TA-PATH-003`, so naming a chapter-39 lesson would place the
  prerequisite after its dependent and fail the ordering rule. The tie is
  carried by `reviews_of`, which is not a revisit. Chapters 40 and 41 are
  planned to close it.
- `forwardReferences` 423 -> 424, and the new entry is a measurement
  improvement rather than fresh damage: `TA-C18-mani-homophone-time` has always
  printed **ஒரு**, but no lesson owned the word, so the checker had no teacher
  to measure against. Naming one made a 65-lesson-old early use visible. It is
  also an argument that **ஒரு** belongs earlier than chapter 39, which the
  runway did not allow.

