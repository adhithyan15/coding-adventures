## 2026-09-07 — The adjective: 142 → 151 of 301 A1 points, which is exactly half

- **Chapters 49–52 realize `SPINE-DESCRIBE-QUALITIES`, an A1 CORE node this
  track had never touched.** Forty-eight chapters could name a house, put a room
  in it, say who it belonged to and how to get there, and could not say one
  thing about what it was **like**. A1 exam coverage moves 142/301 (47%) →
  **151/301 (50%)**.
- **Twelve items, nine points:** `ADJ-01` (attributive and predicative),
  `ADJ-05` (prenominal position), `ADJ-07` (the invariable class), `NG1-03`
  (stating a general quality), `NG6-03` (attractiveness), `NG6-04` (good or
  bad), `NG6-08` (ease and difficulty), `NE01-02` (physical characteristics),
  `NE02-01` (character). **The adjective 2/7 → 5/7**, **Evaluative notions 4/8 →
  7/8**, **The person: the body 0/3 → 3/3**, and **The person: character** off
  the empty-category list.
- **What is taught, one item per lesson:**

  | ch. | items |
  |---|---|
  | 49 | **मोठा** big · the three-gender agreement · the prenominal position |
  | 50 | **लहान** small · the invariable class · **सुंदर** beautiful |
  | 51 | **चांगला** good · **खराब** bad · **हुशार** clever |
  | 52 | **सोपा** easy · **कठीण** hard · **उंच** tall |

- **The two classes are taught as a CONTRAST, not as a list.** A learner who
  meets only **मोठा** will say *लहानी खोली* by analogy, which no Marathi speaker
  has ever heard. So the invariable class arrives in the very next chapter and
  every lesson after it states which class its word is in, until the test — *does
  it end in **-आ**?* — is automatic. Six of the nine adjectives do **not** agree,
  which is worth the reader knowing so the endings stop feeling like the
  default.
- **A forward reference that had been sitting in the corpus since chapter 14.**
  `MR-C07-khane` demonstrated three-gender agreement with a table of
  **चांगला / चांगली / चांगलं** — a word the reader would not meet for another
  180 lessons. Teaching **चांगला** made that visible as a real
  `forwardReferences` defect, and the fix is better than a suppression: the table
  now runs on **माझा / माझी / माझं**, which the reader has owned since chapter
  nine, so the demonstration is on a word they can already say. `forwardReferences`
  went 4 → 5 and is back at 4.
- **Zero new glyphs, and the glyph GATE caught something the closure gate could
  not.** The first draft printed the Arabic **kharāb** and the Persian
  **hushyār** in their own scripts. Devanagari closure was fine — they are not
  Devanagari — but `glyph-coverage.test.ts` reported eight characters the book's
  main font cannot render, which is a compile failure waiting to happen. Both
  etymologies are now transliterated, which loses nothing a reader of this book
  could have used.
- **Two more regressions, both caught by re-measuring rather than reading.** A
  wrap-up in `MR-C50-sundar` tripped the rule-statement detector (30 → 31, a
  ceiling that may fall and never grow) and is rephrased; and four lessons
  derived `sight` on the phrase *"Look at the last letter"*, which is the whole
  chapter's method and does not require eyes — all four now derive `voice` and
  every chapter opens hands-free.
- **`reinforcementWindowMisses` 348 → 344; the tranche's own debt is zero.**
  R3 fell 144 → 142 and R4 80 → 78. Decomposed: **0 created**, **27 exposed** by
  the added length (R2 for 3 older atoms, R3 for 12, R4 for 17) and all
  retrieved on purpose, **4 pre-existing misses cleared**.
- **The corpus-wide banned-word debt FALLS, 1077 → 1076.** The prenominal
  lesson's first draft wrote *"can simply trust the order"*, which `HL10 §7.4`
  bans and which is a ceiling that may fall and never grow. Rather than spend
  the word and then buy it back, two pre-existing occurrences in chapters 14 and
  15 went with it — both were the same tic, and neither sentence needed it.
- **`atomsTaught` 272 → 284; `atomsNeverRevisited` 1; `durationViolations` 0;
  `scriptClosureViolations` 0.**
- **What was deliberately left uncovered, with the reason written into the
  inventory:** `MR-A1-ADJ-06` and `MR-A1-AP-01` (there is plenty to modify now;
  *खूप*, *जरा* and *पुरेसा* do not exist, so an adjective can be applied and not
  graded), `MR-A1-PRON-07` (both ingredients of the exclamative now exist as
  words and the CONSTRUCTION does not — not claimed on the strength of its
  parts), `MR-A1-ADJ-04` (the singular column of the paradigm is complete; the
  plural needs `MR-A1-N-05` to `-07`), `MR-A1-NP-01` (the adjective complement
  closed; the relative clause needs `MR-A1-PRON-05`), and `MR-A1-NG5-04` (age
  needs a number above five).

