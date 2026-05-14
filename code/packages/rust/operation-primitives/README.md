# operation-primitives

Rust port of the operation envelope generated for Go capability-cage packages.

The crate provides:

- `OperationResult<T>`: success, expected failure, or unexpected failure plus a value.
- `ResultFactory<T>`: helpers for producing operation results inside callbacks.
- `Operation<T, F>`: named callback wrapper with a property bag and panic capture.
- `start_new(...)`: constructor matching the Go `StartNew` shape.

Use this crate when a package wants one uniform envelope for capability checks,
host calls, sandbox planning, or generated code.
