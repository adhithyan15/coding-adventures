# logic-builtins

`logic-builtins` adds the first practical Prolog-inspired control and term
inspection predicates to the Python logic stack.

These functions are library goals, not syntax. They compose with
`logic-engine`, `logic-stdlib`, and the VM/bytecode layers because they return
ordinary logic goal expressions.

## What It Adds

- `callo(goal)`
- `onceo(goal)`
- `noto(goal)` for negation as failure
- `groundo(term)`
- `varo(term)` and `nonvaro(term)`
- `atomo(term)`, `numbero(term)`, `stringo(term)`, and `compoundo(term)`
- `functoro(term, name, arity)`
- `argo(index, term, value)`

## Quick Start

```python
from logic_builtins import argo, functoro, groundo, noto, onceo
from logic_engine import atom, conj, eq, fail, num, program, solve_all, term, var

X = var("X")
Name = var("Name")
Arity = var("Arity")
Arg = var("Arg")

assert solve_all(program(), X, onceo(eq(X, "first"))) == [atom("first")]
assert solve_all(program(), X, noto(fail())) == [X]
assert solve_all(program(), X, conj(eq(X, term("box", "tea")), groundo(X))) == [
    term("box", "tea"),
]
assert solve_all(
    program(),
    (Name, Arity, Arg),
    conj(
        functoro(term("box", "tea"), Name, Arity),
        argo(1, term("box", "tea"), Arg),
    ),
) == [(atom("box"), num(1), atom("tea"))]
```

## Dependencies

- logic-engine

## Development

```bash
bash BUILD
```
