# Changelog — `vm-debug`

## 0.1.0 — 2026-05-30 (VMDEBUG01 — extraction from twig-vm)

Initial release.  Lifts the generic debug substrate out of `twig-vm`
0.22.0 so per-language DAP adapters can depend on it without pulling
in twig-vm itself.

### What's here

- `DebugFrame` trait — two methods (`register_names`,
  `read_register`) — that abstracts over a VM's local-frame data.
- `DebugHooks` trait — `before_instruction(fn_name, depth, pc, &dyn DebugFrame)`
  + a default-no-op `on_function_exit`.  This is what dispatchers
  call on every safepoint.
- `StopReason` enum (`Breakpoint`, `Step`, `Pause`, `Entry`) — the
  wire-format reasons the VM sends in `{event:"stopped"}` events.
- `DebugServer` — TCP-backed production `DebugHooks` impl.  Speaks
  the newline-delimited JSON wire protocol documented in
  `dap-adapter-core::vm_conn`.  Tracks breakpoints, single-step
  flag, pause flag, call stack, last frame's registers.
- `MAX_LINE_BYTES` constant + `read_line_capped` helper — DoS guard
  preventing a malicious peer from forcing unbounded `String` growth
  by streaming bytes without a newline.

### Migrating from `twig-vm`

`twig-vm` previously exposed `twig_vm::debug::{DebugHooks, FrameView}`
and `twig_vm::debug_server::{DebugServer, StopReason, MAX_LINE_BYTES}`.

After this release:

- `twig_vm::debug::FrameView` still exists (it's twig-vm's concrete
  frame view) but now `impl vm_debug::DebugFrame for FrameView<'_>`.
- `twig_vm::debug::DebugHooks` is now a re-export of
  `vm_debug::DebugHooks`.
- `twig_vm::debug_server::{DebugServer, StopReason, MAX_LINE_BYTES}`
  are now re-exports of the `vm_debug` types.

Source-compatible for downstream users that referenced the twig-vm
re-exports (no `use vm_debug::*` required for existing code).

### Tests

The `DebugServer`'s direct-loopback tests, the call-stack-tracking
test, and the `read_line_capped` DoS-guard tests all move with the
code — no test coverage is lost in the extraction.

### Compared to alternatives

- **Polymorphic `DebugHooks<F: DebugFrame>`** would push the frame
  type into every signature site.  Reaching for `&dyn DebugFrame`
  keeps the trait object-safe and `Box<dyn DebugHooks>`-friendly,
  which matters when the host wires up the hook from a CLI flag at
  runtime.  Mono-cost per VM is negligible — the hook is called
  once per IIR instruction; a vtable jump is well below the cost of
  the instruction itself.
- **Keep it in `twig-vm` and let other VMs depend on twig-vm** —
  ruled out: twig-vm transitively pulls in `lispy-runtime`,
  `twig-ir-compiler`, the whole Lispy heap model.  A 1500-line
  Brainfuck VM does not need that surface to host a debug server.
