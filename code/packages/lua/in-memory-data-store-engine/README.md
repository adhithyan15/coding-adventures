# In-Memory Data Store Engine (Lua)

A pure Lua 5.4 execution engine for the repository's in-memory data store
stack. It consumes `CommandFrame` values from the sibling protocol package and
returns the shared `EngineResponse` IR.

```lua
local engine = require("coding_adventures.in_memory_data_store_engine").new()

engine:execute_parts({"SET", "answer", "41"})
assert(engine:execute_parts({"INCR", "answer"}).value == 42)
```

The engine implements binary-safe strings, hashes, lists, sets, sorted sets,
HyperLogLog, expiry and persistence, globbed key lookup, 16 logical databases,
and administrative commands. Its 57-command surface includes the Redis-style
string, hash, list, set, sorted-set, HLL, TTL, database, and server operations
implemented by the other language lanes.

`DataStoreEngine.new` accepts optional `store`, `database_count`, and
`time_provider` fields. The clock hook makes TTL behavior deterministic in
tests without filesystem, network, process, environment, or randomness access.

Run the package gate from this directory with `BUILD` or `BUILD_windows`.
