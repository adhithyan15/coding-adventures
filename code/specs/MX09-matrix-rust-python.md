# MX09 — `matrix-rust-python`: Python binding to the Rust matrix execution layer

## Why this spec exists

[ARCH02](./ARCH02-rust-native-execution-backbone.md) Phase 7 calls
for a Python binding to the matrix execution layer.  The Node
binding (matrix-rust-napi) shipped through MX07 across PRs
#3518 / #3527 / #3539 / #3546 / #3551; this spec is its
Python counterpart.

Reading ARCH02 alone leaves the same questions open that MX07
answered for Node — they need answering again for Python:

1. **Where does the crate live?**  Under `code/packages/rust/`
   like every other Rust crate, or co-located with the Python
   wrapper package?
2. **What binding tooling?**  pyo3, pyo3-ffi, cffi, ctypes, or
   the workspace's own `python-bridge`?
3. **What is the binding surface?**  Pass `Graph` instances as
   PyCapsules, or by JSON wire format?  Expose individual ops or
   whole-graph execution?  Async or sync?
4. **How are `.so`/`.pyd` binaries distributed?**  Built at
   install time (requires a Rust toolchain on the consumer
   machine), or per-platform wheels uploaded to PyPI?
5. **How does CI prove the binding works?**  Just `cargo test`,
   or a round-trip that builds the `.so` and exercises it from
   Python?
6. **How does this not break MX00?**  Same answer as MX07 —
   matrix-ir / matrix-cpu / matrix-runtime stay zero-dep; the
   Python binding is the edge.
7. **How does this interact with the existing `ml-framework-*`
   Python packages?**

This spec answers each.  Where the answer mirrors MX07's, it
says so explicitly rather than re-deriving — the *patterns* are
identical, only the **toolchain** differs (Python C API vs
Node.js N-API; PyCapsule vs napi_wrap; cdylib still cdylib).

---

## Binding tooling: `python-bridge`, not pyo3

The workspace convention (per
[ARCH02 §"Bindings layer: workspace bridges, not ecosystem crates"](./ARCH02-rust-native-execution-backbone.md))
is to use the workspace's own `*-bridge` crate, not the
ecosystem-standard binding library.  For Python that means
**`python-bridge`** — a zero-dependency safe Rust wrapper over
Python's stable Limited C API (PEP 384, stable since Python 3.2
in 2011), not `pyo3` or `pyo3-ffi`.

This was the same call MX07 made for Node (use `node-bridge`,
not `napi-rs`), and the reasons carry over:

* **Zero crates.io dependencies.**  matrix-rust-python has the
  same dependency surface as matrix-rust-napi: workspace
  matrix-* crates plus the one workspace bridge crate.
* **One reviewer skillset.**  Reviewing the Python binding is
  no harder than reviewing the Node binding once you know the
  `*-bridge` shape.  font-parser-python (the precedent
  consumer of python-bridge) is ~340 LOC and structured the
  same way as font-parser-node.
* **No proc-macro at the binding boundary.**  `#[pyfunction]`
  / `#[pyclass]` proc-macros from pyo3 hide what's actually
  happening at the C ABI boundary; raw `extern "C"`
  declarations make it inspectable under `gdb`/`lldb` without
  macro indirection.
* **ABI-stable by design.**  Python's Limited API is
  guaranteed stable across all Python 3.x versions; the extern
  declarations just work on every supported Python without
  per-version rebuilds.

If a future binding ever hits a real ceiling that only `pyo3`
can break through (async patterns its `pyo3-asyncio` integrates
out of the box, very complex class hierarchies with deep
inheritance), the option remains open — but each binding makes
that call on its own merits, and the default is `python-bridge`.

---

## Where the crate lives

```
code/packages/rust/matrix-rust-python/
  Cargo.toml             # cdylib only (workspace convention)
  BUILD                  # cargo test invocation
  BUILD_windows
  CHANGELOG.md
  README.md
  package.toml           # Phase 3: setuptools / maturin metadata
  src/
    lib.rs               # graph_round_trip (Phase 1) + future
                         # Graph / Runtime PyCapsule wrappers + module init
  required_capabilities.json
  tests/
    smoke.py             # python -m unittest (lands with Phase 4)
```

It lives under `code/packages/rust/` because it **is** a Rust
crate (`Cargo.toml`, `src/lib.rs`, `cargo test`).  The fact that
its build output ends up imported by Python is incidental — the
same way `font-parser-python` is a Rust crate that ships a
`.so` Python imports.

Companion Python wrapper package:

```
code/packages/python/matrix-rust-python/
  pyproject.toml         # depends on the prebuilt platform wheels
  src/coding_adventures_matrix_rust_python/
    __init__.py          # thin re-export with type stubs
  README.md
  CHANGELOG.md
  BUILD                  # pip install -e . && python -m unittest
  tests/
    test_smoke.py        # Phase 4 — the actual end-to-end tests
```

Companion exists so the future `ml-framework-*` Python packages
have exactly *one* import target (`coding_adventures_matrix_rust_python`),
instead of having to do per-platform wheel selection themselves.

---

## The binding surface

The Python surface **mirrors MX07's Node surface** — same
shapes, same semantics, idiomatic Python syntax:

```python
import coding_adventures_matrix_rust_python as matrix

# Phase 1 — JSON round-trip smoke (matches Node's graphRoundTripJson).
round_tripped = matrix.graph_round_trip_json(json_string)

# Phase 2 — JSON-envelope one-shot execution (matches Node's runGraphOnCpu).
#   { "graph": <matrix-ir-json schema>,
#     "inputs": [ <hex-string>, ... ] }
result = matrix.run_graph_on_cpu(json.dumps({
    "graph": {...},
    "inputs": ["3f8000003f000000", ...],
}))
outputs = json.loads(result)["outputs"]

# Phase 2b — class-based API with bytes I/O (matches Node's Graph + Runtime).
graph = matrix.Graph(json_string)
#  or:  matrix.Graph.from_json(json_string)
print(graph.describe())   # "Graph(tensors=4, ops=3, inputs=1, outputs=1, constants=2)"

rt = matrix.Runtime()
#  or:  matrix.Runtime.create()

outputs = rt.run(graph, [input_bytes_1, input_bytes_2])
# outputs: list[bytes] — one entry per graph.outputs(), each a
# raw little-endian f32 payload.
```

### PyCapsule for Graph / Runtime handles

Following the `font-parser-python` precedent: the `Graph` and
`Runtime` "classes" are exposed via Python C API class
definitions whose `tp_init` slot calls into the Rust addon and
stores the wrapped `Box<matrix_ir::Graph>` (or `Box<()>` for
Runtime) inside a **PyCapsule** held by the instance.

PyCapsule is Python's equivalent of N-API's `napi_wrap`.  Two
properties of PyCapsule that obviate one of MX07's harder
problems:

1. **PyCapsules have an explicit `name` field** for type
   discrimination.  We name capsules `"matrix_rust_python.Graph"`
   and `"matrix_rust_python.Runtime"`; `PyCapsule_IsValid` rejects
   the wrong type before our extractor ever dereferences the
   pointer.  This is the Python solution to the napi
   type-confusion bug we hit in MX07 Phase 2b — *built-in to the
   API*, no 128-bit tag invention needed.
2. **PyCapsules carry a destructor function pointer.**  The
   destructor runs deterministically when Python collects the
   capsule's owning object, calling `Box::from_raw(...)` to free
   the wrapped Graph.  Same shape as N-API's `napi_finalize`.

The PyCapsule design choice eliminates the `napi_value` →
`napi_ref` lifetime trap that MX07 Phase 4 had to fix (PR
#3551).  Python C API objects in general are reference-counted
and lifetime-stable from the moment they enter scope; no
"local handle scope" concept exists.

### Bytes I/O

`runtime.run(graph, [input_bytes_1, input_bytes_2])` takes a
Python `list[bytes]`.  python-bridge already exposes
`bytes_to_py` / `bytes_from_py` (zero-copy view at the boundary,
data copied into Rust-owned `Vec<u8>` immediately to avoid
any GIL-release UB).  Outputs come back as a fresh
`list[bytes]` — each `bytes` is a freshly-allocated Python
object owning its own copy.

Same copy-in / copy-out discipline as `node-bridge`'s Buffer
helpers (PR #3529).  Future Phase 4b can switch to
`memoryview` / `bytearray` for zero-copy where it matters; not
in MX09 v0.

---

## Distribution model

We follow the **manylinux + per-platform wheel** model that's
standard for compiled Python extensions:

```
coding-adventures-matrix-rust-python           (sdist, all platforms)
└── coding-adventures-matrix-rust-python-0.1.0-cp39-cp39-manylinux2014_x86_64.whl
    coding-adventures-matrix-rust-python-0.1.0-cp39-cp39-macosx_11_0_arm64.whl
    coding-adventures-matrix-rust-python-0.1.0-cp310-cp310-manylinux2014_x86_64.whl
    ...
```

Each wheel embeds the per-platform `.so` (or `.pyd` for
Windows).  `pip install` picks the right wheel for the host's
Python version + OS + architecture.  When no matching wheel is
available, pip falls back to building from the sdist — which
requires a Rust toolchain on the host (acceptable for
Python developers; otherwise install a prebuilt wheel).

**v0 ships only `cp310-cp312` for `manylinux2014_x86_64` and
`macosx_11_0_arm64`** — the four platforms our CI runners
support out of the box.  Other Python versions / platforms get
added as needed.  The wrapper sdist raises a clear `ImportError`
on load if no matching wheel was installed.

Wheel building uses `maturin build --release` per-platform on
GitHub Actions, uploaded to a per-version PyPI package in a
follow-up Phase 5.  v0 lands the build workflow but no PyPI
publishing.

---

## CI strategy

Three tiers, in order of expense — mirrors MX07 §"How CI proves
this works":

1. **`cargo test -p matrix-rust-python`** — pure-Rust tests.
   The crate's pure-Rust tests cover:
   - Graph JSON round-trip (already covered by `matrix-ir-json`,
     but re-exercise here too).
   - Runtime construction + execution on simple graphs through
     an `#[cfg(test)]`-only internal helper that bypasses the
     Python C API and calls `run_graph_on_cpu` directly.
   - Error mapping (`matrix_runtime::Error` → Python
     `ValueError` / `RuntimeError` via python-bridge's
     `new_exception`).
2. **Build smoke** — `maturin build --release` produces the
   wheel; `pip install ./target/wheels/*.whl` installs it; then
   `python -c "import coding_adventures_matrix_rust_python; print(dir(...))"`
   asserts the four expected names are present.
3. **End-to-end smoke** — Phase 4's `tests/test_smoke.py`
   exercises `Graph` + `Runtime` through real Python (constructs
   a 2×2 MatMul graph, builds inputs as `bytes`, calls
   `runtime.run`, asserts on output bytes).

Tier 1 runs on every PR via the existing `cargo build` gate.
Tiers 2-3 run only when matrix-rust-python files change (path
filter on the new workflow).

---

## MX00 compatibility

Identical to MX07 §"MX00 compatibility": matrix-ir,
matrix-runtime, matrix-cpu, etc. are bound by the MX00
zero-dependency mandate (CI-enforced).  None of them gains a
dependency from this work.  `matrix-rust-python` is the
binding-edge crate; its `Cargo.toml`:

```toml
[dependencies]
matrix-ir          = { path = "../matrix-ir" }
matrix-ir-json     = { path = "../matrix-ir-json" }
matrix-runtime     = { path = "../matrix-runtime" }
matrix-cpu         = { path = "../matrix-cpu" }
compute-ir         = { path = "../compute-ir" }
executor-protocol  = { path = "../executor-protocol" }
python-bridge      = { path = "../python-bridge" }

[lib]
crate-type = ["cdylib"]
name = "matrix_rust_python"
```

Zero crates.io dependencies (same as matrix-rust-napi).

---

## Phases

Each phase is a separately-PR'd, independently-reviewable
change — same shape as MX07's phase plan.

| Phase | Lands | Status |
|-------|-------|--------|
| 0 | This spec. | **this PR** |
| 1 | Rust crate skeleton — `Cargo.toml` (`cdylib`), `src/lib.rs` exporting `graph_round_trip_json` (one function, JSON-in → matrix-ir-json::decode → matrix-ir-json::encode → JSON-out), 5 unit tests on the pure-Rust core.  No Python side yet. | pending |
| 2 | `run_graph_on_cpu(envelope_json)` — end-to-end execution on `matrix-cpu` via the planner. JSON envelope with hex-encoded byte payloads (one string-in, string-out function — same pattern as MX07 Phase 2). | pending |
| 2b | `Graph` + `Runtime` as Python C API classes with `bytes` I/O via `python-bridge`'s `bytes_to_py` / `bytes_from_py` helpers.  PyCapsule with type-name validation for handle storage.  No type-confusion class to invent — Python's PyCapsule name field is the type-tag. | pending |
| 3 | `pyproject.toml` + `maturin build` GitHub Actions workflow.  Builds the wheel on `ubuntu-latest` (linux-x64) and `macos-latest` (darwin-arm64); confirms `import coding_adventures_matrix_rust_python` exposes all four names.  No PyPI publish step yet. | pending |
| 4 | Python wrapper package `code/packages/python/matrix-rust-python/` — pyproject.toml with the per-platform wheels as install dependencies, `__init__.py` re-exporting the extension's symbols with proper type hints (`.pyi` stubs).  `tests/test_smoke.py` round-trips a `MatMul` graph through the binding. | pending |
| 5 | (Separately scoped, MX10.) Refactor any existing `ml-framework-*` consumer to use this binding under a conditional fall-back (`try: import coding_adventures_matrix_rust_python; except ImportError: use_pure_python_fallback()`). | future |

Phase 0 (this PR) does not ship code — only this spec.  Per
CLAUDE.md: **specs first, implementation after**.

---

## Non-goals

To pre-empt overreach:

* **GPU execution is not added in MX09.**  The runtime planner
  already lifts to Metal / CUDA when available; the Python
  binding inherits that for free.  GPU executor implementations
  are out of scope.
* **Browser support is not scoped here.**  ARCH02 exempts the
  browser; browser execution goes through `matrix-ir-ts` +
  `matrix-webgpu-ts` + `matrix-cpu-ts` (ARCH02 Phases 4-5).
* **Async / `runAsync()` is not scoped.**  `Runtime.run` is
  synchronous, blocking the GIL during the dispatch.  A future
  `Runtime.run_async()` returning an `asyncio.Future` is added
  when profiling shows the GIL-blocked dispatch matters.
* **Memory pooling across calls is not scoped.**  Each `run()`
  is independent.  Buffer reuse is a profile-driven
  optimisation for v1+.
* **NumPy interop is not scoped here.**  The MX09 v0 surface
  takes / returns `bytes` lists.  A future MX11 (or similar)
  adds a `runtime.run_numpy(graph, [np.ndarray, ...])` adapter
  that wraps the bytes API; out of scope to keep the dependency
  surface minimal (NumPy would be the binding's first non-stdlib
  consumer-visible dep).

---

## Open questions

Deferred until the implementation PRs hit them:

* **Python version floor** — Python 3.10 is the current floor
  for the workspace's Python packages (per `pyproject.toml`
  surveys).  MX09 inherits unless a `python-bridge` extern
  requires newer.
* **`maturin` vs `setuptools-rust` for the build** — `maturin`
  is the modern standard for pure Rust extensions; this spec
  assumes it.  Revisit if `setuptools-rust` integration with the
  workspace's build-tool turns out to be smoother.
* **Linux distro coverage** — `manylinux2014` (centos 7 base)
  is the conservative-compatibility choice; `manylinux_2_28` is
  newer/smaller.  Pick at Phase 3 based on the runner
  availability.
* **PyPI publishing** — Phase 3 ships the build, not the
  publish.  When publishing lands (probably an MX09 Phase 3b)
  it'll need a PyPI account + token in CI secrets; that's a
  separately-coordinated change.

---

## Relationship to other specs

* **ARCH02 §"Phases"** — MX09 is Phase 7 of that roadmap
  ("First non-Rust, non-Node FFI binding — Python via the
  workspace `python-bridge` crate").
* **MX07** — sibling spec for the Node binding.  MX09's
  phase plan deliberately mirrors MX07's so the implementation
  pattern stays familiar.
* **MX08** — `typescript/matrix` refactor consuming the Node
  binding; the Python equivalent (refactoring an existing
  Python framework consumer) is MX10 (future).
* **MX11+** — NumPy-flavoured surface for the Python binding,
  if and when there's a real consumer for it.
