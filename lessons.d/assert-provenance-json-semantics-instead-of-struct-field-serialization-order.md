---
category: Testing & coverage
---

# Assert provenance JSON semantics instead of struct field serialization order

The full compiler suite failed an EOF tombstone test because it searched for source followed immediately by reason. Canonical export correctly sorts object keys, so the record existed but its fields appeared in another order. Parse JSON and inspect source, reason and token kind on the same entry. Check surviving token records separately. Also name the test for EOF: the lexer currently skips comments and whitespace, so this fixture cannot prove their provenance coverage.
