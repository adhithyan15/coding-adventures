### Added — HL12 payment two: Hindi joins, and Devanagari's citations get used
- **8 Hindi script segments** in chapters 6-13. Hindi had eleven writing lessons
  and **not one reached the page** — all eleven sit in the handwritten chapters
  1-5 and rendered only in the answer key, while chapter 1's prose promised
  *"Each lesson introduces the letters its word needs."* These are the first
  Hindi script lessons a reader of the book actually meets.
- **अ and आ now carry a cited stroke order** in both Hindi and Sanskrit: the
  numbered pen path, the pen-lift count, the source and its variation note, all
  read from `devanagari.json`. The first pass shipped them as tracing because it
  never looked for a citation. Nine of Devanagari's 28 letters are cited; the
  rest still trace and still say why.
- Every Devanagari vowel-sign segment shows the script file's worked example —
  **न + ◌ा = ना** *nā* — and every letter its component breakdown. Devanagari
  records these and the three Dravidian files do not, so their segments print
  what their script has rather than the least common denominator.
- Example words are now chosen so the reader can **say** them: a headword with no
  romanization is script they are only now learning to decode. Malayalam's first
  segment page had two of four bullets in exactly that state.
- Non-words are no longer offered as examples. A lesson whose headword is the
  character itself (Hindi's inherent-vowel lesson, headword अ) or a list of marks
  (`ा, े`) is not a word to find a character inside.
- The eleven existing `HI-W*` lessons now declare `delivery: script`, which they
  always were — adopting the marker made the *"script strand is declared, not
  inferred"* check apply to Hindi and it found all eleven at once.


