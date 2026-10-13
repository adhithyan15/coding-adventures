---
category: Testing & coverage
---

# A clean-checkout absence test requires removing the ignored rendered view locally

The Japanese migration regenerated its ignored `CHANGELOG.md` local view before
running a test whose contract is that a clean checkout has no aggregate file.
That test checks filesystem presence, not Git trackedness, so the recoverable
local view made it fail even after `git rm --cached`. Validate reconstruction,
then remove the exact ignored rendered view before running the clean-checkout
absence assertion; authors can regenerate it on demand afterward.
