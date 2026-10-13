---
category: Testing & coverage
---

# Every new task-shape inventory needs a matching reading-reach floor owner

Adding a task-shape inventory changes the exact set of identities enforced by
the reading-reach floor tests. The Sanskrit A2 inventory initially omitted its
matching floor-owner shard, so the focused suite failed even though the task
shape itself was valid. Add the matching floor owner in the same change as a
new inventory—even when the initial ratchet is zero because no mock passage
exists yet—and run the reading-reach tests before review.
