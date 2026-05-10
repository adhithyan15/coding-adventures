//! # `dap-adapter-core` — Generic Debug Adapter Protocol adapter.
//!
//! **LS03 PR A** — Generic DAP adapter library.  Implements all stepping
//! algorithms, breakpoint management, sidecar-based offset↔source resolution,
//! and DAP message handling.  Language authors implement the thin
//! [`LanguageDebugAdapter`] trait (compile + launch) to get a full debugger.
//!
//! ## Architecture
//!
//! ```text
//! Editor (VS Code / Neovim / …)
//!    │  DAP (JSON over stdio)
//!    ▼
//! DapServer<A: LanguageDebugAdapter>   ← this crate
//!    │
//!    ├── protocol::{read_message, write_message}   — Content-Length framing
//!    ├── BreakpointManager                          — file → lines map
//!    ├── StepController                             — step state machine
//!    ├── SidecarIndex (wraps debug_sidecar)         — offset ↔ source
//!    └── VmConnection (trait, mockable)             — wire to VM debugger
//!
//!    │  via LanguageDebugAdapter
//!    ▼
//! impl LanguageDebugAdapter   (e.g. TwigDebugAdapter in twig-dap)
//!    compile(source) → (bytecode, sidecar_bytes)
//!    launch_vm(bytecode, debug_port) → Child + Box<dyn VmConnection>
//! ```
//!
//! ## Offset model
//!
//! Throughout the DAP layer we identify a VM execution point as a
//! [`VmLocation`] — the pair `(function_name, instr_index)`.  This matches
//! the per-function instruction indexing already used by the
//! `debug-sidecar` crate, and is also the natural shape for the planned
//! VM Debug Protocol (`{fn: "foo", offset: 5}` JSON frames).
//!
//! ## Wire protocol abstraction
//!
//! [`vm_conn::VmConnection`] is a **trait**, not a concrete TCP client.
//! This lets us:
//! - Test `DapServer` without a real VM by passing a [`vm_conn::MockVmConnection`].
//! - Defer the concrete TCP implementation to a later PR once `twig-vm`
//!   actually grows a debug server.
//!
//! ## Status — LS03 PR A complete
//!
//! All 13 DAP request handlers, the three stepping algorithms, sidecar
//! indexing, and a scripted mock VM are implemented.  37+ unit tests plus
//! one end-to-end integration test exercise the full
//! launch → setBreakpoints → configurationDone → stopped → stackTrace →
//! variables → continue → terminated flow.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod adapter;
pub mod breakpoints;
pub mod protocol;
pub mod server;
pub mod sidecar;
pub mod stepper;
pub mod vm_conn;

pub use adapter::LanguageDebugAdapter;
pub use breakpoints::BreakpointManager;
pub use server::DapServer;
pub use sidecar::SidecarIndex;
pub use stepper::{StepController, StepMode};
pub use vm_conn::{
    MockVmConnection, StoppedEvent, StoppedReason, TcpConnectOptions, TcpVmConnection,
    VmConnection, VmFrame, VmLocation,
};
