# Lua tree set

An AVL-backed mutable ordered set with duplicate suppression, sorted iteration,
rank and selection helpers, range queries, custom comparison, and set algebra.
The AVL backend keeps insertion, deletion, and lookup logarithmic while set
operations return independent sets.

```lua
local TreeSet = require("coding_adventures.tree_set").TreeSet

local set = TreeSet.from_values({5, 1, 3, 3, 9})
set:add(7)
assert(set:contains(7))
assert(set:kth_smallest(3) == 5)
assert(set:backend():is_valid_avl())
```

`add` returns the set for chaining. `delete`, `remove`, and `discard` mutate the
set and report whether a value was present. Union, intersection, difference,
and symmetric difference leave both inputs unchanged.

## Development

```bash
bash BUILD
```
