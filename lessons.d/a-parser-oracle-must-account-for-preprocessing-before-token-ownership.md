---
category: Testing & coverage
---

# A parser oracle must account for preprocessing before token ownership

The first Timeline direction test expected an authored init-like span inside
an accessibility block to remain in the description. Shared preprocessing
already blanks every such span before tokenization. The configuration ownership
scanner independently prevents it from selecting padding; neither production
stage was broken. The explicit-direction control failed, correctly withholding
the first red receipt.

Keep the authored source unchanged, declare the description after preprocessing,
and rerun the red control before repairing direction. Read the full ingestion
pipeline when specifying whole IR; a later ownership boundary cannot restore
text removed by an earlier stage. Preserve failed evidence and never widen
production scope to make an unrelated new oracle pass.
