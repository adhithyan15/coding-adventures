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
- `listo(...)`
- `membero(...)`
- `appendo(...)`
- `selecto(...)`
- `permuteo(...)`
- `reverseo(...)`

## Quick Start

```python
from logic_engine import atom, conj, eq, logic_list, program, solve_all, solve_n, var
from logic_stdlib import appendo, listo, membero, permuteo, reverseo

X = var("X")
Prefix = var("Prefix")
Suffix = var("Suffix")
Order = var("Order")

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

assert solve_n(
    program(),
    2,
    Order,
    permuteo(logic_list(["tea", "cake", "jam"]), Order),
) == [
    logic_list(["tea", "cake", "jam"]),
    logic_list(["tea", "jam", "cake"]),
]

assert solve_all(
    program(),
    Order,
    reverseo(logic_list(["tea", "cake", "jam"]), Order),
) == [logic_list(["jam", "cake", "tea"])]

assert solve_all(
    program(),
    Order,
    conj(eq(Order, logic_list(["tea", "cake"])), listo(Order)),
) == [logic_list(["tea", "cake"])]
```

## Dependencies

- [`logic-engine`](../logic-engine/)

## Development

```bash
bash BUILD
```
