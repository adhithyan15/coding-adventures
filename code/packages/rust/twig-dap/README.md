# `twig-dap`

Twig Debug Adapter Protocol server built on top of `dap-adapter-core`.

## What it does

Provides `TwigDebugAdapter` (implementing `LanguageDebugAdapter`) and the
`twig-dap` binary.  All DAP protocol logic lives in `dap-adapter-core` —
this crate only knows how to compile Twig source and spawn `twig-vm`.

## Stack position

```
Editor (VS Code, …)
    │  DAP (JSON over stdio)
    ▼
twig-dap  ←  binary in this crate
    │
    ▼
dap-adapter-core  (all DAP logic: breakpoints, stepping, sidecar)
    │
    ▼
twig-vm  (spawned as subprocess, connects back on --debug-port)
```

## Configuring VS Code

Add to `.vscode/launch.json`:

```json
{
  "type": "twig",
  "request": "launch",
  "name": "Debug Twig file",
  "program": "${file}"
}
```

Install the Twig VS Code extension (separate package) to register the
`"type": "twig"` debug type and point it at the `twig-dap` binary.

## How it works

1. VS Code launches `twig-dap` as a subprocess.
2. `twig-dap` receives a `launch` DAP request with `program: /path/to/file.twig`.
3. `TwigDebugAdapter::compile()` runs `twig-ir-compiler` → IIR bytecode +
   debug sidecar.
4. `TwigDebugAdapter::launch_vm()` spawns `twig-vm --debug-port <N>
   <bytecode>`.
5. `dap-adapter-core` connects to the VM over TCP and proxies all DAP
   requests (breakpoints, step, continue, variables, call stack).

## Status — LS03 PR B complete (0.2.0)

`TwigDebugAdapter::compile()` runs `twig-ir-compiler` and emits a
real `debug-sidecar` blob; `TwigDebugAdapter::launch_vm()` spawns a
sibling `twig-vm` process.  An end-to-end smoke test in `tests/`
spawns `twig-vm --debug-port` and walks through the full
launch → breakpoint → continue → exited flow over TCP.

Variable inspection is the one feature still in flight — Twig's IIR
doesn't yet carry user-variable-name → register-index mapping that DAP's
`variables` panel needs.  Stepping, breakpoints, and stack traces all
work today.

## Spec reference

`code/specs/LS03-dap-adapter-core.md` and `code/specs/05e-debug-adapter.md`
