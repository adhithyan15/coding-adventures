---
category: BUILD files & dependency management
---

# Repository-wide orphan validation requires all package roots to be materialized

The Go build selector successfully evaluated BUILD files in a sparse X.509
worktree, then its repository-wide orphan checks reported every sparse-hidden
package named in `code/BUILD-EXEMPTIONS` as a stale missing root. The entries
existed at `HEAD`; only their directories were absent from the working tree.

An orphan-root gate needs a fully materialized package namespace. Before using
the selector as a final gate in a sparse worktree, either materialize every
language package root covered by its census or disable sparse checkout in that
isolated worktree. Do not edit the exemptions ledger in response to missing
paths until `git ls-tree` confirms they are actually absent from the commit.
