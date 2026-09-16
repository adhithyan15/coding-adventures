## HL-C379 — the forward-reference detector has no word sense, so a homonym inside the target language reads as an early use

Chapter 424 authored **la coma**, the comma. Spanish `forward-language` went
**371 → 376**, and all five new entries point at lessons merged long ago:

| lesson | what `coma` is doing there |
|---|---|
| `ES-C18-vivir-subjuntivo` | the present subjunctive of *comer* |
| `ES-C18-practice` | the same |
| `ES-C50-hable-usted` | the *usted* imperative — *coma usted* |
| `ES-C50-repaso-mandatos` | the same |
| `ES-C50-sintesis-pedir-bien` | the same |

None of them is an early use of anything. **Spanish has two words spelled
`coma`**: the noun *la coma*, "comma," from Greek *kómma*; and a form of the
verb *comer*, "to eat." The detector matches bare tokens, so teaching either one
retroactively flags every lesson that taught the other.

This is **not** the HL-C378 case and should not be folded into it. There the
flagged token was English prose that happened to look Spanish, and the missing
signal was *which language this word is*. Here both uses are unambiguously
Spanish and correct, and the missing signal is *which word this is*. A fix that
only marks target-language spans — the remedy HL-C378 proposes — closes that one
and leaves this one firing.

**Nothing was reworded.** *Coma usted* is the correct imperative and central to
what those five lessons teach; *la coma* is the name of the mark. Neither is
available to be changed, which is the clearest possible demonstration that the
measurement is wrong rather than the corpus.

**The fix worth making, when someone touches `continuity.ts`:** a use is a
forward reference only when the later lesson's atom is the one the earlier
lesson's token realizes. The corpus already records enough to check this — each
lesson declares a `concept_tag` and an `introduces.knowledge` list — so the
comparison can be made on atoms rather than on spelling. Until then, expect this
class to fire on every Spanish homonym pair the campaign separates, and confirm
by reading the cited lessons before believing a `forward-language` rise.
