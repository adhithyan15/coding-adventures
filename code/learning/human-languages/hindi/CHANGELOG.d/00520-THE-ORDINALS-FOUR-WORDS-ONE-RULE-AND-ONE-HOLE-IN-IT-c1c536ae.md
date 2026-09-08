## The ordinals: four words, one rule, and one hole in it

Twelve lessons in three chapters (82-84) close `HI-A1-NUM-04`, and Hindi's exam
coverage goes 192/282 to 193/282. HL-C350 measured ordinals as the weakest single
column in the corpus -- twenty tracks enumerate an ordinal point and eighteen left
it uncovered. The old note on this point read: "Not one ordinal is introduced, so
nothing can be put in sequence -- no first floor, no second platform, no 'one ...
the other'."

**`HI-A1-NUM-04` is a compound point and both halves are closed.** The ordinals to
tenth, and the distributive **एक ... दूसरा** -- and the second needed **no new
word**, because **दूसरा** is taught with both its senses, *second* and *other*, in
its own lesson.

**The order is Hindi's own.** Four ordinals go their own way and have to be
learned as words: **पहला** (from Sanskrit *prathama*, with no **एक** in it, and
the same word as the adverb **पहले**, "before"), **दूसरा** and **तीसरा** sharing a
**-सरा** ending they share with nothing else, and **चौथा** with a third ending.
The **-वाँ** rule then arrives at **five**, and it has exactly **one hole**:
**छठा**, inherited whole from Sanskrit *shashtha* before the rule existed, which
gets a lesson of its own. Five words and one ending is the whole system, and the
last lesson carries the ending past ten so there is no second list.

**Script and vocabulary were both checked before design.** Every candidate word
was tested against the union of Devanagari the track has already shown -- nothing
failed -- and the worked examples were held to the same rule: **किताब**, **दिन**,
**आदमी**, **नया** and **पुराना** are all taught headwords, and an earlier draft
using **उसकी**, **अंदर** and **बाहर** was rewritten because none of the three is
taught anywhere in the track.

**Reinforcement, decomposed.** Measured, not asserted:

```
reinforcementWindowMisses           980 -> 934
reinforcementMissesByWindow-R3      364 -> 349
reinforcementMissesByWindow-R4      291 -> 260
atomsTaught                         409 -> 423
```

Adding twelve lessons makes 30 pre-existing atoms window-judged for the first time
(R1 +1, R2 +5, R3 +12, R4 +12). The tranche's own fourteen atoms create ZERO new
debt in any window. Against the 30 exposed, the openers pay down 76.

**The Hindi inventory had no coverage assertion**, which is the failure mode this
work was told to avoid. `tests/corpus/hindi.test.ts` now pins the coverage total
and checks that every probe names an atom that exists; both were falsified before
being kept.
