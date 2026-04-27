# PR11 - Prolog DCG Expansion

## Overview

The Prolog frontends already recognized `-->` lexically, but they still rejected
Definite Clause Grammar rules during parsing. This batch turns DCGs into a real
frontend feature by expanding grammar rules into ordinary executable clauses
that the existing logic engine can already run.

## Scope

### Shared frontend semantics in `prolog-core`

Add shared helpers for DCG lowering:

- `expand_dcg_clause(head_term, body_term) -> Clause`
- `expand_dcg_body(body_term, dcg_input, dcg_output) -> Term`
- `expand_dcg_phrase(goal_term, input_term, output_term=None) -> Term`

The first expansion pass supports:

- callable nonterminals
- list terminals such as `[a, b]` and `[Head | Tail]`
- conjunction with `,`
- disjunction with `;`
- cut `!`
- braced goals `{ Goal }`

### Operator-aware parser integration

`prolog-operator-parser` now accepts top-level DCG rules:

```prolog
digits --> [a], [b].
```

and expands them into ordinary clauses with two appended difference-list state
arguments while parsing the source.

### Dialect frontends

`iso-prolog-parser` and `swi-prolog-parser` now:

- accept DCG rules in their grammar-owned `parse_*_ast(...)` entrypoints
- accept DCG rules in executable source/program parsing

### Loader builtin adapter

`prolog-loader` now adapts:

- `phrase/2`
- `phrase/3`

into executable runtime goals using the shared DCG expansion helper.

## Non-Goals

This batch does not yet implement:

- `term_expansion/2` or `goal_expansion/2`
- `call_dcg/3`
- module-aware DCG expansion
- string-literal terminal semantics beyond ordinary list terms
- the full ISO/SWI DCG helper library

Those can layer naturally on top of the shared DCG expansion foundation added
here.
