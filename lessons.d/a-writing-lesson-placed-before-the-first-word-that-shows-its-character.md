---
category: Testing & coverage
---

# A writing lesson placed before the first word that shows its character is builds-toward and raises a ceiling that may only fall

The Chinese stage-2 plan put each new character's writing lesson right BEFORE the first
word that uses it, so the hand would write a shape before the eye met it in a word.
`letter-anchoring.ts` classifies such a letter lesson as `builds-toward` (it builds toward a
word that comes later), and Chinese's ceiling for that class is 51 and may only fall
(`tests/letter-anchoring-ceilings/chinese.json`): the first new writing lesson would have
failed CI.

The order that passes is the one chapter 30 used for 很: the anchor word comes first, with a
hanzi headword showing the character, and the writing lesson follows directly after it, which
the measure counts as `anchored`. The builder (`zh_common.Write`) now refuses a writing lesson
that does not follow a word holding its character.

Do differently: before planning an order of letter or character lessons, read the
letter-anchoring classes and the track's ceilings; a ceiling that may only fall decides the
shape of the ramp, not just its size.
