---
category: CI & GitHub Actions
---

# build-tool -validate-build-files without -dry-run validates and then builds the whole repository

**What happened.** To check two new BUILD files I ran the Go build tool as
`build-tool -validate-build-files -root .`. The first run failed validation and
stopped. After the fix, the second run passed validation and then carried on
into a full build of every package it could discover, 5,337 of them, in the
worktree. It ran past a 10-minute timeout and left behind modified tracked
files (`pubspec.lock`, Elixir `cover/*.html`), 156 untracked outputs and 35
`.dotnet/` state directories. One of those then broke the next scan with
"linked or reparse-backed entry is not valid source evidence". A later
`git add -A` staged all of it.

**Fix.** Validate with `build-tool -validate-build-files -dry-run -root .`:
it validates, prints the plan (`WOULD-BUILD`) and exits. To clean up after an
accidental run, restore the tracked files from git (`git checkout -- <file>`)
and move the untracked and `.dotnet/` outputs aside rather than deleting
them.

**Do differently.** Treat `-validate-build-files` as a mode switch on a
build, not a separate command, and always pair it with `-dry-run`. Never
`git add -A` after running a repo-wide tool: list `git status --porcelain`
first and stage paths explicitly.
