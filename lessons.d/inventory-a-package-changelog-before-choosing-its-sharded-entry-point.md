---
category: Testing & coverage
---

# Inventory a package changelog before choosing its sharded entry point

The temporal package has CHANGELOG.md, while the painter has CHANGELOG.d.
A read assumed both packages had a sharded entry point and failed for temporal.
Inventory actual files before selecting a documentation path. Recording this
read mistake after semantic-red completion but before artifact copying then
made the copier's current-clean guard refuse. The native terminal receipt
remains accepted; its sources, HEAD and index did not change. Preserve the
copier refusal and explicitly bind the sole post-terminal Markdown note when
copying the still-matching executable/raw artifacts. Do not hide the note,
relocate it to satisfy a guard, or relabel a dirty checkout as clean. Keep the
source freeze through copying, including documentation writes where required.
