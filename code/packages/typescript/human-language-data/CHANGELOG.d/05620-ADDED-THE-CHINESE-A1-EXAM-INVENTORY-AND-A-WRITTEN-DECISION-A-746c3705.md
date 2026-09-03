### Added — the Chinese A1 exam inventory, and a written decision about what pinyin coverage claims

- `core/exam-inventory-chinese-a1.json` enumerates **191** A1 points and the
  corpus covers **66**, 35%. Zero partials: every probe atom was checked
  mechanically against the 134 atoms the 175 chinese lessons introduce.
- **The Spanish orthography column does not survive contact with a logographic
  script, and it was not forced.** `A1-O1-01` asks for the alphabet — the closed
  set of units a reader learns once and reuses forever. Reading that as "the
  characters" would ask a beginner's track for tens of thousands and report
  every Mandarin course that has ever existed as failing. It is answered here by
  the **stroke**, which is the set the question was really about and which the
  corpus teaches, opening on `yi`: one horizontal that is both a stroke and a
  whole character. The two case points are stated as having **no character
  analogue at all** — hanzi is unicameral — and re-landed in pinyin, where the
  demand is real. Only `A1-O1-06`, superscript letters in abbreviations, is
  dropped, and its reason names both sides of the script question and names the
  track (Russian) that derived the same point.
- **Pinyin is treated as a pronunciation claim, not a script claim, and the
  argument is written into the file** at `ZH-A1-PY-06` rather than into this
  entry. Three pieces of internal evidence: the corpus files pinyin in the
  `romanization` field, which `script-closure.ts` treats as the promise that the
  reader need *not* decode; not one of the 45 `ZH-SCRIPT` atoms names a pinyin
  letter, initial, final or tone mark; and the track already splits the two
  claims itself, teaching a word by ear as a `ZH-LEX` atom and later as a
  separate `ZH-ORTHO` atom when it becomes readable and writable. The
  consequence is stated rather than hidden: the character column counts
  characters only, and pinyin's own orthography — its initials, finals,
  tone-mark placement, capitals and word division — is **untaught**, recorded as
  four uncovered points in a column of its own. A test asserts that no point in
  the pinyin column may be probed with a `ZH-SCRIPT` atom.
- **Tone gets five points and the proxy has no column for it anywhere.** Spanish
  spends pitch on mood; Mandarin spends it on the lexicon. The corpus is at its
  best here: tone is lexical in **chapter 1**, third-tone sandhi is taught on
  `ni hao` — the first word in the book, which is written with two third tones
  and said with a rising one — and `bu` sandhi on `bu shi`. 4 of 5. The missing
  one is tone across a phrase, which is what a listening paper presents.
- **Not one particle is taught.** `de`, `le`, `ma`, `ne`, `ba`, `guo` and `zhe`
  each return zero occurrences across all 175 lesson files, in characters and in
  tone-marked pinyin. Mandarin carries almost all of its grammar in these
  toneless syllables, so this is not a vocabulary gap: a Mandarin course with no
  particles has taught vocabulary and script and has not yet taught grammar.
  `ma` alone would turn every statement in the book into a question.
- **The joining column is 0 of 8**, the seventh track running and the second
  outside South Asia. `he`, `gen`, `huozhe`, `haishi`, `keshi`, `danshi`,
  `yinwei` and `suoyi` are all zero — every raw pinyin match for `he` is inside
  `heng` or `shenme`.
- **The repair column has one move and the track built a chapter around it.**
  Chapter 9 stages an exchange that goes wrong and gets repaired, and says why:
  "until now every exchange in this book has gone perfectly, which is not what
  conversations do". That instinct is right. What it has is `shenme?` — an
  interjection asking for a word back. `duibuqi` (sorry), `dong` (understand),
  `zai shuo` (say it again) and `man` (slowly) are each **zero**.
- `Fanyi` (translation and mediation) exists because `exam-levels.json`'s chinese
  caveat records that GF0025-2021 defines its levels "across listening,
  speaking, reading, writing, and translation" — a fifth skill a monolingual
  function inventory cannot enumerate. It reads **0 of 2**: zero of the 175
  lesson files declare `mediation` in `modes`.
- The surprises worth naming: **education is the strongest specific-notion field
  measured in any track in this sitting** — three institutions and four kinds of
  student built productively out of four characters — while **food and drink is
  five Spanish points and not one word**, in a language whose learners eat on
  day one. And the track can name the language and the script (`Hanyu`,
  `Zhongwen`, `Hanzi`, `zi`) and cannot say "to speak" or "to write".
- No HSK word list or character list is cited anywhere in the file, and no point
  claims a count against one; a test asserts that. The character column reports
  42 taught and 15 shown-but-untaught and does not divide either by a number
  nobody read.
