# logic-core

`logic-core` is the Python-first relational programming prototype for the
Prolog work in this repo.

It deliberately starts with the reusable semantic pieces:

- atoms, numbers, strings, variables, and compound terms
- substitutions and unification
- goals and backtracking search
- helpers like `fresh`, `conj`, `disj`, `run_all`, and `run_n`

The later Prolog implementation should lower parsed clauses and queries into
this API instead of inventing a second execution model.

## Quick Start

```python
from logic_core import atom, disj, eq, run_all, term, var

X = var("X")

answers = run_all(
    X,
    disj(
        eq(term("parent", X, atom("bart")), term("parent", atom("homer"), atom("bart"))),
        eq(X, atom("marge")),
    ),
)

assert answers == [atom("homer"), atom("marge")]
```

## Dependencies

- [`symbol-core`](../symbol-core/)

## Development

```bash
bash BUILD
```
