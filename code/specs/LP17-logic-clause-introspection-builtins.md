# LP17 - Logic Clause Introspection Builtins

## Overview

LP16 gave Python callers persistent clause database helpers: programs can be
extended, filtered, and rebuilt without mutating the original database. The
next Prolog-level gap is introspection from inside a logic query.

Prolog's `clause/2` treats program clauses as data. That ability is central to
metaprogramming, interpreters, debuggers, and eventually a parser-backed Prolog
implementation. LP17 adds the first library-shaped version of that idea while
keeping the current engine pure and predictable.

## Design Goal

Expose source clauses as first-order terms that can be matched by ordinary
unification:

- facts should appear with body `true`
- rules should expose their head and body terms
- clause variables should be standardized apart for each inspection result
- unsupported host-only goal nodes should not crash ordinary enumeration

This is an inspection layer, not dynamic database mutation during an active
proof. Runtime `assert` and `retract` predicates need scoped database state and
backtracking-aware rollback, so they remain future work.

## Packages

Update:

```text
code/packages/python/logic-engine
code/packages/python/logic-builtins
```

`logic-engine` should move from `0.5.0` to `0.6.0`.

`logic-builtins` should move from `0.5.0` to `0.6.0`.

## Public API

Add engine helpers:

```python
clause_body(clause)
clause_as_term(clause)
goal_as_term(goal)
freshen_clause(clause, next_var_id)
```

Add builtin:

```python
clauseo(head, body)
```

## Engine Semantics

### `clause_body(clause)`

Return the body goal for a rule. For facts, return `succeed()` so callers can
observe facts as clauses whose body is logical truth.

### `goal_as_term(goal)`

Convert representable engine goals into first-order terms:

- relation call `parent(X, Y)` becomes `parent(X, Y)`
- `succeed()` becomes atom `true`
- `fail()` becomes atom `fail`
- `eq(X, Y)` becomes `=(X, Y)`
- `neq(X, Y)` becomes `\=(X, Y)`
- conjunction becomes nested `,(Left, Right)` terms
- disjunction becomes nested `;(Left, Right)` terms

Conjunction and disjunction should preserve source order and use right-nested
binary terms for more than two children.

Host-only goals do not have a stable first-order representation in LP17:

- `fresh(...)`
- `defer(...)`
- `native_goal(...)`

Encoding those should raise `TypeError`. Later parser/compiler work can add a
goal-term lowering layer once callable data can be safely turned back into
engine goals.

### `clause_as_term(clause)`

Return `:-(Head, Body)`, where `Head` is the relation call's compound term and
`Body` is `goal_as_term(clause_body(clause))`.

Facts are therefore encoded as `:-(Head, true)`.

### `freshen_clause(clause, next_var_id)`

Expose the engine's standardize-apart operation as a public helper. It should
return a freshened clause and the next available variable id, preserving
variable aliasing within the clause while avoiding capture with active query
variables.

## Builtin Semantics

### `clauseo(head, body)`

`clauseo` is the library spelling of Prolog's `clause/2`.

For every source clause in the current program:

1. standardize the clause apart from the active search state
2. encode the clause body as a first-order term
3. unify `head` with the fresh clause head term
4. unify `body` with the encoded body term
5. yield every resulting state in source order

Facts should unify with body atom `true`.

Rules should expose their body as a term. For example:

```python
rule(child(X, Y), parent(Y, X))
```

can be observed as:

```python
clauseo(child(bart, homer), Body)
```

which yields:

```python
parent(homer, bart)
```

after unifying the rule head with the requested head pattern.

Clauses whose bodies cannot be represented as first-order terms in LP17 should
be skipped by `clauseo`. Skipping keeps mixed programs enumerable even when
some clauses use host-only helper nodes.

## Error Model

Engine helpers validate host-language inputs eagerly:

- non-clause inputs to `clause_body` and `clause_as_term` raise `TypeError`
- non-goal inputs to `goal_as_term` raise `TypeError`
- negative `next_var_id` values passed to `freshen_clause` raise `ValueError`
- non-integer `next_var_id` values passed to `freshen_clause` raise `TypeError`

`clauseo` uses logical failure for ordinary mismatches. Host-language coercion
errors from unsupported Python objects may still raise `TypeError`, matching
the existing builtin conventions.

## Test Strategy

Required engine tests:

- `clause_body` returns `true` for facts and the original body for rules
- `goal_as_term` encodes relation calls, truth, failure, equality,
  disequality, conjunction, and disjunction
- `goal_as_term` rejects host-only `fresh`, `defer`, and `native_goal`
- `clause_as_term` encodes facts and rules as `:-/2` terms
- `freshen_clause` standardizes clause variables apart and preserves aliasing
- `freshen_clause` validates `next_var_id`

Required builtin tests:

- `clauseo` enumerates facts in source order with body `true`
- `clauseo` enumerates rules and returns instantiated body terms after head
  matching
- `clauseo` filters by head pattern
- `clauseo` filters by body pattern
- `clauseo` standardizes returned variables apart from source/query variables
- `clauseo` skips clauses with unsupported host-only body nodes

## Future Extensions

Later milestones can add:

- body-aware host database helpers
- callable goal-term lowering
- `current_predicate` and predicate-property inspection
- dynamic predicate declarations
- backtracking-safe runtime `assert` and `retract` predicates
- VM instructions for clause enumeration and predicate metadata

## Summary

LP17 makes program structure queryable from the Python logic layer. That gives
the library a real Prolog metaprogramming primitive while preserving the clean
engine boundaries needed for future parser and VM work.
