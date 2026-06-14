# `dap-adapter-core`

Generic Debug Adapter Protocol library for language VMs.

## What it does

`dap-adapter-core` implements the full DAP protocol layer — message framing,
breakpoint management, stepping algorithms, sidecar-based source mapping, and
VM TCP communication — as a reusable library.

Language authors implement two methods and get a complete debugger:

```rust
pub trait LanguageDebugAdapter: Send + Sync + 'static {
    /// Compile source → (bytecode_path, sidecar_bytes).
    fn compile(&self, source: &Path, workspace: &Path)
        -> Result<(PathBuf, Vec<u8>), String>;

    /// Spawn the VM in debug mode on the given port.
    fn launch_vm(&self, bytecode: &Path, port: u16)
        -> Result<std::process::Child, String>;

    fn language_name(&self)       -> &'static str;
    fn file_extensions(&self)     -> &'static [&'static str];
}
```

## Architecture

```
Editor (VS Code, …)
   │ DAP (JSON over stdio)
   ▼
DapServer<A: LanguageDebugAdapter>   ← this crate
   ├── DapProtocol     — message framing + JSON
   ├── BreakpointManager — source-line → bytecode-offset set
   ├── StepController  — step-over / step-in / step-out
   ├── SidecarIndex    — offset ↔ source-line (O(1) lookups)
   └── VmConnection    — TCP client for VM Debug Protocol
   │  calls LanguageDebugAdapter
   ▼
impl LanguageDebugAdapter (e.g. TwigDebugAdapter in twig-dap)
   │ VM Debug Protocol (TCP)
   ▼
VM (e.g. twig-vm --debug-port N)
```

## Stepping algorithms

| Command | Algorithm |
|---|---|
| `next` (step-over) | Send `step_instruction` until call_depth returns to entry level |
| `stepIn` | Send `step_instruction` once; stop at any new source line |
| `stepOut` | Send `step_instruction` until call_depth decreases |

All algorithms use the `SidecarIndex` to translate VM offsets to editor
source lines.

## Status — LS03 PR A complete (0.2.0)

All 14 DAP request handlers, three stepping algorithms, sidecar indexing,
and a scripted mock VM are implemented.  74 unit tests + one end-to-end
integration test cover the full launch → setBreakpoints → stopped →
stackTrace → variables → continue → terminated flow.

The crate ships two `VmConnection` implementations:
- `MockVmConnection` — scripted, in-memory (used by tests).
- `TcpVmConnection` — real client built on the in-repo `tcp-client` (NET01)
  crate.  Speaks newline-delimited JSON; will plug into `twig-vm` once it
  grows a `--debug-port` flag (LS03 PR C).

## Crate layout

| File | Purpose |
|---|---|
| `src/adapter.rs` | `LanguageDebugAdapter` trait |
| `src/server.rs` | `DapServer<A>` — top-level event loop + request dispatch |
| `src/protocol.rs` | DAP message framing (Content-Length headers) |
| `src/breakpoints.rs` | `BreakpointManager` — source-line ↔ offset set |
| `src/stepper.rs` | `StepController` — stepping state machine |
| `src/sidecar.rs` | `SidecarIndex` — wraps debug-sidecar for O(1) lookups |
| `src/vm_conn.rs` | `VmConnection` — TCP client for VM Debug Protocol |

## Spec reference

`code/specs/LS03-dap-adapter-core.md` and `code/specs/05e-debug-adapter.md`
