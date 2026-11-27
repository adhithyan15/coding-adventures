---
category: Python
---

# A loop-local graph oracle should bind its inputs explicitly

Ruff's B023 check caught a helper function defined inside a graph-enumeration
loop. It closed over `adjacency` and `costs`, so a future call after the next
iteration could silently use a different graph's state. Move the oracle to a
module-level helper and pass its adjacency explicitly; an iterative stack also
avoids a recursive closure. Run Ruff after building the bounded oracle, not
only after the production algorithm changes.
