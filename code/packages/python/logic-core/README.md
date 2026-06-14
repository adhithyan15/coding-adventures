# logic-core

`logic-core` is the Python-first relational programming prototype for the
Prolog work in this repo.

It deliberately starts with the reusable semantic pieces:

- atoms, numbers, strings, variables, and compound terms
- substitutions and unification
- goals and backtracking search
- delayed disequality constraints via `neq(...)`
- extension slots on `State` for higher layers such as runtime databases and
  finite-domain constraint stores
- helpers like `fresh`, `conj`, `disj`, `run_all`, and `run_n`

The later Prolog implementation should lower parsed clauses and queries into
this API instead of inventing a second execution model.

## Quick Start

```python
from logic_core import State, atom, conj, eq, neq, run_all, var

X = var("X")

assert run_all(
    X,
    conj(
        neq(X, atom("homer")),
        eq(X, atom("marge")),
    ),
) == [atom("marge")]

state = list(neq(X, atom("homer"))(State()))[0]
assert state.constraints
assert State(database={"branch": "local"}).database == {"branch": "local"}
assert State(fd_store={"domains": "local"}).fd_store == {"domains": "local"}
assert State(prolog_flags={"unknown": "fail"}).prolog_flags == {"unknown": "fail"}
```

## Dependencies

- [`symbol-core`](../symbol-core/)

## Development

```bash
bash BUILD
```
