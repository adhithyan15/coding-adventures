# `oct-dap` — Oct Debug Adapter Protocol adapter

The Oct-language counterpart to `twig-dap`, `basic-dap`, and
`nib-dap`.  Wires `oct-iir-compiler` and the `vm-debug` substrate
into `dap-adapter-core` so VS Code (and any other DAP editor) can
debug Oct programs with breakpoints, single-step, the variables
panel, and the call-stack view.

## Architecture

```text
Editor (VS Code / Neovim / …)
    │  DAP / JSON over stdio
    ▼
oct-dap binary  (bin/oct_dap.rs in this crate)
    │  DapServer::new(OctDebugAdapter).run_stdio()
    ▼
dap-adapter-core  (DAP message handling, breakpoints, stepping)
    │  OctDebugAdapter::{compile, launch_vm}
    ▼
oct-vm --debug-port N  (sibling binary; speaks the vm-debug protocol)
```

`oct-dap` does **not** depend on `twig-vm`.  Same substrate as the
other per-language DAP crates (`dap-adapter-core` editor-side,
`vm-debug` VM-side); otherwise independent.

## What's wired (V1)

- `compile`: runs `oct_iir_compiler::compile_source`, then walks the
  resulting `IIRFunction::source_map` to emit a `debug-sidecar`
  byte blob.  Source-loc threading lives in `oct-iir-compiler`
  0.4.0 (OCT05).
- `launch_vm`: spawns the sibling `oct-vm` binary with
  `--debug-port <port>`.  `oct-vm` itself is on the roadmap; this
  crate is the editor-side surface.

## Configuration in VS Code

```json
{
  "type": "oct",
  "request": "launch",
  "name": "Debug Oct file",
  "program": "${file}"
}
```

## Versions

- `0.1.0` — initial release.

See [CHANGELOG.md](./CHANGELOG.md) for details.
