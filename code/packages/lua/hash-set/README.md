# Hash Set (Lua)

A pure Lua 5.4 implementation of [DT19](../../../specs/DT19-hash-set.md).
It wraps the sibling DT18 `hash-map` package and stores elements as keys with a
single sentinel value. Add and remove operations are persistent: they return a
new set and leave the input unchanged.

```lua
local hash_set = require("coding_adventures.hash_set")

local base = hash_set.from_list({ "Ada", "Grace", "Ada" })
local next_set = base:add("Linus")

assert(base:size() == 2)
assert(next_set:contains("Linus"))
assert(next_set:intersection(base):equals(base))
```

The package includes union, intersection, difference, symmetric difference,
subset, superset, disjoint, and equality operations. Hash-map capacity,
collision strategy, and hash-function options are preserved across persistent
operations.

Run the package gate from this directory with the commands in `BUILD` or
`BUILD_windows`.
