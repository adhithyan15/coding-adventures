---
category: Testing & coverage
---

# A per-property ratchet and its aggregate expectation must move together

When a native style-degradation ratchet changes, checking only the generated
report against the per-property map can miss stale allowances if the contract
accepts counts as upper bounds. Recompute the sum of the map, update the
aggregate expectation in the contract's own test, and run that test suite. A
fresh report total and the ratchet-map total must agree exactly before the
change is publishable.
