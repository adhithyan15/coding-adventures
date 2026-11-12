---
category: CI & GitHub Actions
---

# Ancestry checks must name the older and newer commits before judging a publication

A Human Languages Books retry moved `gh-pages` from `deploy: e096344e` to
`deploy: 64b0c930`. Reading the two `git merge-base --is-ancestor` probe
results backward made that look like a rollback. It was not: the probe with
`e096344e` first exited zero, while the reverse exited one. `e096344e` is an
ancestor of `64b0c930`, so the candidate was newer. A premature issue comment
and status update needed correction.

Before describing a publication as forward or backward, label the arguments:
`git merge-base --is-ancestor CURRENT CANDIDATE` returns zero only when the
candidate descends from the current source. Cross-check with GitHub's
`compare/CURRENT...CANDIDATE`: `status: ahead` and `behind_by: 0` mean the
candidate is newer. Record both source SHAs, the Pages commit messages, and the
comparison result before posting a regression claim or rerunning a publisher.
