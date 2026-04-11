# mini-redis

Rust port of DT25 Mini-Redis.

This crate is the single-node Redis baseline for the DT series:

- RESP2/TCP server
- in-memory strings, hashes, lists, sets, sorted sets, and HyperLogLog
- TTLs, database selection, flushing, key inspection, and AOF replay
- designed to lean on the repo's own packages where possible

What is still missing from real Redis:

- transactions: `MULTI`, `EXEC`, `WATCH`, `UNWATCH`
- blocking list operations
- pub/sub
- scripting
- streams
- `SCAN`-family iteration
- replication and failover
- cluster mode
- eviction policies and memory management
- RESP3, ACLs, modules, and other operational surfaces
