---
category: Testing & coverage
---

# Mutation fixtures should start from the pinned plain profile

When a test validates two allowed configurations by mutating one copied project,
copying the adjacent real project as the baseline can accidentally start in the
second configuration. Adding that configuration again creates a duplicate
setting, so the positive check fails for the fixture setup rather than the
policy. Build the mutation target from the checked-in plain fixture each time,
then analyze the adjacent real project independently when it is available.
