---
category: Security boundaries
---

# Security attribution and cross-domain fetch are separate policies

An address map can permit a more-privileged data access without permitting
instruction fetch to enter a less-privileged domain. Model attribution first,
including the enable-state scope of default controls and the architecture's
overlap reduction, then apply separate fetch and data policies. Test each
truth-table row and a same-address case where data succeeds but fetch fails
atomically until an explicit architectural transition occurs.
