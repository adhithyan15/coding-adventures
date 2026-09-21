## The ordinals: one irregular word, one ending, and the number line

Eleven lessons in two chapters (81-82) close `TE-A1-NUM-04`, the **last open
point in Telugu's numeral column** -- against the highest cardinal ceiling any
track in this corpus reaches. Coverage goes 213/326 to 214/326. HL-C350 measured
ordinals as the weakest single column in the whole corpus: twenty tracks
enumerate an ordinal point and eighteen left it uncovered.

**Telugu's ordinals are the cheapest in the corpus to teach, and the tranche is
shaped to say why.** Exactly one word is irregular -- `మొదటి` (*modaṭi*), built
on `మొదలు` "a beginning" and not on `ఒకటి` at all -- and every other ordinal is
the cardinal with its final vowel dropped and `-వ` added. So the second lesson is
the rule, the remaining nine are the rule working, and the last one takes it past
ten (`పదకొండు` gives `పదకొండవ`) to show it was never a fact about one-to-ten but
about numbers.

That gives the chapter a fact worth stating: **Telugu asks you to memorize one
word where Latin asks for ten and English still does.** `మొదటి` also lands on
`ఒకటి`, which the first numbers lesson already called Telugu's maverick -- the
sisters do not share its cardinal, and Telugu does not build its ordinal on it
either, so ONE is doubly on its own.

**Script was checked before design, not after.** Every candidate word was tested
against the union of Telugu characters the track has already shown -- headwords
AND worked examples, `రోజు`, `వారం`, `పుస్తకం` and `సంవత్సరం` included. One
character failed: the independent long **ō** of the spoken `-ō` variant
(`రెండో` uses the dependent sign and is fine; the suffix written on its own is
not). It is written around in romanization rather than shipped untaught.

**Reinforcement, decomposed.** Measured, not asserted:

```
reinforcementWindowMisses           875 -> 841
reinforcementMissesByWindow-R2      162 -> 157
reinforcementMissesByWindow-R3      353 -> 347
reinforcementMissesByWindow-R4      281 -> 258
atomsTaught                         449 -> 461
```

Adding eleven lessons makes **34** pre-existing atoms window-judged for the first
time (R1 +1, R2 +6, R3 +16, R4 +11). The tranche's own twelve atoms create
**zero** new debt in any window. Against the 34 exposed, the openers pay down
**68**.

**The Telugu inventory had no test at all**, which is the exact failure mode this
work was told to avoid: land the atoms, wire the probes, and let a number nothing
reads stay whatever it was. `tests/corpus/telugu.test.ts` now pins the coverage
total and checks that every probe names an atom that exists. Both were falsified
before being kept.

