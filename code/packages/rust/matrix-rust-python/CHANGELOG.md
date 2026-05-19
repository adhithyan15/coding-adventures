# Changelog — matrix-rust-python (Rust)

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
