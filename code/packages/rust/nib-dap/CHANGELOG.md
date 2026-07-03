# Changelog — `nib-dap`

## 0.1.0 — 2026-05-30 (NIB-DAP01 — initial Nib DAP adapter)

Initial release.  Sibling of `basic-dap` 0.1.0 (PR #4596) and
`twig-dap` 0.3.1, modelled on the same `dap-adapter-core` +
`debug-sidecar` substrate.

### What's here

- `NibDebugAdapter` — `impl LanguageDebugAdapter for NibDebugAdapter`
  with stateless `compile` / `launch_vm` hooks.
- `build_sidecar(module, source_path) -> Vec<u8>` — walks every
  emitted IIR function's `source_map` and emits a `debug-sidecar`
  byte blob with one row per non-synthetic instruction, plus
  variable declarations with the stable alphabetical slot indexing
  `vm_debug::DebugServer::new_with_module` uses.
- `find_sibling_binary(name)` — locate `nib-vm` next to the
  currently-running `nib-dap` executable, with the same
  path-traversal guard as `twig-dap` / `basic-dap`.
- `nib-dap` binary — a one-screen `main` that wires
  `NibDebugAdapter` into `dap_adapter_core::DapServer::run_stdio`.

### Dependencies

- `dap-adapter-core` — editor-side DAP plumbing.
- `nib-iir-compiler` 0.6.0 — frontend (source → IIR with
  source-loc threading from NIB06).
- `interpreter-ir` — shared IR shape.
- `debug-sidecar` — sidecar writer.

Notably **not** dependent on `twig-vm` or `vm-debug` directly —
the wire-protocol contract lives in `vm-debug` but only the
sibling `nib-vm` binary needs to import it.

### What's next

- A sibling `nib-vm` binary that listens on `--debug-port` and
  speaks the `vm-debug` protocol.

### Tests

- 4 unit tests:
  - `adapter_metadata_correct` — `language_name() == "nib"`,
    `file_extensions()` contains `"nib"`.
  - `compile_produces_sidecar_with_line_table` — a multi-line Nib
    program compiles and the sidecar contains at least one
    non-synthetic line-table entry.
  - `find_sibling_binary_rejects_path_traversal` — leading `..`,
    `/`, `\`, NUL all rejected.
  - `compile_propagates_compile_errors` — malformed Nib (parse
    error) produces a non-empty `Err`.
