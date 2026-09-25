## HL-C440-0303c912 — Tamil is the largest reinforcement debt and the last of them

**Status: CLOSED (2026-09-24) — implemented.** The sixth and largest of the
reinforcement tranches, and the last one this template answers.

```
tamil: reinforcement 73 -> 0   (nine lessons, NO new chapter)
```

**Twenty-two of twenty-three tracks** now carry no pre-A1 reinforcement debt.
Only arabic remains, and it is deliberately not a tranche of this shape.

### BIGGEST SLOT COUNT, CHEAPEST SHAPE

```
73 thin atoms, 15 with ZERO revisits  ->  (15 x 2) + 58 = 88 slots
42 of the 73 on TWO segments (TA-PATH-100, TA-PATH-003), both SPINE-MEET-GREET
```

88 slots is the largest of the programme — larger than punjabi's 40-atom spread
that needed a whole new chapter in three strands. It was still cheap, because
**slot count and cost are different questions**. Punjabi was expensive at 40
atoms because seventeen segments meant no chapter could host anything. Tamil is
cheap at 73 because the debt concentrates, and because chapters 84-86 sat at the
far end of a track whose debt lives in chapters 1-39 — maximum retrieval
distance, on segments that already existed.

Nine lessons rather than eight: one planned 16-atom page was split. **Slots are
a floor, not a cap**, and the binding constraint on a retrieval page is the
300-second duration ceiling, not the atom count.

### THE GATE THAT NEARLY BIT: A CEILING WITH ZERO HEADROOM

```
tamil forwardReferences   8  against a ceiling of 8
      lessonsEarly > 1    4  against a ceiling of 4
```

`keeps-tamil-s-opening-free-of-future-farewells-and-pronouns` reads, from its
title, as if it governs chapters 1-2. **Two of its three assertions are
whole-track ceilings, and both were exactly met before this tranche.** One
retrieval page printing a word the course teaches later would have failed the
build.

The mitigation was structural: hosts in chapters 84-86 of an 86-chapter track
leave only eight words that count as "later", so the blocklist was small and
checkable. The last page names **நாம்** and **நாங்கள்**, which ARE on that list —
taught at chapter 85, used at chapter 86, therefore backward and free. That was
confirmed by running the count, not by reasoning about it. It held at 8 and 4.

**Read the assertions, not the test name.** A title that sounds scoped to the
opening can carry a corpus-wide ceiling.

### AN OFFLINE CHECKER PAID FOR ITSELF

HL-C439 spent six trimming passes per lesson discovering the duration formula by
collision. This tranche mirrored the pipeline's gates in a standalone script
against markdown files **not yet in the repo**, verified it reproduces
`estimateLessonDuration` exactly on all five committed hindi lessons, and then
wrote to the budget:

```
nine drafts, nine first-pass results, zero trimming rounds
201s 219s 258s 203s 258s 222s 237s 219s 227s   (ceiling 300)
```

It also caught two banned words (`simply`) before they reached CI, and the
9-file coverage check proved 88 atom-passes against 88 slots required with zero
under-covered atoms before a single file entered the repo. **When a gate is
cheap to model and expensive to trip, model it.**

### WHAT THE ARITHMETIC COULD NOT HAVE TOLD US

The slot count sizes a tranche. It says nothing about what belongs on the page.
Reading the 73 atoms together produced the teaching:

- **Three n-letters, two l-letters, two r-letters.** ந ண ன arrived one per
  chapter, pages apart, never side by side. Sorting by what a reader confuses
  rather than by arrival date is the lesson.
- **The u-family is a 2x2 grid** — standing உ/ஊ against hanging ு/ூ — whose
  corners arrived in chapters 17, 36, 37 and 38. No chapter could draw the
  square because no chapter held all four. **உணவு** holds two corners in one
  word.
- **Tamil has no word for goodbye.** போய் வருகிறேன் is *I will go and come
  back*; மீண்டும் சந்திப்போம் is *we will meet once more*; நாளை பார்க்கலாம் is
  *tomorrow, let us see*. Not one of the three names the parting.
- **The copula is absent twice over** — என் பெயர் … and இவர் என் நண்பர் are both
  verbless. Taught in two places twenty chapters apart, never linked.
- **நாம் vs நாங்கள் is a social act.** One *we* takes the listener in, the other
  shuts them out, audibly, in front of them.

### Remaining

**Arabic only**, and the framing this entry's predecessors used for it was
WRONG, corrected here from measurement:

```
78 thin atoms, 68 with ZERO revisits  ->  146 slots
129 lessons across 45 chapters        (not 15 -- that figure was wrong)
52 of the 78 atoms on THREE segments  (AR-PATH-008 24, -007 15, -009 13)
```

HL-C438, HL-C439 and this entry's first draft all said arabic "is not a tranche
of this template" because answering it would need more retrieval lessons than
the track has content chapters. **It would not.** 146 slots is about thirteen
lessons against forty-five chapters — larger than tamil's nine, and ordinary.
The claim came from a chapter count of 15 that was never checked; arabic has 45.

The debt is also CONCENTRATED, which is the cheap shape: three segments carry
two thirds of it, exactly the pattern that made kannada, malayalam and tamil
cheap rather than the seventeen-segment spread that made punjabi expensive.

What IS genuinely distinctive is the **68 of 78 with no revisit at all** — 87%,
against tamil's 21% and malayalam's 2%. That is not a sizing problem, it is a
content-design gap: arabic's pre-A1 has almost no review layer, so nearly every
atom costs the full two lessons instead of one. It doubles the slot count but
changes nothing about the method.

**Arabic is the next tranche, not a special case.** Roughly thirteen lessons,
hosted in the late chapters, retrieving chapters 1-41.
