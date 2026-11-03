---
category: BUILD files & dependency management
---

# Diff the real repository's dependency edges before and after changing a manifest reader

While widening the Cargo reader to build and target tables, the first draft
of the contract required an inline dependency value to open and close its
braces on one line, and called multi-line inline tables out of scope. Unit
tests and the shared fixture all passed. Only comparing the forced plan of
the origin/main tool with the new one showed the regression: the edge
`vault-pm-cli -> programs/vault-pm-cli-drill` disappeared, because that
manifest writes `{ path = "...", features = [` with the array on later lines,
which TOML 1.0 allows and the old lenient reader had kept.

Before committing a reader change, build the base tool from
`git archive origin/main code/programs/go/build-tool code/packages/go` into
scratch space, run both with `-root <worktree> -force -emit-plan <file>`
(`-emit-plan` exits before hashing), and diff `dependency_edges`. Every
removed edge needs an explanation, and the added edges are the evidence for
the change. Check the new graph for cycles too.
