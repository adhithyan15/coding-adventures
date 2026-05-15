# operation-primitives

Rust port of the operation envelope generated for Go capability-cage packages.

The crate provides:

- `OperationResult<T>`: success, expected failure, or unexpected failure plus a value.
- `ResultFactory<T>`: helpers for producing operation results inside callbacks.
- `Operation<T, F>`: named callback wrapper with a property bag and panic capture.
- `start_new(...)`: constructor matching the Go `StartNew` shape.
- `OperationHttpClient`: operation-side HTTP preflight wrapper fed by generated
  code from `required_capabilities.json`; it refuses undeclared HTTPS domains
  before transport callbacks can run.

Use this crate when a package wants one uniform envelope for capability checks,
host calls, sandbox planning, generated code, or host-side HTTP operations.
