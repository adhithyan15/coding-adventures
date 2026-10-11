### Fixed — 133 atoms no later lesson practised are now revisited, and every track holds that at zero

The gap report's `atomsNeverRevisited` falls from **134 to 1**. Each of the 134
was an atom a lesson introduces that no later lesson in its track practised,
either in `practises.knowledge` or in a block's `assesses`. Most were etymology
atoms (`*-ETYMON-*`). Every one was read against the lessons after it, in 13
tracks (spanish 74, german 23, tamil 11, french 7, chinese 4, persian 4,
sanskrit 3, portuguese 2, telugu 2, italian 1, marathi 1, punjabi 1, urdu 1):

- **52 were already being practised and not declared.** A later lesson's body
  already reuses the word, rule or etymology, so only `practises.knowledge` and
  that block's `assesses` changed. Example: ES-C40-repaso-palabras-con-historia
  asks "where did you come from?" of *la hora*, *veinte*, *sábado*, *azul* and
  *amarillo*, and gives each one's history, but it only declared their `ES-LEX-*`
  twins. Another: GE-C06-zahlen-1-5's table says *zwei* keeps "the *tw-* German
  still says" and *fünf* has "an older **p**, now an **f**". These are the
  `*-ETYMON-*` claims, inside a lesson that declared only the numbers.
- **81 had no later use, so they got one short spoken recall.** It is a
  `[PAUSE 2s]` question with its answer in brackets, placed in a later review,
  practice or warm-up. For example, GE-R20 asks "Which old sound says *no* in
  *nein*, English *no* and Latin *nōn* alike?" TA-C23-naal asks whether
  *mañcaḷ* is borrowed from Sanskrit. Each one names only words that were
  already taught. Each one stays spoken, with no table, page or column cue. Each
  one keeps its lesson under the 300-second budget (the highest effective
  duration among edited lessons is 290 s). Eight practising lessons gained the
  introducing lesson as a prerequisite so the closed-world validator can see the
  atom. Each of those prerequisites comes earlier in curriculum path order.
- **1 is left: ZH-SKILL-CONNECTED-READING.** ZH-C29-diyi-ke introduces it, and
  that is the last lesson of the Chinese track. Nothing comes after it to
  practise it. HL09 has no "one-off atom" notion, and every etymology atom in
  the corpus is otherwise practised, so no atom was waved through as one.

The reinforcement windows close with it: R1 misses 1,606 → 1,553, R2 19,891 →
19,841, R3 16,011 → 15,991, R4 24,363 → 24,357.

**New gate.** `tests/corpus/assert-language-corpus.ts` gains
`atomsNeverRevisitedPastTheTail(report)`, and `expectLanguageContinuity`, which
all 23 tracks' corpus pins call, now requires it to be empty. The raw count
cannot be a ceiling: the shared continuity test explains that every atom at a
track's tail is unrevisited until something is appended. So the gate judges
only atoms the track has gone a whole R1 window (three lessons) past. That was
133 before this change and is 0 now. A synthetic fixture in
`tests/continuity.test.ts` proves it fires on a non-tail atom. It ignores an
atom with one lesson after it, and the boundary is exactly R1. It does not read
the debt off the real corpus.

Punjabi's exact `missedByWindow` pins in `tests/corpus/punjabi/` each lose one
R3 miss. There are 33 of them across 17 case files, and each one carries an
annotation saying why. PA-R48-courtesy-and-parting now declares
PA-ETYMON-RAAT-RATRI, 52 lessons after PA-C36-shubh-raat. Three pins measure
only a slice of the track before chapter 36, so they do not move.

Unchanged: validator errors (0), durations (0 violations), script closure (0),
glyph budget, chapter gates and payoffs, forward references (1,173), drivable
counts (186 = 178 by type + 8 by content), and the technical metalanguage uses
before introduction (6,586). Two recalls first broke these numbers and were
reworded. One leaked *difícil* and *llave* as forward references. The other said
"the table", which made ES-C44-repaso-articulos sight-only. No source file
changed. The book, narration and modality outputs are regenerated for the 93
edited lessons.
