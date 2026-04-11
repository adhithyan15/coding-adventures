# in-memory-data-store-engine

Pluggable execution engine for the in-memory data store stack.

This crate owns the registry-driven command execution layer, backing storage
types, TTL handling, AOF replay, and the VM-style registration seam that lets
new command families or features plug in without rewiring the transport.
