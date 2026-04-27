# cas-list-operations

Standard list-manipulation primitives for the symbolic IR. Backfills
the universal functional-programming primitives onto the existing
``List`` head, which `symbolic-vm` currently treats as a passthrough.

## Operations

| Function          | Behavior                                          |
|-------------------|---------------------------------------------------|
| ``length(lst)``   | Number of elements.                               |
| ``first(lst)``    | First element.                                    |
| ``rest(lst)``     | All but the first element (returns a List).       |
| ``last(lst)``     | Last element.                                     |
| ``reverse(lst)``  | Reverse order.                                    |
| ``range_(...)``   | Generate ``[1..n]``, ``[a..b]``, ``[a..b step s]`` (1-based per MACSYMA). |
| ``map_(f, lst)``  | Apply f to each element. ``f`` is an ``IRSymbol`` or compound head — invocation is via ``IRApply``. |
| ``apply_(f, lst)``| Replace head: ``Apply(Add, [a, b, c])`` → ``Add(a, b, c)``. |
| ``select(lst, p)``| Keep elements where p(elem) returns ``True``.     |
| ``sort_(lst)``    | Stable sort by canonical IR key.                  |
| ``part(lst, n)``  | 1-based indexed access.                           |
| ``flatten(lst, depth=1)`` | Flatten nested lists by one level (or all if depth=∞). |
| ``join(*lsts)``   | Concatenate multiple lists.                       |

## Heads

The package introduces sentinel heads (``LENGTH``, ``FIRST``, ``REST``, …)
so backends can install handlers for them. The functions above accept
raw ``IRApply`` lists directly and don't require backend integration.

## Reuse story

These are universal across CAS frontends — Maxima, Mathematica, Maple,
SymPy all expose the same set under different surface names. New
language frontends just add their surface name to their runtime's
name table.

## Dependencies

- `coding-adventures-symbolic-ir`
