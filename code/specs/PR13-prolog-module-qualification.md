# PR13: Prolog Module Qualification

## Summary

This batch teaches `prolog-loader` how to execute explicit module-qualified
goals such as `family:ancestor(homer, Who)` after multi-source linking.

The module linker already rewrites unqualified local and imported predicate
references into the internal qualified predicate names used by the executable
`Program`. This PR extends that same rewriting pass to explicit Prolog module
qualification so linked projects can address another module directly.

## Goals

- support explicit `module:goal` qualification in linked SWI projects
- rewrite qualified top-level queries and `initialization/1` goals
- support common meta-goal wrappers:
  - `call/1`
  - `once/1`
  - `not/1`
  - `\\+/1`
  - `phrase/2`
  - `phrase/3`
- resolve qualified goals using the target module's own local predicates and
  imports rather than the caller's import surface
- raise a clear linking error when a qualified goal references an unknown module

## Design

Linking now happens in two phases:

1. Build one relation resolver per loaded source.
2. Re-run source rewriting using both the source resolver and a lookup table of
   module resolvers.

When the rewriter encounters a goal term with the `:/2` functor and an atom
module name on the left, it switches to that module's resolver and rewrites the
qualified goal term into the executable predicate names used by the linked
program.

This lets cases like `family:ancestor/2` work even when `ancestor/2` itself
calls helpers imported into `family` from a different module.

## Non-goals

- file/path-based `use_module` resolution
- `consult/1` and source graph loading
- `module:term` data semantics outside goal and meta-goal rewriting
- `reexport/1` and `reexport/2`
- meta-predicate declaration driven module propagation
- full SWI module transparency rules
