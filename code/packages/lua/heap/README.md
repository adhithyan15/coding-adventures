# coding-adventures-heap (Lua)

Comparator-based binary heap package for Lua.

## Usage

```lua
local heap = require("coding_adventures.heap")

local min_heap = heap.MinHeap.new()
min_heap:push(5)
min_heap:push(1)
min_heap:push(3)
print(min_heap:pop()) -- 1

local max_heap = heap.MaxHeap.new()
max_heap:push(5)
max_heap:push(1)
max_heap:push(3)
print(max_heap:pop()) -- 5

local tuple_heap = heap.MinHeap.new(function(left, right)
    if left[1] == right[1] then
        if left[2] == right[2] then
            return 0
        end
        return left[2] < right[2] and -1 or 1
    end
    return left[1] < right[1] and -1 or 1
end)
tuple_heap:push({1, "b"})
tuple_heap:push({1, "a"})
print(tuple_heap:pop()[2]) -- "a"
```

## API

- `heap.MinHeap.new(compare)` creates an empty min-heap.
- `heap.MaxHeap.new(compare)` creates an empty max-heap.
- `from_iterable(items, compare)` heapifies a Lua array.
- `push(value)` inserts a value.
- `pop()` removes and returns the root, or `nil` when empty.
- `peek()` returns the root, or `nil` when empty.
- `len()` and `size()` return the number of elements.
- `is_empty()` reports whether the heap is empty.
- `to_array()` returns a shallow copy of the underlying array-backed heap.

## Running Tests

```bash
luarocks make --local --deps-mode=none coding-adventures-heap-0.1.0-1.rockspec
cd tests && LUA_PATH="../src/?.lua;../src/?/init.lua;;" busted . --verbose --pattern=test_
```
