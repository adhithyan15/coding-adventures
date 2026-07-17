# bloom-filter (Lua)

A dependency-light Lua 5.4 Bloom filter for space-efficient probabilistic set
membership. Inserted values never produce false negatives; positive lookups may
be false positives at the configured rate.

```lua
local bloom_filter = require("coding_adventures.bloom_filter")

local filter = bloom_filter.new(1000, 0.01)
filter:add("hello")

assert(filter:contains("hello"))
print(filter:fill_ratio())
print(filter:estimated_false_positive_rate())
```

The filter computes its bit count and hash count from the expected number of
items and target false-positive rate. `from_params(bit_count, hash_count)` is
available for explicit layouts. String inputs are binary-safe; scalar and table
values use a deterministic encoding, including stable map-key ordering.

Double hashing derives every probe from FNV-1a and DJB2 supplied by the sibling
`hash-functions` package. MurmurHash3 finalization removes correlation between
the two base hashes, and the probe step is forced odd for good coverage.

## Development

```bash
luarocks make --local --deps-mode=none coding-adventures-bloom-filter-0.1.0-1.rockspec
cd tests
LUA_PATH="../src/?.lua;../src/?/init.lua;../../hash-functions/src/?.lua;../../hash-functions/src/?/init.lua;;" busted . --verbose --pattern=test_
```
