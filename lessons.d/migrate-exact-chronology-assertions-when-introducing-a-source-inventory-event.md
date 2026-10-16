---
category: Rust
---

# Migrate exact chronology assertions when introducing a source inventory event

The first full D1 suite passed strict lint but its old unit test expected a
pass_order string prefix beginning with lex. The new finalized source inventory
is a real earlier contribution and must remain visible. Preserve that failed
full receipt; specify the migration first, then assert the whole parsed stage
array, exact root contribution source/tag pairs and their journal ordering
around the typed pipeline. Keep every old test and stage ordering invariant.
Do not hide the new event or weaken the comparison to mere field presence.
