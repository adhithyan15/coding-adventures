//! # `twig-dap` — Twig Debug Adapter Protocol adapter.
//!
//! **LS03 PR B** — Twig instantiation of `dap-adapter-core`.  Implements
//! [`LanguageDebugAdapter`] for Twig and provides the `twig-dap` binary.
//!
//! ## Status — SKELETON (LS03 PR B, depends on LS03 PR A)
//!
//! Implement `dap-adapter-core` (LS03 PR A) first, then fill in:
//!
//! ### compile()
//!
//! ```rust,ignore
//! fn compile(&self, source_path: &Path, _workspace: &Path)
//!     -> Result<(PathBuf, Vec<u8>), String>
//! {
//!     // 1. Run twig-ir-compiler on source_path → IIR bytecode file.
//!     //    Check: code/packages/rust/twig-ir-compiler/src/lib.rs for API.
//!     //    The compiler should accept a source file path and return (iir_bytes, sidecar_bytes).
//!
//!     // 2. Write iir_bytes to a temp file → bytecode_path.
//!
//!     // 3. Return (bytecode_path, sidecar_bytes).
//!
//!     todo!()
//! }
//! ```
//!
//! ### launch_vm()
//!
//! ```rust,ignore
//! fn launch_vm(&self, bytecode_path: &Path, debug_port: u16)
//!     -> Result<std::process::Child, String>
//! {
//!     // Spawn: twig-vm --debug-port <debug_port> <bytecode_path>
//!     //
//!     // Prerequisites:
//!     // 1. Verify twig-vm accepts --debug-port flag.
//!     //    File: code/packages/rust/twig-vm/src/main.rs (or CLI entry)
//!     // 2. The binary must be findable (same dir as twig-dap, or on PATH).
//!     //    Use std::env::current_exe() to find sibling binaries.
//!
//!     std::process::Command::new("twig-vm")
//!         .arg("--debug-port").arg(debug_port.to_string())
//!         .arg(bytecode_path)
//!         .spawn()
//!         .map_err(|e| format!("failed to launch twig-vm: {e}"))
//! }
//! ```
//!
//! ## File locations to read before implementing
//!
//! - `code/packages/rust/twig-ir-compiler/src/lib.rs` — compiler API
//! - `code/packages/rust/twig-vm/src/main.rs` — VM CLI flags
//! - `code/specs/05e-debug-adapter.md` — VM Debug Protocol spec

use dap_adapter_core::LanguageDebugAdapter;
use std::path::{Path, PathBuf};

/// Debug adapter for Twig.
///
/// Compiles Twig source via `twig-ir-compiler` and launches `twig-vm`
/// in debug mode.
///
/// ## TODO — implement (LS03 PR B)
pub struct TwigDebugAdapter;

impl LanguageDebugAdapter for TwigDebugAdapter {
    fn compile(
        &self,
        _source_path: &Path,
        _workspace_root: &Path,
    ) -> Result<(PathBuf, Vec<u8>), String> {
        // TODO (LS03 PR B): run twig-ir-compiler, return (bytecode_path, sidecar_bytes).
        Err("TwigDebugAdapter::compile: not yet implemented (LS03 PR B)".into())
    }

    fn launch_vm(
        &self,
        _bytecode_path: &Path,
        _debug_port: u16,
    ) -> Result<std::process::Child, String> {
        // TODO (LS03 PR B): spawn twig-vm --debug-port <port> <bytecode>.
        Err("TwigDebugAdapter::launch_vm: not yet implemented (LS03 PR B)".into())
    }

    fn language_name(&self) -> &'static str {
        "twig"
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["twig", "tw"]
    }
}
