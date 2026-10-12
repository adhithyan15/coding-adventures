---
category: Testing & coverage
---

# A single full-corpus fixture read can consume an assertion budget without being duplicated

The complete local corpus suite reported the real cousins snapshot at 59.3
seconds against the ordinary 30-second deadline. That case loads the corpus
only once, so an audit looking only for duplicate loads misses it. Its input
is immutable; initialize that input at module scope while retaining both cousin
indexes, joins and all assertions in the timed case. Never cache production
loader results or share mutable synthetic fixtures to accomplish this.

The same run measured live Spanish/narration report checks at 60.5-72.3 seconds
and all-ledger shard checks at 31.9-32.7 seconds. Those calls must still read and
validate the filesystem inside their cases. Confirm the complete failure
diagnostics before assigning measured, case-specific integration budgets; do
not move live CLI work into unbounded module initialization or raise ordinary
deadlines. Preserve exact byte/exit comparisons and every path-security guard.
