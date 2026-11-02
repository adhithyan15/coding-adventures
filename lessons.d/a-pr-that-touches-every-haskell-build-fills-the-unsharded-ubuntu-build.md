---
category: CI & GitHub Actions
---

# A PR that touches every Haskell BUILD fills the unsharded ubuntu build job's disk

PR #18810 added the Darwin Command Line Tools guard to about 200 Haskell
BUILD files. The planner reported `202 packages changed, 220 affected`, and an
ordinary pull request runs that whole set in **one** `build (ubuntu-latest)`
job. That job died with `No space left on device`, and the runner crashed
while writing its own diagnostic log, so GitHub kept no job log at all
(`BlobNotFound`). Only the check run's annotation survived. The Windows leg
passed.

The code was not the cause. Every Haskell package's `cabal test` gets its own
`dist-newstyle/`, and each one recompiles the local packages it depends on
from `cabal.project`. So 220 packages build the shared local libraries many
times over, and the total does not fit on one runner. Main never sees this,
because its full build splits the same packages across 15 weighted shards.

**Fix.** Before touching the PR, let the macOS leg finish on the current
head, because the macOS fix was the reason for the PR. Then add the
`ci-full-build` label and push. The labelled run uses main's sharded Linux
build. The `CI gate` accepts the `build` result from the shards, and it
accepts the skipped macOS and Windows legs.

**Do differently.**

- When an ubuntu `build` job fails with ENOSPC and has no log, read the check
  run's annotations (`gh api .../check-runs/<id>/annotations`). Then read the
  `detect` job's `N packages changed, M affected` line. Once M reaches the
  hundreds, the unsharded PR leg is the problem, not the change.
- A repo-wide mechanical edit (a guard in every BUILD, say) needs the
  `ci-full-build` label from the start. Prove the platform it fixes on its
  ordinary leg first, then add the label.
- Pushing a new head cancels the running legs. Wait for the leg you need
  evidence from before you push.
