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

The per-track curriculum digest case also measured 36.5 seconds around one real
graph read. Its parsed default-root graph can be a module fixture, while digest
creation, exact pin comparisons and intentional pin updates remain inside the
case. Temporary-root ownership checks must continue to read their changed files.

Persian's writing-ladder snapshot then measured 64.8 seconds around a whole-corpus
read. Seven exam coverage/probe cases in the Kannada, Tamil, Telugu and Sanskrit
shards read the same immutable input. Give those cases a shared parsed fixture,
leaving live inventory reads, fresh measurements and all assertions inside their
original deadlines. Check consumers for mutation before sharing their inputs;
retain the normal worker count and heap cap when validating the change.

The same run measured live Spanish/narration report checks at 60.5-72.3 seconds
and all-ledger shard checks at 31.9-32.7 seconds. Those calls must still read and
validate the filesystem inside their cases. Confirm the complete failure
diagnostics before assigning measured, case-specific integration budgets; do
not move live CLI work into unbounded module initialization or raise ordinary
deadlines. Preserve exact byte/exit comparisons and every path-security guard.
