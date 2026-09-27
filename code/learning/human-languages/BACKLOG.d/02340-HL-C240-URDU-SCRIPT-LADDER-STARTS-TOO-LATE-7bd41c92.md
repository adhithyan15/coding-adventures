## HL-C240 — Urdu's script ladder teaches the right letters in the wrong place

Chapters 17 and 18 added six Nastaliq letters — **ہ ے ں آ پ و** — interleaved
one letter per two or three glossed lessons, and they moved every number they
could reach. Urdu now teaches 15 of the 37 glyphs it shows, up from 9, so
*shown but never taught* fell 28 → 22. Its pre-A1 cumulative writing-stage
ladder is complete for the first time, and its pre-A1 headword count went
43 → 51.

**Closure violations did not move at all: 46 before, 46 after.** That is not a
defect in the new chapters. It is the shape of the track.

`measureScriptClosure` credits a glyph to a lesson only when a script lesson
taught it **earlier in reading order**. Urdu's entire script ladder lives at the
end of the book: chapter 16 opens at sequence 730, and chapters 17 and 18 run
from 820 to 1010. Every one of the 46 violating lessons sits at sequence 810 or
below. Teaching **ہ** at sequence 840 cannot retire the **ہ** that
`UR-C01-shukriya` puts in front of a reader at sequence 20.

So the remaining Urdu closure debt is not bought with more letters. It is bought
by **moving letters earlier**, which is the same thing the owner's gloss-first
rule asks for and the same thing `chapter-policy.json` already encodes as
`minLessonsBetweenScriptSegments: 2`. Chapter 16 breaks that policy outright —
seven letter lessons in seven consecutive positions — and chapters 17 and 18
were written to obey it, which is why they are ten lessons each rather than
four.

The work, in the order it pays:

1. **Redistribute chapter 16's seven letters into chapters 1–8.** Alif, lam,
   sīn and mīm are needed by *salām* in chapter 1 and are taught in chapter 16;
   moving each one to sit two or three lessons after the word that first glosses
   it would close a large share of the 46 at a stroke. This is a resequencing of
   existing lessons, not new authoring, and it will move `sequence`, the path
   segments and the book chapter map together.
2. **Then chapters 17–18's six letters into chapters 9–15**, by the same rule.
3. **Only then author new letters.** Twenty-two glyphs remain untaught, and the
   next-highest-value ones by lesson reach are **د** (28 lessons), **ج** (22),
   **خ** (22), **ف** (21), **ت** (19), **ٹ** (19), **ح** (18), **ب** (17). The
   two aspirate-bearing shapes **ھ** (30 lessons) and **ٹ** together unlock
   *ṭhīk*, which is the one line of the opening exchange chapters 17 and 18
   could not reach.

Two smaller findings from the same tranche, recorded so they are not
rediscovered:

- **A vocabulary lesson's body may only show script the reader can read.** The
  headword-plus-romanization exemption clears the headword's own glyphs from the
  whole lesson, and nothing else. Writing `**بہن** *bahan*` inside a lesson whose
  headword is **ماں** adds a violation; writing *bahan* does not. Every one of
  the twenty new lessons was checked against a running set of taught glyphs
  before it was committed, and three drafts had to be repaired.
- **`UR-SCRIPT-NUN-GHUNNA-01` was already taken** by `UR-C01-ji-han`, which
  introduces recognition of **ں** in chapter 1 without teaching the hand to form
  it. The writing atom is `UR-SCRIPT-NUN-GHUNNA-WRITE-01`. Expect the same
  recognition-versus-formation split for other letters chapter 1 names in
  passing.
