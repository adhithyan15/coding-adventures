---
category: Repo policy / workflow reminders
---

# Materialize the scaffold generator local Go replacements before running it

In a sparse worktree the scaffold generator's directory can exist while its
`go.mod` replacement targets do not. `go run .` then fails before the tool can
parse arguments. Inspect the generator's `replace` block and materialize
`cli-builder`, `directed-graph`, and `state-machine` before invoking it; the
transitive local module graph is part of the command prerequisite.
