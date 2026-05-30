# Changelog — `oct-dap`

## 0.1.0 — 2026-05-30 (OCT-DAP01 — initial Oct DAP adapter)

Initial release.  Sibling of `basic-dap` 0.1.0 (PR #4596),
`nib-dap` 0.1.0 (PR #4602), and `twig-dap` 0.3.1.  Completes the
three-language DAP-adapter trilogy (task #37 part 3 of 3).

### What's here

- `OctDebugAdapter` — `impl LanguageDebugAdapter for OctDebugAdapter`
  with stateless `compile` / `launch_vm` hooks.
- `build_sidecar(module, source_path) -> Vec<u8>` — walks every
  emitted IIR function's `source_map` and emits a `debug-sidecar`
  byte blob with one row per non-synthetic instruction, plus
  variable declarations with the stable alphabetical slot indexing
  `vm_debug::DebugServer::new_with_module` uses.
- `find_sibling_binary(name)` — locate `oct-vm` next to the
  currently-running `oct-dap` executable, with the same
  path-traversal guard as `twig-dap` / `basic-dap` / `nib-dap`.
- `oct-dap` binary — a one-screen `main` that wires
  `OctDebugAdapter` into `dap_adapter_core::DapServer::run_stdio`.

### Dependencies

- `dap-adapter-core` — editor-side DAP plumbing.
- `oct-iir-compiler` 0.4.0 — frontend (source → IIR with
  source-loc threading from OCT05).
- `interpreter-ir` — shared IR shape.
- `debug-sidecar` — sidecar writer.

Notably **not** dependent on `twig-vm` or `vm-debug` directly —
the wire-protocol contract lives in `vm-debug` but only the
sibling `oct-vm` binary needs to import it.

### What's next

- A sibling `oct-vm` binary that listens on `--debug-port` and
  speaks the `vm-debug` protocol.

### Tests

- 4 unit tests:
  - `adapter_metadata_correct` — `language_name() == "oct"`,
    `file_extensions()` contains `"oct"`.
  - `compile_produces_sidecar_with_line_table` — a multi-line Oct
    program compiles and the sidecar contains at least one
    non-synthetic line-table entry.
  - `find_sibling_binary_rejects_path_traversal` — leading `..`,
    `/`, `\`, NUL all rejected.
  - `compile_propagates_compile_errors` — malformed Oct produces a
    non-empty `Err`.
