## Unreleased — the joining column, the past tense, and the letter ऋ: 126 -> 141 of 164 A1 points

**Chapters 52-61 close every coordination and subordination point in the file,
give the track a past tense, and A1 exam coverage moves 126/164 (77%) ->
141/164 (86%).** Measured with `measureExamCoverage` against
`core/exam-inventory-sanskrit-a1.json`, before and after, on the tree — never
composed from deltas.

RANKED BEFORE ANYTHING WAS DESIGNED

The inventory's 38 uncovered points were sorted by **points closed per item
taught**, and the tranche is the top of that list rather than a topic somebody
liked:

    yad … tad          2 points / 1 item     the correlative IS the relative clause
    ca, va, kintu, na  4 points / 4 items    one enclitic each, the cheapest in the file
    iti                1 point  / 1 item     and it unblocks every reporting verb at once
    the danda          1 point  / 1 item     one short lesson, as the note predicted
    saknoti            1 point  / 1 item
    rocate + mahyam    2 points / 2 items
    desa/Bharata/-iya  2 points / 3 items
    -tva and -tum      1 point  / 2 items    but the socket saknoti and icchati need
    the past           1 point  / 3 items    lowest ratio in the tranche, taken anyway

The past tense scores worst on that ranking and is in the tranche because the
ratio is not the only argument: it was the largest structural absence in the
file, and `hyaḥ` had been taught since the time words with no verb form that
could stand in it. A track can teach a word and leave it unusable.

WHAT CLOSED

    Samuccaya (coordination)          1/5  -> 5/5
    Subordination                     1/3  -> 3/3
    Dhatu and kriyapada (the verb)   10/12 -> 12/12
    Visesana (the adjective)          5/6  -> 6/6
    Opinions, attitudes, knowledge    4/5  -> 5/5
    Sarvanama (the pronoun)           4/8  -> 6/8
    Likes, wishes and feelings        0/2  -> 1/2
    Spatial notions                   2/4  -> 3/4
    Lipi (script and orthography)     6/10 -> 7/10

Four spine omissions close with them — `VERB-INFINITIVE`, `VERB-PAST`,
`VERB-CAN` and `VERB-WANT` — and `SPINE-TALK-ABOUT-PAST` and
`SPINE-SAY-WHAT-I-WANT` now omit nothing at all.

TAUGHT AGAINST THE SYSTEM THE TRACK ALREADY HAS

The relative pronoun is not introduced as new machinery. The deixis lesson
taught **अ- · त- · क-** and called it a system; it was three legs of four, and
**य-** is the fourth. `यत्र` is `तत्र` with the opening swapped, which makes the
correlative a **slot** rather than a pair of words to memorise.

The past is two changes to a verb the reader already says — the augment on the
front, the ending cut short — and Greek's **e-** is shown doing the identical
job, inherited rather than borrowed. Latin's supine **-tum** and Sanskrit
**-तुम्** are one suffix, and `रुच्` "to shine" is Latin *lūx*, so *the fruit
shines for me* is what "I like the fruit" literally says.

ऋ IS NOW TAUGHT, BY THE ROUTE THE INVENTORY PRESCRIBED

`SA-C61-rtu` schedules **ऋतुः** so the character has a headword to be recognised
inside, and `SA-S225-letter-vocalic-r` then draws it from the four-stroke ductus
cited in `data/scripts/devanagari.json` to Saurmandal's four-panel Commons
diagram. Vocabulary first, then the shape, which is what HL-C217 requires.
Never-taught characters **5 -> 4**.

**`ङ` and `ँ` remain refused and that refusal is load-bearing.** Neither is in
`devanagari.json` at all, so neither has a sourced pen path, and a stroke order
must be sourced and cited rather than invented. `ङ` sits in a headword already
and that does not help it.

RE-MEASURED, THE FIVE SPLIT THREE AND TWO RATHER THAN TWO AND TWO. `ई` and `घ`
are in exactly the position `ऋ` was in — each has a cited ductus in
`devanagari.json` and neither appears in any Sanskrit headword, so both are
blocked on **vocabulary** and can be unblocked the same way. Only `ङ` and `ँ`
are blocked on **sourcing**. The inventory note and the roadmap both described
this as two refusals on opposite halves; the file says otherwise and both are
now corrected.

`SA-A1-RG-02` IS DELIBERATELY STILL UNCOVERED

The traditional ladder asks for śabda-rūpa and dhātu-rūpa tables, sandhi rules
by name, samāsa analysis and a set text. This tranche teaches none of those and
does not pretend to. Covering that point would be exactly the error it exists to
prevent.

REINFORCEMENT WENT DOWN WHILE 31 LESSONS WENT IN

Whole-track atoms revisited fewer than twice: **89 -> 80**. Nine pre-existing
atoms rescued, and **none of the tranche's own 25 atoms is thin**. Three landed
thin on the first measurement — two of ours and `mātulaḥ`, which 31 new lessons
made judgeable — and all three were fixed with the cross-boundary retrieval the
tranche already uses at every other chapter seam, not by reseating a pin.
`atomsNeverRevisited` 49 -> 46, forward references unchanged at 3.

`reinforcementWindowMisses` rises 1008 -> 1070. The +62 decomposes, by running
`measureContinuity` with and without these 31 lessons, as **25 the tranche's own
atoms and 37 pre-existing atoms newly judgeable** — a longer track makes windows
fit that the end of the book had cut off. **R1 does not move at all (81 -> 81)**,
which is HL-C313 holding in a second track: a chapter that retrieves at distances
1-4 never touches R2, and all 25 are R2, R3 and R4.

SCRIPT CLOSURE IS UNCHANGED AT 21 VIOLATIONS

Every headword and every worked example is spelled from the glyphs the track
already teaches. The first draft of `SA-C59-bharatiyah` cited the gentilic suffix
as **-ईय** with the independent `ई`, which is one of the never-taught four; it
was rewritten to point at the **ी** and the **य** inside the word instead, which
is both closed and the better lesson.

TWENTY-FOUR CHAPTER-NUMBER POINTERS WERE WRITTEN AND THEN REMOVED

The first draft said "in chapter 14", "since chapter 17" and so on twenty-four
times. `chapter-references.test.ts` caught it: prose that names a chapter number
rots the moment a chapter splits. All twenty-four now name the thing — "the
deixis lesson", "the time words", "your first farewell" — and Sanskrit's baseline
of 20 is unchanged.

PINS MOVED

    tests/corpus/sanskrit.test.ts   lessons 304 -> 335

Book compiles clean with XeLaTeX at 502 pages, with errors, overfull, underfull
and missing characters all zero, and the changed pages were read as rendered PDF.

