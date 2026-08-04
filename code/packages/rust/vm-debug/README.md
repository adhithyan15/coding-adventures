# `vm-debug` — language-agnostic debug substrate for IIR-based VMs

The shared debug substrate used by every LANG-VM-based interpreter:
`twig-vm` today, the upcoming `basic-vm` / `nib-vm` / `oct-vm`
tomorrow.  Lets per-language DAP adapters (`twig-dap` and the
forthcoming `basic-dap` / `nib-dap` / `oct-dap`) depend on a single
substrate without each one re-implementing the wire protocol or
dragging in another language's VM.

## What's here

| Item            | What it does |
|-----------------|--------------|
| `DebugFrame`    | Trait abstracting per-VM frame access (`register_names`, `read_register`). |
| `DebugHooks`    | Trait the dispatcher calls at every safepoint.  `before_instruction` takes `&dyn DebugFrame`. |
| `StopReason`    | Wire-format-aligned enum (`Breakpoint`, `Step`, `Pause`, `Entry`). |
| `DebugServer`   | TCP-backed production hook that speaks the newline-delimited JSON protocol documented at the top of [`dap-adapter-core::vm_conn`](../dap-adapter-core/src/vm_conn.rs). |
| `MAX_LINE_BYTES`| Hard cap on a single wire-protocol line (DoS guard). |

The trait set is intentionally minimal — anything a debugger needs to
do beyond "step / continue / read a variable" lives in
`dap-adapter-core` (per-IDE adapter side) and per-language DAP crates
(language-specific stack frames + value renderers).

## How a VM plugs in

A VM that wants debugger support:

1. Wraps its frame type in a `FrameView` (or similar) and impls
   `vm_debug::DebugFrame` for it — two methods.
2. Calls `vm_debug::DebugHooks::before_instruction(...)` between every
   instruction it executes.
3. Constructs a `vm_debug::DebugServer` when its CLI sees
   `--debug-port N`, passes it as the `&mut dyn DebugHooks` to its
   dispatcher.

That's it — the wire protocol, breakpoint set, single-step flag,
pause flag, call-stack reconstruction, and DoS-guarded I/O all live
in this crate.

## How a DAP adapter plugs in

`{language}-dap` (e.g. `twig-dap`, `basic-dap`, `nib-dap`, `oct-dap`)
each depend on `dap-adapter-core` for the DAP-side wire protocol
plumbing and on **this crate** for the matching VM-side
wire-protocol message shapes (`StopReason`, the `{event:"stopped"}`
JSON, etc.).  No per-language DAP needs to depend on `twig-vm`
itself.

## Why a separate crate

Before this crate existed, `DebugHooks` and `DebugServer` lived in
`twig-vm`.  Adding `basic-dap` / `nib-dap` / `oct-dap` would have
forced each to either:

- pull in `twig-vm` (the whole Twig→IIR→Lispy stack including
  `dynval-runtime` and `twig-ir-compiler`), or
- copy-paste the 600+-line `DebugServer` impl.

Neither is sustainable.  Extracting the substrate into one
generic-over-`DebugFrame` crate is the way.

## Versions

- `0.1.0` — initial extraction from `twig-vm` 0.22.0.

See [CHANGELOG.md](./CHANGELOG.md) for details.
