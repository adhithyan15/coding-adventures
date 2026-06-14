# matrix-rust-python

Python C extension crate exposing the Rust matrix execution layer
(`matrix-ir`, `matrix-ir-json`, future `matrix-runtime` / `matrix-cpu`)
to Python.  Sibling crate to
[`matrix-rust-napi`](../matrix-rust-napi) (the Node.js N-API binding)
— same shape, different toolchain.

Currently at **Phase 3b** of [MX09](../../../specs/MX09-matrix-rust-python.md).

| Phase | Surface |
|-------|---------|
| 1 ✅ | `graph_round_trip_json(json_string: str) -> str` |
| 2 ✅ | `run_graph_on_cpu(envelope_json: str) -> str` |
| 2b ✅ | `m.Graph(json_str)` + `.to_json()` / `.describe()`; `m.Runtime()` + `.run(graph, [bytes]) -> [bytes]` |
| 3 ✅ | `pyproject.toml` + cargo+cp+smoke CI workflow (ubuntu + macos × py 3.11) |
| 3b ✅ | Extends CI to **windows-latest** and **py 3.10 / 3.11 / 3.12** (9 cells) |
| 4 ✅ | Companion Python wrapper package + `pytest` smoke |
| 3c  | Maturin shim / actual wheel packaging |
| 5   | PyPI publish |

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

## What ships today (Phases 1 + 2 + 2b)

```python
import matrix_rust_python as m
import json

# ── Phase 1: round-trip a matrix-ir-json wire-format Graph through Rust.
out = m.graph_round_trip_json(json_in)

# ── Phase 2: plan + execute on matrix-cpu via matrix-runtime (hex JSON envelope).
envelope = json.dumps({"graph": {...}, "inputs": ["3f80...", "..."]})
result = json.loads(m.run_graph_on_cpu(envelope))
output_bytes = bytes.fromhex(result["outputs"][0])

# ── Phase 2b: idiomatic class-based API (parse once, bytes I/O).
graph = m.Graph(json_string)
print(graph.describe())             # "Graph(tensors=4, ops=3, inputs=1, outputs=1, constants=2)"

rt = m.Runtime()
outputs = rt.run(graph, [b1, b2])   # list[bytes] in -> list[bytes] out
```

The Phase 2 envelope is bit-identical to `matrix-rust-napi`'s
`runGraphOnCpu` envelope; the Phase 2b class API matches the
matrix-rust-napi class shape modulo Python idioms (snake_case vs
camelCase, `bytes` vs `Buffer`).

Phases 3+ add the `maturin` wheel build and the companion
`code/packages/python/matrix-rust-python/` wrapper package — each as
its own PR per the MX09 spec.

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

Pure-Rust tests run via `cargo` (no Python interpreter required):

```
cargo test -p matrix-rust-python -- --nocapture
```

25 unit tests exercise the JSON round-trip helper, the envelope-shaped
execution helper, the 4 GiB DoS cap, and the Graph/Runtime class
layout invariants.

To produce the wheel that Python actually loads (Phase 3 ✅):

```
pip install maturin
cd code/packages/rust/matrix-rust-python
maturin build --release            # writes ./target/wheels/*.whl
pip install target/wheels/matrix_rust_python-*.whl
python -c "import matrix_rust_python as m; print(dir(m))"
```

CI (`.github/workflows/matrix-rust-python.yml`) runs the cargo
tests + the maturin build + the `pip install` + the import smoke
on `ubuntu-latest` and `macos-latest` (per MX09 §"Distribution
model") on every PR that touches this crate, `python-bridge`, or
any of the matrix-* dependencies.

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
