# logic-stdlib

`logic-stdlib` is the first relational standard library built on top of
[`logic-engine`](../logic-engine/).

It does not add a new evaluator. It adds reusable relational vocabulary so
users can solve list-shaped problems directly in Python without hand-writing the
same recursive helper relations every time.

The first slice includes:

- `emptyo(...)`
- `conso(...)`
- `heado(...)`
- `tailo(...)`
- `membero(...)`
- `appendo(...)`

## Quick Start

```python
from logic_engine import atom, logic_list, program, solve_all, solve_n, var
from logic_stdlib import appendo, membero

X = var("X")
Prefix = var("Prefix")
Suffix = var("Suffix")

assert solve_all(program(), X, membero(X, logic_list(["tea", "cake"]))) == [
    atom("tea"),
    atom("cake"),
]

answers = solve_n(
    program(),
    3,
    (Prefix, Suffix),
    appendo(Prefix, Suffix, logic_list(["tea", "cake"])),
)

assert answers == [
    (logic_list([]), logic_list(["tea", "cake"])),
    (logic_list(["tea"]), logic_list(["cake"])),
    (logic_list(["tea", "cake"]), logic_list([])),
]
```

## Dependencies

- [`logic-engine`](../logic-engine/)

## Development

```bash
bash BUILD
```
