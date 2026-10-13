---
category: Testing & coverage
---

# semantic evidence assertions should use stable single-line substrings when source prose is hard-wrapped

I added a semantic regression triple with a meaningful Sanskrit evidence phrase,
but the Markdown source wrapped that phrase across a newline, so a literal
`toContain` check failed even though the evidence was present. Normalize
whitespace in both the examiner source and the expected evidence before the
containment assertion. This preserves the semantic link while making ordinary
prose reflow irrelevant to the test.
