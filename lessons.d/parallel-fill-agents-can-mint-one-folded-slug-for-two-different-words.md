---
category: Testing & coverage
---

# Parallel fill agents can mint one folded slug for two different words

`roots:` slugs fold IAST to ASCII (ā→a, ś→sh), which erases vowel
length. Three agents filled Sanskrit etymology in parallel, and two of
them minted `sanskrit-vamana` independently: one for *vamana* "vomiting",
the other for *vāmana* "dwarfish". `check:root-slug-splits` cannot see
this: it detects one lemma spelled with its language tag in different
positions, not two lemmas that fold to one string. The cousins panel
would have joined two unrelated words.

The second agent noticed and renamed to `sanskrit-vamana-dwarf`. The
reviews then found older cross-track slugs with the same defect:
`sanskrit-nava` (new / nine), `sanskrit-mala` (garland / dirt),
`sanskrit-patra` (leaf / vessel), `sanskrit-pita-drunk` used for *pīta*
"yellow".

**What to do:** before reusing or minting a slug, grep every user of
that exact string and read their hooks, not the slug's name. When
several agents fill at once, diff each agent's new slugs against the
others' before merging. Give homonyms a gloss suffix from the start
(`-dwarf`, `-dirt`, `-vessel`).
