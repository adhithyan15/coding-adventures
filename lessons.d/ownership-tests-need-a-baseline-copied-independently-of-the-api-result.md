---
category: Compiler / VM / language pipeline
---

# Ownership tests need a baseline copied independently of the API result

The first Go STP01 ownership test retained three API results and treated one as
canonical. If a constructor regressed to return shared storage, mutating the
third result also changed the canonical value. Equality could therefore pass
while the ownership contract failed.

Clone the expected slice and copy each expected map into independently allocated
storage before mutation. Compare both the retained second result and a fresh
result against that frozen baseline, then check direct lookups. Do not assume
append reallocates. Local mutation experiments replaced each of AllOps and the
four map constructors with a shared-storage implementation; all five variants
failed the corrected test. The production source was restored byte-for-byte.

