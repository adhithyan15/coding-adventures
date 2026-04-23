# PR07 - Prolog `op/3` Directive Execution

## Overview

PR06 taught the parser to *consult* operator tables, but those tables were
still fixed at the start of parsing. Real Prolog source files often evolve
their operator surface as they are read:

```prolog
:- op(500, yfx, ++).
value(Result) :- Result = a ++ b ++ c.
```

PR07 adds the first execution step in the frontend pipeline: parsed `op/3`
directives now mutate the active operator table while the source file is still
being parsed.

## Scope

### `prolog-core`

Add shared helpers for interpreting `op/3` directive terms:

- `apply_op_directive(operator_table, directive_term) -> OperatorTable`

This helper:

- ignores non-`op/3` directive terms
- validates precedence, associativity, and operator names
- supports single-atom names and proper lists of atoms
- supports precedence `0` as operator removal

### `prolog-operator-parser`

The token-level operator parser now:

- executes `op/3` directives as it walks a source file
- uses the updated operator table for later clauses, directives, and queries
- returns the final table as part of `ParsedOperatorSource`

### Dialect Frontends

`iso-prolog-parser` and `swi-prolog-parser` now expose the final file-scoped
operator table produced by parsing the source, not just the initial dialect
default table.

For ISO/Core parsing, top-level directives are now accepted in executable
source/program entrypoints so file-scoped `op/3` declarations can work.

## Non-Goals

This PR does not yet implement:

- runtime `op/3` as a query-time builtin
- arbitrary directive execution beyond operator declarations
- module-aware operator scopes
- DCG expansion
- term-expansion hooks

Those remain later frontend/runtime batches.
