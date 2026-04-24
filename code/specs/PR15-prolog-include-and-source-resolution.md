# PR15: Prolog Include and Source Resolution

## Summary

This batch extends the SWI loader with two missing source-loading features:
`include/1` and pluggable source resolution.

The loader can now splice included files into the including source before
project linking, and callers can provide a `SourceResolver` hook to map
references like `library(family)` into concrete file paths without baking one
global search policy into `prolog-loader`.

## Goals

- recognize `include/1` as a file dependency in the SWI loader
- recursively load included files and merge their clauses, directives, queries,
  initialization directives, and file dependencies back into the parent source
- reject included files that declare `module/2`
- allow custom source resolution for `consult/1`, `ensure_loaded/1`,
  `use_module/1,2`, and `include/1`
- keep the existing default relative-path and sibling-atom file resolution
  behavior for plain file-backed references
- make `library(...)`-style resolution possible through a caller-supplied hook

## Design

`prolog-loader` now exports a shared:

- `SourceResolver = Callable[[Term, Path], Path | None]`

Every file-loading entry point accepts this optional resolver and threads it
into directive dependency parsing.

Resolution order is:

1. ask the custom `SourceResolver`
2. fall back to the existing built-in relative file resolution rules

For `include/1`, the file loader recursively loads included sources and merges
them back into the parent `LoadedPrologSource`. The merged source keeps the
parent's identity while inheriting:

- included clauses
- included queries
- included directives
- included predicate registry state
- included module imports
- included file dependencies

That lets nested included files continue to contribute `consult/1` and
`use_module/1,2` dependencies to the later project-loading pass.

## Non-goals

- exact textual include semantics across parse-time operator mutations
- parse-time operator propagation from an included file into later parent text
- a built-in `library(...)` search path policy
- `include/1` of module files
- SWI term expansion semantics for include processing
