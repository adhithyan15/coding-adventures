# NN31: CPU, Rust Core, and Accelerated Backend Parity Labs

## Status

Version 1 is implemented by the language-neutral fixture under
[`fixtures/backend-parity-v1`](./fixtures/backend-parity-v1/README.md).

## Purpose

NN31 teaches the boundary between a model's meaning and the engine that performs
its arithmetic. The first lab keeps one dense layer fixed while four execution
lanes expose different schedules, numeric formats, and buffer locations.

The correctness rule is:

```text
same graph + same input + declared precision -> outputs within tolerance
```

A backend name is not execution evidence. Required lanes run in deterministic
tests. Optional hardware lanes report `executed`, `unavailable`, or `failed`
after an actual runtime probe.

## Canonical Example

The V1 fixture uses a three-row, one-feature dense layer:

```text
X = [[1], [2], [3]]
W = [[2]]
B = [[1], [1], [1]]

XW = [[2], [4], [6]]
y  = [[3], [5], [7]]
```

Every number is exactly representable in binary32, so the parity tolerance is
not hiding a rounding difference. Later precision labs deliberately introduce
values that are not exact.

## Required Lanes

| Lane | Program | Arithmetic | Required evidence |
| --- | --- | --- | --- |
| Scalar CPU | NN00 bytecode | JavaScript binary64 | production interpreter trace |
| TypeScript matrix CPU | NN01 CANM | JavaScript binary64 | production matrix-plan trace |
| Rust matrix CPU | MatrixIR JSON | IEEE-754 binary32 | `matrix-cpu` output bytes |
| WebGPU | NN01 async backend | IEEE-754 binary32 | actual browser probe when available |

The Rust lane uses the canonical MatrixIR graph and raw little-endian payloads
under `code/specs/fixtures/backend-parity-v1`. The browser must not bundle or
pretend to load the native Node addon. It displays fixture-backed native-test
evidence and separately probes its own WebGPU backend.

## Residency Trace

Each lane lists where live values reside. CPU lanes keep values on the host.
The Rust lane crosses an ABI boundary as byte buffers. The accelerator lane
uploads the input, keeps intermediates on device, and first downloads the final
output. The current educational runner then downloads `x`, `bias`, and
`y` again to populate its visible value trace. Those extra reads are recorded,
not hidden; removing them belongs to the later residency/performance tranche.

This is a semantic trace, not a timing benchmark. Transfer counts and device
placement are visible because a numerically correct accelerator can still be
slower if it moves tiny buffers unnecessarily.

## Fixture Contract

V1 is the closed directory
`code/specs/fixtures/backend-parity-v1`:

- one strict lab JSON document;
- one MatrixIR JSON graph;
- one input and one expected-output `f32le` payload;
- canonical lane, step, and residency rosters;
- an absolute tolerance of `1e-6`.

Consumers must reject duplicate JSON keys, unknown fields, non-finite or
unbounded numbers, path traversal, malformed hex, wrong shapes, reordered
lanes, dishonest expected values, and a MatrixIR graph that changes the pinned
dense calculation.

## Validation

```text
python code/scripts/validate_backend_parity_labs.py
pytest code/scripts/tests/test_backend_parity_labs.py -q
cargo test -p matrix-cpu --test backend_parity_fixture
```

The TypeScript visualizer must also run its production scalar and matrix paths
and compare both with the fixture. Its deterministic tests exercise the async
backend contract without claiming hardware. Responsive browser QA performs the
real WebGPU availability probe and records the result honestly.

## Cross-Language Direction

MatrixIR JSON plus little-endian buffers are the portable consumer boundary.
The existing Rust core already has Node N-API, Python, and Ruby bindings. A Go,
Swift, Java, or C# consumer can first replay this fixture in native code, then
replace its arithmetic with a Rust binding without changing the oracle.

NN31 does not claim a stable C ABI exists. That remains a later cross-language
tranche; when it lands, this exact fixture becomes its first conformance test.

## Non-Goals

- Benchmarking throughput or latency.
- Claiming simulated vendor APIs are real hardware acceleration.
- Training or gradient parity.
- Precision conversion, quantization, or persistent multi-step buffers.
- Defining the future stable Rust C ABI.
