## Unreleased — the eight marks the book has been printing and never naming

Spanish A1 exam coverage **237/273 (87%) → 245/273 (90%)**. Chapter 424 closes
the **entire `Puntuacion` category, 1/9 → 9/9** — eight points in one chapter.

### This is a different kind of gap from every point closed so far

Every previous tranche in this campaign closed a **vocabulary** gap: a word the
corpus did not have, authored so it did. This one closes nothing of the sort.
All eight marks are already on the page, in their thousands. The corpus simply
never said what any of them was.

The clearest case is **la raya**. Every printed exchange this book has ever set
opens its turns with it, including the three chapters merged immediately before
this one:

> — ¿Vamos al cine?
> — Sí. ¿Qué película es?

An English-trained reader looks for quotation marks, finds none, and concludes
there is no dialogue. There is. It is marked with a dash, nobody is named, and
until now the book had let the reader work that out alone.

### Two of the eight have a consequence a marker can see

Most orthography is a fact you can look up. Two of these are habits that show
in a written paper:

| | Spanish | English |
|---|---|---|
| before the final *y* of a list | *rojo, azul y verde* — no comma | *red, blue, and green* — allowed |
| after the greeting of a letter | *Querida Ana:* | *Dear Ana,* |

An English-trained hand puts a comma in front of the *y* and after the name.
Both are visible, and both are cheap to fix once named.

The comma lesson also picks up something the corpus had already made the reader
do without explanation: *simpática **e** inteligente* in the character chapter
changed *y* to *e* to keep two *i* sounds apart. That alternation, and *o* to
*u* before *o-*, now have their names.

### The eight, sorted by what they do

| mark | name | job |
|---|---|---|
| . | el punto | divides — a sentence, a paragraph, a text |
| , | la coma | divides — inside a sentence |
| : | los dos puntos | divides — and announces |
| — | la raya | sets apart — a turn of speech |
| « » | las comillas | sets apart — a quoted or foreign word |
| ( ) | los paréntesis | sets apart — an aside |
| - | el guion | **joins** — two words into one name |
| / | la barra | divides — alternatives, or *per* |

**El guion** is the only one of the eight that joins, which is the cleanest way
to stop it being confused with **la raya**, the mark it most resembles.

Three of the names are plural in Spanish and singular in English — *los dos
puntos*, *las comillas*, *los paréntesis* — so each takes a plural article and
a plural verb.

Two small facts that an A1 reading paper actually prints: **c/** on an address
is *calle*, and **km/h** is read aloud as *kilómetros **por** hora*, with the
slash spoken rather than skipped.

### Four characters the corpus had never typeset

**« », º and ª had appeared in zero Spanish lessons.** That is the exact shape of
the two failures `glyph-coverage.ts` was written after — a character absent from
the book's font, caught only by XeLaTeX, eleven minutes into CI.

So the font was checked **before** the lessons were written rather than after:
all four are present in `core/main-font-charset.json`. `check-book-compile.sh
spanish` then confirmed it under real XeLaTeX, with seven angular quotes in the
generated chapter.

### Terminal-lesson debt, budgeted

A *repaso* and a *síntesis* follow the eight word lessons. Measured against a
clean-main equivalent: `attained` held at **A1** and the A2 reinforcement
shortfall stayed **flat at 33**.

### Pins

| pin | before → after |
|---|---|
| `coverage.covered` | 237 → 245 |
| `coverage.percent` | 87 → 90 |
| `coverage.unmapped` | 36 → 28 |
| `byCategory["Puntuacion"]` | 1/9 → **9/9** |
| mock-audit `lessonCount` | 969 → 979 |
| mock-audit `taughtForms` | 1610 → 1630 |

### Two counts went up, and both stay up

`measurement-blind` **10 → 11** — the one new *síntesis* lesson, HL-C377 for the
fourth time. Not relabelled.

`forward-language` **371 → 376**, and all five are false positives of a
**second, distinct** mechanism, recorded as **HL-C379**. Spanish has two words
spelled *coma*: the comma, and a form of *comer*. Teaching the noun retroactively
flagged five long-merged lessons that teach the verb. Unlike HL-C378, where the
flagged token was English, both uses here are correct Spanish — so neither can
be reworded, which is the clearest demonstration available that the measurement
is wrong rather than the corpus.

### One track was picked up and put down again

Marwadi is the weakest Indian track at 40%, and five of its gaps share one
insight, so it was the obvious next item. It was **not** written, and the reason
is recorded as **HL-C380**: 212 of its 346 lessons carry a citable `Source:`
line, and every domain they cite is refused by this environment's network egress
proxy. Authoring Marwari morphology from recall, in the one track that cites a
source per lesson, is the failure this backlog exists to prevent.

### Verification

`human-language-data` 145 files / 2083 tests; `language-ladder` `bash BUILD`
39 files / 442 tests; all twelve `check:*` gates; `check-book-compile.sh
spanish` under XeLaTeX. Table widths, banned words and chapter references were
all checked **before** generation this time, and none of the three fired.

### What this does not claim

Coverage means the teaching exists, not that a reader scores. No track has had a
single exam item graded against it.
