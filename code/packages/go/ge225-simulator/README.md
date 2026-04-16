# GE-225 Simulator (Go)

Behavioral Go simulator for the **GE-225 instruction repertoire**.

This package is the first non-Python GE-225 port in the repo and is intended to
track the Python simulator closely enough for backend cross-checking.

## Scope

- 20-bit word-addressed machine state
- documented memory-reference and fixed-word instruction families
- host-side control-switch, typewriter, and queued record helpers
- focused tests covering arithmetic, branching, block move, and console flow
