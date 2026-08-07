# Hash Map (Lua)

A pure Lua 5.4 implementation of [DT18](../../../specs/DT18-hash-map.md).
It builds its own table storage with either separate chaining or linear-probing
open addressing, including tombstone deletion and automatic resizing. Bucket
selection uses the sibling `hash-functions` package.

```lua
local hash_map = require("coding_adventures.hash_map")

local map = hash_map.new(16, "open_addressing", "fnv1a")
map:set("language", "Lua")
assert(map:get("language") == "Lua")

-- Functional wrappers clone before writes.
local next_map = hash_map.set(map, "year", 1993)
assert(not map:has("year"))
assert(next_map:get("year") == 1993)
```

Run the package gate from this directory with the commands in `BUILD` or
`BUILD_windows`.
