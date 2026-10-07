### Fixed — list filmstrips are captioned as lists, with no script comma in the macro

- New `src/filmstrip-caption.ts` (no imports, so the app can bundle it):
  `LIST_SEPARATORS` moves here from `figure-targets.ts`, with
  `listItemsOf` (two or more one-grapheme items: the test
  `writingSequenceOf` already used to draw a list, now in one place),
  `listNoun` ("letters", "signs", or "letters and signs"),
  `listCaptionParts` and `filmstripCaption` (the app's caption).
- `figure-targets.ts` `filmstripImageMarkdown`: a list strip now reads
  "How these letters are written, one after another, stroke by stroke:
  ن, ت, ث" (or "…, part by part, …" when a piece is a sign). The items come
  from the headword as authored, joined by an ASCII comma and a space. Before,
  the headword went into the sentence as it stood, so an Arabic list kept its
  Arabic comma ، (Script_Extensions=Arab), which the book renderer folded
  into the letter's macro: `\ar{ن،} \ar{ت،} \ar{ث}`. It now prints
  `\ar{ن}, \ar{ت}, \ar{ث}`. Lists that were captioned "How ૂ — ટ — ઈ — ઢ
  is written, part by part" read "How these letters and signs are written,
  part by part, stroke by stroke: ૂ, ટ, ઈ, ઢ". A single letter, a word, a
  sign-bearing word and a Devanagari word or phrase read exactly as before.
- Book chapters regenerated (captions only): arabic 4, gujarati 9,
  malayalam 2, persian 1, punjabi 3, russian 1, spanish 2, tamil 3, urdu 1.
  No figure, figure hash or chapter source hash changes.
- `language-ladder` captions its lazily loaded filmstrip with the same
  `filmstripCaption`: "How ক — ণ — শ is written, stroke by stroke" becomes
  "How these letters are written, stroke by stroke: ক, ণ, শ".
- Japanese JA-W01-hai-read: "Now write them touching:" becomes "Now write
  them side by side, one sign and then the other:", matching its strip,
  which draws は and then い in panels of their own. Book chapter 1,
  narration ch01 and the lesson's modality owner regenerated.
- Tests: `list-captions-name-their-letters` covers which headwords are
  lists, the noun, the Arabic caption and its LaTeX (each letter in its own
  `\ar{}`, the comma outside), other separators, sign lists, unchanged
  single-letter and word captions, and the app caption; the sequence-strip
  and Latin cases pin the new list wording.
