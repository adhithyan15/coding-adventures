---
category: Testing & coverage
---

# Check the actual failing assertion before accepting semantic red

The third CLOC41 red run had the exact predicted4P9F native and1P3F CLI names
and counts. Independent terminal review still withheld aggregate acceptance:
the repeated six-method CLI test stopped at a wrong formatting oracle before
its result-owned graph assertion. The emitter concatenates statements with
one final LF; the test expected a LF after each report. The other two CLI
failures and the native lineage failures did reach their intended assertions.

Preserve the raw receipt and its status-classification flag without promoting
it to independent causal acceptance. Specify existing formatting first, retain
every explicit numeric value and statement-order entry, and correct only LF
placement. Re-run unchanged production and inspect actual assertion sites
before installing the lineage patch. Exact failure names and counts prevent
control swaps, but do not prove why a test failed.
