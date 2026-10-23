---
category: BUILD files & dependency management
---

# Expand sparse checkout for build-tool Go dependencies before compiling

The repository build tool's Go module uses local replacements for
`directed-graph`, `progress-bar`, and `starlark-interpreter`. In a sparse
worktree those directories may be absent even though the build-tool source is
present, causing `go build` to fail with replacement-directory errors.

Before compiling the build tool in a sparse checkout, inspect the complete
transitive local-replacement closure, not only the build tool's direct module.
The `starlark-interpreter` replacement itself requires
`starlark-ast-to-bytecode-compiler`, `starlark-vm`, and `virtual-machine`.
Add only the missing repository-owned dependency directories to the sparse
set. Do not interpret missing sparse content as a broken module or change
dependency declarations to compensate. Resolve the closure from tracked
`go.mod` files before the first retry; do not discover one replacement layer
per failed compiler invocation. Compilation is not the full sparse prerequisite:
the tool also loads repository Starlark rule files at runtime, including
`code/packages/starlark/library-rules`, so include its current load roots before
running plan discovery or BUILD validation.
