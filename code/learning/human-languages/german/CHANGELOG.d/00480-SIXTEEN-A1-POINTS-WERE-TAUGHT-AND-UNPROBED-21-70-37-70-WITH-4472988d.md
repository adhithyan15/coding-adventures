## Sixteen A1 points were taught and unprobed: 21/70 → 37/70, with no lesson changed

`core/exam-inventory-german-a1.json` had **49 points with `probe: null` and not
one note between them**. `exam-inventory.ts` documents that value as "no atom in
the corpus corresponds to this point, which is a finding rather than a gap in the
data" — so all 49 scored as uncovered and the track read **21/70 (30%)**, the
lowest A1 coverage in the corpus.

Reading the 297 lessons rather than the notes shows that **sixteen of the 49 are
taught in full**. They were not findings. They were points nobody had written a
probe for, and the eleven generated chapters that closed them landed after the
inventory was authored.

| point | what already teaches it |
|---|---|
| A1-N-01 nouns are capitalised | `GE-W03-capitalization` states the rule and the part-of-speech signal under it |
| A1-N-02 three genders, unpredictable | `GE-C01-der-die-das` — "German **kept all three** … the gender is unpredictable and must be learned *with* the noun" |
| A1-ART-01 der / die / das | the same lesson |
| A1-ART-02 ein / eine | `GE-C06-ein-eine`, off the der/die/das split |
| A1-PRO-01 nominative pronouns | eight pronouns, eight lessons, chapters 2–5 |
| A1-PRO-03 du / ihr / Sie | `GE-C02-du-sie` and `GE-C05-ihr`, which prints the register grid |
| A1-V-01 present of *sein* | chapter 26 gives **one lesson per person**, all six |
| A1-V-02 present of *haben* | chapter 22 does the same, four regular plus two broken |
| A1-V-03 weak present endings | stem, *-e*, *-st*, *-t*, *-en*, *ihr -t* — five atoms, chapters 2 and 5 |
| A1-V-08 Perfekt with *haben* | chapter 24, the helper plus four persons |
| A1-V-09 Perfekt with *sein* | chapter 29, the motion rule plus its learned list |
| A1-SATZ-01 verb second | named in `GE-C02-wie-heissen-sie` |
| A1-AUS-01 the three umlauts | *ä*, *ö*, *ü* as three sound atoms plus `GE-W02-umlauts` |
| A1-LEX-01 greetings and farewells | seven, chapters 1 and 4 |
| A1-LEX-08 days, months, seasons | all seven, all twelve, all four |
| A1-LEX-12 everyday verbs | fourteen with their own lessons |

### Why this was invisible

Every one of those points is carried by **several atoms, not one**. The
paradigms are the clearest case: chapter 26 deliberately teaches *sein* one cell
per lesson because `maxNewGrammarCellsPerLesson` is 1, so there is no
`GE-GRAMMAR-SEIN-PRESENT` atom to notice — there are six. An inventory written by
looking for a single obvious atom name finds nothing and writes `null`, and
`null` then reads as *the corpus does not teach this*.

The consequence was not cosmetic. A tranche aimed at this inventory would have
written a second lesson for *der/die/das*, for the six forms of *sein*, and for
the twelve months. **An inventory exists to prevent exactly that**, and this one
had been pointing the other way for months.

### What is left, and why it is now readable

The other **33 points now each carry a note** saying what the corpus holds and
what is missing, so the next reader does not repeat the 297-lesson pass. The
notes are specific rather than "untaught": *nicht* is half owned (the word, not
its position); *einen* is an **explicit deferral** — `GE-C14-einen` says on the
page that "the system behind it … gets a chapter of its own later" and that
chapter is unwritten; `in` and `auf` are both owned as words and want only the
two-way rule; three of the six W-words have lessons and *wer*, *wann*, *warum* do
not.

Two gates were added rather than assumed. `tests/exam-inventory.test.ts` now
holds German to the rule Marathi has had since HL-C290 — **an unmapped point must
say why** — and the coverage pin was falsified in both directions before it was
kept: a fabricated atom id fails the "probes only atoms that exist" test, and
nulling `A1-V-01` drops the total to 36 and fails the pin.

