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

## Status — SKELETON (LS03 PR B)

`TwigDebugAdapter` is defined with stub implementations.  The binary exits
with an error until LS03 PR A (`dap-adapter-core`) is implemented.

**TODO (LS03 PR B):**
1. Implement `compile()` — run `twig-ir-compiler`, write bytecode to temp
   file, return sidecar bytes.
2. Implement `launch_vm()` — spawn `twig-vm --debug-port <port> <bytecode>`.
3. Implement `twig_dap` main — construct `DapServer::new(TwigDebugAdapter)`
   and call `run_stdio()`.

## Spec reference

`code/specs/LS03-dap-adapter-core.md` and `code/specs/05e-debug-adapter.md`
