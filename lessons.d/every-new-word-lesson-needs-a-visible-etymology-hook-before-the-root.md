---
category: Testing & coverage
---

# Every new word lesson needs a visible etymology hook before the root-ledger pins can stay flat

Three new Sanskrit future-form lessons had correct atoms, retrieval, and
learner-facing explanations, and every focused Sanskrit test passed. The full
package suite still failed both etymology-gap pins because all three lessons
were `type: word` and none contained a parser-visible etymology section.

Before running focused tests, give every new word or phrase lesson a factual
`Taken Apart` or `Across the family` section with a cited source. Then run the
full package suite: track-focused corpus tests do not own the cross-track root
ledger, and raising its monotone missing-evidence pins is not an acceptable
way to add vocabulary.
