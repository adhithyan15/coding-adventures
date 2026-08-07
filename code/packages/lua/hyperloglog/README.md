# hyperloglog

Pure Lua HyperLogLog sketch for approximate distinct counting with fixed
memory. The implementation uses deterministic internal FNV-1a hashes and has
no non-Lua runtime dependencies.

## Usage

~~~lua
local HyperLogLog = require("coding_adventures.hyperloglog").HyperLogLog

local sketch = HyperLogLog.new(10)
for value = 1, 10000 do
    sketch:add("user-" .. value)
end
print(sketch:count())
~~~

Precision may range from 4 to 16 and defaults to 10. The sketch supports
add, count, non-mutating merge, merge_in_place, clear, is_empty, and accessors
for precision, register count, theoretical error rate, packed memory size, and
a defensive register snapshot.

## Tests

~~~sh
cd code/packages/lua/hyperloglog
luarocks make --local --deps-mode=none coding-adventures-hyperloglog-0.1.0-1.rockspec
cd tests
LUA_PATH="../src/?.lua;../src/?/init.lua;;" busted . --verbose --pattern=test_
~~~
