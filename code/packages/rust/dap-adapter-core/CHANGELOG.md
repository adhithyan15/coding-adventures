# Changelog — `dap-adapter-core`

## 0.2.0 — 2026-05-04

**LS03 PR A — Full generic DAP adapter implementation.**

Replaces the 0.1.0 skeleton with a complete, end-to-end testable DAP server.
Language authors get a fully-functional debugger by implementing the
~25-line [`LanguageDebugAdapter`] trait.

### Architectural choices

- **VM offset model.**  Execution points are represented as
  [`VmLocation { function, instr_index }`] rather than a global flat `u64`
  offset.  This matches how `debug-sidecar` actually indexes (per-function
  instruction tables) and how the future VM Debug Protocol's call-stack
  frames will be shaped.

- **`VmConnection` as a trait.**  The VM wire protocol is an interface, not
  a concrete TCP client.  The crate ships two implementations:
  - [`MockVmConnection`] — scripted, in-memory; the entire `DapServer` is
    unit-tested against it.
  - [`TcpVmConnection`] — real TCP client built on the in-repo `tcp-client`
    (NET01) crate, speaking newline-delimited JSON.  Documented protocol;
    `twig-vm` will implement the server side in a future PR.

- **`BreakpointManager` is decoupled from `VmConnection`.**  `set_breakpoints`
  returns a [`BreakpointDiff`] (lists of locations to clear / install) that
  the server applies to the VM.  This keeps the manager testable without a
  mock and side-steps a `dyn` lifetime variance issue.

- **Single-threaded event loop.**  `DapServer::run` reads DAP requests
  blocking, drains VM events at the top of every iteration and after every
  response.  Async event delivery latency is bounded by one DAP round-trip,
  which is fine for editor-driven debugging.

### Implemented modules

| Module          | What it does                                                |
|-----------------|-------------------------------------------------------------|
| `protocol.rs`   | Content-Length framing, `read_message` / `write_message`, `SeqCounter`, response/event builders |
| `adapter.rs`    | `LanguageDebugAdapter` trait (compile + launch_vm hooks)    |
| `sidecar.rs`    | `SidecarIndex` wrapping `debug_sidecar::DebugSidecarReader`; `(file, line) ↔ VmLocation` lookups |
| `breakpoints.rs`| `BreakpointManager` + `BreakpointDiff`                      |
| `stepper.rs`    | `StepController` + `StepMode` + `StepDecision` — pure step-over/in/out state machine (spec 05e) |
| `vm_conn.rs`    | `VmConnection` trait, `VmLocation`, `VmFrame`, `StoppedEvent`, `MockVmConnection`, `TcpVmConnection`, `TcpConnectOptions` |
| `server.rs`     | `DapServer<A>` with all 14 request handlers (initialize, launch, setBreakpoints, configurationDone, continue, pause, next, stepIn, stepOut, stackTrace, scopes, variables, source, threads, disconnect) and the `run` event loop |

### Test coverage — 74 tests

- **protocol** (12): framing round-trips, EOF handling, Content-Length
  validation, response/event builders, request parsing, `SeqCounter`.
- **vm_conn** (15): `VmLocation` equality, `MockVmConnection` set/clear/
  step/stack/slot/event semantics, shared-state cloning, `parse_event_value`
  for stopped + exited + unknown shapes, TcpVmConnection budget honoured
  on dead-port connect.
- **sidecar** (9): build from bytes, garbage rejection, single-function and
  cross-function source-to-locs lookups, fallback DWARF-style lookup,
  `source_files`, `reader` accessor.
- **breakpoints** (8): empty manager, install/verify, unverified lines,
  re-set diff, file-clear, hit detection, sidecar-less mode.
- **stepper** (14): step-over/in/out across same-depth, deeper, shallower,
  same-line and different-line scenarios; cancel; idle no-op; sequencing.
- **server** (10): handler unit tests for initialize, launch, setBreakpoints,
  configurationDone, continue/pause/step dispatch, stackTrace, scopes,
  variables, threads, disconnect; **plus one full integration test** that
  drives setBreakpoints → configurationDone → stopped → stackTrace →
  continue → exited → terminated end-to-end with mocks.

### Dependencies added

- `debug-sidecar` (workspace path) — wrapped by `SidecarIndex`.
- `tcp-client` (NET01) — used by `TcpVmConnection`.

### What's deferred to LS03 PR B / PR C

- Concrete `twig-vm` debug server (no `--debug-port` flag exists yet).
  When it lands, `TcpVmConnection` plugs in unchanged.
- Conditional breakpoints, watch expressions (Phase 2).
- Multi-thread debugging.

## 0.1.0 — 2026-05-04

Initial skeleton. Spec, types, and module structure committed.
Implementation stubs in place with detailed inline TODO guides.
See spec `LS02-grammar-driven-language-server.md` / `LS03-dap-adapter-core.md`.
