## HL-C311 — Kannada script: nineteen characters left, six of them already sourced

Filed by the chapters 67–73 tranche, which taught eight of the twenty-seven
characters `KA-A1-L-09` reported and deliberately stopped there rather than let
the script work swallow the joining column it was sent to write.

### Where the count stands

Measured, not carried over from another track:

| kannada | before | after |
|---|---:|---:|
| characters a script lesson NAMES | 42 | 50 |
| distinct characters used in headwords | 69 | 69 |
| closure | 61% | 72% |
| never-taught glyphs (`script-closure.ts`) | 0 | 0 |
| load-bearing closure violations | 10 | 10 |

The eight taught are the eight most-used untaught characters, in order of the
headwords that need them: **ಮ** *ma* (36 headwords), **ಲ** *la*, **ವ** *va*,
the vowel sign **ೇ** *ē*, **ಟ** *ṭa*, **ಜ** *ja*, **ಅ** *a*, and the vowel sign
**ೂ** *ū*. The case the inventory led with is closed: a learner who finishes
the ladder can now read *māṭanāḍu*, the verb chapter 5 is built on.

### The nineteen that remain split cleanly in two

**SIX have a sourced ductus this project has already paid for and not spent.**
`code/learning/human-languages/data/scripts/kannada.json` carries a cited
Wikimedia Commons animation, with a `variation` field describing the attested
movement order, for every independent vowel: ಅ ಆ ಇ ಈ ಉ ಊ ಋ ಎ ಏ ಒ ಓ ಐ and the
visarga ಃ. Of those, **ಆ, ಎ, ಏ, ಒ, ಐ and ಋ are still untaught** — 7, 6, 4, 4, 1
and 1 headwords respectively.

Chapter 68 shows what a lesson built on one of them looks like: it teaches ಅ
with the four-movement order from Gopala Krishna A's 35-frame animation, the
pen-lift count, and the citation printed under it — the shape Hindi's
`HI-S110-letter-ra` established. Six more lessons of that shape are available
today and need no new research.

**THIRTEEN have no sourced ductus anywhere in this project**: the vowel signs
ೀ, ೈ and ೌ, and ಖ, ಘ, ಠ, ಢ, ಣ, ಧ, ಫ, ಭ, ಶ and ಷ. Those were taught by nobody
here and must not be invented. The honest options, in the order the repo has
already used them:

1. Query the Wikimedia Commons API for a Kannada equivalent of the
   `Kannada-alphabet-<name>.gif` pattern Gopala Krishna A used for the vowels,
   and record which consonants have nothing. That pattern is what supplied the
   thirteen entries already in `kannada.json`, so it is worth asking whether the
   same author published a consonant series.
2. Where no animation exists, write the recognition lesson this ladder already
   writes — the `KA-S1xx` shape, which teaches the character to the eye and says
   in as many words that this book does not yet know where the pen starts.
   Seven of the eight script lessons in chapters 67–73 use exactly that.
3. Failing both, cite the Unicode chart by codepoint, as the Marathi mātrā
   lessons do.

### Also observed, and not acted on

`data/scripts/kannada.json` has a sourced ductus for **ಇ**, **ಈ**, **ಉ**, **ಊ**,
**ಓ** and the visarga **ಃ**, all six of which the ladder already teaches — and
all six of those lessons print the "this book does not yet tell you where to
start the character" note, which for them is no longer true. Verified by reading
the six files. Correcting that prose is a small, separate chore: it is six
lessons, it changes the generated book, and it belongs with the six untaught
vowels rather than in a tranche about conjunctions.

### Also parked: the Kannada diglossia caveat

`KA-A1-REG-04` asks for two things and this tranche supplied one. Chapter 69's
`alva` lesson now names the spoken/literary divide in prose. What is still
missing is a lesson whose OWN atom is that fact, and the diglossia caveat in
`core/exam-levels.json` that `tracks.tamil` carries and `tracks.kannada` does
not. Half a pair buys nothing, so the point stays open.
