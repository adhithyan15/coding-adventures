//! Re-export of the generic [`vm_debug::DebugServer`] for backward
//! compatibility with consumers that wrote
//! `twig_vm::debug_server::DebugServer`.
//!
//! ## VMDEBUG01 extraction
//!
//! Pre-`twig-vm` 0.23.0, this module contained the full TCP-backed
//! debug server (~620 lines).  The implementation was generalised
//! and moved to the [`vm_debug`] crate so per-language DAP adapters
//! (`twig-dap`, the upcoming `basic-dap` / `nib-dap` / `oct-dap`)
//! can depend on a single shared substrate without pulling in
//! `twig-vm`'s full Twig→IIR→Lispy stack.
//!
//! Existing code that wrote
//!
//! ```text
//! use twig_vm::debug_server::{DebugServer, StopReason, MAX_LINE_BYTES};
//! ```
//!
//! continues to compile unchanged — they're re-exported here.  New
//! code should depend on `vm-debug` directly.

pub use vm_debug::{DebugServer, StopReason, MAX_LINE_BYTES};
