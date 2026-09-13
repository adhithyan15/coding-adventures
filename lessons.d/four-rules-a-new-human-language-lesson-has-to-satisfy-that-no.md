# Four rules a new human-language lesson has to satisfy that no single gate names

Authoring seven review lessons hit all four in one pass, each from a different
test:

1. **Every activity id must begin with its lesson id plus a hyphen.**
   `integration.test.ts`, not the activity compiler.
2. **A block's `hl-knowledge: assesses=[...]` must list every atom its
   activities assess.** Declaring the atom on the activity alone fails with
   "assesses X outside block Y".
3. **A lesson beyond the one realizing its path segment's spine node needs an
   extension node** -- "is local support but belongs to no extension node". Add
   it to an existing extension's `lessons`, or create one and list it in the
   path segment's `inline`.
4. **Transitive prerequisites must actually introduce every atom in `requires`
   and `practises`.** A review of four verbs needs a prerequisite chain reaching
   all four, not just the nearest lesson.

Also: `answer` and every `accepted` variant must be distinct after
normalization, which lowercases -- so `"english"` and `"English"` collide. And
a table with **four or more columns** is refused by the narrator and counts
against a corpus-wide refusal pin; three columns are speakable, so put the
fourth column's content in prose.
