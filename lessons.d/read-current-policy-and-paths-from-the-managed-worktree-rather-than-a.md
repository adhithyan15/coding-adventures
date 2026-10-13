---
category: Repo policy / workflow reminders
---

# Read current policy and paths from the managed worktree rather than a stale primary checkout

While the Perl72 managed worktree was still being created, reads from the
primary checkout found old CLAUDE.md instructions and no current perl-parser,
release grammar or lessons.d paths. Its local branch was intentionally old;
fetched origin/main already contained all of these files.
Use git show origin/main:<discovered path> for interim read-only audits, or
wait for the managed worktree's completed path before normal file reads.
Never infer current policy or missing implementation from a preserved stale
checkout, and never update that checkout merely to simplify inspection.
