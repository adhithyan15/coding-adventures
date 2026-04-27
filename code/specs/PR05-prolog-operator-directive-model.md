# PR05 - Prolog Operator Table and Directive Model

## Overview

The dialect frontends can now tokenize and parse separate Prolog dialects, but
they still need a shared runtime-facing model for two Prolog ideas that sit
above raw syntax:

- operator declarations and default operator tables
- top-level directives

PR05 adds that shared model.

## Scope

Add a new shared package:

```text
code/packages/python/prolog-core
```

This package defines:

- `OperatorSpec`
- `OperatorTable`
- `PrologDirective`
- `operator(...)`
- `directive(...)`
- `iso_operator_table()`
- `swi_operator_table()`

It intentionally does **not** attempt operator-aware parsing yet. The first job
is to make operator and directive information first-class data that dialect
packages can share.

## Parser Integration

`prolog-parser` exports:

```python
lower_goal_ast(ast: ASTNode) -> ParsedQuery
```

Dialect parser packages use this to lower directive goals into shared
`PrologDirective` values instead of exposing raw AST nodes.

`iso-prolog-parser` now returns a dialect-shaped parsed source with:

- executable clauses and queries
- `directives=()`
- `operator_table=iso_operator_table()`

`swi-prolog-parser` now returns a dialect-shaped parsed source with:

- executable clauses and queries
- `directives: tuple[PrologDirective, ...]`
- `operator_table=swi_operator_table()`

## Non-Goals

This PR does not implement:

- `op/3` execution
- parser precedence climbing or Pratt parsing
- operator-sensitive clause parsing
- directive execution
- module semantics

Those belong in the next parser-pipeline batch, now that the shared runtime
model exists.
