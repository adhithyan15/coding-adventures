---
category: CI & GitHub Actions
---

# Verify the retained pull request head after GitHub deletes a merged source branch

After expected-head squash merge of Closure foundation PR #16905, the audit
successfully verified fetched-main reachability and exact blobs/modes for all
120 reviewed paths. It then incorrectly assumed `git ls-remote` would still
return the source branch and indexed an empty result. GitHub's automatic branch
deletion removed that ref even though the full reviewed head remained published
as `refs/pull/16905/head`.

Verify publication before merging, then use the live PR's reviewed head and its
retained pull-request reference for post-merge history preservation. Require the
actual merge commit on fetched main and exact reviewed file contents separately;
a squash merge does not make every feature commit an ancestor of main. Do not
interpret an automatically deleted branch as lost or unpublished work and do
not recreate it merely to satisfy an obsolete cleanup assertion.

The repaired receipt verifies PR head
`7debd2f26bd36ed243b9cf8273c6c576cd59a886` through the retained PR ref and merge
`851ce71fc033f11d0c1716ed895b7026e5c64a81` through fetched main. The managed
archive then retains recoverable local history. The reviewed compiler binary is
preserved externally because ignored build outputs are not part of that archive.

Use an existing lesson category from `_meta.md`; `Git` is not a registered
category. This lesson uses `CI & GitHub Actions` after the CLI rejected the
unknown category. This record was staged externally and is now committed with the next
specification rather than changing the already merged predecessor head.
