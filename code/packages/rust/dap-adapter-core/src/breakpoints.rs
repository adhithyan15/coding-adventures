//! Breakpoint management.
//!
//! ## Implementation plan (LS03 PR A)
//!
//! The BreakpointManager tracks:
//!   source_file → Vec<SourceBreakpoint { line, condition? }>
//!
//! When the editor sends `setBreakpoints`:
//! 1. Store the new breakpoint list (replaces old list for that file).
//! 2. For each breakpoint line, look up instruction offsets via SidecarIndex.
//! 3. Send `set_breakpoint(offset)` to the VM via VmConnection.
//! 4. Return a `setBreakpoints` response with verified=true for each resolved BP.
//!
//! When the VM sends `stopped { reason: "breakpoint" }`:
//! 1. The VM stopped at some offset.
//! 2. Look up offset in SidecarIndex → (file, line, col).
//! 3. Match against stored breakpoints to find which one was hit.
//! 4. Send `stopped` event to editor with source location.

/// Manages active breakpoints.
///
/// ## TODO — implement (LS03 PR A)
pub struct BreakpointManager {
    // TODO: HashMap<PathBuf, Vec<SourceBreakpoint>>
}

impl BreakpointManager {
    /// Create an empty breakpoint manager.
    pub fn new() -> Self {
        BreakpointManager {}
    }
}

impl Default for BreakpointManager {
    fn default() -> Self { Self::new() }
}
