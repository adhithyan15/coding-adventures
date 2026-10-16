---
category: Testing & coverage
---

# Early retention refusal does not prove final replay runs

Independent D3b test review withheld the first contract: all compiler sentinel
cells failed at zero source-retention capacity. Omitting the final replay call
could still pass those cells and direct helper tests. Add a separately derived
work threshold that admits retention exactly, then refuses at the actual final
replay boundary, across every level/format/filter. Keep both refusal families.

The same review found missing wrong-path values, original Some/None capture
status controls, retain-level root order/slot controls and fingerprint-failure
prefix rollback. Add them before production. Derive literal small-catalog replay
accounting and UTF8/NUL/hash-block boundary fingerprints independently; the gate's
own counter and short Name-only fingerprints cannot establish complete charging.
