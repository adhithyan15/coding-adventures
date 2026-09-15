# Accumulator-PR pattern: GitHub auto-deletes the head branch on merge — a push right after merge silently recreates an empty-history branch with no PR

Running a "keep adding commits to one PR" loop (per explicit user instruction, see
`feedback_consolidate_pr_when_reviewer_away.md`), a background push to
`content/dravidian-please` succeeded with `git push` printing `[new branch]` instead
of a normal fast-forward update — the tell that something was wrong. The repo has
"automatically delete head branches" enabled, so the moment the user merged PR
#9210, GitHub deleted `content/dravidian-please` on the remote. My next `git push`
to that same branch name didn't fail or warn — it just recreated the ref from
scratch, accepting local history that was still based on the *old* pre-merge
`main`. The commit landed on a branch with no PR, silently orphaned, invisible
until `gh pr list --head <branch>` came up empty.

**The check that catches it:** after ANY push in an accumulator-PR loop, don't
just verify `git rev-parse HEAD == origin/<branch>` (that was true here — the
push "succeeded"). Also check `git push`'s own output for `[new branch]` on a
branch you believe already has history/a PR, and independently confirm the PR
still exists and is still open (`gh pr view <N> --json state,mergedAt`) *before*
building the next slice, not just before pushing it. A merge can land at any
point mid-loop, not only when you happen to check.

**The fix, once caught:** `git fetch origin --quiet`, diff origin/main against
the orphaned branch to see which commits are already merged (`git ls-tree
origin/main -- <path>` per file, or compare `git log --oneline` lines) vs. still
new, then `git switch -c <fresh-name> origin/main` and `git cherry-pick` only the
truly-new commits onto a clean base — don't just re-push the stale branch. Delete
the orphaned remote branch afterward (`git push origin --delete <name>`) so it
doesn't linger with no PR pointing at it.
