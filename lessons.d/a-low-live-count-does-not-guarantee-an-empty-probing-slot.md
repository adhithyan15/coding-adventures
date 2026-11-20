---
category: Testing & coverage
---

# A low live count does not guarantee an empty probing slot

A DT18 fixture-planning assumption treated the tombstone-only fallback as
unreachable because live entries stay below the 75% growth threshold. That
bound says nothing about deleted slots. Deletion marks a tombstone and does
not restore an empty slot, so repeated insertion/deletion can consume every
empty slot while the live count stays small.

The actual public Swift reference supplies a small witness: use capacity 4,
linear probing and FNV-1a-32. Tagged byte keys `00`, `01`, `02`, `03` have home
indices 2, 1, 0, 3. Insert and remove each: the table has zero live entries and
four tombstones. Inserting `04` requires full-cycle tombstone reuse; lookup of
absent `05` must terminate after a bounded traversal. Filling to three live
entries and adding another key must grow before reusing a tombstone.

This sequence passed through the public fixture consumer. Derive reachability
from occupancy transitions, then execute the witness before labeling a branch
internal-only or unreachable. Keep empty, occupied and deleted counts distinct.
