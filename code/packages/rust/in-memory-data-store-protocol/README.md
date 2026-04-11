# in-memory-data-store-protocol

RESP-to-engine protocol adapter for the in-memory data store stack.

This crate translates RESP values into typed command frames that the engine
can execute. It is intentionally transport-free so it can be reused by the TCP
server, future WebSocket adapters, WASM builds, and test harnesses.
