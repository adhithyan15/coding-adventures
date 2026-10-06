---
category: CI & GitHub Actions
---

# Evidence pinned to a PR's head commit breaks main once the PR squash-merges

The barcode layout registry (`code/specs/fixtures/barcode-layout-1d-v1/targets.json`)
pins each lane's `verified_revision`. A test runs
`git rev-parse <verified_revision>:<package_root>` against it. Both #16343 and
#16361 pinned the PR's own head commit. The PRs squash-merge, so that commit
reaches no branch, and every CI run after the merge fails with
`git rev-parse ... returned non-zero exit status 128`. That includes
unrelated PRs that merge main, like #16391.

#16343's pin was fixed in 704a8ba761. #16361 then repeated the mistake
(`e32c3e4b`), and the fix was the same: point `verified_revision` and the
loop state's `implementation_revision`/`validation_revision` at the
squash-merge commit (02e8b7fc6d). Before doing so, check that
`git rev-parse <merge>:<package_root>` still equals the pinned `package_tree`.

**Do instead:** never pin evidence to a commit a squash merge will discard.
Pin to a revision already on main, or repoint the pin in the merge itself or
in a follow-up. Better still, have the test name the cause ("verified_revision
is not reachable; repoint it at the merge commit") instead of failing on a
raw git error.
