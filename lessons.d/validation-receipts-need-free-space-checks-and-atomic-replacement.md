---
category: Testing & coverage
---

# Validation receipts need free-space checks and atomic replacement

Native25 producer24222 exhausted C: while rewriting its validation receipt.
Opening the JSON for writing truncated it before OSError28, and exception
recording failed for the same reason. The empty JSON and incomplete test log
are failed evidence, not a green result. Preserve them under their original
names and use a separately named retry after space recovers.

Check available space before starting a compiler producer and write each
working receipt to a sibling temporary file before atomic replacement. This
preserves the prior snapshot if writing fails; it cannot create acceptance for
an incomplete run. Disable regenerable incremental/debug output for the
isolated retry when appropriate, retaining the same tests and assertions.
Never clean a frozen validation checkout/target or immutable evidence to
recover space; preserve the actual process result and original failure.
