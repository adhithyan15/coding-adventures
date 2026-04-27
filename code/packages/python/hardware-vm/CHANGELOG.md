# Changelog

## [0.1.0] — Unreleased

Initial implementation of the event-driven simulator. Combinational scope; covers the canonical 4-bit adder smoke test end-to-end.

### Added
- `HardwareVM`: top-level simulator that initializes from an HIR document, drives inputs via `set_input`, reads via `read`, supports `force`/`release`, and emits `Event(time, signal, new_value, old_value)` to subscribers.
- `evaluate(expr, lookup)`: HIR expression evaluator handling Lit, NetRef/PortRef/VarRef, Slice, Concat, Replication, UnaryOp, BinaryOp, Ternary. Bitwise + arithmetic + shift + comparison + logical ops all implemented.
- `referenced_signals(expr)`: dependency analysis for sensitivity inference on continuous assignments.
- Sensitivity tables: each ContAssign indexes by which signals it reads; updates propagate when those signals change.
- 4-bit adder runs end-to-end: 5+3 → sum=8 cout=0; 15+1 → sum=0 cout=1.

### Out of scope (v0.2.0)
- Behavioral processes (always/initial/process).
- `wait`, `@`, `#delay`.
- 9-state StdLogic resolution.
- Clocked sequential simulation (no delta cycles past 0).
- Multi-driver tristate resolution.

### Quality
- 70% test-coverage gate.
- ruff clean.
