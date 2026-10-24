---
category: Repo policy / workflow reminders
---

# Compare squash deltas when the base advanced

A protected squash merge can have a newer first parent than the pull request's
reviewed base when another change lands first. In that case, comparing the
entire squash tree with the reviewed head tree fails even when GitHub applied
the reviewed change exactly, because the squash tree also contains the
intervening protected changes.

Verify the pull request head and the squash's actual first parent, then compare
the exact reviewed base-to-head binary diff with the actual
first-parent-to-squash binary diff. Check the squash is an ancestor of fresh
`origin/main` separately; require whole-tree identity only when the actual
first parent is the reviewed base.
