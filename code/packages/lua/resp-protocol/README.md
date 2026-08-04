# coding-adventures-resp-protocol

A dependency-free Lua 5.4 encoder and incremental decoder for the Redis
Serialization Protocol (RESP2). The package preserves simple strings, errors,
integers, bulk strings, arrays, and the distinct null bulk/array wire types.

```lua
local resp = require("coding_adventures.resp_protocol")

local command = resp.Value.array({
    resp.Value.bulk_string("SET"),
    resp.Value.bulk_string("key"),
    resp.Value.bulk_string("value"),
})

local wire = resp.encode(command)
local decoded, next_offset = resp.decode(wire)
assert(resp.equal(command, decoded))
assert(next_offset == #wire + 1)
```

Bulk strings are binary-safe. `decode` returns `nil` without consuming bytes
for incomplete input, while malformed frames raise an error. `Decoder` handles
arbitrary stream fragmentation and multiple messages per chunk.
