# Changelog — matrix-rust-python (Rust)

## [0.3.2] — 2026-05-20

### Added — MX09 Phase 3b: Windows wheel + cross-Python CI matrix

Extends the Phase 3 build-smoke workflow from `{ubuntu, macos} ×
{py 3.11}` (2 cells) to `{ubuntu, macos, windows} × {py 3.10, 3.11,
3.12}` (9 cells, all run in parallel with `fail-fast: false`).

#### `build.rs` — Windows linker support

Added an `emit_windows_link_flags()` path that:

1. **Probes for Python** via the standard `python` / `python3`
   command on PATH (or `PYO3_PYTHON` / `PYTHON_SYS_EXECUTABLE`
   env-var overrides — same convention as pyo3 / maturin so the
   same environment setup works in both ecosystems).
2. **Queries `sysconfig`** to locate `<install>/libs/`, where
   CPython on Windows ships `python3.lib` (Limited API import
   library, ABI-stable across all Python 3.x — see
   <https://docs.python.org/3/c-api/stable.html>).
3. **Emits `cargo:rustc-link-search=native=...` + `rustc-link-lib=python3`**
   so the cdylib resolves every Python C API symbol at link time
   (Windows linkers don't have a `-undefined dynamic_lookup`
   equivalent).

All the symbols `python-bridge` declares are part of the Limited
API since 3.2 (PEP 384), so linking against `python3` (not the
version-specific `python3X`) yields one `.pyd` that loads under
every Python 3.x install on Windows.

#### CI workflow — 3 × 3 matrix

`name: build + import smoke (${{ matrix.os }}, py ${{ matrix.python-version }})`

| OS / Python | 3.10 | 3.11 | 3.12 |
|-------------|:----:|:----:|:----:|
| ubuntu-latest | ✅ | ✅ | ✅ |
| macos-latest | ✅ | ✅ | ✅ |
| windows-latest | ✅ | ✅ | ✅ |

Cache key now includes `python-version` so each cell has its own
slot (build.rs probes Python at build time on Windows, so the
linked artifact is version-sensitive).

Per-OS cdylib name mapping + per-OS Python-importable name mapping
(`.so` vs `.pyd`) live in the matrix `include:` table so the bash
in the staging step stays simple and identical across all 9 cells.
`defaults.run.shell: bash` selects Git-Bash on the Windows runners
so the same script runs everywhere.

`sys.path.insert` from inside the smoke-import heredoc replaces
shell-level `PYTHONPATH=...` so the path separator (`:` on POSIX,
`;` on Windows) and path style (`/c/foo` vs `C:\foo`) are handled
by Python via `cygpath -w` on Windows.

#### What's still **not** in Phase 3b

- **No maturin shim.**  The Phase 3 PR documented that maturin
  auto-detects our raw-C-extension cdylib as `bindings = "cffi"`
  mode (which generates a cffi wrapper instead of packaging the
  cdylib as a CPython extension).  Maturin's `Binding` enum
  has no "raw" / "no-binding" variant in current releases —
  workarounds either require fake pyo3 deps (hacky) or a custom
  cffi stub (defeats the purpose).  **Deferred to a future
  Phase 3c** which will either patch maturin upstream, switch
  to setuptools-rust, or hand-roll a minimal wheel-packaging
  script (cargo build → zip into wheel format).  The current
  cargo+cp+smoke workflow proves the same Phase 3 acceptance
  property (cdylib links and CPython can `import` it) without
  needing a wheel artifact.

- **No PyPI publish** (Phase 5).

## [0.3.1] — 2026-05-19

### Added — MX09 Phase 3: maturin wheel-build + CI smoke import

Adds the missing piece between "the cdylib compiles" and "Python can
actually `import matrix_rust_python`":

- **`pyproject.toml`** — `maturin>=1.0` as the build backend; the
  same configuration pattern `font-parser-python` already uses.
  Maturin auto-detects the `PyInit_matrix_rust_python` symbol in
  the resulting cdylib and packages it as a C-extension wheel.
  `requires-python = ">=3.10"` to match the workspace's Python
  floor.
- **`.github/workflows/matrix-rust-python.yml`** — new
  path-filtered workflow (mirrors `matrix-rust-napi.yml`).  Runs
  on `ubuntu-latest` and `macos-latest` (per MX09 §"Distribution
  model"):
    1. `cargo test -p matrix-rust-python --release` — re-runs the 25
       pure-Rust tests as a build-only smoke regression gate.
    2. `maturin build --release --out target/wheels` — produces the
       per-platform wheel.
    3. `pip install <wheel>` — installs into the workflow's
       Python 3.11.
    4. `python -c "import matrix_rust_python; ..."` — asserts the
       four expected exports (`graph_round_trip_json`,
       `run_graph_on_cpu`, `Graph`, `Runtime`) are present, that
       `Graph` / `Runtime` are types, and that the module-level
       names are callable.
    5. Reports wheel size + `file` info as a "did the linker pull
       everything in" smoke (anything under ~500 KiB is suspicious
       for this crate stack).

### What's not shipped in Phase 3

- **No PyPI publish.**  v0 builds the wheel artifact in CI but does
  not push it to PyPI.  Publishing lands as a follow-up (the spec
  calls this "Phase 3b") once a PyPI account + token are in CI
  secrets.
- **No Windows wheel.**  Same exclusion `matrix-rust-napi.yml` has —
  the win32 toolchain has its own quirks (Python linking on Windows
  uses `.lib` import libraries rather than dynamic lookup, so the
  maturin path is slightly different).  Add as a Phase 3 follow-up.
- **No cross-Python-version matrix.**  CI builds for Python 3.11
  only in v0.  Phase 3b can fan out to 3.10/3.11/3.12 once the
  3.11 build is stable.
- **No companion Python wrapper package.**  Phase 4 adds
  `code/packages/python/matrix-rust-python/` with `pytest` smoke
  tests that depend on the per-platform wheels.

### Local repro

```
cd code/packages/rust/matrix-rust-python
pip install maturin
maturin build --release            # writes ./target/wheels/*.whl
pip install target/wheels/matrix_rust_python-*.whl
python -c "import matrix_rust_python as m; print(dir(m))"
```

## [0.3.0] — 2026-05-19

### Added — MX09 Phase 2b: `Graph` and `Runtime` Python classes with `bytes` I/O

Adds the class-based API described in [MX09 §"The binding surface"](../../../specs/MX09-matrix-rust-python.md):

```python
import matrix_rust_python as m

graph = m.Graph(json_string)                  # parses once, holds Box<Graph>
print(graph.describe())                        # "Graph(tensors=4, ops=3, ...)"
re_serialised = graph.to_json()

rt = m.Runtime()
outputs = rt.run(graph, [b1, b2])              # list[bytes] in -> list[bytes] out
```

Mirrors `matrix-rust-napi`'s Phase 2b (PR #3539 / PR #3546 / PR #3551).
Two improvements over the napi version are inherent to the Python C API:

1. **No 128-bit type-tag invention needed.**  napi's `napi_unwrap` is
   type-agnostic and forced matrix-rust-napi to invent a 16-byte
   `[u64; 2]` discriminator prefix to defend against
   `Graph` ↔ `Runtime` type confusion.  Python's type system
   already provides instance-of discrimination via
   `PyObject_IsInstance(obj, GRAPH_TYPE)` — built into the runtime,
   ~free.
2. **No `napi_value` → `napi_ref` lifetime trap.**  N-API constructor
   handles are scope-local; matrix-rust-napi's Phase 4 had to fix a
   latent bug (PR #3551) by switching to persistent `napi_ref`
   storage.  Python's `PyTypeObject*` from `PyType_FromSpec` is
   persistent for the life of the interpreter — store the raw
   pointer in an `AtomicUsize`, done.

### Implementation notes

- **New module `src/classes.rs`** (~590 lines) — defines `GraphInstance`
  and `RuntimeInstance` `#[repr(C)]` structs starting with
  `PyObject_HEAD`-sized opaque headers, followed by inline payload.
  `PyType_FromSpec` creates heap types from `PyType_Spec` + slot tables
  at module-load time.
- **Slots used:** `Py_tp_init = 60`, `Py_tp_dealloc = 52`,
  `Py_tp_methods = 72`.  Defaults used for `tp_new` and `tp_alloc`
  (`PyType_GenericNew` / `PyType_GenericAlloc`), so the instance
  struct is zero-initialised before `tp_init` runs.
- **Refcounting discipline.**  Methods return new references via
  `str_to_py` / `bytes_to_py` / `PyList_New` (CPython takes
  ownership); we never decref what we return.  `PyList_SetItem`
  steals the bytes reference — no Py_DecRef needed on the items
  we add to the output list.
- **Bytes I/O via python-bridge's `bytes_to_py` / `bytes_from_py`** —
  the latter copies into an owned `Vec<u8>` immediately (no
  borrowed-buffer lifetime hazards across a GIL release).
- **Inline extern declarations** — `PyObject_Free` and
  `PyObject_IsInstance` aren't yet in `python-bridge`; declared
  inline in `classes.rs` (same pattern font-parser-python uses for
  `PyCapsule_New` etc.).  Both are stable Limited API symbols since
  Python 3.2.

### Tests (now 25, was 19)

6 new pure-Rust tests on the class-layout invariants the unwrap
helpers depend on (mirrors matrix-rust-napi's tag-layout tests):

- `graph_instance_head_lives_at_offset_zero`
- `runtime_instance_head_lives_at_offset_zero`
- `graph_instance_head_size_matches_constant`
- `graph_instance_inner_is_one_pointer_wide`
- `graph_and_runtime_basicsize_at_least_one_pointer_past_head`
- `slot_constants_match_python_stable_abi`

End-to-end class behavior (calling `m.Graph(...)` from real Python,
asserting `graph.describe()`, etc.) is exercised in Phase 4's
`pytest` smoke suite via the wrapper package.

### Security

The 4 GiB DoS cap (`MAX_TOTAL_BUFFER_BYTES` from Phase 2) is unchanged
and still fires before any `AllocBuffer` call — `rt.run` goes through
the same `run_graph_on_cpu` pure-Rust helper as the Phase 2 envelope
path.

Type discrimination is enforced via `PyObject_IsInstance(obj, GRAPH_TYPE)`
before any `obj as *mut GraphInstance` cast.  This makes
`rt.run(rt, [])` (passing a Runtime where a Graph is expected) raise
`TypeError` cleanly rather than blindly dereferencing.

### What's not shipped in Phase 2b

- No `pyproject.toml` / `maturin` wheel build (Phase 3).
- No companion Python wrapper package (Phase 4).
- No static-method sugar (`Graph.from_json`, `Runtime.create`) —
  matches napi's eventual Phase 4 shape; Python users can just do
  `m.Graph(json)` / `m.Runtime()`.

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
