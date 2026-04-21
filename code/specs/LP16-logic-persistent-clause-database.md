# LP16 - Logic Persistent Clause Database

## Overview

LP01 introduced immutable programs: a `Program` is a source-ordered collection
of facts and rules indexed by relation. LP11 through LP15 then added enough
builtins to write practical Prolog-shaped programs from Python.

The next gap is database metaprogramming. Prolog programmers expect to inspect,
extend, and retract clauses. The Python library should expose those ideas before
the parser exists, but it should do so in a way that preserves the engine's
current immutability and backtracking guarantees.

LP16 adds a persistent clause database API to `logic-engine`.

## Design Goal

Expose Prolog-inspired database operations as pure Python helpers:

- inspect clauses that match a relation-call pattern
- assert a clause at the beginning or end of a program
- retract the first matching clause
- retract every matching clause
- abolish all clauses for one relation

Each update returns a new `Program`; it never mutates the original program.
This keeps the solver deterministic and makes the API safe to use in tests,
examples, future parser lowering, and future VM compilation.

## Package

Update:

```text
code/packages/python/logic-engine
```

The package version should move from `0.4.0` to `0.5.0`.

## Public API

Add:

```python
asserta(program, clause)
assertz(program, clause)
abolish(program, relation)
clauses_matching(program, head_pattern)
retract_first(program, head_pattern)
retract_all(program, head_pattern)
```

Where:

- `program` is a `Program`
- `clause` is a `Clause`
- `relation` is a `Relation`
- `head_pattern` is a `RelationCall`

## Semantics

### `asserta(program, clause)`

Return a new program with `clause` inserted before every existing clause.

The name mirrors Prolog's `asserta/1`: "assert at the beginning." Source order
matters because the solver tries clauses in order.

### `assertz(program, clause)`

Return a new program with `clause` appended after every existing clause.

The name mirrors Prolog's `assertz/1`: "assert at the end."

### `clauses_matching(program, head_pattern)`

Return every clause whose head unifies with `head_pattern`, in source order.

Only the head participates in this first slice. Rule bodies are returned with
their clauses but are not themselves pattern-matched.

### `retract_first(program, head_pattern)`

Return a new program with the first clause whose head unifies with
`head_pattern` removed.

If no clause matches, return `None`. Returning `None` makes the absence of a
match explicit without inventing a sentinel program.

### `retract_all(program, head_pattern)`

Return a new program with every clause whose head unifies with `head_pattern`
removed.

If no clause matches, return a program with the same clause sequence as the
input. The returned program may be a new object, but its logical contents should
be unchanged.

### `abolish(program, relation)`

Return a new program with all clauses for `relation` removed.

`abolish` is relation-key based, not pattern based. It removes every clause with
the same relation symbol and arity.

## Matching Model

Head matching uses ordinary LP00 unification against an empty substitution:

- `parent(homer, X)` matches `parent(homer, bart)`
- `parent(homer, X)` does not match `parent(marge, bart)`
- relation symbol and arity must match, because unrelated compounds should not
  be considered database matches

The helper does not expose the matching substitution in LP16. Later layers can
add a richer inspection API if callers need bindings for a matched pattern.

## Error Model

Database helpers should validate their host-language inputs eagerly:

- non-`Program` program inputs raise `TypeError`
- non-`Clause` assertion inputs raise `TypeError`
- non-`Relation` abolish inputs raise `TypeError`
- non-`RelationCall` head patterns raise `TypeError`

Pattern mismatches are ordinary non-matches, not exceptions.

## Test Strategy

Required tests:

- `assertz` appends clauses and preserves existing source order
- `asserta` prepends clauses and affects solver answer order
- assertion helpers reject non-clause inputs
- `clauses_matching` returns unifiable head matches in source order
- matching ignores unrelated relation symbols and arities
- `retract_first` removes only the first matching clause
- `retract_first` returns `None` when no clause matches
- `retract_all` removes every matching clause
- `abolish` removes all clauses for a relation and keeps other relations
- database updates do not mutate the original program

## Future Extensions

Later database milestones can add:

- body-aware `clause/2` style matching
- substitution-returning clause inspection
- solver-context dynamic predicates that can be asserted during a proof
- transaction-like scoped database updates for backtracking-safe mutation
- compilation of persistent database operations into the logic VM instruction
  set

## Summary

LP16 gives Python callers a Prolog-like way to manipulate clause databases while
keeping the current engine pure and predictable. It is the bridge between static
program construction and future runtime dynamic predicates.
