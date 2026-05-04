# LS03 — DAP Adapter Core

> **Depends on**: [`05e`](05e-debug-adapter.md) (full DAP architecture spec),
> [`05d`](05d-debug-sidecar-format.md) (sidecar binary format),
> [`LANG13`](LANG13-debug-sidecar.md) (compiler sidecar writer API),
> [`LANG14`](LANG14-native-debug-info.md) (DWARF / CodeView emitters),
> [`LANG06`](LANG06-debug-integration.md) (debug integration plan)

`05e` fully specifies the Debug Adapter Protocol architecture for this VM
stack. This spec describes the **implementation plan**: the generic
`dap-adapter-core` crate that provides all stepping algorithms, breakpoint
management, and DAP message handling — plus the thin per-language
`LanguageDebugAdapter` trait that a language author implements to get full
DAP support.

---

## Motivation

`05e` establishes three components:
1. The VM Debug Protocol (VM-side TCP server).
2. The debug adapter (bridges DAP ↔ VM Debug Protocol using the sidecar).
3. The VS Code extension (launches the adapter as a subprocess).

The adapter described in `05e` is written against a specific VM. This spec
extracts the language-agnostic portions into a reusable library so that:
- Twig gets a working debugger.
- Any future language built on the generic VM stack gets a debugger by
  implementing ≤ 20 lines of Rust (compile + launch hooks).

The stepping algorithms, breakpoint management, DAP request/response
serialisation, and sidecar queries are all language-independent. Only
"compile this source file" and "launch this VM with this bytecode" are
language-specific.

---

## Architecture

```
Editor (VS Code, Neovim, …)
       │  DAP (JSON over stdio)
       ▼
dap-adapter-core::DapServer          ← NEW (this spec)
  ├── DapProtocol                    DAP message parsing + serialisation
  ├── BreakpointManager              set/clear/hit-test breakpoints
  ├── StepController                 step-over / step-in / step-out algorithms
  ├── VariableResolver               slot inspection via sidecar
  ├── StackTraceBuilder              call stack + frame list via sidecar
  └── SidecarIndex                   fast offset↔source lookups
       │  calls LanguageDebugAdapter trait
       ▼
impl LanguageDebugAdapter for TwigDebugAdapter   ← twig-dap crate (NEW)
  compile(source_path) → (bytecode_path, sidecar_bytes)
  launch_vm(bytecode_path, debug_port) → Child process handle
       │  VM Debug Protocol (TCP)
       ▼
VM (twig-vm --debug-port <N>)        ← already implemented
```

### What is generic vs. per-language

| Concern | Generic (dap-adapter-core) | Per-language (LanguageDebugAdapter) |
|---------|---------------------------|--------------------------------------|
| DAP message parsing | ✅ | |
| `initialize` + capabilities | ✅ | |
| `setBreakpoints` | ✅ | |
| `configurationDone` | ✅ | |
| `launch` | hooks into → | `compile()` + `launch_vm()` |
| `continue` / `pause` | ✅ | |
| `next` (step-over) | ✅ | |
| `stepIn` | ✅ | |
| `stepOut` | ✅ | |
| `stackTrace` | ✅ | |
| `scopes` + `variables` | ✅ | |
| `source` | ✅ | |
| `disconnect` | ✅ | |
| Offset ↔ source translation | ✅ (via SidecarIndex) | |
| Compile source → bytecode | | ✅ `compile()` |
| Launch VM subprocess | | ✅ `launch_vm()` |
| VM-specific debug port flag | | ✅ `debug_port_flag()` |

---

## `LanguageDebugAdapter` trait

```rust
/// The per-language contract for the debug adapter.
///
/// A language author implements this trait (≤ 20 lines) and passes it to
/// `DapServer::new()`. The rest of the debug adapter is generic.
pub trait LanguageDebugAdapter: Send + Sync + 'static {
    /// Compile the source file at `source_path` to bytecode.
    ///
    /// Returns:
    /// - `bytecode_path`: path to the compiled `.iir` / `.aot` bytecode file.
    /// - `sidecar_bytes`: the raw debug sidecar (offset ↔ source map).
    ///
    /// Called when the editor sends `launch`. The adapter compiles first,
    /// then launches the VM.
    fn compile(
        &self,
        source_path: &Path,
        workspace_root: &Path,
    ) -> Result<(PathBuf, Vec<u8>), String>;

    /// Launch the VM with the compiled bytecode in debug mode.
    ///
    /// The VM must start a TCP debug server on `debug_port` and pause
    /// (wait for CONTINUE) before executing. See spec `05e` §VM Debug Protocol.
    ///
    /// Returns a `Child` handle so the adapter can monitor the process and
    /// clean up on disconnect.
    fn launch_vm(
        &self,
        bytecode_path: &Path,
        debug_port: u16,
    ) -> Result<std::process::Child, String>;

    /// The name of the language, used in log messages and error reporting.
    fn language_name(&self) -> &'static str;

    /// File extensions this adapter handles (for editor registration).
    fn file_extensions(&self) -> &'static [&'static str];
}
```

---

## `DapServer` — the generic adapter

```rust
pub struct DapServer<A: LanguageDebugAdapter> {
    adapter: A,
    protocol: DapProtocol,     // message framing + JSON parsing
    bps: BreakpointManager,    // source-line → offset set
    stepper: StepController,   // step state machine
    sidecar: SidecarIndex,     // offset ↔ source lookups
    vm_conn: Option<VmConnection>,  // TCP connection to the VM
    vm_proc: Option<Child>,         // VM subprocess handle
}

impl<A: LanguageDebugAdapter> DapServer<A> {
    pub fn new(adapter: A) -> Self { ... }

    /// Run the adapter over stdio (the standard VS Code launch mode).
    pub fn run_stdio(self) -> Result<(), DapError> { ... }

    /// Run the adapter over a TCP connection (for testing).
    pub fn run_tcp(self, stream: TcpStream) -> Result<(), DapError> { ... }
}
```

The `run_stdio` / `run_tcp` methods drive the event loop:
1. Read a DAP message from the input stream.
2. Dispatch to the appropriate handler (see below).
3. Send the DAP response + any queued events.
4. Repeat until `disconnect` or VM exit.

### Request handlers

Each handler is a method on `DapServer`:

| DAP request | Handler | Notes |
|-------------|---------|-------|
| `initialize` | `handle_initialize` | Returns capabilities; always the same |
| `launch` | `handle_launch` | Calls `adapter.compile()` then `adapter.launch_vm()` |
| `setBreakpoints` | `handle_set_breakpoints` | Translates source lines → offsets via sidecar |
| `configurationDone` | `handle_configuration_done` | Sends CONTINUE to VM |
| `continue` | `handle_continue` | Forwards CONTINUE to VM |
| `pause` | `handle_pause` | Forwards PAUSE to VM |
| `next` (step-over) | `handle_next` | Runs step-over algorithm from `05e` |
| `stepIn` | `handle_step_in` | Runs step-in algorithm from `05e` |
| `stepOut` | `handle_step_out` | Runs step-out algorithm from `05e` |
| `stackTrace` | `handle_stack_trace` | Queries VM call stack; resolves via sidecar |
| `scopes` | `handle_scopes` | Returns local + global scope frames |
| `variables` | `handle_variables` | Queries VM slots; resolves names via sidecar |
| `source` | `handle_source` | Returns source file content |
| `disconnect` | `handle_disconnect` | Kills VM process; cleans up |
| `terminated` event | emitted when VM exits | Detected via process monitor thread |

### Stepping algorithms (from `05e`)

The stepping algorithms are implemented exactly as specified in `05e`. They
are self-contained in `StepController` and depend only on:
- The sidecar (`SidecarIndex`) for offset → source-line resolution.
- The VM connection (`VmConnection`) for `step_instruction` / `get_call_stack`
  commands.
- The breakpoint set for "did we hit a user breakpoint?" checks.

No per-language hooks are needed for stepping.

---

## `SidecarIndex`

```rust
/// Fast index over a debug sidecar for offset ↔ source-line lookups.
///
/// Built from the raw sidecar bytes produced by the compiler.
/// Wraps the `debug-sidecar` query API with caching.
pub struct SidecarIndex {
    inner: debug_sidecar::SidecarReader,
    offset_to_line_cache: HashMap<u64, (PathBuf, u32, u32)>,
    line_to_offsets_cache: HashMap<(PathBuf, u32), Vec<u64>>,
}

impl SidecarIndex {
    pub fn from_bytes(sidecar_bytes: &[u8]) -> Result<Self, String>;

    /// Resolve an instruction offset to (source_file, line, column).
    pub fn offset_to_source(&self, offset: u64)
        -> Option<(PathBuf, u32, u32)>;

    /// Resolve a source line to the set of instruction offsets on that line.
    pub fn source_to_offsets(&self, file: &Path, line: u32)
        -> Vec<u64>;
}
```

---

## Crate layout

```
code/packages/rust/dap-adapter-core/
├── Cargo.toml          version = "0.1.0"
│                       deps: debug-sidecar, serde, serde_json, tokio (async I/O)
├── README.md
├── CHANGELOG.md
└── src/
    ├── lib.rs          pub use server::*, adapter::*, protocol::*, sidecar::*
    ├── adapter.rs      LanguageDebugAdapter trait
    ├── server.rs       DapServer<A> + event loop
    ├── protocol.rs     DAP message framing, JSON de/serialise
    ├── breakpoints.rs  BreakpointManager
    ├── stepper.rs      StepController (step-over/in/out algorithms)
    ├── variables.rs    VariableResolver (slot inspection)
    ├── stack.rs        StackTraceBuilder
    ├── sidecar.rs      SidecarIndex
    └── vm_conn.rs      VmConnection (TCP client for VM Debug Protocol)
```

```
code/packages/rust/twig-dap/
├── Cargo.toml          version = "0.1.0"
│                       deps: dap-adapter-core, twig-ir-compiler
├── README.md
├── CHANGELOG.md
├── src/
│   ├── lib.rs          pub struct TwigDebugAdapter;  impl LanguageDebugAdapter
│   └── adapter.rs      compile() invokes twig-ir-compiler;
│                       launch_vm() spawns twig-vm --debug-port <N>
└── bin/
    └── twig-dap.rs     main() → DapServer::new(TwigDebugAdapter).run_stdio()
```

---

## PR sequence

### PR LS03-A — `dap-adapter-core` crate

**Scope**: Generic crate only; no Twig wiring.  Tests use a mock
`LanguageDebugAdapter` that produces canned bytecode + sidecar.

**Deliverables**:
- All modules listed in the crate layout above.
- `LanguageDebugAdapter` trait.
- `DapServer<A>` with all 13 request handlers.
- `SidecarIndex` wrapping `debug-sidecar`.
- `StepController` implementing the three stepping algorithms from `05e`.
- Unit tests: each request handler round-trips a DAP message pair via
  `run_tcp` against a mock VM TCP server.
- Integration test: full `launch → setBreakpoints → configurationDone →
  stopped event → stackTrace → variables → continue → terminated event`
  sequence using a mock adapter and VM.

**Acceptance criteria**:
- `cargo test -p dap-adapter-core` passes.
- The integration test drives the full launch→stop→inspect→continue flow
  and asserts the correct DAP events are emitted.
- The `SidecarIndex` correctly resolves offsets ↔ source lines for the
  test sidecar fixture.

### PR LS03-B — `twig-dap` + server binary

**Scope**: Twig instantiation + runnable DAP adapter binary.

**Deliverables**:
- `TwigDebugAdapter` implementing `LanguageDebugAdapter`.
  - `compile()`: runs `twig-ir-compiler` on the source path; returns bytecode
    + sidecar bytes.
  - `launch_vm()`: spawns `twig-vm --debug-port <N> <bytecode>`.
- `twig-dap` binary (the adapter process VS Code launches).
- Smoke test: compile a 3-function Twig file, launch the adapter against the
  resulting bytecode, set a breakpoint on line 2, send `continue`, assert
  `stopped` event arrives.

**Acceptance criteria**:
- `cargo build -p twig-dap` produces a binary.
- The smoke test passes end-to-end (compile → launch → breakpoint → stopped).
- The adapter correctly reports variable values at the breakpoint.

---

## Risk register

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|-----------|
| `twig-vm` debug protocol not yet fully implemented | Medium | High | Audit `twig-vm`'s `--debug-port` path before LS03-A; stub if missing |
| `debug-sidecar` query API changed since LANG13 | Low | Medium | Read current API before coding `SidecarIndex` |
| DAP async I/O complexity (tokio vs threads) | Medium | Medium | Use synchronous threads first; async is an optimisation |
| VM process lifecycle races (slow startup) | Low | Medium | Adapter retries TCP connect with exponential backoff (max 5s) |
| Stepping over tail calls produces wrong stack depth | Low | Low | Known limitation; document it; fix in Phase 2 |

---

## Out of scope

- Conditional breakpoints (Phase 2).
- Watch expressions (Phase 2).
- Hot-reload / edit-and-continue (future).
- VS Code extension packaging (tracked in `LANG07`).
- DAP for JIT-compiled code (future; requires JIT sidecar).
- Multi-threaded debugging (future).

---

## Acceptance criteria

1. Any language can get a DAP adapter by implementing `LanguageDebugAdapter`
   (≤ 20 lines: `compile()` + `launch_vm()`).
2. The Twig DAP adapter (`twig-dap`) correctly handles the full
   launch → breakpoint → step → inspect → continue → exit flow.
3. The stepping algorithms from `05e` are implemented and tested against the
   mock VM.
4. `SidecarIndex` offset ↔ source round-trips correctly for a known fixture.
