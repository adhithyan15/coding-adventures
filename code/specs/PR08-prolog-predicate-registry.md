# PR08 - Prolog Predicate Registry And Frontend Directive Semantics

## Overview

PR07 introduced file-scoped `op/3` execution, which gave the parser its first
real frontend state mutation. The next missing layer is predicate metadata:
Prolog source files often declare whether predicates are dynamic, multifile, or
discontiguous, and they also attach startup behavior through
`initialization/1`.

This batch adds a shared immutable predicate registry in `prolog-core` and
threads it through the operator-aware parser plus the ISO and SWI frontends.

## Scope

### `prolog-core`

Add shared frontend metadata types:

- `PredicateSpec`
- `PredicateRegistry`
- `empty_predicate_registry()`
- `apply_predicate_directive(predicate_registry, directive_value)`

Supported directives in this batch:

- `dynamic/1`
- `discontiguous/1`
- `multifile/1`
- `initialization/1`

For now:

- `dynamic/1`, `discontiguous/1`, and `multifile/1` accept a predicate
  indicator like `parent/2` or a proper list of predicate indicators.
- `initialization/1` stores the structured directive for later execution; it is
  not executed during parsing.

### `prolog-operator-parser`

The token-level parser now maintains both:

- the evolving operator table
- the evolving predicate registry

As top-level directives are parsed:

- `op/3` mutates the active operator table
- predicate-property directives mutate the active predicate registry

The parsed source result now exposes the final registry, and the assembled
`logic-engine.Program` receives any `dynamic/1` declarations through
`Program.dynamic_relations`.

### Dialect Frontends

`iso-prolog-parser` and `swi-prolog-parser` now surface the shared predicate
registry through their parsed source results, including structured
`initialization/1` metadata.

## Non-Goals

This PR does not yet implement:

- execution of `initialization/1`
- module-aware predicate properties
- runtime query-time directive execution
- `meta_predicate/1`, `public/1`, `thread_local/1`, or other dialect-specific
  predicate properties
- DCG expansion

Those build naturally on top of the registry introduced here.
