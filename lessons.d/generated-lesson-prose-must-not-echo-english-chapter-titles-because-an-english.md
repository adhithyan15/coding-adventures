---
category: Repo policy / workflow reminders
---

# Generated lesson prose must not echo English chapter titles, because an English word can be a target-language headword

The vocabulary tranche generator wrote the same line into every word lesson:
"One more word for {chapter title}". The chapter titles are English lists,
such as "Fruit, Apple, Banana, Orange, Grape" or "City, Town, Forum, Temple,
Shop". An English title word is sometimes a target-language headword too:
Italian and Portuguese **banana**, Portuguese **hotel** and **zero**, Latin
**forum**.

Every lesson in that chapter that comes before the word's own lesson then
contained the headword as a whole word. `measureContinuity` counts each of these
as a forward reference. Italian's pinned ceiling (34 in `continuity.test.ts`)
caught it at 36. Portuguese and Latin have no such pin, so theirs went
unnoticed: Latin's count had risen by 2 (the lessons before **forum**).

Fix: the line is now "One more word for this chapter." The Latin and Russian
tranches were regenerated in the same change (Latin's forward-reference count
fell from 39 to 37). The Portuguese and Italian tranches were regenerated with
the fixed template before they shipped.

Do differently: generated scaffolding text must not interpolate content that
could contain a target-language word. The per-candidate checks only vet the
headwords, never the template around them. After generating, compare the
track's forward-reference count before and after the tranche, whether or not
the track has a pin.
