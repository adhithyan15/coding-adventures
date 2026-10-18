---
category: Testing & coverage
---

# Start changelog shards with their required level-two heading

The actual cumulative-Timeline documentation gate rejected a new changelog
shard beginning with a level1 heading. The shard runtime requires its first
heading at level2; the preamble belongs in CHANGELOG.d/_meta.md. Native tests
and strict lint had passed, but they did not establish documentation validity.
Keep the failed docs receipt and raw error, repair the heading to level2, and
rerun the real documentation gate. Do not relabel docs failure as native failure
or carry publication approval across a rejected carrier. Preserve the native
execution head separately when only Markdown changes afterward.
