# Changelog — `basic-dap`

## 0.1.0 — 2026-05-30 (BASIC-DAP01 — initial Dartmouth BASIC DAP adapter)

Initial release.  Modelled on `twig-dap` 0.3.1 but rooted in the
generic `vm-debug` substrate so it doesn't pull in `twig-vm`.

### What's here

- `BasicDebugAdapter` — `impl LanguageDebugAdapter for BasicDebugAdapter`
  with stateless `compile` / `launch_vm` hooks.
- `build_sidecar(module, source_path) -> Vec<u8>` — walks every
  emitted IIR function's `source_map` and emits a `debug-sidecar`
  byte blob with one row per non-synthetic instruction, plus
  variable declarations with the stable alphabetical slot indexing
  `vm_debug::DebugServer::new_with_module` uses.
- `find_sibling_binary(name)` — locate `basic-vm` next to the
  currently-running `basic-dap` executable, with the same
  path-traversal guard as `twig-dap`.
- `basic-dap` binary — a one-screen `main` that wires
  `BasicDebugAdapter` into `dap_adapter_core::DapServer::run_stdio`.

### Dependencies

- `dap-adapter-core` — editor-side DAP plumbing.
- `dartmouth-basic-iir-compiler` 0.4.0 — frontend (source → IIR
  with source-loc threading).
- `interpreter-ir` — shared IR shape.
- `debug-sidecar` — sidecar writer.

Notably **not** dependent on `twig-vm` or `vm-debug` directly —
the wire-protocol contract lives in `vm-debug` but only the
sibling `basic-vm` binary needs to import it.

### What's next

- A sibling `basic-vm` binary that listens on `--debug-port` and
  speaks the `vm-debug` protocol.  Until that lands, `launch_vm`
  attempts to spawn `basic-vm` and surfaces the standard
  "executable not found" error if it isn't installed.  The
  `BasicDebugAdapter::compile` half — the editor-side compile
  step — works today and is unit-tested in this crate.

### Tests

- 4 unit tests:
  - `adapter_metadata_correct` — `language_name() == "basic"`,
    `file_extensions()` contains `"bas"` and `"basic"`.
  - `compile_produces_sidecar_with_line_table` — a 3-line BASIC
    program compiles and the sidecar contains at least one
    non-synthetic line-table entry.
  - `find_sibling_binary_rejects_path_traversal` — leading `..`,
    `/`, `\`, NUL all rejected.
  - `compile_propagates_compile_errors` — malformed BASIC
    produces a non-empty `Err`.
