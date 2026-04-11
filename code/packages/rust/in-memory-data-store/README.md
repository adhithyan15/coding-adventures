# in-memory-data-store

Composable in-memory data store server in Rust.

This crate is the current single-node baseline for the DT series:

- RESP2/TCP server
- in-memory strings, hashes, lists, sets, sorted sets, and HyperLogLog
- TTLs, database selection, flushing, key inspection, and AOF replay
- designed to lean on the repo's own packages where possible

What is still missing from a broader in-memory data platform:

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
