# Changelog — coding-adventures-matrix-rust-python

## [0.1.0] — 2026-05-19

### Added — MX09 Phase 4: companion Python wrapper package

Initial release.  Pure-Python wrapper that re-exports the symbols from
the [`matrix_rust_python`](../../rust/matrix-rust-python) C extension
with type hints.

#### Why a separate package

Two reasons (per the MX09 spec §"Where the crate lives"):

1. **Stable import path for consumers.**  Future `ml-framework-*`
   Python packages do `import coding_adventures_matrix_rust_python as m`
   and get a stable namespace.  If the underlying C extension's
   internal module name ever changes (or it gets split / wheel-build
   tooling switches), this wrapper absorbs the change.
2. **Type hints.**  C extensions don't carry inline type
   annotations; the bundled `__init__.pyi` stub gives IDEs and
   `mypy` the signatures of `Graph`, `Runtime`,
   `graph_round_trip_json`, `run_graph_on_cpu`.

#### Public API

Re-exports the full Phase 1/2/2b surface from `matrix_rust_python`:

```python
import coding_adventures_matrix_rust_python as m

# Module-level helpers (Phases 1 + 2)
m.graph_round_trip_json(json_string) -> str
m.run_graph_on_cpu(envelope_json)     -> str

# Class-based API (Phase 2b)
graph = m.Graph(json_string)
graph.to_json()        -> str
graph.describe()       -> "Graph(tensors=4, ops=3, ...)"

rt = m.Runtime()
rt.run(graph, [b1, b2, ...]) -> list[bytes]
```

#### Smoke test

`tests/test_smoke.py` round-trips a 2x2 MatMul graph through the
wrapper:

```
[[1, 2], [3, 4]] @ [[5, 6], [7, 8]] == [[19, 22], [43, 50]]
```

Plus:

- four-symbol re-export verification
- Graph.to_json round-trip semantic equality
- Graph.describe topology counts
- Runtime.run argument-count error
- Runtime.run type-discrimination (passing a non-Graph raises TypeError)
- module-level graph_round_trip_json sanity

**The tests skip cleanly** if the underlying `matrix_rust_python` C
extension isn't installed — so this package is import-safe even
without the extension built (e.g. on a fresh checkout before
`cargo build -p matrix-rust-python --release` has run).

#### Installation notes

The underlying C extension is **not yet published to PyPI**.  Until
MX09 Phase 5 lands a publish workflow, install the extension
manually:

```
cd code/packages/rust/matrix-rust-python
cargo build --release
cp target/release/libmatrix_rust_python.{so,dylib} \
   $(python -c 'import sysconfig; print(sysconfig.get_paths()["purelib"])')/matrix_rust_python.so
```

Then `pip install -e .` for this wrapper package, and
`import coding_adventures_matrix_rust_python` will work.

A precise `ImportError` (not Python's default `ModuleNotFoundError`)
fires if the extension is missing, pointing at the install
instructions.

### What's not shipped in Phase 4

- No PyPI publish (Phase 5)
- No `runtime.run_async()` / asyncio wrapper (deferred — see
  MX09 §"Non-goals")
- No NumPy interop (`runtime.run_numpy`) — deferred to MX11+
- No GPU executor — inherited from matrix-runtime's planner when
  matrix-metal / matrix-cuda are registered (out of scope here)
