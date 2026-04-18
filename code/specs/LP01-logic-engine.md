# LP01 — Logic Engine: Relations, Clauses, Programs, And Resolution

## Overview

`logic-core` gives us terms, unification, goals, and generator-based
backtracking. That is the semantic kernel.

What it does **not** yet give us is a reusable way to write logic *programs*:

- named relations like `parent/2` and `ancestor/2`
- facts and rules
- recursive relation calls
- clause databases / programs
- a solver that resolves relation calls against those clauses

This package is that missing layer.

It should let a Python user write something structurally close to Prolog, but
as a host-language library rather than a new text syntax.

## Design Goal

This layer should be:

- **engine-first**, not parser-first
- **library-native**, so users can solve problems directly in Python
- **reusable** by the later Prolog implementation

That means the future Prolog frontend should parse source code into the same
`Relation`, `Clause`, `Program`, and goal-expression objects defined here.

## Layer Position

```text
SYM00 Symbol Core
    ↓
LP00 Logic Core
    - terms
    - unification
    - goals
    - state / backtracking
    ↓
LP01 Logic Engine          ← this package
    - relations
    - clauses
    - programs
    - recursive solving
    ↓
Future Prolog Frontend
    - lexer
    - parser
    - lowering into LP01
```

## What This Package Adds

### Relations

A relation is a named predicate with fixed arity:

```python
parent = relation("parent", 2)
ancestor = relation("ancestor", 2)
```

Calling a relation should produce a relation-call expression:

```python
parent("homer", "bart")
ancestor(var("X"), var("Y"))
```

The relation object should enforce arity immediately.

### Relation Calls

A relation call is a goal expression representing:

```text
parent(homer, bart)
ancestor(X, Y)
```

It is *not* solved by itself. It becomes meaningful only relative to a
`Program`.

### Clauses

The engine should support two kinds of clauses:

- **facts**: head only
- **rules**: head plus body

Examples:

```python
fact(parent("homer", "bart"))

rule(
    ancestor(X, Y),
    parent(X, Y),
)

rule(
    ancestor(X, Y),
    fresh(1, lambda z: conj(
        parent(X, z),
        ancestor(z, Y),
    )),
)
```

### Programs

A program is an immutable collection of clauses, indexed by relation.

```python
family = program(
    fact(parent("homer", "bart")),
    fact(parent("homer", "lisa")),
    rule(ancestor(X, Y), parent(X, Y)),
    rule(ancestor(X, Y), fresh(1, lambda z: conj(parent(X, z), ancestor(z, Y)))),
)
```

The first implementation should use a simple relation index:

```text
(relation symbol, arity) -> ordered clauses
```

Clause order matters because the first search strategy remains:

- left-biased
- depth-first
- backtracking

That keeps the library aligned with the first Prolog engine we want later.

## Goal Expression Model

This package introduces a *declarative expression layer* above LP00.

The engine should support:

- `eq(left, right)`
- `neq(left, right)`
- `defer(builder, *args)`
- `succeed()`
- `fail()`
- `conj(g1, g2, ...)`
- `disj(g1, g2, ...)`
- `fresh(count, fn)`
- `all_different(...)`
- relation calls like `parent(X, Y)`

Unlike LP00, these are not plain `State -> Iterator[State]` functions. They are
goal expressions that the solver interprets relative to a `Program`.

Conceptually:

```text
GoalExpr :=
    Succeed
  | Fail
  | Eq(term, term)
  | Neq(term, term)
  | Defer(builder, term[])
  | Conj(goal[])
  | Disj(goal[])
  | Fresh(vars, goal)
  | Call(relation, args[])
```

## Why Use Expressions Instead Of Raw LP00 Goals?

Because recursive program solving needs access to:

- which relation is being called
- which clauses belong to that relation
- when to standardize a clause apart
- how to index clause lookup

LP00 goals are already-compiled executable functions. That is perfect for the
kernel, but it hides the structure that a logic-programming engine needs.

LP01 therefore adds a structured expression tree and a solver that lowers those
expressions into actual search steps using LP00's unification and `State`.

## Resolution Semantics

To solve a relation call:

1. look up matching clauses by relation
2. for each clause, **standardize it apart**
3. unify the query call with the freshened clause head
4. if unification succeeds:
   - facts yield a solution immediately
   - rules continue by solving the freshened body
5. backtrack across remaining clauses on failure

This is the first reusable Prolog-like solving engine.

## Standardize-Apart

Every time a clause is used, its variables must be replaced with fresh search
variables.

Why?

Because recursive rules must not accidentally share variable identities across
different clause applications.

Without standardizing apart, a rule like:

```text
ancestor(X, Y) :- parent(X, Z), ancestor(Z, Y).
```

would reuse the same `Z` across unrelated branches, corrupting the search.

So each clause application should behave as if it got a brand-new copy of all
its variables.

This package must therefore include:

- term renaming
- goal-expression renaming
- clause renaming
- search-state counter advancement for newly allocated variables

## Public API

The first Python API should look like this:

```python
from logic_engine import (
    defer,
    atom,
    conj,
    disj,
    eq,
    fact,
    fresh,
    logic_list,
    neq,
    program,
    relation,
    rule,
    all_different,
    solve_all,
    solve_n,
    term,
    var,
)

parent = relation("parent", 2)
ancestor = relation("ancestor", 2)

X = var("X")
Y = var("Y")

family = program(
    fact(parent("homer", "bart")),
    fact(parent("homer", "lisa")),
    rule(ancestor(X, Y), parent(X, Y)),
    rule(
        ancestor(X, Y),
        fresh(1, lambda z: conj(parent(X, z), ancestor(z, Y))),
    ),
)

answers = solve_all(
    family,
    Y,
    ancestor("homer", Y),
)

assert answers == [atom("bart"), atom("lisa")]
```

Recommended exports:

```python
Relation
RelationCall
Clause
Program
GoalExpr

relation(name: str | Symbol, arity: int) -> Relation
fact(head: RelationCall) -> Clause
rule(head: RelationCall, body: GoalExpr) -> Clause
program(*clauses: Clause) -> Program

succeed() -> GoalExpr
fail() -> GoalExpr
eq(left: object, right: object) -> GoalExpr
neq(left: object, right: object) -> GoalExpr
defer(builder: Callable[..., GoalExpr], *args: object) -> GoalExpr
conj(*goals: GoalExpr) -> GoalExpr
disj(*goals: GoalExpr) -> GoalExpr
fresh(count: int, fn: Callable[..., GoalExpr]) -> GoalExpr
all_different(*terms: object) -> GoalExpr

solve(program: Program, goal: GoalExpr) -> Iterator[State]
solve_all(program: Program, query: object | tuple[object, ...], goal: GoalExpr) -> list[Term | tuple[Term, ...]]
solve_n(program: Program, n: int, query: object | tuple[object, ...], goal: GoalExpr) -> list[Term | tuple[Term, ...]]
```

`defer(...)` exists so higher-level helper libraries can define recursive
goal builders without forcing eager Python recursion while the expression tree
is still being constructed.

The package should also re-export the core term constructors from LP00:

- `atom`
- `num`
- `string`
- `var`
- `term`
- `logic_list`

## Search Strategy

The first search strategy stays intentionally simple:

- depth-first
- left-biased
- generator-based
- clause order preserving

That is enough to validate the relational engine end to end before we invest in
fair search, tabling, or parser machinery.

## What This Package Does NOT Include Yet

To keep the first engine slice focused, it should not yet include:

- textual Prolog parsing
- operator precedence
- cut
- negation-as-failure
- dynamic predicates (`assert`, `retract`)
- arithmetic built-ins
- tabling
- finite-domain constraints beyond disequality
- module systems

## Python Package Location

The first implementation should live in:

- `code/packages/python/logic-engine`

Dependencies:

- `logic-core`

## Test Strategy

Required tests:

- relation objects enforce arity
- facts solve directly
- rules solve through body evaluation
- recursion works on a small family-tree program
- standardize-apart prevents variable capture across recursive rule use
- `solve_n` truncates answer streams
- list-shaped relations can be expressed with canonical list terms
- tuple queries reify multiple variables correctly
- missing relations produce no answers rather than crashing

## Why This Milestone Matters

Once LP01 exists, the repo can validate the big architectural promise:

1. users can write logic programs directly as a Python library
2. recursive solving and backtracking work end to end
3. the later Prolog frontend can become mostly a parser and lowering layer

That is exactly the engine-first path we want.
