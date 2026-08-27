## 0.23.0 — flag the special-consonant rows in the matrix (syllabary, PR 7)

- **The matrix now marks its tricky rows.** In the consonant × vowel grid the
  retroflex **ḷa** and alveolar **ṟa / ṉa** rows — the ones a reader confuses
  with the ordinary *la / ra / na* — now carry a **★** and the same teal tint the
  Browse tiles give them, so in the full grid the confusable rows stand out at a
  glance. Connects the special-consonant flag (PR 4) to the matrix (PR 5).
- **No new judgement.** `buildSyllableMatrix` gains a `special` flag per row,
  computed by reusing the already-tested `specialConsonant` classifier on the
  row's base syllable — so the matrix flags exactly the same rows the tiles do
  (Telugu ḷa / ṟa; Malayalam also ṉa; Telugu has no ṉa). Control-tested: a "ḷa"
  row is flagged, "ka"/"la"/"ra" are not, and the real Telugu grid flags exactly
  the ḷa / ṟa rows. Zero new data.

