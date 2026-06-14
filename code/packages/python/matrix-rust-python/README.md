# coding-adventures-matrix-rust-python

Python wrapper for the Rust matrix execution layer.  Thin re-export
of the [`matrix_rust_python`](../../rust/matrix-rust-python) C
extension, plus type hints.

This is **Phase 4** of [MX09](../../../specs/MX09-matrix-rust-python.md).

| Phase | Surface |
|-------|---------|
| 1 ✅ | `graph_round_trip_json(json_string: str) -> str` |
| 2 ✅ | `run_graph_on_cpu(envelope_json: str) -> str` |
| 2b ✅ | `m.Graph(json_str)` + `.to_json()` / `.describe()`; `m.Runtime()` + `.run(graph, [bytes]) -> [bytes]` |
| 3 ✅ | `pyproject.toml` + cargo+cp+smoke CI workflow (ubuntu + macos) |
| 4 ✅ | This package — Python wrapper with type hints + pytest smoke |
| 5   | PyPI publish |

## Quick start

```python
import coding_adventures_matrix_rust_python as m
import json
import struct

# Build a 2x2 MatMul graph.
graph_json = json.dumps({
    "matrix_ir_version": 1,
    "tensors": [
        {"id": 0, "dtype": "f32", "shape": [2, 2]},
        {"id": 1, "dtype": "f32", "shape": [2, 2]},
        {"id": 2, "dtype": "f32", "shape": [2, 2]},
    ],
    "inputs": [0, 1],
    "outputs": [2],
    "ops": [{"kind": "MatMul", "lhs": 0, "rhs": 1, "output": 2}],
    "constants": [],
})

graph = m.Graph(graph_json)
print(graph.describe())   # Graph(tensors=3, ops=1, inputs=2, outputs=1, constants=0)

rt = m.Runtime()
a = struct.pack("<ffff", 1.0, 2.0, 3.0, 4.0)   # [[1,2],[3,4]]
b = struct.pack("<ffff", 5.0, 6.0, 7.0, 8.0)   # [[5,6],[7,8]]
outputs = rt.run(graph, [a, b])
result = struct.unpack("<ffff", outputs[0])
print(result)                                    # (19.0, 22.0, 43.0, 50.0)
```

## Why a separate Python package?

The MX09 spec calls for it (§"Where the crate lives"):

1. **Stable import path for consumers.**  Future `ml-framework-*`
   Python packages couple to `coding_adventures_matrix_rust_python`,
   not the underlying C extension's internal name.
2. **Type hints.**  IDEs and `mypy` need a `.pyi` stub to
   typecheck calls into the C extension; we ship one.
3. **Mirrors the matrix-rust-napi shape.**  The Node side has a
   `typescript/matrix-rust-napi` wrapper package over the Rust
   crate; this is the Python analog.

## Installation

The underlying `matrix_rust_python` C extension is **not yet on
PyPI** (that's MX09 Phase 5).  Until then, install it manually:

```
# Build the C extension from the Rust crate.
cd code/packages/rust/matrix-rust-python
cargo build --release

# Copy it to your site-packages.
cp target/release/libmatrix_rust_python.{so,dylib} \
   $(python -c 'import sysconfig; print(sysconfig.get_paths()["purelib"])')/matrix_rust_python.so

# Install the wrapper.
cd ../../python/matrix-rust-python
pip install -e .
```

Then:

```python
import coding_adventures_matrix_rust_python as m
```

## Smoke test

```
cd code/packages/python/matrix-rust-python
uv pip install -e ".[dev]"
python -m pytest tests/ -v
```

The pytest suite skips cleanly if the C extension isn't installed —
so this package is import-safe even on a fresh checkout.

## Related

- [`matrix-rust-python`](../../rust/matrix-rust-python) — the
  underlying Rust C extension crate
- [`matrix-rust-napi`](../../rust/matrix-rust-napi) — sibling
  Node.js binding (MX07)
- [MX09 spec](../../../specs/MX09-matrix-rust-python.md)
