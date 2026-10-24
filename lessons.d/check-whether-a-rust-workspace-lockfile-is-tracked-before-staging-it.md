---
category: Repo policy / workflow reminders
---

# Check whether a Rust workspace lockfile is tracked before staging it

A tests-first package commit tried to stage `code/packages/rust/Cargo.lock`
after Cargo resolved the workspace, but this repository deliberately ignores
that lockfile. The failed `git add` interrupted an otherwise explicit staging
command. Before naming a generated lockfile in a focused commit, check it with
`git ls-files`; if it is untracked and ignored, leave it out rather than using
`git add -f`.
