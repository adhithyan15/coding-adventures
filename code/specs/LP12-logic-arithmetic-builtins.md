# LP12 - Logic Arithmetic Builtins

## Overview

LP11 added the first practical Prolog-style builtins for control and term
inspection. The next functionality-level gap is arithmetic.

Classic Prolog arithmetic is intentionally not the same thing as unification:

- `X = 1 + 2` says `X` is the symbolic expression `1 + 2`.
- `X is 1 + 2` evaluates the expression and binds `X` to `3`.
- `1 + 2 =:= 3` compares evaluated numeric values.

The Python library should preserve that distinction. Syntax can come later; for
now, arithmetic should be ordinary Python data and goal expressions that compose
with the existing solver.

## Design Goal

Extend the Python `logic-builtins` package with a small, reusable arithmetic
layer:

- expression constructors
- an `is/2`-style evaluation goal
- numeric comparison goals

This milestone deliberately does not introduce constraint logic programming.
Arithmetic is evaluative and directional, like ordinary Prolog arithmetic
builtins. A later CLP(FD) milestone can add bidirectional finite-domain
constraints.

## Package

Update:

```text
code/packages/python/logic-builtins
```

The package version should move from `0.1.0` to `0.2.0`.

## Public API

Add expression constructors:

```python
add(left, right)
sub(left, right)
mul(left, right)
div(left, right)
floordiv(left, right)
mod(left, right)
neg(value)
```

Add evaluative goals:

```python
iso(result, expression)
numeqo(left, right)
numneqo(left, right)
lto(left, right)
leqo(left, right)
gto(left, right)
geqo(left, right)
```

The names are Python-friendly spellings of Prolog concepts:

- `iso` represents `is/2`
- `numeqo` represents `=:=/2`
- `numneqo` represents `=\=/2`
- `lto`, `leqo`, `gto`, and `geqo` represent `<`, `=<`, `>`, and `>=`

The `o` suffix continues the package convention that public predicates return
logic goals.

## Arithmetic Expressions

Arithmetic expressions should be represented as normal logic terms:

```python
add(1, 2)          # +(1, 2)
mul(add(1, 2), 3)  # *(+(1, 2), 3)
neg(4)             # -(4)
```

These constructors should return `Compound` terms whose functors are the
familiar arithmetic operator symbols.

Supported operators for LP12:

| Constructor | Functor | Meaning |
| --- | --- | --- |
| `add(a, b)` | `+` | addition |
| `sub(a, b)` | `-` | subtraction |
| `mul(a, b)` | `*` | multiplication |
| `div(a, b)` | `/` | true division |
| `floordiv(a, b)` | `//` | floor division |
| `mod(a, b)` | `mod` | modulo |
| `neg(a)` | `-` | unary negation |

## Evaluation Semantics

Arithmetic evaluation should happen through the active substitution:

1. Reify the expression under the current state.
2. Recursively evaluate numeric terms and arithmetic compounds.
3. If evaluation succeeds, unify the result with the target.
4. If evaluation cannot proceed, fail logically instead of raising for normal
   predicate failure.

Examples:

```python
X = var("X")
solve_all(program(), X, iso(X, add(1, 2))) == [num(3)]
```

```python
X = var("X")
Y = var("Y")
solve_all(program(), Y, conj(eq(X, 4), iso(Y, mul(X, 2)))) == [num(8)]
```

## Directionality

LP12 arithmetic is intentionally directional.

This should succeed:

```python
conj(eq(X, 4), iso(Y, add(X, 1)))
```

This should fail because `X` is not instantiated enough to evaluate:

```python
iso(Y, add(X, 1))
```

This is the ordinary Prolog `is/2` model and keeps the implementation simple
until a real CLP milestone exists.

## Numeric Comparisons

Numeric comparisons should evaluate both sides before comparing:

```python
lto(add(1, 2), 4)
numeqo(mul(2, 3), 6)
numneqo(add(1, 1), 3)
```

If either side cannot be evaluated to a number, the comparison fails.

## Error Model

Arithmetic builtins should prefer logical failure for ordinary runtime
predicate failure:

- unbound variables inside arithmetic expressions fail
- non-numeric terms inside arithmetic expressions fail
- unknown arithmetic functors fail
- division by zero fails

Python exceptions remain appropriate for host-language API misuse caught by the
underlying term constructors, such as attempting to coerce unsupported Python
objects into logic terms.

## Test Strategy

Required tests:

- arithmetic constructors produce ordinary compound terms
- `iso` evaluates integer and floating-point expressions
- `iso` uses current logic variable bindings
- `iso` fails when an expression is insufficiently instantiated
- `iso` fails for non-numeric terms
- division by zero fails logically
- every numeric comparison succeeds and fails in representative cases
- arithmetic goals compose with relation search
- arithmetic goals compose with `onceo` and `noto`

## Future Extensions

Later arithmetic-related milestones can add:

- symbolic simplification and rewriting
- richer numeric functions such as `abs`, `min`, `max`, and powers
- type-preserving rational arithmetic
- CLP(FD)-style bidirectional finite-domain constraints
- VM opcodes for arithmetic evaluation and comparison

## Summary

LP12 gives the library-first Prolog stack evaluative arithmetic without adding
language syntax. It makes the Python API capable of real numeric guards,
derived values, and score/range-style logic while keeping symbolic unification
and arithmetic evaluation cleanly separate.
