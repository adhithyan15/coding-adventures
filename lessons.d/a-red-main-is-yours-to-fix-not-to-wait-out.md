---
category: Repo policy / workflow reminders
---

# A red main is yours to fix, not to wait out

**What happened:** a barcode PR was squash-merged while its fixture registry
(`code/specs/fixtures/barcode-layout-1d-v1/targets.json`) still pinned
`verified_revision` to the PR's pre-squash head commit. Main no longer contained
that commit, so `test_barcode_layout_1d_fixtures.py` failed its
`git rev-parse <revision>:<package_root>` step on every branch built from main.
"Repo-wide metadata contracts", and with it "CI gate", went red on an
unrelated curriculum PR that I was driving to merge.

I diagnosed the failure and posted the exact two-value patch as a PR comment.
Then I waited for the owning change's author to fix main. Nothing merged for
about three hours. The owner's direction afterwards: unblock yourself; do not
wait for another person to fix main.

**What to do differently:**

1. When main is red because of someone else's change and no fix PR is open,
   push the minimal fix yourself on the first wake. If you can only push to one
   branch, put it on that branch as its own commit and say so in the PR title
   and body.
2. Verify the fix the way CI does before pushing. For a "revision is not
   reachable" failure, repoint to the squash commit and check that the
   recorded tree still matches:
   `git rev-parse <squash-sha>:<package_root>` must equal `package_tree`.
3. A comment that proposes a patch is not a fix. Waiting is only right while a
   fix PR is already in flight, and then you port its change.
