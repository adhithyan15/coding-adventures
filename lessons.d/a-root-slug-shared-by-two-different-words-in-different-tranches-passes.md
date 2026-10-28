---
category: Testing & coverage
---

# A root slug shared by two different words in different tranches passes slugsplit

Chinese A1 tranche c gave 炒 *chǎo* 'to stir-fry' the bare root slug
`middle-chinese-tsrhaewx`, and tranche d gave 吵 *chǎo* 'noisy' the same slug. In the
Guangyun they are two words with two glosses ('to roast' and 'sound'), so one slug
spelled one etymon for two etyma. Every machine screen passed: `slugsplit.mjs` and
`check:root-slug-splits` refuse one etymon spelled two ways, not two etyma under one
spelling, and each tranche's own variants run saw only its own words. Tranche d's
independent reviewer caught it by comparing new bare slugs character by character
against the earlier tranche; 炒 (first) keeps the bare slug and 吵 is now
`middle-chinese-tsrhaewx-noisy`.

Do differently: when a later tranche reuses a bare slug, check which character and
gloss the earlier tranche put under it; a shared Baxter spelling is not a shared
word. A reviewer brief for any CJK tranche should ask for that comparison
explicitly.
