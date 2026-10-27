---
category: Testing & coverage
---

# A canonical verb tag on its own spine node moves the core-verb coverage pin, but a conceptAliases entry does not

The Chinese A1 core put six canonical verb tags on lessons (VERB-GIVE, VERB-DO-MAKE,
VERB-PUT, VERB-BRING, VERB-STAND on NAME-EVERYDAY-ACTIONS, VERB-CAN on
SAY-WHAT-I-HAVE-AND-CAN-DO) and added 36 `conceptAliases` (喝 for VERB-DRINK, 去 for
VERB-GO and the rest) to clear the spine omits. The full human-language-data vitest
then failed one pin it had not been planned for: `tests/verbs.test.ts`
`meanCoveredPercent` 40 -> 41.

`verbCoverage` reads the `concept_tag` a lesson carries, nothing else. The aliases
realize the concepts for the curriculum ledger (omits, `curriculum-omission-ledger-drift`)
but are invisible to the verb join, so only the six lesson tags moved Chinese 2 -> 8
covered, and the "SEVEN verbs no Chinese lesson teaches" assertion still held although
VERB-WRITE and VERB-READ are now aliased.

Before pushing a chapter that adds a canonical VERB- tag, measure `verbCoverage` with and
without the new lessons (filter them out of `loadLessons()`), and move the mean pin with a
comment that names the tags and both numbers. Do not expect an alias to move it, and do not
retag an aliased lesson canonically to make it move: a canonical tag must sit on its own node.
