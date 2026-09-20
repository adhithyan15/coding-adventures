## Unreleased — chapters 67–73: the joining column, closed, and eight letters

Twenty-seven headwords and eight script lessons, in seven chapters of five. Not
one was chosen by topic. Every one came off the uncovered list in
`core/exam-inventory-kannada-a1.json`, largest cluster first, because an
uncovered exam point NAMES the missing word and a vocabulary count does not.

    kannada A1 exam coverage      167/258 (65%) -> 193/258 (75%)
    joining column                  2/11        -> 11/11   (complete)
    negation                        4/5         -> 5/5     (complete)
    determiners                     1/2         -> 2/2     (complete)
    questions                       5/9         -> 8/9
    communicative functions        32/51        -> 42/51
    corpus exam-point backlog     786 -> 851    (this branch also lands the
                                                 inventory; see below)
    kannada lessons               268 -> 303
    kannada atoms taught          362 -> 412
    script characters TAUGHT       42 -> 50     (of 69 used in headwords)
    reinforcement misses          966 -> 895    (improved)
    atoms never revisited          20 -> 15     (improved)
    forward references             13 -> 13     (held)
    duration violations             0 -> 0      (held)
    never-taught glyphs             0 -> 0      (held)
    load-bearing closure viol.     10 -> 10     (held)
    distinct Kannada characters    69 -> 69     (held — no new glyph spent)

Every figure re-MEASURED against the merged tree. The denominator did not move:
no point was added, removed or reworded, and `partial` stayed at 0, so every
probe still names atoms that exist.

### The finding this answers, and what it cost to answer

The inventory measured the joining column at 2 of 11 and neither covered point
was a conjunction. `mattu` (and), `athava` (or), `aadare` (but), `eekendare`
(because) and the quotative `anta`/`endu` occurred **zero** times in 268 lessons
and zero times in the generated book — verified again here in both scripts, in
romanization, in markdown and in the compiled `.tex`.

Twenty-seven headwords closed twenty-six points, and nine of them are this
column. That ratio is a consequence of reading the list before designing the
chapters rather than after: `athava` closes "or" AND unblocks expressing a
preference; `alla` closes the negation pair AND gives the only way Kannada has
to disagree AND is half of the confirmation tag; the `-aa` question particle,
which no lesson had ever NAMED, closes asking whether somebody knows and asking
what they like at once.

### Order by dependency, not by topic

`anta` is taught before `eekendare`, because `eekendare` is literally *eeke*
("why") plus *endare*, the written quotative wearing the same `-dare` ending
that makes `aadare` out of the verb `aagu`. The learner meets the pieces, then
the word. Likewise `yaavaaga` before the `-aaga` when-clause, `ondu` before
`innondu`, `heelu` before "say it once more", and the quotative before the
opinion frame that needs it.

Kannada's real clause-chaining machinery — the `-i` participle of *hoogi
baruttene* — was already properly taught, and chapter 73 names it beside `-aaga`
and `-alu` as three shapes of one habit rather than three unrelated endings.

### Why NOT chapter 64

Chapter 64 is named `JOIN` and its five words connect turns, not clauses. It was
left alone. Its five words are a coherent set doing a real discourse job, the
chapter's `canDo` promises exactly that job, and cramming eleven clause-joiners
in beside them would have broken both the promise and the one-new-item-per-lesson
rate. The new material is in new chapters, and `KA-A1-CJ-10`'s note now records
that decision rather than leaving it to be rediscovered.

### The script: eight characters, one of them with a cited stroke order

The inventory measured 42 characters taught against 69 used in headwords. The
eight added are the eight most-used untaught ones — `ma` (36 headwords), `la`,
`va`, the vowel sign `ee`, `tta`, `ja`, `a` and the vowel sign `uu` — so a
learner who finishes the ladder can now read *maatanaaDu*, the verb chapter 5 is
built on.

Seven are recognition lessons in the ladder's established shape, which says in
as many words that this book does not yet know where the pen starts. **`a` is
different: its stroke order is SOURCED**, from Gopala Krishna A's 35-frame
Wikimedia Commons animation already cited in `data/scripts/kannada.json`, and
the lesson prints the four movements, the pen-lift count and the citation. It
also says why it is different from the seven before it.

Nineteen characters remain untaught and were left untaught. Six are independent
vowels whose ductus IS sourced and simply has not been spent; thirteen have no
source anywhere in this project and none was invented for them. `HL-C311`
records both lists and the method.

### Reinforcement went DOWN, and the parts are separated

A naive append measured 1040 misses against a prior 966. Decomposed:

    on atoms this tranche introduces        48
    on OLD atoms, window judgeable before  934   (966 before; 32 repaired)
    on OLD atoms, window NEWLY judgeable    58   (pre-existing debt exposed
                                                  by the extra length)

Three changes brought the total to **895**, below the starting point. Each
lesson retrieves at the R2 and R3 intervals as well as R1, arranged from the
ATOM's side — for every lesson here, the first WORD lesson standing at the right
distance is given its atoms to ask for. Each lesson also retrieves up to four
OLD words whose R4 window it can fill, naming their lessons as prerequisites,
because Kannada's prerequisite chain reaches only 113 of its 268 existing
lessons and the validator was right to refuse them otherwise. Only LEXICAL atoms
were taken for the distant retrieval: "say it once more" retrieves a word and
does not retrieve a claim about its etymology.

Final split: **3** misses on this tranche's own atoms, **868** pre-existing,
**24** newly exposed. Every declaration is backed by a `[YOU RECALL: …]` line
that asks for the words.

### Three ratchets held rather than reseated

Forward references stayed at 13. Making `bēku` a headword would have turned two
earlier passing mentions into forward references, so the lesson teaches the
fuller utterance `ನನಗೆ ಚಹಾ ಬೇಕು` — which is what a learner actually says, and
what `KA-A1-F-21` asks for. Chapter payoffs stayed at 4 findings: each new
chapter's payoff gathers everything the chapter taught up to that point, which
is what a payoff is for. Durations stayed at 0 violations, computed rather than
declared — the longest new lesson lands at 271s against the 300s ceiling, and
the opinion lesson was cut from 296s by removing prose rather than by editing
its declared number.

### Spent no new glyph, and checked the examples

All 27 headwords and every worked example are spelled from characters already in
the corpus. The distinct-character count held at 69.

