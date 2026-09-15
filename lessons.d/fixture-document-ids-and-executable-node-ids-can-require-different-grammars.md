---
category: TypeScript / JavaScript
---

# Fixture document IDs and executable node IDs can require different grammars

Corpus IDs commonly use descriptive hyphens (`nn27-dynamic-graph-and-saved-values`), while graph node IDs benefit from a tighter identifier grammar for portable map keys. Reusing the node-ID regex for the top-level lab ID rejected a valid checked-in fixture before execution. Validate each namespace according to its contract instead of sharing the strictest helper by convenience.
