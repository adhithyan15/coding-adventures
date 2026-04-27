# simulator-protocol

Shared simulator contract for TypeScript packages in this repo.

It defines:

- `Simulator<StateT, TraceT>` for structural conformance
- `StepTrace` for normalized execution traces
- `ExecutionResult<StateT>` for end-to-end run results

The protocol is intentionally generic so architecture-specific simulators can keep
their richer step traces while still exposing a uniform `execute()` and
`getState()` surface for compiler pipelines and tests.
