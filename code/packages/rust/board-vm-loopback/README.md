# board-vm-loopback

In-memory Board VM endpoint for exercising host/protocol/runtime behavior.

This crate is not a simulator for a real MCU. It is a deterministic fake board
that accepts BVM01 raw frames, stores uploaded BVM02 modules, executes them
through `board-vm-runtime`, and records GPIO/time effects for tests and future
SDK conformance fixtures.
