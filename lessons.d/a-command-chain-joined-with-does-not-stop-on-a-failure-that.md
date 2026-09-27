---
category: Repo policy / workflow reminders
---

# A command chain joined with && does not stop on a failure that a pipe to tail hides

A pipeline's exit status is its **last** command's, unless `pipefail` is set.
So in an interactive shell (no `set -o pipefail`), this chain:

```bash
git cherry-pick A B 2>&1 | tail -1 && \
  (python3 -m unittest tests.x 2>&1 | tail -1) && \
  git push -u origin my-branch
```

ran every step even though the cherry-pick stopped on a conflict and the test
run died with a `SyntaxError` on the conflict markers: each `| tail` exited 0.
The push went out. It happened to publish only the base commit, so no harm was
done — but a chain written to gate a push on green tests did not gate it.

**Do instead:** start any chain that ends in a push (or anything outward-facing)
with `set -o pipefail`, or don't pipe the gating commands at all. Check
`git status` for "You are currently cherry-picking" (or rebasing/merging)
before pushing after any history operation. CI steps already use
`set -euo pipefail`; local command chains need it just as much.
