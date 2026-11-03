---
category: Testing & coverage
---

# A capped prerequisite list leaves an assessed atom unavailable, so a review that recalls many atoms must list every owner

The hand-authored Punjabi A2 core (chapters 211-224) was written by a small
builder that derived each lesson's `prerequisites` from the owners of the atoms
it assesses, then kept only the first six to make the front matter tidy. Every
generated-artifact check passed (books, narration, modality, shards, doc
shards) and so did the scratch self-check, the level gate and the selfcheck
tool. `tests/integration.test.ts` ("has a valid shared spine covering every
registered language") then reported 89 errors in eight lessons: "required atom
X is not introduced by a transitive prerequisite", "practised atom X is not yet
available", "block assesses X before it is available". The lessons that broke
were the ones that recall the most: the two final review passes (22 and 29
errors), the reading, and grammar lessons whose Guided Practice reaches back to
an older time word or name.

The validator's availability rule is per atom: every atom a lesson requires
must be introduced somewhere in its transitive prerequisite closure. Sequence
order and chapter order do not count, and an earlier lesson in the same chapter
only helps if the chain reaches the owner.

The fix was to list every owner, however long the list. Do not cap or tidy a
derived prerequisite list; if it is too long to read, assess fewer atoms in that
block instead. Run `tests/integration.test.ts` on the first generated chapter,
before the slower generated-artifact checks, because none of those read
prerequisites.
