---
category: Compiler / VM / language pipeline
---

# A recorded fold branch is not the replacement node's identity

Independent CLOC35 contract review found that an in-bounds array pick calls
fork_cv, discards the returned new_cv, and returns selected.clone() retaining
the old child CV. Finding a folded contribution whose metadata names its own
log ID does not prove the actual replacement AST owns that ID.

Begin ownership tests at the actual typed replacement CV and join its exact
contribution and parents. Keep original-source occurrence certification separate
from this family-specific rewrite gap; explicitly defer the in-bounds repair
under its own spec instead of calling an unused recorded branch complete.
