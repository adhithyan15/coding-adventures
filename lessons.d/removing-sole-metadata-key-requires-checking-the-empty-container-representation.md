---
category: Testing & coverage
---

# Removing sole metadata key requires checking the empty container representation

The authored line-hop scene oracle removed the sole hop flag, then compared
Some(empty metadata) with the disabled scene's None. Every rendered field
matched, but the container representations differed. Assert the flag's exact
value, prove no other metadata remains, and verify the disabled representation
before normalizing the empty container. Keep complete scene equality and the
work bounds. Preserve the original failed receipt and rerun the affected suite.
The lesson helper accepts the catalog's exact category, Testing & coverage;
the abbreviated category testing is rejected without creating a shard.
