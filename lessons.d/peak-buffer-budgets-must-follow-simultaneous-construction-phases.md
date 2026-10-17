---
category: Testing & coverage
---

# Peak buffer budgets must follow simultaneous construction phases

CLOC35 mixed-five receipts had their longest interval at capacity4, followed
by a shorter interval at capacity8. The first exact-budget fixture combined the
final capacity and maximum interval from different callbacks, overbuying104
bytes and incorrectly expecting one byte less to fail. Static review caught it.

Compute the maximum per actual callback capacity/interval and old+new receipt
growth overlap, matching the checked allocation schedule. Keep1/3/5 mixed cases,
assert final capacity and test exact/one-less work, peak, ledger and metadata.
Independent static cost/accounting review remains separate from measured totals.
