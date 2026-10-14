# Fetch the recorded merge SHA before running an exact post-merge report

The pull request API returned the exact squash-merge SHA, but the local
worktree had not fetched `origin/main` after auto-merge. Passing that valid
remote SHA to the local HL42 reporter therefore failed at `git rev-parse`.
Fetch the merged target branch before invoking any exact-revision local audit,
then verify the recorded SHA resolves locally. The report must still name the
recorded merge SHA rather than the moving branch tip.
