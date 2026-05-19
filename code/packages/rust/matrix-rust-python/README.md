# matrix-rust-python

Python C extension crate exposing the Rust matrix execution layer
(`matrix-ir`, `matrix-ir-json`, future `matrix-runtime` / `matrix-cpu`)
to Python.  Sibling crate to
[`matrix-rust-napi`](../matrix-rust-napi) (the Node.js N-API binding)
— same shape, different toolchain.

This is **Phase 1** of [MX09](../../../specs/MX09-matrix-rust-python.md).
Phase 1 ships the crate skeleton plus one exported function
(`graph_round_trip_json`) — the smoke test that proves the
matrix-ir-json wire format survives a trip through the Python C API
boundary.  Phases 2+ add execution and class-based APIs.

## What this crate is

A `cdylib` that produces a `.so` / `.dylib` / `.pyd` file Python
imports as `matrix_rust_python`.  The crate is **a Rust crate** that
happens to expose a Python C extension — same way
[`font-parser-python`](../font-parser-python) is a Rust crate that
ships a font-parsing extension.

```
code/packages/rust/matrix-rust-python/
  Cargo.toml             # cdylib only (workspace convention)
  src/lib.rs             # graph_round_trip_json + module init + 5 tests
  BUILD                  # cargo test -p matrix-rust-python
  BUILD_windows
  CHANGELOG.md
  README.md              # ← you are here
  required_capabilities.json
```

## What ships in Phase 1

One exported Python function:

```python
import matrix_rust_python as m

# Round-trip a matrix-ir-json wire-format Graph through Rust.
out = m.graph_round_trip_json(json_in)
# out is a compact JSON string with canonical field order;
# `json.loads(out)` decodes to a Graph value semantically identical
# to `json.loads(json_in)`.
```

That's it.  The function is intentionally minimal — three things it
must prove:

1. **Build pipeline works.**  The cdylib compiles, exposes a
   `PyInit_matrix_rust_python` symbol, and is importable from
   Python.
2. **Python C API boundary works.**  Strings move in and out of the
   extension without data loss.
3. **`matrix-ir-json` is the right interop wire format.**  Anything
   constructable on either side (Rust, hand-written JSON, future TS)
   must survive `graph_round_trip_json` unchanged.

Phases 2+ add `run_graph_on_cpu` (envelope JSON execution),
`Graph` / `Runtime` classes with `bytes` I/O via `PyCapsule`, the
`maturin` wheel build, and the companion `code/packages/python/`
wrapper package.

## Why `python-bridge`, not `pyo3`

Workspace convention (per
[ARCH02 §"Bindings layer"](../../../specs/ARCH02-rust-native-execution-backbone.md)
and [MX09](../../../specs/MX09-matrix-rust-python.md)).  Zero
crates.io dependencies, raw `extern "C"` declarations are auditable
under `gdb`/`lldb`, Python's Limited API (PEP 384) is ABI-stable
across every Python 3.x, no proc-macros hiding the actual C boundary.

If a future binding ever hits a real ceiling that only `pyo3` can
break through, the option remains open — but the default is
`python-bridge`, same as the workspace's other Python extensions.

## Build & test

Pure-Rust tests run via `cargo`:

```
cargo test -p matrix-rust-python -- --nocapture
```

The 5 unit tests exercise the pure-Rust `round_trip_json` helper.
They do **not** require a Python interpreter — the Python C API
wrapper is exercised end-to-end by Phase 4's `pytest` suite via the
companion wrapper package.

To produce the actual `.so` / `.dylib` / `.pyd` that Python loads,
Phase 3 will add a `maturin build --release` invocation.  Until
then, `cargo build -p matrix-rust-python --release` produces
`target/release/libmatrix_rust_python.{so,dylib}` (or
`matrix_rust_python.dll` on Windows) which Python can `import`
directly after renaming the extension.

## How it fits in the stack

```
                    Python user code
                          │
                          ▼
              matrix_rust_python.so   ← this crate (the .so)
                          │
                          ▼
   matrix-ir-json ── matrix-ir   (zero-dep workspace crates)
                          │
                          ▼  (Phase 2+)
   matrix-runtime ── matrix-cpu / matrix-metal / matrix-cuda
```

The `*-bridge` crate (`python-bridge`) provides the safe Rust
wrappers over the Python C API extern declarations; no other
binding-tooling dependency.

## Related

- [`matrix-rust-napi`](../matrix-rust-napi) — sibling Node.js binding
  (MX07).  Same pattern, same shape, `napi_value` / `napi_wrap`
  instead of `PyObject*` / `PyCapsule`.
- [`font-parser-python`](../font-parser-python) — precedent
  consumer of `python-bridge`; the lib.rs there is the canonical
  reference for module-init + extern-fn patterns.
- [`matrix-ir-json`](../matrix-ir-json) — the JSON wire format crate
  this extension wraps.
- [MX09 spec](../../../specs/MX09-matrix-rust-python.md) — the
  multi-phase plan this crate implements.
