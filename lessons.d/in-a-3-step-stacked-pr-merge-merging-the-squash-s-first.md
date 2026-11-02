---
category: Repo policy / workflow reminders
---

# In a 3-step stacked-PR merge, merging the squash's first parent is never optional, even when the squashed PR was a child that carried your base

**What went wrong.** #18446 (llvm-internal) and #18447 (jit-car) were both stacked on #18443 (pair-guard). #18447 squash-merged first, and its squash carried all of pair-guard. To update #18446 I merged jit-car's head into it, then did `merge -s ours <squash>`, then merged `origin/main`. I skipped step 1, merging `<squash>^1` (main just before the squash). The result's diff against main showed 107 files and about 6,800 deleted lines instead of #18446's own 10 files. That happened because #18446's tree lacked every main commit made between its last main merge and the squash. Once the `-s ours` merge made the squash an ancestor, those commits looked like deliberate deletions, and the final main merge kept the deletions.

**The fix.** I redid the merge with step 1 added: merge `<squash>^1` first. After that, `git diff origin/main FINAL` matched #18446's own patch line for line. The bad merge was never pushed.

**What to do differently.**
- Always do all three steps: `merge <squash>^1`, `merge -s ours <squash>`, `merge origin/main`. That applies whether the squashed PR was the branch's parent, a sibling, or a child that carried the branch's base.
- Before pushing, compare the whole patch, not just the stats: `diff <(git diff origin/main FINAL) <(git diff <old-base> <old-head>)`, filtered to the `+`/`-` lines. A diff-stat that shows unrelated files or large deletions means a step was skipped.

