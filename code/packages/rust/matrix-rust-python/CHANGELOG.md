# Changelog — matrix-rust-python (Rust)

## [0.2.0] — 2026-05-19

### Added — MX09 Phase 2: envelope-shaped one-shot CPU execution

Adds the second exported function — `run_graph_on_cpu` — that takes a
JSON envelope describing a graph plus its inputs and returns a JSON
envelope holding the executed outputs.  This is the smallest possible
slice of "real execution through the binding" — no `Graph` / `Runtime`
classes (Phase 2b), no bytes-typed I/O (Phase 2b), just one
string-in / string-out function that exercises the full
plan → alloc → upload → dispatch → download pipeline on
`matrix-cpu` via `matrix-runtime`.

The envelope shape is bit-identical to `matrix-rust-napi`'s
`runGraphOnCpu` Phase 2 envelope so a single JSON test fixture can
verify either binding.

Envelope shape:

```
in : {
        "graph":  <matrix-ir-json schema>,
        "inputs": [ "<lowercase-hex bytes>", ... ]
      }

out: {
        "outputs": [ "<lowercase-hex bytes>", ... ]
      }
```

Per-tensor byte strings use the same hex encoding `matrix-ir-json`
uses for constants — lowercase, no separator, no `0x` prefix, length
always `2 * num_bytes`.

### Security — `MAX_TOTAL_BUFFER_BYTES = 4 GiB` cap

Implements the same hard cap as `matrix-rust-napi::exec`: any graph
whose total placed-tensor byte size would exceed 4 GiB is refused
*before* any `AllocBuffer` call.  Without this, a ~500-byte JSON
envelope declaring a tensor like `shape=[1_000_000_000, 1_000_000_000],
dtype=F32` would flow `~4e18` bytes into `vec![0u8; bytes]` inside
`matrix-cpu::BufferStore` and abort the Python interpreter via
`handle_alloc_error`.

The cap matches the napi binding so the CPU executor's effective
DoS posture is the same regardless of which FFI edge invoked it.

### Added — pure-Rust tests (now 19, was 5)

- `exec::tests` (6 new tests, ported from matrix-rust-napi exec.rs):
  - `add_two_vectors_executes_end_to_end`
  - `matmul_2x2_executes_end_to_end`
  - `relu_layer_executes_end_to_end`
  - `rejects_wrong_input_count`
  - `rejects_wrong_input_byte_length`
  - `rejects_graph_with_oversized_output`  ← exercises the DoS cap
- `tests::envelope_*` (5 new tests on the envelope helper):
  - `envelope_runs_add_end_to_end` (headline JSON-in / JSON-out)
  - `envelope_rejects_missing_graph`
  - `envelope_rejects_non_array_inputs`
  - `envelope_rejects_invalid_hex_input`
  - `envelope_rejects_garbage_json`
- `tests::hex_*` (3 new tests on the hex codec):
  - `hex_round_trips`
  - `hex_decoder_rejects_odd_length`
  - `hex_decoder_rejects_bad_chars`

### Implementation notes

- **New module `src/exec.rs`** — direct port of
  `matrix-rust-napi/src/exec.rs`.  Same `run_graph_on_cpu(graph: &Graph,
  inputs: &[Vec<u8>]) -> Result<Vec<Vec<u8>>, String>` signature.
  Pre-allocates every buffer up front (trades a bit of peak memory
  for a much simpler glue layer; honouring `PlacedOp::Alloc/Free`
  lifetimes is Phase 2b work).
- **New dependencies** — adds `matrix-runtime`, `matrix-cpu`,
  `compute-ir`, `executor-protocol`, `coding-adventures-json-value`,
  `coding-adventures-json-serializer` to `Cargo.toml`.  All workspace
  path deps; zero new crates.io deps.  MX00 zero-dep mandate
  preserved (matrix-ir / matrix-runtime / matrix-cpu themselves
  gain no deps).
- **Python C API wrapper** `py_run_graph_on_cpu` follows the same
  pattern as `py_graph_round_trip_json`: extract args[0] as str,
  call the pure-Rust helper, return a fresh str.  Errors raise
  `ValueError`.  `extern "C"` boundary stays panic-free
  (`extern "C-unwind"` would be required to propagate panics; we
  don't enable it, so the "return Err, never panic" discipline is a
  hard safety invariant).
- **Methods table** grew from 2 to 3 entries (`graph_round_trip_json`,
  `run_graph_on_cpu`, sentinel).

### What's not shipped in Phase 2

- No `Graph` / `Runtime` Python classes via PyCapsule (Phase 2b).
- No `bytes` I/O — Phase 2 still uses hex-string-in / hex-string-out
  for parity with napi Phase 2.  Phase 2b switches to
  `python-bridge`'s `bytes_to_py` / `bytes_from_py`.
- No `pyproject.toml` / `maturin` wheel build (Phase 3).
- No companion Python wrapper package (Phase 4).

## [0.1.0] — 2026-05-19

### Added — MX09 Phase 1: crate skeleton + JSON round-trip

Initial release.  Ships the Phase 1 surface area from
`code/specs/MX09-matrix-rust-python.md`:

- **`graph_round_trip_json(json_string: str) -> str`** — Decode a
  matrix-ir-json wire-format `Graph` and re-encode it.  Returns the
  re-encoded JSON string (compact form, canonical field order).
  Raises Python `ValueError` on malformed JSON, schema-invalid JSON
  (wrong `matrix_ir_version`, missing required fields, etc.), or any
  other decode failure.

This is the smoke function that proves the matrix-ir-json wire format
survives a trip through the Python C API boundary — exact analog of
`matrix-rust-napi`'s `graphRoundTripJson` from MX07 Phase 1
(PR #3518).

### Implementation notes

- **`crate-type = ["cdylib"]`, lib name `matrix_rust_python`** —
  produces a `.so`/`.dylib`/`.pyd` that Python loads via its import
  machinery.  Rename / wheel-package handled in Phase 3.
- **`PyInit_matrix_rust_python`** is the entry point Python looks up
  via `dlsym` / `GetProcAddress` when the module is imported.
- **`python-bridge`** is the only binding-tooling dependency — zero
  pyo3, zero pyo3-ffi, zero bindgen.  Python's Limited C API (PEP 384)
  is ABI-stable across all Python 3.x versions, so the extension
  works on every supported interpreter without per-version rebuilds.
- **Pure-Rust core** (`pub fn round_trip_json`) splits the testable
  work from the `unsafe extern "C"` wrapper.  The 5 unit tests run
  on `cargo test -p matrix-rust-python` without requiring a Python
  interpreter.
- **`m_size = -1`** opts out of sub-interpreter reinitialisation
  (Phase 1 has no per-interpreter state; the methods table is
  process-global).

### Tests

Five pure-Rust unit tests in `src/lib.rs`:

1. `round_trip_preserves_graph_under_binary_wire_format` — encode →
   decode → re-encode produces a `Graph` byte-equal to the original
   under the canonical binary wire format.
2. `round_trip_handles_multi_op_graph` — exercises Add / Mul / Neg
   in a single graph to confirm multi-op coverage.
3. `round_trip_rejects_garbage_json` — fail-closed on syntactically
   invalid JSON.
4. `round_trip_rejects_unsupported_version` — fail-closed on
   `matrix_ir_version: 9999`.
5. `round_trip_is_idempotent` — `round_trip_json(round_trip_json(x))`
   equals `round_trip_json(x)` byte-for-byte.

### What's not shipped in Phase 1

- No `run_graph_on_cpu` envelope execution (Phase 2).
- No `Graph` / `Runtime` Python classes (Phase 2b).
- No `pyproject.toml` / `maturin` wheel build (Phase 3).
- No companion Python wrapper package (Phase 4).

Each follow-up phase ships as its own PR per the MX09 spec.
