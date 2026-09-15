## 0.170.0 — 2026-08-13 — conditional transitive selector idempotence

A conditional selector assignment whose leaves are all the same bare selector
now leaves that known transitive selector stable even when the conditional
predicate is dynamic. Differing, computed, controlled, and unsupported selector
effects remain conservative without recursive effect inference.

