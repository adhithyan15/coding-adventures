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
