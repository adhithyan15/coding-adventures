---
category: Testing & coverage
---

# A normal merge can drop sparse worktree-only inputs

In a sparse worktree, selected tracked files can be materialized in the working directory while retaining their `skip-worktree` index bits. A normal merge may remove those physical files even when the resulting Git tree is unchanged. Therefore, a same-tree comparison alone does not prove that a test's required inputs are available.

After a merge, compare the test's declared input list with physical files. For any missing input that is still tracked at the final `HEAD`, restore only those exact paths from `HEAD` with `git restore --source=HEAD --worktree --ignore-skip-worktree-bits -- <paths>`. Verify each restored file against its Git blob and expected raw hash, then confirm the source tree and index remain unchanged. Do not broaden sparse patterns or claim carried validation until those physical inputs are present.
