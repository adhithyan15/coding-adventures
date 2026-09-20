## Ordinals: the only track in the column whose set has no exception at all

`ML-A1-NUM-05` was one of the thirteen ordinal points HL-C354 left open. It is
closed by twelve lessons in two new chapters, 67-68, one word a lesson, and
Malayalam A1 coverage moves **162/243 -> 163/243**, with the numeral column at
**7 of 9**. The old note also said the *ordering notion* was unavailable; that is
closed in the same tranche.

**THE ORDER IS NUMERICAL HERE, AND THAT IS THE FINDING RATHER THAN A DEFAULT.**
Every one of the five tracks repaired before this one had a reason NOT to count:
Latin's *prīmus* and *secundus* carry no cardinal, Portuguese already had five of
its ten as weekday names, Italian has a seam at eleven, Telugu has one irregular
word, Kannada has an irregular stem. Malayalam has **none of that**. **ആം**
attaches to a cardinal with no exception anywhere in the set — *including at one*
— so **ഒന്ന് → ഒന്നാം** is exactly what the rule predicts, and this is the one
track in the column where the honest way to teach the set is to count. The first
lesson opens on *first* in order to say so, and the chapter payoff states the
arithmetic: **five of five were built, none was memorized.**

**THE SISTER MALAYALAM IS NOT.** Tamil *mudal*, Kannada *modalu* and Telugu
*modalu* are one Dravidian word for "a beginning", and all three build their
"first" on it rather than on their word for **one**. Malayalam has the cognate —
**മുതൽ** *mutal* — but in Malayalam it drifted to mean "**from**", the point a
thing starts at. The slot its sisters filled with a special word stayed empty,
and the ordinary ending filled it. That box is the payoff the Tamil-comparison
thread this track runs was built for, and it is placed at *second* rather than at
*first*, because it is a claim about what Malayalam did **not** do and only lands
once the rule has been seen working twice.

**THE ORDERING NOTION, CLOSED WITH THE SAME TRANCHE.** The last lesson teaches
**ആദ്യം** (*ādyaṁ*), "at first", against the **പിന്നെ** the track already had:
*ādyaṁ ūṇŭ, pinne chāya*. It is deliberately not a twelfth ordinal. ഒന്നാം picks
a thing out of a line; ആദ്യം puts an action first in time — and it is a Sanskrit
borrowing sitting next to a native machine so regular it never needed one, which
is worth carrying as a habit of reading: a borrowing is evidence about a **slot**,
not about a language.

**THE CENSUS SAID THE SCRIPT WAS CLEAR AND THE CLOSURE GATE SAID IT WAS NOT, AND
THE GATE WAS RIGHT.** A census of the 67 distinct Malayalam characters the
track's lessons already use — headwords AND worked examples — covers every
character of every word in the tranche. But **ഏ**, the independent long *ē* that
opens *ēḻāṁ*, has **no sourced stroke order** in `data/scripts/malayalam.json`:
it is a recognition-only row, and a headword containing it does not enter glyph
closure. A Donald R. Davis Jr. handwriting clip for it exists on the source the
other vowels cite, but it is a video clip and could not be observed here, so **no
stroke order was invented**. The seventh lesson is therefore headed by its
romanization — the precedent this track's own chapter-7 counting lessons already
set — and the lesson says on the page that ഏ is a letter you may read and should
not yet copy. The corpus-wide glyph-gap queue stays empty.

That is worth recording as a general lesson: **a body-text character census
answers "can the reader see this shape before?" and the closure gate answers "has
the reader been taught to write it?"** Those are different questions, and here
they gave different answers about the same letter.

**REINFORCEMENT, DECOMPOSED, AND VERIFIED ATOM BY ATOM RATHER THAN BY TOTALS.**

    reinforcementWindowMisses        770 -> 765
    reinforcementMissesByWindow-R1   117 -> 117
    reinforcementMissesByWindow-R2    59 ->  59
    reinforcementMissesByWindow-R3   319 -> 319
    reinforcementMissesByWindow-R4   275 -> 270
    atomsTaught                      358 -> 373
    atomsNeverRevisited               24 ->  24

The twelve lessons make **30 (lesson, window) slots newly judgeable** — debt the
added length EXPOSES rather than creates — and **none of the 30 is a miss**. The
tranche's own **15 atoms** create **zero** debt in any window. Each lesson recalls
the previous lesson's ordinal (R1, distance 1), the ordinal five lessons back
(R2, distance 5), the atom exactly twenty positions back (R3) and the atom
exactly eighty positions back (R4), so every slot the new length opens is
answered by the lesson that opens it.

**THE FIVE R4 DEFECTS THE TRANCHE PAYS DOWN ARE ALL THE CARDINALS THEMSELVES:**
`ML-CONCEPT-C07-NUMBERS-1-5-01`, all three of `ML-CONCEPT-C07-NUMBERS-6-10-*`,
and `ML-CONCEPT-C20-PATHINONNU-IRUPATHU-01`. The numbers had been taught and then
never needed again at distance. Teaching the ordinals is what finally gave them
something to do.

**THE MALAYALAM A1 INVENTORY HAD NO ASSERTION IN ITS OWN TEST FILE** — the hole
HL-C354 found in Telugu and Hindi. `tests/corpus/malayalam.test.ts` now pins the
coverage total AND checks that every probe names an atom that exists. Both halves
were falsified before being kept.

**NOT TAUGHT, and the inventory note says which and why:** the longer attributive
**-ആമത്തെ** (no source was found stating when it is required over bare **ആം**, so
no usage rule is claimed); and "the first street on the **left**", which is still
out of reach because no word for left or right is taught anywhere in the track —
the ordinal half of that phrase now exists and the direction half does not.

