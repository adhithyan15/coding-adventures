---
category: BUILD files & dependency management
---

# A test that walks the whole repository is never scheduled by diff-based CI unless it is registered as a repository input consumer

**What went wrong.** `rust/mosaic-package-resolver`'s
`every_repository_requirement_is_satisfied` walks every `mosaic-package.toml`
under `code/`. The Go build tool maps a changed file only to the package whose
directory contains it, so a PR that bumped a `mosaic-pkg-*` version or edited
an app `.mll` never ran the guard, and CI was green on exactly the drift the
guard exists to catch. Mosaic packages also had no graph edges at all, so
their manifest dependents were never re-run either.

**The fix.** The resolver now reads `mosaic-package.toml` `[dependencies]`,
and `code/programs/go/build-tool/repository_inputs.go` lists packages whose
tests read the whole tree, with the changed paths that select them. A
consumer is added with its prerequisites but not its dependents: seeding the
guard as a changed package would have rebuilt 72 packages for a one-line
layout edit.

**Next time.** When you write a test that globs or walks outside its own
package directory, add a row to `repositoryInputConsumers` in the same PR, and
prove it with `go -C code/programs/go/build-tool run . -root "$PWD" -dry-run
-diff-base origin/main` on a scratch commit that touches only an input file.
