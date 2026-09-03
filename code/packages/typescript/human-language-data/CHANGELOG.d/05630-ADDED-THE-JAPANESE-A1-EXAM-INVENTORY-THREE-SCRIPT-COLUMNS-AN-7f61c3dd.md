### Added — the Japanese A1 exam inventory, three script columns, and the first repair column that closes

- `core/exam-inventory-japanese-a1.json` enumerates **179** A1 points and the
  corpus covers **66**, 37%. Zero partials: every probe atom was checked
  mechanically against the 111 atoms the 117 japanese lessons introduce.
- **The script column had to be three columns.** This corpus stands in three
  completely different places — **31 of 46 hiragana** signs taught, **2 of 46
  katakana**, and **3 kanji** — and a single "script" column averages those into
  a number that describes nothing and hides the fact that a reader who can
  decode a hiragana sentence still cannot read a menu, a name or a sign. So
  `Hiragana`, `Katakana` and `Kanji` are three categories, `Hyouki` holds what
  they share, and the mixed-script fact itself — the thing a single column would
  most obviously lose — is its own point at `JA-A1-HYO-01`, answered by chapter
  7's doorway dialogue, "six lines, three writing systems, and not one word that
  has not already been taught".
- **Three proxy orthography points have no Japanese analogue at all**, and are
  dropped in two entries with a reason each rather than swept into one.
  `A1-O1-04` and `A1-O1-05` (capital and lower-case letters): Japanese is
  caseless in *all three* of its scripts, and — unlike Chinese, where the same
  demand survives in pinyin — it does not survive in romaji, because romaji is
  not part of Japanese literacy and appears on no JLPT paper. `A1-O1-06`
  (superscript letters in abbreviations): no counterpart, and the entry names
  the track that *derived* the same point (Russian, `RU-A1-L-09`).
- **The exam anchor cannot score half the construct, and the file says so.**
  `exam-levels.json` records that the published CEFR indication covers "only the
  language knowledge, reading, and listening competence JLPT tests" and that
  "JLPT does not test production (speaking and writing) or interaction". An
  inventory that quietly reported reading and listening coverage would flatter
  this track exactly the way HL20 §1 warns about. So the four production tasks
  named in `japanese/assessment-spec.md#a1` are enumerated as points one for one
  — complete a small form, write a 30–40 character message for a named reader,
  answer familiar personal questions, complete a rehearsed role-play — plus
  interaction. **One of the four tasks is reachable**, and interaction, the half
  JLPT cannot see, is the track's strongest column.
- **The repair column reads 7 of 8, and it is the only complete CEFR A1 repair
  kit in the corpus.** Chapters 9 and 10 teach `sumimasen` (apologise and get
  attention), `wakarimasen` (report the failure), `mou ichido onegai shimasu`
  (ask for a repeat), `mou sukoshi yukkuri itte kudasai` (ask for slower
  speech), `koko` (point at the word you missed) and `wakarimashita` (confirm
  the repair worked) — the last of which almost nobody teaches. Russian has two
  of those moves and no word for sorry. Chinese has one and no word for sorry.
  Gujarati had none.
- **And the joining column is still 0 of 8.** Eighth track in a row, third
  outside South Asia. `demo`, `kara`, `node` and the quotative `to` return zero
  occurrences in kana and in romaji. Japanese is one of only **two tracks in the
  whole corpus with zero findings in every gentle-ramp queue** — no missed
  reinforcement window, no script-closure debt, no forward reference, no atom or
  glyph spike, no measurement-blind lesson — and it has Gujarati's joining
  column. That is the clearest evidence available that the hole is structural
  rather than a symptom of a track being neglected.
- What the track does differently, measured: **17 of its 117 lessons introduce
  nothing at all** and exist only to reach back, and its 392 declared reviews
  are spread almost flat across the windows — 123 at distance 1–3, **128 at
  5–15, 110 at 20–60** and 31 at 80–250. Chinese, for comparison, front-loads
  (232/105/45/24) and Russian collapses (120/44/28/2). Reviews are *scheduled*
  at each expanding distance rather than taken wherever they are convenient.
- The shape of what is missing is unusual and worth naming. Fourteen body words
  and nine family words, each with four-skill checkpoints — and **no word for
  "I"**, no `desu`, no question word of any kind, no adjective, no dictionary-form
  verb, and no numeral above the `ichi` inside `ichido`. A reader can name every
  part of their own head and cannot say that any of it hurts, and cannot say "I
  am Tanaka".
- Two script findings against `data/scripts/japanese.d`, which carries 49
  letters and 3 marks with cited stroke order: **seven of the nine already-sourced
  katakana are untaught**, and so are three of the six sourced kanji — including
  the `kon` inside `konnichiwa` and the `yuu` and `nan` inside `arigatou`, both
  of which the corpus already explains in etymology. Evidence checked in and
  unused. And three signs the writing lessons *do* teach — `ra`, `da`, `do` —
  are **not in the ductus at all**, so their pen path rests on prose plus a
  Unicode code chart, which shows a shape and does not order strokes.
- The one clear small debt: the **interpunct appears in 23 of 117 lesson files**
  — more than any other punctuation mark, because the body-part and family
  checkpoints list their words with it — and is introduced nowhere.
