# LP00 — Logic Core: Terms, Unification, Goals, And Search

## Overview

This package is the reusable logic programming library that everything else
will stand on.

Its job is to bring logic programming ideas into ordinary host languages
without requiring a new language first. Prolog will be built later as a
frontend on top of this core.

The package provides:

- symbolic logic terms
- logic variables
- substitutions
- unification
- delayed disequality constraints
- reification
- goals
- states
- backtracking search streams

This is the layer that lets the repo explore logic programming the same way
SymPy explores symbolic mathematics inside Python.

## Design Principle

The design is intentionally **library-first** and **Prolog-second**.

That means the center of gravity is:

- terms
- unification
- goals
- search

and not:

- parser tokens
- clause file syntax
- operator precedence
- Prolog-specific control operators like cut

If this package is well designed, then:

1. host-language users can do logic programming directly in Python
2. a future Prolog interpreter can compile/lower syntax into this API
3. other frontends beyond Prolog remain possible

## Layer Position

```
SYM00 Symbol Core
    ↓
LP00 Logic Core            ← this package
    ↓
Future Prolog Frontend
    - lexer
    - parser
    - clause loader
    - REPL
    ↓
Future Constraint / Tabling / Datalog Layers
```

## What This Package Defines

### Terms

The term model for the first version should be small and explicit:

- `Atom`
- `Number`
- `String`
- `LogicVar`
- `Compound`
- `LogicList` convenience constructors

The internal term universe is:

```text
Term :=
    Atom(symbol)
  | Number(value)
  | String(value)
  | LogicVar(id, display_name?)
  | Compound(functor_symbol, args[])
```

Lists should be representable in two equivalent ways:

1. ergonomic host API, e.g. `logic_list([a, b, c])`
2. canonical term form using `.` / 2 and `[]`

This preserves Prolog compatibility without forcing host-language users to work
with cons cells directly from day one.

### Atoms

An atom is a zero-arity symbolic constant:

```python
atom("homer")
atom("[]")
atom("true")
```

Internally:

```text
Atom(Symbol("homer"))
```

### Logic Variables

A logic variable is a bindable placeholder used during search.

Important rule:

> Variable identity is not based on the displayed name.

So these must be different variables:

```python
X1 = var("X")
X2 = var("X")
assert X1 != X2
```

They may print the same way for teaching convenience, but internally they need
unique identities.

### Compound Terms

A compound term is a functor symbol plus zero or more argument terms:

```python
term("parent", atom("homer"), atom("bart"))
term("likes", var("X"), atom("donuts"))
```

Internally:

```text
Compound(Symbol("parent"), [Atom("homer"), Atom("bart")])
```

## Unification

Unification is the heart of the package.

It answers:

> "Can these two terms be made equal by binding variables?"

Examples:

```text
parent(X, bart)    ~    parent(homer, bart)
⇒ X = homer

f(X, 2)            ~    f(1, Y)
⇒ X = 1, Y = 2

f(X)               ~    g(X)
⇒ fail
```

The API should look like:

```python
subst2 = unify(left, right, subst1)
```

where:

- success returns a new substitution
- failure returns `None`

### Occurs Check

The first version should perform the occurs check by default.

This means unifying `X` with `f(X)` fails:

```text
X ~ f(X)   ⇒ fail
```

Why keep the occurs check?

- semantically cleaner
- more educational
- avoids rational-tree surprises early

If performance later becomes important, an opt-out fast path can be added, but
the spec should begin with correct, unsurprising behavior.

## Substitutions

A substitution is a mapping from logic variables to terms:

```text
{ X ↦ homer, Y ↦ bart }
```

Substitutions should be treated as persistent / functional values in the first
version. Extending a substitution returns a new substitution rather than
mutating shared state in place.

Recommended API:

```python
class Substitution:
    def get(self, var: LogicVar) -> Term | None: ...
    def walk(self, term: Term) -> Term: ...
    def extend(self, var: LogicVar, value: Term) -> "Substitution": ...
    def reify(self, term: Term) -> Term: ...
```

`walk()` follows chains:

```text
X ↦ Y, Y ↦ homer
walk(X) ⇒ homer
```

## Reification

Search results should be turned back into user-facing terms by **reification**.

If a user asks for `X` and the final substitution is:

```text
X ↦ parent(homer, Y)
Y ↦ bart
```

then reifying `X` should return:

```text
parent(homer, bart)
```

If a variable remains unbound, reification should preserve it in a stable,
readable form:

```text
_0, _1, _2, ...
```

or a similar implementation-defined notation.

## Goals

A goal is a function from one state to zero or more successor states.

This is the relational core of the system.

The first package version should include:

- `eq(a, b)` — unify two terms
- `neq(a, b)` — require two terms to remain different
- `succeed()`
- `fail()`
- `conj(g1, g2, ...)`
- `disj(g1, g2, ...)`
- `fresh(...)`

Conceptually:

```text
Goal : State → Stream[State]
```

### State

A state should minimally contain:

- current substitution
- current disequality constraints
- fresh-variable counter

```python
@dataclass(frozen=True, slots=True)
class State:
    substitution: Substitution
    constraints: tuple[Disequality, ...]
    next_var_id: int
```

Later extensions may add:

- trace / proof tree metadata
- search depth counters

## Search Streams

The package needs a lazy stream of answers so that queries can produce:

- zero answers
- one answer
- many answers
- potentially infinite answers

In the Python prototype, generators are the right first representation.

```python
def run(goal: Goal) -> Iterator[State]: ...
```

And ergonomic helpers:

```python
run_all(vars, goal) -> list[Term | tuple[Term, ...]]
run_n(n, vars, goal) -> list[Term | tuple[Term, ...]]
```

The initial search strategy should be simple and explicit:

- left-biased
- depth-first
- generator-based backtracking

That matches the first future Prolog frontend naturally. If later we want
fairer miniKanren-style interleaving search, we can add alternate runners
without rewriting the term/unification layer.

## Public API

The ergonomic Python API should look like this:

```python
from logic_core import (
    atom,
    term,
    var,
    neq,
    eq,
    conj,
    disj,
    fresh,
    run_all,
    run_n,
)

X = var("X")

answers = run_all(
    X,
    disj(
        eq(X, atom("homer")),
        eq(X, atom("marge")),
    ),
)

assert answers == [atom("homer"), atom("marge")]
```

A compound example:

```python
X = var("X")

goal = eq(
    term("parent", X, atom("bart")),
    term("parent", atom("homer"), atom("bart")),
)

assert run_all(X, goal) == [atom("homer")]
```

Recommended module surface:

```python
Term
Atom
Number
String
LogicVar
Compound
Substitution
State
Disequality

atom(name: str | Symbol) -> Atom
num(value: int | float) -> Number
string(value: str) -> String
var(name: str | Symbol | None = None) -> LogicVar
term(functor: str | Symbol, *args: Term) -> Compound
logic_list(items: list[Term], tail: Term | None = None) -> Term

unify(left: Term, right: Term, subst: Substitution | None = None) -> Substitution | None
reify(term: Term, subst: Substitution) -> Term

succeed() -> Goal
fail() -> Goal
eq(left: Term, right: Term) -> Goal
neq(left: Term, right: Term) -> Goal
conj(*goals: Goal) -> Goal
disj(*goals: Goal) -> Goal
fresh(count: int, fn: Callable[..., Goal]) -> Goal

run(goal: Goal) -> Iterator[State]
run_all(vars: Term | tuple[Term, ...], goal: Goal) -> list[Term | tuple[Term, ...]]
run_n(n: int, vars: Term | tuple[Term, ...], goal: Goal) -> list[Term | tuple[Term, ...]]
```

## What This Package Does NOT Include Yet

To keep the first version teachable and correct, it should not include:

- textual Prolog parsing
- clause databases
- `cut`
- negation-as-failure
- arithmetic predicates
- dynamic predicates (`assert`, `retract`)
- tabling
- finite-domain / CLP constraints

Those all belong in higher layers.

## Python Prototype

The first implementation should live in:

- `code/packages/python/logic-core`

Dependencies:

- `symbol-core`

Why Python first?

- fastest iteration loop
- clearest literate implementation
- easiest environment for a relational library that users can poke at directly

## How Prolog Builds On Top Of This

The later Prolog layer should not invent a second execution model.

Instead, it should lower source code into the same logic-core objects:

```text
Source:
    parent(homer, bart).

Lowered:
    Compound(Symbol("parent"), [Atom("homer"), Atom("bart")])
```

And a clause:

```prolog
ancestor(X, Y) :- parent(X, Z), ancestor(Z, Y).
```

becomes a rule representation that ultimately constructs logic-core goals using
the same `var`, `term`, `conj`, `disj`, and `unify` machinery.

That is the key architectural promise of this package.

## Test Strategy

Required tests:

- atoms, variables, compounds construct correctly
- variables with the same display name are still distinct
- unification succeeds on simple matching cases
- unification fails on mismatched functors / arities
- occurs check rejects cyclic bindings
- substitution walking resolves chains
- reification fully resolves nested structures
- `eq` yields one state on success and zero on failure
- `neq` stores delayed constraints when disequality is undecided
- `eq` rechecks stored disequalities after unification
- conjunction threads substitutions correctly
- disjunction yields multiple answers
- `fresh` produces distinct variables
- `run_n` truncates without exhausting infinite or long streams
- list term helpers round-trip with canonical cons representation

## Future Extensions

- fair interleaving search in addition to depth-first search
- proof traces / explanation trees
- arithmetic relations
- tabling / memoization
- Datalog subset on the same term substrate

## The Long-Term Payoff

With `symbol-core` plus `logic-core`, the repo gets a reusable foundation for:

- Prolog
- miniKanren-style host-language libraries
- symbolic rule engines
- future symbolic algebra experiments where symbols are already first-class

That is exactly why the library should come first.
