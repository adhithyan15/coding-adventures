---
category: BUILD files & dependency management
---

# The repository build tool needs its local Go replacement paths present in a sparse worktree

The Go build selector uses relative `replace` directives for repository-owned
packages such as `directed-graph`, `progress-bar`, and `starlark-interpreter`.
Running it from a sparse worktree that exposes only the selector but not those
packages fails before change selection. Before the final repository gate,
either add every local replacement path to the sparse definition or disable
sparse checkout in the isolated worktree, then rerun the selector from
`code/programs/go/build-tool` with the worktree root passed explicitly.

This recurred after the Rust package gates were already green: building the Go
selector failed on those same three absent replacement directories before it
could inspect the X.509 diff. Run `git sparse-checkout list` and materialize the
selector's local replacements before compiling the tool, not after treating
its dependency-resolution failure as a product gate.

`git sparse-checkout add` also resolves its path arguments from the command's
current directory. Running it inside `code/programs/go/build-tool` with
repository-root spellings left the sparse list unchanged and repeated the
failure. Run sparse-scope changes from the worktree root, then verify both
`git sparse-checkout list` and the materialized paths before retrying.

Materializing only the three replacements named by the first compiler error is
still insufficient: `starlark-interpreter` has its own local replacements, so
the next build stopped on that transitive closure. For this build tool, add the
bounded `code/packages/go` subtree or compute the complete recursive `replace`
closure; do not iterate one missing package per compiler run.
