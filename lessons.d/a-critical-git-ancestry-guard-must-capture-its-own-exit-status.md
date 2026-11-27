---
category: Repo policy / workflow reminders
---

# A critical Git ancestry guard must capture its own exit status

`git merge-base --is-ancestor` is silent when it fails. In a multi-command
review block without `set -e`, a later successful diff or status command can
make the whole block look successful even though the ancestry predicate
failed. That can turn an out-of-date candidate into a false freshness claim.

For any ancestry, clean-worktree, or expected-head predicate that gates a
commit or push, check and report its own exit status immediately. Prefer an
explicit `if git merge-base --is-ancestor "$base" "$candidate"; then ...; else
fail; fi` branch. When collecting multiple checks, save each result separately
before running the next command; never infer an earlier predicate's result from
the final command's exit code.
