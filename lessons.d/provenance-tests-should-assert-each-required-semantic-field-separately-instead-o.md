---
category: Testing & coverage
---

# Provenance tests should assert each required semantic field separately instead of requiring one exact contiguous phrase across intervening consent and date fields.

A key correctly required speaker provenance, consent date, recording date, tradition evidence, and reviewer confirmation, but a test demanded the exact contiguous phrase `speaker provenance and reviewer confirmation`. The intervening fields made the semantically complete sentence fail. Assert `speaker provenance` and `reviewer confirmation` separately (and pin the additional required fields independently) so punctuation or legitimate field insertion does not create a false negative.
