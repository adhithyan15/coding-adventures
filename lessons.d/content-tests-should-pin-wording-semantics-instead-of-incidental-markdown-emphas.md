---
category: Testing & coverage
---

# Content tests should pin wording semantics instead of incidental Markdown emphasis

An A1 rubric clearly said that every paper must reach 60/100, but its test
expected the bold markers around a different substring. The semantic rule was
present and the test failed only because Markdown emphasis moved.

When formatting is not the contract, normalize or use a narrowly bounded
regular expression that accepts equivalent emphasis. Reserve exact-string
assertions for wording or punctuation whose exact rendering carries meaning.
