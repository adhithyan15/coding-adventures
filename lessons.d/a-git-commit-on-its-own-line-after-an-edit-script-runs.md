---
category: Repo policy / workflow reminders
---

# A git commit on its own line after an edit script runs even when the edit failed, unless the block starts with set -e

A shell block runs every line, whatever the line before it returned. Take
this block:

```bash
python3 - <<'PY'
# edits README, CHANGELOG and the spec; asserts each anchor
PY
git add -A docs/ src/
git commit -q -F message.txt
```

The edit script failed its first assertion, because the anchor text had
wrapped differently than expected. It wrote nothing. `git add` and
`git commit` ran anyway, so a commit whose message said it updated the docs
held only the code changes. It was caught only because the script's
traceback showed in the output. The commit was still local, and
`git commit --amend` fixed it.

**What to do instead:**

1. Join the commit to the edit with `&&`. **Do not rely on `set -e`**:
   bash ignores it inside any compound command that is followed by `||`
   or used as a condition, and a tool harness may wrap your block exactly
   that way. That happened here: a block starting with `set -e` committed
   anyway after its edit script failed.
2. Remember that a `| tail` in the chain hides failures unless `pipefail`
   is set. The sibling lesson on `&&` chains covers this.
3. Read the output before trusting the commit: `git show --stat HEAD` must
   list every file the message claims.
