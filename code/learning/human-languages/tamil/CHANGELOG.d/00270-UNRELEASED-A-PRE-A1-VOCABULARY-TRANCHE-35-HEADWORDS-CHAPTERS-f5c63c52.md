## Unreleased — a pre-A1 vocabulary tranche: 35 headwords, chapters 67-73

The completion plan (`npm run plan`, spec HL15) measures **10,419 outstanding
vocabulary tranches — 92% of all remaining work to C2**, against roughly 1% for
script closure. Tamil's own entry read *"teaches 166 distinct headwords at or
below pre-A1, against 300"*: 134 short, four tranches. This is the first of
those four.

```
tamil pre-A1 vocabulary       166/300 -> 201/300   (shortfall 134 -> 99, 4 tranches -> 3)
corpus vocabulary shortfall   364,653 -> 364,618   (10,419 tranches -> 10,418)
tamil lessons                     317 -> 359
tamil script lessons               68 -> 75
taught glyphs                      51 -> 52       (ஓ, the last untaught independent vowel)
script-closure violations           0 -> 0        (held)
never-taught glyphs                 0 -> 0        (held)
headwords without romanization      0 -> 0        (held)
forward references                  7 -> 7        (held)
reinforcement misses (pre-A1)      80 -> 80       (held)
atom-budget violations              1 -> 1        (held)
```

Every number above was re-measured against the tree, not derived by arithmetic.

### 1. Seven chapters, five words each, all on pre-A1 nodes

| chapter | spine node | five headwords |
|---|---|---|
| 67 What You Wear | `SPINE-COURTESY-THANK` | சட்டை, வேட்டி, புடவை, செருப்பு, தொப்பி |
| 68 At the Shop | `SPINE-RESPOND-BASIC` | கடை, விலை, பணம், வாங்கு, பை |
| 69 Getting There | `SPINE-TAKE-LEAVE` | வண்டி, பேருந்து, ரயில், சாலை, நிலையம் |
| 70 The Room You Are In | `SPINE-EXCHANGE-NAMES` | மேசை, கட்டில், சுவர், தரை, கூரை |
| 71 Through the Day | `SPINE-MEET-GREET` | எழு, தூங்கு, ஓடு, கழுவு, விளையாடு |
| 72 How It Feels | `SPINE-CHECK-WELLBEING` | கோபம், பயம், வருத்தம், சிரி, அழு |
| 73 Today, Yesterday, and the Year | `SPINE-POLITE-REQUEST-REPAIR` | இன்று, நேற்று, முன்பு, மாதம், ஆண்டு |

**The node is what makes the words count.** `level-gate.ts` reads a lesson's
level from the spine node that claims it, so a tranche of perfectly good words
hung on A1 nodes moves the pre-A1 number by exactly zero. All seven pre-A1 nodes
are in rotation, one per chapter; nodes are freely reusable, so this is authoring
volume rather than a design problem.

**One new word per lesson, reuse unlimited.** Six lessons per chapter — five
words and one script lesson — 42 lessons for 35 headwords. Chapter and page count
are not a cost here and were not treated as one.

### 2. The two-back rollback, and why 42 lessons added no reinforcement debt

The first draft of this tranche was written the obvious way: each lesson requires
the word before it. It measured **98** reinforcement misses against a prior 80 —
because an appended run leaves each chapter's last atom, and every script atom,
revisited fewer than twice.

So every lesson now declares the **two** preceding items in `requires` /
`practises`, and — this is the part that makes the declaration honest rather than
bookkeeping — closes its Guided Practice with a line that actually retrieves them:

    - [YOU RECALL: say *vēṭṭi*, then read **சட்டை**, then say *puḍavai*]

Every atom in the tranche is now genuinely practised by the next two lessons. The
finding went back to **80: unchanged**, with 42 lessons added on top of it. It
also mixes the modalities by construction, since the chain alternates spoken
words with read ones.

### 3. Reading interleaved, one letter at a time

Each chapter carries a script lesson in **third** position, so the reader meets a
word by ear, says it twice, and only then sees it:

* `TA-W23-read-sattai` — the புள்ளி doubling a letter onto itself
* `TA-W24-read-kadai` — a sign drawn left of its letter and said after it
* `TA-W25-read-vandi` — the same புள்ளி between two *different* letters
* `TA-W26-read-mesai` — two front-hanging signs in one word
* `TA-W27-read-payam` — a word with no signs at all, which is what proves a bare
  Tamil consonant is not silent
* `TA-W28-read-inru` — a word opening on an independent vowel

Only one lesson in the tranche spends a new glyph. `TA-S126-letter-oo` teaches
**ஓ**, the last independent vowel with no lesson of its own — the same gap
`TA-S125-letter-u` closed for உ — and it lands **one lesson before** ஓடு, the
first word in the book that begins with it. `neverTaughtGlyphs` therefore stays
at 0 and closure stays exact: 52 shown, 52 taught.

### 4. வாரம் was cut rather than shipped

"Week" was the obvious fifth word for chapter 73, sitting between நாள் and
மாதம். It is not in the tranche, for a reason worth recording:

`TA-C10-vaara-kizhamai` **already prints வாரம்**, and prints it as the *Sanskrit*
word that Tamil's own கிழமை weekday names deliberately do not build on. A lesson
teaching வாரம் as "the first half of வார கிழமை" would have contradicted a chapter
already in the book, and it pushed `forwardReferences` from 7 to 8 — a word shown
sixty-three chapters before it is taught.

**முன்பு** ("before, earlier") took the slot: absent from every existing lesson,
and the opposite of பிறகு, which the reader already owns. The forward-reference
count went back to 7.

One further forward reference was authored and removed the same way:
`TA-C71-sleep` listed வருத்தம் and கோபம் as example nouns a chapter before either
was taught. The sentence now uses only தூக்கம், which the reader has.

### 5. What was regenerated

`generate:books`, `generate:narration`, `generate:modality`,
`generate:gentle-snapshots`, `generate:assessment-artifacts` and
`generate:figures`, with all eleven `check:*` gates green, the full
`human-language-data` suite at **124/124 files**, the `language-ladder` suite at
**39/39 files** plus its typecheck, build and bundle gate, and the Tamil book
compiled with XeLaTeX: **542 pages, zero errors, zero missing characters.**

