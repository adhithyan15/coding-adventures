# Backend Parity Fixture V1

This directory is the deterministic, language-neutral NN31 corpus. It proves
that one dense layer keeps the same meaning when its arithmetic moves between
a scalar CPU interpreter, a TypeScript matrix backend, the Rust matrix core,
and an optional accelerated backend.

## Contents

- [`schema.json`](./schema.json) describes the closed lab JSON shape.
- [`labs/00-dense-batch.json`](./labs/00-dense-batch.json) pins the equation,
  four execution lanes, buffer residency, and the expected intermediates.
- [`matrix-ir/00-dense-batch.graph.json`](./matrix-ir/00-dense-batch.graph.json)
  is the canonical MatrixIR JSON consumed by the Rust core.
- [`payloads/00-input-x.f32le.hex`](./payloads/00-input-x.f32le.hex) and
  [`payloads/00-expected-output.f32le.hex`](./payloads/00-expected-output.f32le.hex)
  are lowercase-hex, little-endian `f32` buffers shared by every consumer.
- [`CHANGELOG.md`](./CHANGELOG.md) records versioned changes.

## Validation

From the repository root:

```text
python code/scripts/validate_backend_parity_labs.py
pytest code/scripts/tests/test_backend_parity_labs.py -q
cargo test -p matrix-cpu --test backend_parity_fixture
```

The Python validator checks the closed schema, exact MatrixIR graph, bounded
payload references, hand calculation, binary32 rounding, lane roster, and
expected trace. The Rust test calls the Node-free execution helper exported by
`matrix-cpu` and re-exported at the N-API binding edge. It decodes the checked-
in MatrixIR and compares returned bytes with the same expected payload without
linking Node host symbols into the test binary.

## Consumer Contract

Consumers must preserve row-major shapes, little-endian `f32` bytes, canonical
lane IDs, and explicit residency events. An unavailable accelerator is a valid
runtime result; relabeling CPU arithmetic as GPU execution is not.
