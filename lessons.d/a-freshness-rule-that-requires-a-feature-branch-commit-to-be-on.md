---
category: CI & GitHub Actions
---

# A freshness rule that requires a feature-branch commit to be on main never passes in a squash-merge repository

VER31 (the tiered release standard) first specified two freshness rules by
commit ancestry: a security-review record had to name a `reviewed_commit` on
`main`, and an assurance report's `commit` had to be an ancestor of the release
tag. Both look strict and neither can ever pass here: this repository
squash-merges, so the feature-branch commit a review or report was made on is
never an ancestor of anything on `main`. (ct-compare's report names
`838ab7d47e2`, which does not even exist in a fresh clone.) The report's
`commit` field is also just a generator argument, not something the generator
observed.

The fix in LV-007a binds *content*, not history. The review record maps each
reviewed path to the SHA-256 of the bytes reviewed, and the gate re-hashes
them at the tagged commit. The report is fresh when re-running the package
BUILD regenerates every evidence manifest and `git diff --exit-code` on the
package is clean.

Next time a rule says "X must be on main" or "X is an ancestor of Y", check the
merge strategy first. If commits are rewritten by squash or rebase, bind to
file digests or to a reproducible regeneration instead.
