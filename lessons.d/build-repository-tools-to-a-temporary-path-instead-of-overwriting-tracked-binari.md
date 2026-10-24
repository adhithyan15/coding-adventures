---
category: Repo policy / workflow reminders
---

# Build repository tools to a temporary path instead of overwriting tracked binaries

Running `go build -o build-tool .` inside `code/programs/go/build-tool`
overwrites the repository's tracked `build-tool` binary and leaves an unrelated
working-tree modification after an otherwise clean validation run. Build the
current source to an explicit temporary path created with `mktemp -d`, invoke
that exact binary with the worktree root and diff base, then clean the single
known file with `unlink` and the now-empty directory with `rmdir`. Do not put
`rm -rf` in the validation command: the safety layer rejects that recursive
cleanup form before the guarded build can run. If the in-tree binary was
already overwritten in a clean isolated worktree, verify the diff is
binary-only and restore that exact tracked path before freezing the review
head.
