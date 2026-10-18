## The ordinals, taught from the five the week already gave you

Eleven lessons in two chapters (27-28) close `PT-A1-Q-04`, and Portuguese's exam
coverage goes 120/208 to 121/208. HL-C350 measured ordinals as the weakest single
column in the corpus -- twenty tracks enumerate an ordinal point and eighteen left
it uncovered -- and Portuguese was among the cheapest to close for a reason
peculiar to this track: **five of the ten ordinals were already in the learner's
mouth as weekdays**. Chapter 7 taught *segunda*, *terça*, *quarta*, *quinta* and
*sexta* and said in as many words that they are ordinals of numbers the learner
had. It never taught the ordinal series.

**The order is Portuguese's own.** `primeiro` opens the chapter because it is the
ordinal the week does NOT contain, and that absence is the question chapter 7 left
open: *domingo* is counted first, so Monday lands on two. `segundo` is next
because Monday is already in the learner's mouth, and because it carries no *dois*
at all -- it is built on *sequī*, "to follow". `terceiro` is THE exception and
gets a lesson of its own: Latin *tertius* left two children in Portuguese, and the
week kept the other one, so Tuesday is *terça*-feira and never *terceira*-feira.
`quarto` and `quinto` then match their weekdays exactly, which is the point at
which the series settles.

`oitavo` takes `-avo` and nothing else in the ten does -- and that exceptional
ending later became the general fraction marker (*um doze avos*), which is a
better fact than the exception itself. `nono` bends the shape the other way, by
contraction. `décimo` closes it and shows the stacking that carries the series
past ten.

**The written form is taught too.** `1.º` and `1.ª` -- a digit and the last letter
of the spoken word left standing above the line, which is why the mark itself
agrees in gender. `U+00AA` and `U+00BA` were added to `core/main-font-charset.json`
for it, following that file's own `howToAdd` procedure: both were verified present
in `lmroman10-regular.otf` twice, by a fontTools cmap query and by a XeLaTeX render
with zero missing-character warnings.

**Reinforcement, decomposed.** Measured, not asserted:

```
reinforcementWindowMisses           381 -> 328
reinforcementMissesByWindow-R1       50 ->  49
reinforcementMissesByWindow-R2      158 -> 139
reinforcementMissesByWindow-R3      134 -> 124
reinforcementMissesByWindow-R4       39 ->  16
atomsTaught                         212 -> 232
```

Adding eleven lessons makes **64** pre-existing atoms window-judged for the first
time (R1 +3, R2 +12, R3 +25, R4 +24). That is debt the added length EXPOSES. The
tranche's own twenty atoms create **zero** new debt in any window: every lesson
recalls the previous lesson's word (R1) and the word five lessons back (R2), and
nothing the tranche introduces is far enough from the end to be R3- or R4-judged.
Against the 64 exposed, the openers pay down **117**.

