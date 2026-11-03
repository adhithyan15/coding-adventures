---
category: Repo policy / workflow reminders
---

# A scripted merge-conflict fix must gate git add on its own success, or a failed check commits the conflict markers

Merging `origin/main` into a stacked PR branch (VER10, #18804) left the workspace
`code/packages/rust/Cargo.toml` in conflict. A short Python script rewrote the
conflict with a regex and asserted that no markers were left. The very next
shell line ran `git checkout --ours … && git add code/packages/rust/Cargo.toml
… && git commit --no-edit`.

The regex expected the `>>>>>>>` line to end in a commit hash, as it had in the
same conflict on another branch an hour earlier. This time git labelled it
`origin/main`, because the merge named a ref rather than a commit. The regex
matched nothing and the assertion failed. But the script was a separate command
from the `git add` chain, so its failure stopped nothing: the file was staged
with its markers and the merge commit was made. It was caught only because a
later `grep -rl '<<<<<<<'` in the same command printed the file. It was fixed
by an amend before anything was pushed.

What to do differently:

- Chain the fix and the staging in one `&&` sequence, so a failed check aborts
  before `git add`: `python3 fix.py && git add … && git commit`. Never put
  the check on its own line in a multi-line command.
- Match conflict markers by shape, not by label: `>>>>>>> .*`. The label is a
  commit hash, a branch name or `origin/main` depending on how the merge was
  started.
- Check marker lines, not marker substrings. During VER26's bootstrap-squash
  reconciliation, a guard searching for `=======` anywhere rejected the CI
  file's decorative comment banner even after its real conflict was removed.
  That check correctly stopped staging. Match a whole `=======` line or a
  line beginning with a real opening/closing marker, then stage only after
  the check succeeds.
- Before committing a merge, run `git diff --cached | grep -c '^+\(<<<<<<<\|>>>>>>>\)'`.
  After committing, run `git diff <previous-head> HEAD | grep -c …` the same
  way. Both must print 0.

