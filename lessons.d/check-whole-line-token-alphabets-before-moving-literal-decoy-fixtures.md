---
category: Testing & coverage
---

# Check whole-line token alphabets before moving literal decoy fixtures

Venn style hardening correctly rejects JSON used as a fill color. The first
valid companion moved that exact JSON into a set label, but both bracket-label
alternatives exclude embedded double quotes, so two tests failed at tokenization.

Preserve the failed62e3c8a0 receipt. Read the actual whole-line token patterns
before choosing a literal route: the title statement admits the unchanged JSON.
Keep the original invalid-color sources as exact errors, preserve full config
oracles and compare the companion title/fill exactly. Do not widen grammar or
weaken source validation to rescue a fixture.
