# PR06 - Operator-Aware Prolog Parser

## Overview

PR05 made operator tables and directives first-class runtime model objects.
PR06 is the next step: the parser now needs to actually consult those operator
tables while parsing terms, goals, clauses, and directives.

## Scope

Add a shared package:

```text
code/packages/python/prolog-operator-parser
```

This package performs token-level parsing over dialect lexer output using a
`prolog-core.OperatorTable`.

The first batch supports:

- atoms, variables, numbers, strings, lists, and compound terms
- prefix, infix, and postfix operators from the supplied operator table
- facts
- rules
- queries
- optional top-level directives

The parser lowers operator notation directly into first-order term trees and
then lowers goal-shaped terms through `logic-engine.goal_from_term(...)`.

## Integration

`iso-prolog-parser` and `swi-prolog-parser` now use the operator-aware parser
for their executable source APIs:

- `parse_iso_source`
- `parse_iso_program`
- `parse_iso_query`
- `parse_swi_source`
- `parse_swi_program`
- `parse_swi_query`

Each now accepts an optional `operator_table=` override, which lets callers
experiment with custom operators before `op/3` execution exists.

## Non-Goals

This PR does not implement:

- `op/3` directive execution
- parser-driven operator-table mutation while reading one file
- full ISO or SWI operator sets
- negation semantics for `\+`
- `->` control-flow semantics in the engine

Those belong in the next execution-focused batch, now that the parser pipeline
can actually consume operator declarations.
