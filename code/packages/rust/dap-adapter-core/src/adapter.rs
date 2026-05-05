//! [`LanguageDebugAdapter`] — the per-language trait.
//!
//! Implement this trait (≤ 20 lines) to get full DAP support.
//! See spec LS03 §"LanguageDebugAdapter trait" for full documentation.
//!
//! ## Implementation guide (LS03 PR A)
//!
//! The trait must be object-safe (use `dyn LanguageDebugAdapter` in DapServer).
//! If compile() or launch_vm() need async, wrap them with std::thread::spawn
//! for now — async is a Phase 2 optimization.

use std::path::{Path, PathBuf};

/// Compile and launch hooks for a specific language.
///
/// Implement this trait for your language and pass it to [`DapServer::new()`].
///
/// ## Minimum implementation
///
/// ```rust,ignore
/// struct MyLangAdapter;
///
/// impl LanguageDebugAdapter for MyLangAdapter {
///     fn compile(&self, source_path: &Path, _workspace: &Path)
///         -> Result<(PathBuf, Vec<u8>), String>
///     {
///         // Run your compiler, return (bytecode_path, sidecar_bytes).
///         todo!()
///     }
///
///     fn launch_vm(&self, bytecode: &Path, debug_port: u16)
///         -> Result<std::process::Child, String>
///     {
///         // Spawn your VM with --debug-port.
///         todo!()
///     }
///
///     fn language_name(&self) -> &'static str { "my-lang" }
///     fn file_extensions(&self) -> &'static [&'static str] { &["ml"] }
/// }
/// ```
pub trait LanguageDebugAdapter: Send + Sync + 'static {
    /// Compile `source_path` to bytecode.
    ///
    /// Returns `(bytecode_path, sidecar_bytes)`:
    /// - `bytecode_path`: path to the compiled bytecode file (`.iir`, `.aot`, etc.).
    /// - `sidecar_bytes`: the raw debug sidecar (offset ↔ source location map).
    ///
    /// The sidecar is produced by the compiler alongside the bytecode.
    /// See spec 05d for the sidecar binary format.
    fn compile(
        &self,
        source_path: &Path,
        workspace_root: &Path,
    ) -> Result<(PathBuf, Vec<u8>), String>;

    /// Launch the VM with `bytecode_path` in debug mode on `debug_port`.
    ///
    /// The VM must:
    /// 1. Start a TCP server on `debug_port`.
    /// 2. Pause (wait for CONTINUE) before executing any bytecode.
    ///
    /// See spec 05e §"VM Debug Protocol" for the exact wire protocol.
    fn launch_vm(
        &self,
        bytecode_path: &Path,
        debug_port: u16,
    ) -> Result<std::process::Child, String>;

    /// Human-readable language name (for logs and error messages).
    fn language_name(&self) -> &'static str;

    /// File extensions this adapter handles (for editor registration).
    fn file_extensions(&self) -> &'static [&'static str];

    /// TCP connection retry timeout in milliseconds (default: 5000).
    ///
    /// The adapter retries connecting to the VM's debug port with exponential
    /// backoff until this timeout expires. Override if your VM starts slowly.
    fn vm_connect_timeout_ms(&self) -> u64 {
        5_000
    }
}
