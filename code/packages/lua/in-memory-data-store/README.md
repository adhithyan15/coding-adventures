# in-memory-data-store (Lua)

A pure Lua 5.4 facade that composes the RESP2 streaming codec, command protocol
IR, and in-memory data store engine. It accepts fragmented or pipelined byte
streams, preserves binary-safe bulk strings, and returns native RESP values or
encoded response streams without opening sockets or using external services.

```lua
local store = require("coding_adventures.in_memory_data_store").new()

local response = store:execute_parts({ "SET", "name", "Ada" })
assert(response.value == "OK")

local wire = store:handle("*2\r\n$3\r\nGET\r\n$4\r\nname\r\n")
assert(wire == "$3\r\nAda\r\n")
```

The facade depends only on the sibling pure-Lua `resp-protocol`,
`in-memory-data-store-protocol`, and `in-memory-data-store-engine` packages.
