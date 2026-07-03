# Changelog

## Unreleased

### Added — v0.1.0: stable C ABI for matrix-cpu graph execution

First release.  Exposes the matrix-cpu execution engine through a
universal C ABI so any language with C FFI (Ruby, Lua, Go, Swift,
Kotlin, Crystal, Nim, Zig, Wasm hosts, …) can drive Rust graph
execution.

#### The two-function contract

```c
char* matrix_cpu_run_graph(const char* envelope_json,
                           char**       err_out);
void  matrix_cpu_free_string(char* s);
```

- **`matrix_cpu_run_graph`** takes a matrix-ir-json envelope (JSON
  string containing the graph definition + hex-encoded input
  tensors), runs it on `matrix-cpu`'s executor, returns a JSON
  envelope with the hex-encoded outputs.  Returns NULL on error
  with the error message written through `err_out`.
- **`matrix_cpu_free_string`** drops a string previously returned
  by `matrix_cpu_run_graph` (return value or via `err_out`).
  Required because Rust's allocator owns the buffers — callers
  cannot use `free()`.

#### Why a separate crate (vs. extending `matrix-rust-python`)?

`matrix-rust-python` is a `cdylib` named `matrix_rust_python` —
exposing functions through Python's C extension protocol.  Other
languages need a stable C ABI, not a Python C extension.

Rather than couple every future language binding to Python's
extension protocol, `c-bridge` is a fresh `cdylib + rlib` named
`matrix_c_bridge` that exports plain `extern "C"` symbols.
Architecturally matches the workspace pattern of `python-bridge` /
`node-bridge` / `ruby-bridge` (each language gets its own bridge
crate).

#### Code reuse

The pure-Rust `run_graph_on_cpu_via_json_envelope` function is
~50 lines of envelope plumbing — identical in shape to
`matrix-rust-python`'s.  v0.1 duplicates it inline rather than
introducing a third workspace crate (`matrix-rust-core`) for one
function.  Once we have ≥2 language bindings live (c-bridge +
matrix-rust-python today, with Ruby/Lua/Go landing soon), a
follow-up refactor PR can DRY into a shared core.

#### Safety guarantees

- **Never panics across the FFI boundary.**  `matrix_cpu_run_graph`
  wraps the pure-Rust core in `std::panic::catch_unwind`; any panic
  becomes a clean error string.  Panicking across an FFI boundary is
  undefined behaviour in some Rust versions, so this is defence in
  depth even though the executor isn't expected to panic.
- **Thread-safe.**  Each call constructs a fresh `CpuExecutor`; no
  shared mutable state across calls.
- **Memory contract documented.**  Caller owns returned strings;
  must free via `matrix_cpu_free_string`; mixing allocators is UB.
- **Null input is rejected**, not dereferenced.

#### Tests

8 unit tests, all run via `cargo test -p c-bridge`:

- `envelope_round_trip_succeeds_on_identity_graph` — full Rust
  round-trip on the smallest possible graph (1 tensor declared as
  both input and output, no ops).
- `malformed_json_envelope_returns_err_not_panic`
- `missing_graph_field_returns_err`
- `missing_inputs_field_returns_err`
- `invalid_hex_in_inputs_returns_err`
- `c_abi_round_trip_succeeds` — drive the C ABI directly from Rust.
- `c_abi_null_envelope_returns_null_with_err`
- `c_abi_malformed_envelope_returns_null_with_err`
- `free_null_string_is_noop`

#### What's next

The c-bridge is the foundation for a series of per-language idiomatic
libraries.  See the multi-language plan in the repo PR descriptions
— in priority order: Ruby pilot, then Lua, JS/TS (via existing
`node-bridge`), Go, Swift.  Each layers on top of this crate.
