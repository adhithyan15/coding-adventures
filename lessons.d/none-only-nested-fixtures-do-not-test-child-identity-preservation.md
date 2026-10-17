---
category: Testing & coverage
---

# None-only nested fixtures do not test child identity preservation

The first CLOC36 selected-root fixtures put a CV only on each root. Removing
only that field before comparing complete subtrees looked like a nested-CV
preservation check, but every nested CV was None. An implementation that erased
nonempty child identities could therefore pass that comparison.

Independent review caught the missing positive witness before implementation.
Add an actual selected call with a separately stamped callee and a folded
argument. Start from the returned typed call, assert the callee's exact old CV
and the argument's fresh CV, then join that child to its own contribution and
ordered arithmetic operands. Keep this subtree-preservation claim distinct
from the still-open root-to-descendant graph closure contract.
