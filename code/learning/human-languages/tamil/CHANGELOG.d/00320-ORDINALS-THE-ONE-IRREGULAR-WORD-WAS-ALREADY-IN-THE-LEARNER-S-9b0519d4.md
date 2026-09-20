## Ordinals: the one irregular word was already in the learner's mouth

`TA-A1-NUM-04` was one of the thirteen ordinal points HL-C354 left open, and its
own note is what the tranche was built on. It said: *"mutalil ('first of all') is
taught in chapter 62 as a discourse word, not as an ordinal, and no other ordinal
exists."* That note was written as a description of a gap. Read again, it is a
description of a **head start**.

**முதலில்** is **முதல்** + the locative **-இல்**. So Tamil's word for *first* had
been in the learner's mouth for twenty chapters, carrying the meaning, without
ever being named. The opening lesson of this tranche does not teach a word — it
takes **முதலில்** apart and points at what was inside it.

Closed by fourteen lessons in two new chapters, 82-83. A1 coverage moves
**174/262 -> 175/262**, numeral column **5/8 -> 6/8**.

**THE ORDER FOLLOWS FROM THAT, AND IT IS THE OPPOSITE OF KANNADA'S.** Kannada's
ordinal rule is exceptionless and its "first" is an unknown irregular stem, so
that chapter had to teach the rule before the exception could be legible. Tamil's
exception is the one item the reader **already owns**, so Tamil opens on it:

1. **முதல்** — revealed inside a word already learned, with the four-way family
   box (below) as its payoff.
2. **இரண்டாவது** — the ending **-ஆவது**, and the statement that exactly one
   number refuses it.
3. **முதலாவது** — and the refusal turns out to be narrower than it looked:
   **முதல்** takes the same ending, just on a different stem. The stem is the
   exception; the ending is not.
4-6. **மூன்றாவது, நான்காவது, ஐந்தாவது**, and a payoff that counts the work:
   four built, one already owned, none learned.

Chapter 83 runs sixth to tenth, then **பதினொன்றாவது** to say the ending was never
a fact about one-to-ten, then a second ending.

**THE FOUR-WAY FAMILY BOX IS THE ONE THIS CORPUS HAS BEEN BUILDING TOWARD.** With
the Telugu, Kannada and Malayalam ordinal tranches landed, Tamil can print all
four at once, and the four did four different things with one word:

| language | what it did with *mutal* | its word for "first" |
|---|---|---|
| Tamil | kept the meaning outright | **முதல்** |
| Kannada | built an ordinal on it | **ಮೊದಲನೆಯ** |
| Telugu | built an ordinal on it | **మొదటి** |
| Malayalam | kept the word, changed the sense to "from" | uses **ഒന്നാം** instead |

**None of the four builds "first" on its own word for one.** Three build it on
this word, and Tamil is where it still simply *means* first.

**TAMIL HAS A SECOND ORDINAL ENDING, AND ONLY WHAT THE SOURCE STATES IS TAUGHT.**
**-ஆம்** is the ending a **date** takes: **சித்திரை இரண்டாம் தேதி**. The last
lesson teaches it with **தேதி**, the day-number, against the Tamil month names
the track already had — so a date is now sayable. What is deliberately **not**
claimed is that **-ஆவது** and **-ஆம்** are freely interchangeable: the source
states **-ஆம்** for dates, and that is the whole of what is taught about it. This
is the same discipline the Malayalam tranche used when it could not source
**-ആമത്തെ** and therefore did not teach it.

**SCRIPT WAS CENSUSED AND WAS NOT THE BLOCKER.** The 52 distinct Tamil characters
the track's lessons already use — headwords **and** worked examples — cover every
character of every word in the tranche, **பதினொன்றாவது** included.

**REINFORCEMENT, DECOMPOSED, AND VERIFIED ATOM BY ATOM RATHER THAN BY TOTALS.**

    reinforcementWindowMisses       1037 -> 1037
    reinforcementMissesByWindow-R1   151 ->  151
    reinforcementMissesByWindow-R2   173 ->  173
    reinforcementMissesByWindow-R3   386 ->  386
    reinforcementMissesByWindow-R4   327 ->  327
    atomsTaught                      477 ->  494
    atomsNeverRevisited               50 ->   49

Every window holds **exactly** where it was. The fourteen lessons make **34
(lesson, window) slots newly judgeable** — debt the added length EXPOSES rather
than creates — and **none of the 34 is a miss**; the tranche's own **17 atoms**
create **zero** debt in any window. Each lesson retrieves the previous lesson's
ordinal (R1, distance 1), the ordinal five lessons back (R2), the atom exactly
twenty positions back (R3) and the atom exactly eighty positions back (R4).

`atomsNeverRevisited` **falls by one**, and that is a consequence of the shape
rather than an accident: the tranche ends on a retrieval lesson that introduces
nothing, so the track's previous final atom is revisited for the first time and
no new tail atom replaces it.

**THE TAMIL A1 INVENTORY HAD NO ASSERTION IN ITS OWN TEST FILE** — the hole
HL-C354 found in Telugu and Hindi. `tests/corpus/tamil.test.ts` now pins the
coverage total AND checks that every probe names an atom that exists; both halves
were falsified before being kept.

**TWO GATES CAUGHT PROSE, AND BOTH WERE ANSWERED BY REWRITING RATHER THAN BY
RAISING A CEILING.** The cross-chapter reference count rose by five, because
naming *chapter 62* is exactly the kind of pointer that rots — the lessons now
name the **word** (*mutalil*) instead of its position. And `info-dump` registered
one rule statement in `TA-C82-first-full`, which was a restatement of the
previous lesson's rule rather than a new one; it is now written as an
observation, and the corpus ceiling is untouched at 32.

