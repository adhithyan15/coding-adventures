# Lua in-memory data store protocol

A dependency-free protocol intermediate representation shared by in-memory data
store engines and transport adapters. It provides normalized command frames over
Lua byte strings and typed engine responses for strings, errors, integers, bulk
strings, and nested arrays.

```lua
local protocol = require("coding_adventures.in_memory_data_store_protocol")

local frame = protocol.CommandFrame.from_parts({"set", "key", "value"})
assert(frame.command == "SET")

local response = protocol.EngineResponse.array({
    protocol.EngineResponse.ok(),
    protocol.EngineResponse.integer(1),
})
```

Lua strings are immutable byte strings, so frame arguments and bulk-string
payloads preserve the reference protocol's byte-oriented semantics. This package
models the engine boundary only; RESP parsing and encoding remain separate.

## Development

```bash
bash BUILD
```
