# Changelog — `twig-dap`

## 0.2.0 — 2026-05-05

**LS03 PR B — Real `TwigDebugAdapter` + `twig-dap` binary.**

The skeleton ships as a complete, working DAP adapter for Twig.  Editors
launch the `twig-dap` binary; it speaks DAP over stdin/stdout to the
editor and the (newline-delimited JSON) VM debug protocol over TCP to
`twig-vm --debug-port <N>`.

### Added
- `TwigDebugAdapter::compile` — runs `twig_ir_compiler::compile_source`
  on the requested file, walks the resulting `IIRFunction::source_map`,
  and emits a `debug_sidecar` byte blob suitable for
  `dap_adapter_core::SidecarIndex`.  Returns the original source path
  as the "bytecode" arg (the `twig-vm` CLI takes Twig source directly —
  there's no separate bytecode artefact).
- `TwigDebugAdapter::launch_vm` — discovers the sibling `twig-vm`
  binary via `std::env::current_exe`'s parent (with `PATH` fallback)
  and spawns it with `--debug-port <PORT> <BYTECODE>`.
- Public `build_sidecar(module, source_path) -> Vec<u8>` helper.
- `find_sibling_binary(name)` — same-directory binary discovery,
  used by `launch_vm` and reusable by other Twig tooling.
- `bin/twig_dap.rs` real `main()` — `DapServer::new(TwigDebugAdapter)
  .run_stdio()`.

### Test coverage
- 8 unit tests for `compile` (sidecar round-trips, line-1 resolves,
  invalid input rejection, missing-file rejection, empty-module shape,
  metadata correctness).
- 1 **end-to-end smoke test** (`tests/end_to_end.rs`) — spawns the real
  `twig-vm` binary in debug mode, connects via `TcpVmConnection`,
  walks through entry stop → set_breakpoint → continue → exited.  Skips
  gracefully when the binary isn't built.

### Dependencies
- `twig-ir-compiler` (workspace path) — for `compile_source`.
- `interpreter-ir` (workspace path) — for `IIRModule` / `SourceLoc`.
- `debug-sidecar` (workspace path) — for `DebugSidecarWriter`.
- `tempfile` (dev-only) — for tests.

### Known limitations
- Variable inspection (DAP `variables` request) returns empty: Twig's
  IIR doesn't yet carry user-variable-name → register-index mappings in
  the sidecar.  Stepping, breakpoints, and stack traces all work.

## 0.1.0 — 2026-05-04

Initial skeleton. Spec, types, and module structure committed.
Implementation stubs in place with detailed inline TODO guides.
See spec `LS02-grammar-driven-language-server.md` / `LS03-dap-adapter-core.md`.
