# `basic-dap` — Dartmouth BASIC Debug Adapter Protocol adapter

The BASIC-language counterpart to `twig-dap`.  Wires
`dartmouth-basic-iir-compiler` and the `vm-debug` substrate into
`dap-adapter-core` so VS Code (and any other DAP editor) can debug
BASIC programs with breakpoints, single-step, the variables panel,
and the call-stack view.

## Architecture

```text
Editor (VS Code / Neovim / …)
    │  DAP / JSON over stdio
    ▼
basic-dap binary  (bin/basic_dap.rs in this crate)
    │  DapServer::new(BasicDebugAdapter).run_stdio()
    ▼
dap-adapter-core  (DAP message handling, breakpoints, stepping)
    │  BasicDebugAdapter::{compile, launch_vm}
    ▼
basic-vm --debug-port N  (sibling binary; speaks the vm-debug protocol)
```

`basic-dap` does **not** depend on `twig-vm`.  It uses the same
substrate `twig-dap` does (`dap-adapter-core` editor-side,
`vm-debug` VM-side) but is otherwise an independent crate — adding
language-specific DAP support didn't require lifting twig-vm into
the dependency graph of every editor's adapter.

## What's wired (V1)

- `compile`: runs `dartmouth_basic_iir_compiler::compile_source`,
  then walks the resulting `IIRFunction::source_map` to emit a
  `debug-sidecar` byte blob — the same shape `twig-dap` produces.
  Source-loc threading lives in
  `dartmouth-basic-iir-compiler` 0.4.0 (BASIC05).
- `launch_vm`: spawns the sibling `basic-vm` binary with
  `--debug-port <port>`.  `basic-vm` itself is on the roadmap; this
  crate is the editor-side surface that lets us land DAP support
  before the VM binary lands.

## Configuration in VS Code

```json
{
  "type": "basic",
  "request": "launch",
  "name": "Debug BASIC file",
  "program": "${file}"
}
```

## Versions

- `0.1.0` — initial release.

See [CHANGELOG.md](./CHANGELOG.md) for details.
