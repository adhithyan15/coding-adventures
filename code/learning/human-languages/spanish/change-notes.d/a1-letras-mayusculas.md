## Unreleased — chapters 425 and 426: the letters, the capitals, and the short forms

Spanish A1 exam coverage **245/273 (90%) → 255/273 (93%)**. Ten points close,
and with them **the entire orthography dimension**:

| category | before | after |
|---|---|---|
| Ortografía de letras y palabras | 0/7 | **7/7** |
| Abreviaturas y siglas | 0/3 | **3/3** |
| Puntuación (closed one chapter earlier) | 1/9 | 9/9 |

Every `O-*` point in the inventory is now covered. **The eighteen points that
remain are all notions and functions** — no grammar, no orthography — which is a
change in the shape of the remaining work and not only its size.

### Like the punctuation chapter, none of this is vocabulary

The twenty-seven letters have been on every page since chapter one. What the
corpus had never done is **name** them, and the names are the load-bearing part:
a listening paper dictates a surname letter by letter, and *hache*, *jota* and
*equis* are not guessable from English.

The letter names are taught as three patterns rather than a list of
twenty-seven, because that is what they are — a vowel put beside the consonant's
sound. Five vowels named by their own sound, most consonants taking **-e** after,
five taking **e-** before, and nine genuine irregulars.

### b and v are homophones, which is why Spanish grew extra names

There is no *v* sound in Spanish. That is unremarkable while speaking and a
problem while spelling, so the language grew **be larga** and **ve corta** —
names whose entire job is to tell two identical-sounding letters apart. An
English speaker never needs this and a Spanish speaker uses it constantly:

> — Vázquez. Uve, a con tilde, zeta, cu, u, e, zeta.
> — ¿Uve o be?
> — Uve. Ve corta.

### The capitalisation lesson is the one a marker sees

Most orthography is a fact you can look up. `A1-O1-04` is a habit visible on
every line of a written paper:

| | English | Spanish |
|---|---|---|
| days | Monday | **lunes** |
| months | January | **enero** |
| languages | Spanish | **español** |
| nationalities | I am Spanish | soy **español** |
| book titles | *One Hundred Years of Solitude* | *Cien años de soledad* |

One idea covers all five: **Spanish capitalises a name, not a category.**
*España* is a name; *español* describes a kind of person. The same idea then
explains the surname rule — *de* meant *from*, so *Pedro de la Cruz* is a
description and takes no capital, while *De la Cruz* standing alone has become
the name and does.

### Three kinds of short form, and the point tells them apart

| kind | example | point? |
|---|---|---|
| abreviatura | Sr., Dña., n.º | **yes** |
| sigla | DNI, ONU, UE | no |
| símbolo | €, km, N | no |

An **abreviatura** is a Spanish word cut short and the point marks the cut. A
**sigla** is a set of initials, pluralised by doubling its letters (*EE. UU.*)
rather than by adding an **-s**. A **símbolo** is not Spanish at all — it is an
international sign, so it takes neither point nor plural. Only something that
was cut needs a mark showing where.

### Three characters the corpus had never typeset

**º, ª and €** appeared in zero lessons before this chapter. As in chapter 424
the font was checked **before** the lessons were written: all three are in
`core/main-font-charset.json`. `check-book-compile.sh spanish` then confirmed it
under real XeLaTeX, with 26 raised **º**, 11 raised **ª** and 5 euro signs in the
generated chapters.

`$`, `&` and `@` were **avoided** rather than tested: `&` and `%` already occur
in Spanish lessons and pass, but `$` and `@` occur nowhere in the corpus, so the
symbols lesson uses **€** — which is the right currency for the source anyway.

### A number jumped, and it was a real defect rather than noise

`forward-language` went **376 → 519**. One hundred and forty-three new entries,
all attributed to one new lesson.

The lesson about surname capitalisation had been given the headword *de, del y
la en los apellidos*. `continuity.ts` treats a `headword:` as the evidence that a
lesson **introduces** that word — which is what makes the check provable rather
than a guess — so the lesson was claiming to introduce **de**, taught back in
chapter 9 as *soy de*, and every earlier lesson using it became a forward
reference.

This is **not** the HL-C378 or HL-C379 class. Those record genuine homograph and
homonym noise, and after writing two such entries the temptation is to read the
next rise the same way. A jump of 143 is not that shape: noise arrives in ones
and fives.

The headword became **la minúscula inicial** — the inventory point's own label —
and the count went straight back to **376, zero new forward references**, with
nothing about the teaching changed. Recorded in `lessons.d`.

### Pins

| pin | before → after |
|---|---|
| `coverage.covered` | 245 → 255 |
| `coverage.percent` | 90 → 93 |
| `coverage.unmapped` | 28 → 18 |
| `byCategory["Ortografia de letras y palabras"]` | 0/7 → **7/7** |
| mock-audit `lessonCount` | 979 → 994 |
| mock-audit `taughtForms` | 1630 → 1668 |

### Terminal-lesson debt, and the one count that stays up

Each chapter carries a *repaso* and a *síntesis*. Measured against a clean-main
equivalent: `attained` held at **A1**, A2 reinforcement shortfall **flat at 33**.

`measurement-blind` moved **11 → 13** — the two new *síntesis* lessons, HL-C377
for the sixth and seventh time. Not relabelled.

### Verification

`human-language-data` 145 files / 2083 tests; `language-ladder` `bash BUILD`
39 files / 442 tests; all twelve `check:*` gates; `check-book-compile.sh
spanish` under XeLaTeX. The table-width, banned-word, chapter-reference and
info-dump checks ran before generation; the banned-word check caught two uses of
*just* in draft warm-ups, which were reworded.

### What this does not claim

Coverage means the teaching exists, not that a reader scores. No track has had a
single exam item graded against it.
