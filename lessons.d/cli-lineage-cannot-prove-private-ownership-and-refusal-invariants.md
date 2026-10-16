---
category: Testing & coverage
---

# CLI lineage cannot prove private ownership and refusal invariants

The first D2 specification required exact CLI result-to-ingestion graph tests,
but those cannot distinguish moving existing ID strings from cloning them.
They also cannot invoke private missing capture, unequal vector lengths or
already-stamped Tokens. Independent contract review withheld acceptance of this
test plan before production code existed.

Add private boundary tests before implementation: preserve vector and ID-buffer
pointers on success, every other Token field, and the caller's borrowed Tokens
on rejected invariants. Explicitly distinguish trusted missing/lexical-error
capture from legacy public source-based fallback, and keep a public detached-ID
compatibility control. Retain the compiled CLI semantic red separately; an
unimplemented helper's compilation failure proves no runtime lineage defect.
