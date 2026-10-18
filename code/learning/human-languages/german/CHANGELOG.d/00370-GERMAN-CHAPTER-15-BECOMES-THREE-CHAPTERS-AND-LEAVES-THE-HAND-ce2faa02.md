## German chapter 15 becomes three chapters, and leaves the hand-written set

`ch15-perfekt.tex` — the top of the German hand-written range — is now
generated, as **chapters 15, 16 and 17**. German's hand-written chapters:
**10 -> 9**. Old chapters 16-33 renumbered to 18-35.

### Sizing it four ways, and none of them was the size

- `handwritten_parity.py` scored it at a gap of **5 blocks**.
- `grep -l '^chapter: 15$' lessons/*.md` said **3 lessons**, where the `.tex`
  rendered **2 sections** — the third, `GE-C15-praeteritum-map`, was on no
  curriculum path at all and reached the book only through the chapter ledger.
- Counting the German forms it teaches gave eleven: *gesagt*, *gemacht*,
  *gelernt*, *gewohnt*, *gehabt*, *gestern*, *sagte*, *machte*, *hatte*,
  *konnte*, and the two tense names.

The real cost is the **person slots of a compound tense**. The `.tex` printed
the *Perfekt* with three rows because three rows are enough for a reader to
infer the fourth, and that inference is exactly what
`maxNewGrammarCellsPerLesson: 1` forbids. Four cells, four lessons — the fourth
because the chapter's own closing line, *Er machte das … Er hat das gemacht*,
uses a person the table never showed.

Add the participle recipe and its four verbs, the clause-final bracket, the
*-te* and its four verbs, register, the survivors and the areal map, and the
material is **twenty-four atoms** against a `maxNewAtomsPerChapter` of twelve.
Length is never a cost here, so it became **three chapters of 7, 8 and 9 atoms**
rather than one chapter at twice the ceiling. Three lessons became
**twenty-seven**.

### The running example was never taught

Every participle in the chapter is built on *sagen*, and *sagen* was not a
headword anywhere in the track. Hand-written prose can use a word it never
introduced; a generated chapter cannot. Chapter 15 now opens with *sagen*
(`GE-C15-sagen`) — one lesson that no count could see, because it is not new
material but material the chapter assumed.

### Every paradigm-shaped table is a recap

The infinitive-to-participle table now sits in `GE-C15-partizip-practice`, after
six lessons that built it one verb at a time. The four *Perfekt* slots sit in
`GE-C15-perfekt-practice`, after four lessons that met them one at a time. The
*Präteritum*-against-*Perfekt* comparison sits in
`GE-C15-praeteritum-practice`, and its first column is a meaning rather than a
person, so it is a comparison and not a paradigm at all. Corpus `paradigmTables`
is unchanged at 95 and `lessonsWithFindings` unchanged at 121: one finding left
`GE-C15-perfekt` and one arrived in `GE-C15-perfekt-practice`. `fullParadigmGrids`
is 21, which is where German chapter 5 left it; nothing here moved it.

### Lesson ids did not move

All twenty-seven lessons keep the `GE-C15-*` prefix and live across chapters 15,
16 and 17 — the Spanish convention, where `ES-C03-*` lessons sit in chapters 4,
5 and 6.

### Counters, re-measured against the tree rather than derived

| Measure | Before | After |
|---|---|---|
| German hand-written chapters | 10 | **9** |
| corpus hand-written chapters | 20 | **19** |
| `handwritten_parity.py` german | 51 | **46** blocks at risk |
| German lessons (schema-v2) | 139 | **166** |
| atoms taught | 297 | **321** |
| atom-measurement-blind lessons | 21 | **18** |
| chapters over the 12-atom budget | 5 | **5** |
| culture claims | 14 | **16** |
| atoms never revisited | 81 | **81** |
| forward references | 36 | **36** |
| cross-chapter prose references | 0 | **0** |
| book pages | not re-measured on `main` | **327** |

Measured against the merged tree, not composed: German chapter 5 landed on
`main` while this branch was open, so every "before" above is `main` after that
commit rather than the number this branch started from.

Forward references held at 36 because both candidates were phrased around rather
than cut: the closing "Next:" line names the job instead of *sein*, and the
survivor list says "the past of the verb for 'to be'" instead of *war*, which
chapter 19 teaches inside the comma-separated headword `bin, ist, war`.

### One defect the gates caught

`GE-C15-du-hast-gesagt` wrote "the part of this tense that **never has** to be
relearned" — a sentence about the learner, not about German. `info-dump.ts`
matched it as a rule statement and pushed the pinned ceiling from 30 to 31. The
sentence was rewritten; the pin was not moved.

### Verification

124/124 test files; all eleven `check:*` gates; language-ladder 39/39 files,
442 tests; the German book compiles under XeLaTeX with zero errors, zero
overfull or underfull boxes and zero missing characters (327 pages), and
chapters 15, 16 and 17 were read on the page. All 52 teaching claims in
`ch15-perfekt.tex` were checked across into the new lessons.

