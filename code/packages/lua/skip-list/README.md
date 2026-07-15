# skip-list

Pure Lua probabilistic ordered map with expected O(log n) insertion, deletion,
lookup, rank, and selection. Forward pointers carry spans, so `rank` and
`by_rank` navigate the skip-list tower instead of scanning the bottom level.

## Usage

```lua
local SkipList = require("coding_adventures.skip_list").SkipList

local list = SkipList.new()
list:insert(5, "five")
list:insert(2, "two")
list:insert(8, "eight")

assert(list:search(5) == "five")
assert(list:rank(5) == 1)
assert(list:by_rank(0) == 2)
```

`SkipList.new(max_level, probability, compare, seed)` defaults to 16 levels,
a 0.5 promotion probability, natural ordering, and a deterministic local seed.
The local Park-Miller generator makes the topology reproducible without
changing Lua's global random state.

## API

- `insert(key, value)` inserts a key or updates its value and reports whether
  the key was new.
- `delete(key)` / `remove(key)` removes a key and reports whether it existed.
- `search(key)` / `get(key)` returns the stored value; `contains(key)` handles
  keys whose stored value is `nil`.
- `rank(key)` and `by_rank(rank)` use zero-based ranks.
- `kth_smallest(k)` uses one-based selection.
- `range_query(minimum, maximum, inclusive)` returns sorted `{key, value}`
  pairs. Bounds are inclusive by default.
- `to_list()`, `entries()`, `iterator()`, `min()`, `max()`, `size()`, and
  `is_empty()` expose ordered-map state.
- `is_valid_skip_list()` checks ordering, height, span, and size invariants.

The package has no non-Lua runtime dependencies and needs no external
capabilities.

## Tests

```sh
cd code/packages/lua/skip-list
luarocks make --local --deps-mode=none coding-adventures-skip-list-0.1.0-1.rockspec
cd tests
LUA_PATH="../src/?.lua;../src/?/init.lua;;" busted . --verbose --pattern=test_
```
