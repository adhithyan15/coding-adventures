---
category: Testing & coverage
---

# Sparse merge conflict paths can be absent from the working tree while index stages retain both versions

During a normal merge, 28 paths had unresolved index stages while several
conflicted files were physically absent from the sparse checkout. A plain
`git checkout --conflict=merge -- <paths>` rejected thirteen paths even though
their stage records were retained. Missing working files did not mean the
parent versions were lost.

Inspect the complete unresolved index first and retain its path, mode and
stage object IDs. Verify the parent blobs are available before reading them.
Use those exact three versions for a normal three-way content merge, then
materialize only the resolved paths. Preserve both sides' semantic additions;
do not select a complete parent merely to remove conflict markers. Recompute
generated boundary pins with the official generator and stage the exact paths
with sparse-aware Git options. After committing, restore and hash the declared
physical test inputs before starting validation.
