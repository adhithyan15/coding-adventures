# @coding-adventures/in-memory-data-store-engine

Pluggable in-memory datastore engine built on top of the TypeScript data
structure packages.

This package is the execution core:

- keyspace storage
- TTL management
- hashes, lists, sets, sorted sets, and HyperLogLog
- command registration and dispatch

The protocol layer can translate RESP frames into commands, and a transport
layer can call into the engine without needing to know anything about the
underlying data structures.
