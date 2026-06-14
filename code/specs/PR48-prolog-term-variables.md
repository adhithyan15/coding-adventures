# PR48: Prolog Term Variables

## Goal

Add `term_variables/2` support to the library-first Prolog runtime and the
Prolog-to-Logic-VM path.

## Motivation

`term_variables/2` is a small but important metaprogramming primitive. It lets
programs inspect which variables remain in a term after the current bindings
are applied, which is useful for analyzers, source transformations, constraint
helpers, and debugging tools.

The project already supports `copy_term/2`, `functor/3`, `arg/3`, `=../2`,
`clause/2`, `call/N`, and `catch/3`. Variable discovery is the next natural
piece of that term-metaprogramming surface.

## Design

- Add `term_variableso(term, variables)` to `logic-builtins`.
- Reify the input term before collecting variables.
- Traverse compound arguments left-to-right.
- Include each surviving variable at most once, at its first occurrence.
- Unify the result with a proper logic list.
- Adapt parsed Prolog `term_variables/2` through `prolog-loader`.

## Covered Source Predicate

- `term_variables/2`

## Acceptance Tests

- Duplicate variables appear once.
- Variables are returned in first occurrence order.
- Variables bound before the call are not included if their reified value is
  ground.
- Ground terms return `[]`.
- Parsed Prolog source can run `term_variables/2` through the Logic VM.
