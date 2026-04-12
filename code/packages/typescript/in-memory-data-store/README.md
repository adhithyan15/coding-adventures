# @coding-adventures/in-memory-data-store

Browser-safe composition layer for the in-memory data store stack.

This package wires together:

- `@coding-adventures/resp-protocol`
- `@coding-adventures/in-memory-data-store-protocol`
- `@coding-adventures/in-memory-data-store-engine`

It provides a small, transport-agnostic pipeline that can:

- decode RESP frames
- translate them into engine commands
- execute them against the pluggable engine
- encode responses back to RESP bytes

The package is intentionally free of Node-only APIs so it can run in both
Node and browser environments.
