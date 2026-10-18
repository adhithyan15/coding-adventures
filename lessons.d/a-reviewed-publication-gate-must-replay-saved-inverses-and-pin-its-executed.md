---
category: Security boundaries
---

# A reviewed publication gate must replay saved inverses and pin its executed audit helpers

Static review of unexecuted Timeline delivery helpers found that saved before/after
hash checks did not themselves replay the claimed inverse, and the publication
helper did not bind a positive exact-candidate verdict or its executed audit helper.
The first proposals were preserved and never executed. Successors replay both
directions, close the document path set and modes, verify current frozen inputs
and positive released reviews, and bind the final helper digest in its publication
approval. Pin every executed subhelper across the mutation boundary.
