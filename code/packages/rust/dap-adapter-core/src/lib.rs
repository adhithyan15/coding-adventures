//! # `dap-adapter-core` — Generic Debug Adapter Protocol adapter.
//!
//! **LS03 PR A** — Generic DAP adapter library.  Implements all stepping
//! algorithms, breakpoint management, sidecar-based offset↔source resolution,
//! and DAP message handling.  Language authors implement the thin
//! [`LanguageDebugAdapter`] trait (compile + launch) to get a full debugger.
//!
//! ## Architecture (see spec LS03 + 05e for full detail)
//!
//! ```text
//! Editor (VS Code, …)
//!    │ DAP (JSON over stdio)
//!    ▼
//! DapServer<A: LanguageDebugAdapter>   ← this crate
//!    ├── DapProtocol     — message framing + JSON
//!    ├── BreakpointManager — source-line → offset set
//!    ├── StepController  — step-over / step-in / step-out (spec 05e)
//!    ├── SidecarIndex    — offset ↔ source lookups
//!    └── VmConnection    — TCP client for VM Debug Protocol
//!    │ calls LanguageDebugAdapter
//!    ▼
//! impl LanguageDebugAdapter   (e.g. TwigDebugAdapter in twig-dap)
//!    compile(source) → (bytecode, sidecar_bytes)
//!    launch_vm(bytecode, debug_port) → Child
//!    │ VM Debug Protocol (TCP)
//!    ▼
//! VM (twig-vm --debug-port N)
//! ```
//!
//! ## Status — SKELETON (LS03 PR A)
//!
//! Types and module structure are defined. Implementations are TODO stubs.
//!
//! ## Prerequisites before implementing LS03 PR A
//!
//! 1. Verify the VM Debug Protocol is fully implemented in twig-vm.
//!    File: code/packages/rust/twig-vm/src/debug_server.rs (or similar)
//!    Commands needed: set_breakpoint, continue, step_instruction,
//!                     get_call_stack, get_slot
//!    Events needed: stopped, exited
//!    Spec reference: 05e §"VM Debug Protocol"
//!
//! 2. Verify debug-sidecar query API.
//!    File: code/packages/rust/debug-sidecar/src/lib.rs (or Python equivalent)
//!    Methods needed: offset_to_source(offset) → (file, line, col)
//!                    source_to_offsets(file, line) → Vec<offset>
//!    Spec reference: 05d §"Query API"
//!
//! 3. Add debug-sidecar to Cargo.toml deps once the above is verified.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod adapter;
pub mod server;
pub mod protocol;
pub mod breakpoints;
pub mod stepper;
pub mod sidecar;
pub mod vm_conn;

pub use adapter::LanguageDebugAdapter;
pub use server::DapServer;
pub use sidecar::SidecarIndex;
