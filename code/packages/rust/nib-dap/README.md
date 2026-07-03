# `nib-dap` — Nib Debug Adapter Protocol adapter

The Nib-language counterpart to `twig-dap` and `basic-dap`.  Wires
`nib-iir-compiler` and the `vm-debug` substrate into
`dap-adapter-core` so VS Code (and any other DAP editor) can debug
Nib programs with breakpoints, single-step, the variables panel,
and the call-stack view.

## Architecture

```text
Editor (VS Code / Neovim / …)
    │  DAP / JSON over stdio
    ▼
nib-dap binary  (bin/nib_dap.rs in this crate)
    │  DapServer::new(NibDebugAdapter).run_stdio()
    ▼
dap-adapter-core  (DAP message handling, breakpoints, stepping)
    │  NibDebugAdapter::{compile, launch_vm}
    ▼
nib-vm --debug-port N  (sibling binary; speaks the vm-debug protocol)
```

`nib-dap` does **not** depend on `twig-vm`.  Same substrate as the
other per-language DAP crates (`dap-adapter-core` editor-side,
`vm-debug` VM-side); otherwise independent.

## What's wired (V1)

- `compile`: runs `nib_iir_compiler::compile_source`, then walks the
  resulting `IIRFunction::source_map` to emit a `debug-sidecar`
  byte blob.  Source-loc threading lives in `nib-iir-compiler`
  0.6.0 (NIB06).
- `launch_vm`: spawns the sibling `nib-vm` binary with
  `--debug-port <port>`.  `nib-vm` itself is on the roadmap; this
  crate is the editor-side surface.

## Configuration in VS Code

```json
{
  "type": "nib",
  "request": "launch",
  "name": "Debug Nib file",
  "program": "${file}"
}
```

## Versions

- `0.1.0` — initial release.

See [CHANGELOG.md](./CHANGELOG.md) for details.
