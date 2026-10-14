---
category: Rust
---

# Save large producer inventories and suppress merge diffstats

The Urdu producer update contained3704 paths. Printing both diffstat and file
names produced7410 lines; a later default merge printed another3713-path stat.
Capture NUL-separated inventories in artifacts, then print counts, overlaps
and relevant-package exceptions only. Use git merge --no-stat for such imports
and retain complete object proofs in receipts rather than terminal dumps.
