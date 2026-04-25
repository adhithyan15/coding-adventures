# cas-list-operations — List Manipulation Heads

> **Status**: New spec. Adds the standard list-manipulation operations
> (`Map`, `Length`, `First`, `Rest`, `Append`, `Range`, `Apply`,
> `Reverse`, `Sort`, `Select`).
> Parent: `symbolic-computation.md`.

## Why this package exists

`symbolic-vm` already treats `List` as a passthrough head. That means
list values exist but you can't *do* anything with them. Every CAS
ships a small standard library of list operations; this package
provides them.

## Reuse story

These are the universal functional-programming primitives. Maxima's
`first`, `rest`, `map`, `length`, `makelist`; Mathematica's `First`,
`Rest`, `Map`, `Length`, `Range`, `Table`; SymPy's same set;
Matlab's analogues for cell arrays. Same heads, same semantics.

## Scope

In:

- `Length(list)` — number of elements.
- `First(list)`, `Rest(list)`, `Last(list)` — head/tail/last.
- `Append(a, b)` — concatenation.
- `Reverse(list)` — reverse the order.
- `Range(n)` / `Range(a, b)` / `Range(a, b, step)` — generate.
- `Map(f, list)` — apply a function to each element.
- `Apply(f, list)` — replace head: `Apply(Add, [a, b, c])` → `Add(a, b, c)`.
- `Select(list, predicate)` — keep elements where predicate is true.
- `Sort(list)` — canonical-order sort (uses the `cas-pretty-printer`
  ordering or a backend hook).
- `Part(list, n)` / `list[[n]]` — index access (1-based, MACSYMA convention).
- `Flatten(list)` — flatten nested lists by one level (or all levels
  if a depth arg is provided).
- `Join(list1, list2, ...)` — concatenation of multiple lists.

Out:

- Associative-array primitives (`Hash`/`Dict`) — separate package.
- Matrix-as-list-of-lists — that's `cas-matrix`.
- Set operations (`Union`, `Intersection`) — future package.

## Public interface

```python
from cas_list_operations import register_handlers

# Length(List(1, 2, 3)) → 3
# First(List(1, 2, 3)) → 1
# Map(Sin, List(0, Pi/2, Pi)) → List(0, 1, 0)  (after simplify)
# Range(5) → List(1, 2, 3, 4, 5)              (1-based, MACSYMA)
# Range(0, 5) → List(0, 1, 2, 3, 4, 5)
```

## Heads added

| Head      | Arity | Meaning                                     |
|-----------|-------|---------------------------------------------|
| `Length`  | 1     | Length of a list.                           |
| `First`   | 1     | First element.                              |
| `Rest`    | 1     | All but first.                              |
| `Last`    | 1     | Last element.                               |
| `Append`  | 2+    | Concatenate.                                |
| `Reverse` | 1     | Reverse.                                    |
| `Range`   | 1–3   | Numeric range.                              |
| `Map`     | 2     | Map function over list.                     |
| `Apply`   | 2     | Replace head.                               |
| `Select`  | 2     | Filter by predicate.                        |
| `Sort`    | 1     | Canonical sort.                             |
| `Part`    | 2     | Indexed access.                             |
| `Flatten` | 1–2   | Flatten nested lists.                       |
| `Join`    | 2+    | Concatenate multiple lists.                 |

## Test strategy

- All operations on simple integer lists.
- Map with both built-in functions (`Sin`) and user-defined functions
  via `Define`.
- `Range` corner cases: `Range(0)` → `[]`, `Range(1, 1)` → `[1]`.
- `Apply(Add, [1, 2, 3])` → `Add(1, 2, 3)` → `6` (after simplify).
- `Select([1, 2, 3, 4], EvenQ)` → `[2, 4]`.
- 1-based vs 0-based indexing (MACSYMA convention).
- Coverage: ≥90%.

## Package layout

```
code/packages/python/cas-list-operations/
  src/cas_list_operations/
    __init__.py
    basics.py        # Length, First, Rest, Last, Reverse
    range.py
    map_apply.py
    select.py
    sort.py
    part.py
    flatten_join.py
    py.typed
  tests/
    test_basics.py
    test_range.py
    test_map_apply.py
    ...
```

Dependencies: `coding-adventures-symbolic-ir`,
`coding-adventures-symbolic-vm`.
