## Ordinals: the rule leads, and "first" is a surprise about the stem only

`KA-A1-NUM-07` was one of the thirteen ordinal points HL-C354 left open, and the
one it priced cheapest. It is closed by thirteen lessons in two new chapters,
74-75, one word a lesson, and Kannada A1 coverage moves **193/258 -> 194/258**,
with the numeral column at **7 of 8**.

**THE ORDER IS NOT NUMERICAL, AND KANNADA'S REASON IS ITS OWN.** Kannada's
ordinal ending, **-ಅನೆಯ**, has no exceptions at all: drop a cardinal's final
**-ು** and add it, and every number the track teaches obeys. The one
irregularity in the whole set is *which word* "first" is built on — **ಮೊದಲು**
(*modalu*), "a beginning", and not **ಒಂದು**.

So the chapter opens on **second**, not on first:

1. **ಎರಡನೆಯ** — the ending, on the first cardinal that shows it working.
2. **ಮೂರನೆಯ** — the rule a second time, needing nothing new.
3. **ಮೊದಲನೆಯ** — the surprise, and it lands third because an exception is only
   legible once you can see what it is an exception *to*. And the surprise is
   narrower than it looks: the **stem** is irregular, the **ending** is not.
   *modalu* drops its **-ು** and takes **-ಅನೆಯ** exactly as *eraḍu* did. Kannada
   asks a learner to memorize one extra word, not one extra rule.
4-5. **ನಾಲ್ಕನೆಯ**, **ಐದನೆಯ**, with the chapter payoff stating the arithmetic:
   four of the five were *built*, one was learned.

Chapter 75 runs sixth to tenth, then **ಹನ್ನೊಂದನೆಯ** to say outright that the
ending was never a fact about one-to-ten (*ippattu* gives *ippattaneya* with no
lesson), then the everyday short form **-ಅನೇ**, then the written **೧ನೇ**.

**THE CROSS-FAMILY BOX IS THE PAYOFF THIS TRACK WAS BUILT FOR.** Kannada's
*modalu*, Tamil's *mudal* and Telugu's *modalu* are the same Dravidian word for
"a beginning". Three sisters have three unrelated words for **one** — *ondu*,
*oṉṟu*, *okaṭi* — and the *same* word for the start of a line. Counting and
ordering were never one idea in this family, and "first" is where the seam shows.

**NO ORDINAL EXISTED IN THE CORPUS IN ANY GUISE.** Portuguese had five of its ten
already in the learner's mouth as weekday names; Kannada's week is fully
Sanskritic and planet-named (*Sōmavāra*, *Maṅgaḷavāra*), so it hides nothing. A
search for *modal-*, *-neya* and *-nē* across all 303 committed lessons returned
**zero** hits before this tranche.

**SCRIPT WAS CENSUSED BEFORE ANYTHING WAS DESIGNED, AND WAS NOT THE BLOCKER.**
The 69 distinct Kannada characters the track's lessons already use — headwords
AND worked examples, not just headwords — cover every character of every word in
the tranche, including **ಹನ್ನೊಂದನೆಯ** and **ಒಂಬತ್ತನೆಯ**. Nothing had to be
written around and no letter was shipped untaught.

**THE WRITTEN FORM CASHES IN A LEDGER THAT HAD NEVER BEEN SPENT.** The ten
Kannada digits each got their own script lesson across chapters 9-18, and until
now the book gave a reader nothing to read them in. `KA-C75-on-a-sign` is that
something: **೧ನೇ** is read *modalanē* — a word with no ಒಂದು in it, off a sign
showing the digit for one. The reinforcement measure agrees, and this is the
sharpest evidence in the tranche: the only two pre-existing R4 defects the
thirteen lessons pay down are **KA-SCRIPT-RECOG-142** (the digit ೩) and
**KA-ETYMON-C20-HANNONDU-IPPATTU-01** (the teens). Both were taught and then
never needed again.

**REINFORCEMENT, DECOMPOSED, AND VERIFIED ATOM BY ATOM RATHER THAN BY TOTALS.**

    reinforcementWindowMisses        660 -> 658
    reinforcementMissesByWindow-R1    84 ->  84
    reinforcementMissesByWindow-R2    68 ->  68
    reinforcementMissesByWindow-R3   323 -> 323
    reinforcementMissesByWindow-R4   185 -> 183
    atomsTaught                      412 -> 428
    atomsNeverRevisited                5 ->   5

The thirteen lessons make **32 (lesson, window) slots newly judgeable** — debt the
added length EXPOSES rather than creates — and **none of the 32 is a miss**. The
tranche's own **16 atoms** create **zero** debt in any window. Both were checked
by walking every atom, not by watching the total: each lesson recalls the
previous lesson's ordinal (R1, distance 1), the ordinal five lessons back (R2,
distance 5), the atom exactly twenty positions back (R3) and the atom exactly
eighty positions back (R4), so every slot the new length opens is answered by the
lesson that opens it.

`atomsNeverRevisited` holds at 5 rather than rising, which took a split: the last
lesson originally introduced both the short form and the written form, and an
atom introduced by a track's final lesson can never be revisited. Splitting it in
two leaves exactly one such atom, replacing the one the old final lesson had —
and it also brought the lesson back inside `maxRuleStatementsPerLesson`.

**THE KANNADA A1 INVENTORY HAD NO ASSERTION AT ALL** — the hole HL-C354 found in
Telugu and Hindi and told the next reader to look for in the other eighteen.
`tests/corpus/kannada.test.ts` now pins the coverage total AND checks that every
probe names an atom that exists. Both halves were falsified before being kept: a
fabricated atom id fails the first, and nulling `KA-A1-NUM-07`'s probe fails the
second.

**NOT TAUGHT, and the inventory note says so:** the nominalized **-ಅನೆಯದು**
("the second *one*"), which is claimed nowhere.

