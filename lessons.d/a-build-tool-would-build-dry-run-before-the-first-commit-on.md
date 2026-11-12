---
category: Repo policy / workflow reminders
---

# A build-tool would-build dry run before the first commit on a branch reports every package, so commit before counting

WX-D1 slice S1 (warrant-grammar) ran
`build-tool -root . -diff-base origin/main -dry-run` to check its expected
would-build count of about 10 while every change was still uncommitted. It
printed `5552 would-build`. The tool diffs `origin/main...HEAD`, not the
working tree, and on a branch whose HEAD still equals the base that diff is
empty, which the tool treats as "build everything" rather than "build
nothing". After the three commits the same command printed
`10 would-build` (warrant-grammar, its dependents warrant-ir and
ca-assurance-report, and their prerequisites).

Do this: commit first, then run the dry run. A count of every package means
the diff was empty or unreadable. It does not mean the change touched a
shared crate. Read the total as a sanity check before acting on it.
