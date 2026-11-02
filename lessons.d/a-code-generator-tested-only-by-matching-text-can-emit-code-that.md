---
category: Testing & coverage
---

# A code generator tested only by matching text can emit code that never compiled

`capability-cage-generator` had a test per capability that checked the
generated source *contained* the expected method signature. `env:read` with an
exact target passed that test, but the file it generated never compiled: the
scoped check emitted `return nil, err` from `Getenv`, which returns a
`string`. No Go package declared an exact-target `env:read`, so no build ever
saw that file.

It surfaced only when a new test type-checked the generated file with
`go/types` and `importer.ForCompiler(fset, "source", nil)`.

**Do instead:** a generator's tests should compile, or at least type-check,
what it emits, for every arm of its emitter. Text assertions check that
something was written, not that it is code. The fix took the method's own
zero value (`nil` or `""`) instead of a boolean "two returns" flag.

A related trap was that `--all` refuses to write anything when one legacy
manifest is malformed. To prove that a generator change leaves existing
output alone, build the `origin/main` binary and diff its `--dry-run` output
against the new one for every manifest. Do not diff against the checked-in
files, whose headers embed whichever machine last generated them.
