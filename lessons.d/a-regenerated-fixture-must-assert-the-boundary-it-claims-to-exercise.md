---
category: Testing & coverage
---

# A regenerated fixture must assert the boundary it claims to exercise

The first DT18 corpus regenerated cleanly and passed its model tests, but the
named chaining 3-to-6 resize case ended with a key alias already in the map.
It replaced a value instead of growing. A separate merge observation copied
the left map's initial capacity 3 into its configuration after the result had
grown to capacity 6.

Canonical regeneration checks agreement with the generator. It does not prove
that a named scenario reaches its intended boundary. Use a genuinely distinct
fourth key and assert literal count 4/capacity 6. Check configuration capacity
against current capacity and preserve the older input's configuration.
Likewise, a deferred-validation case must identify the later rejecting call;
successful configuration construction is part of the expected behavior.

Keep independent intent regressions beside regeneration, compare the corpus
through a real public reference, and demonstrate that well-formed wrong
expectations fail at the public comparator. A scenario name is not evidence
that its transition ran.
