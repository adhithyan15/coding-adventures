//! TCP client for the VM Debug Protocol.
//!
//! ## Implementation plan (LS03 PR A)
//!
//! The VM Debug Protocol is described in spec 05e §"VM Debug Protocol".
//!
//! Commands sent to the VM (newline-delimited JSON over TCP):
//!   { "cmd": "set_breakpoint", "offset": N }
//!   { "cmd": "continue" }
//!   { "cmd": "pause" }
//!   { "cmd": "step_instruction" }
//!   { "cmd": "get_call_stack" }  → response: [{ "fn": name, "offset": N }, ...]
//!   { "cmd": "get_slot", "slot": N } → response: { "kind": "integer", "repr": "42" }
//!
//! Events received from the VM:
//!   { "event": "stopped",  "reason": "breakpoint"|"step"|"pause", "offset": N }
//!   { "event": "exited",   "exit_code": N }
//!
//! ### connect(port: u16, timeout_ms: u64) → Result<VmConnection, String>
//!
//! Retry with exponential backoff until connected or timeout.
//! The VM may not have opened its debug server yet immediately after launch.
//!
//! ### Receiving events
//!
//! Spawn a background reader thread. Push events into a channel.
//! DapServer polls the channel in its event loop.

/// TCP connection to the VM debug server.
///
/// ## TODO — implement (LS03 PR A)
pub struct VmConnection {
    // TODO: TcpStream + background reader thread + event channel
}

impl VmConnection {
    /// Connect to the VM debug server on `port`.
    ///
    /// Retries with exponential backoff for `timeout_ms` milliseconds.
    ///
    /// ## TODO — implement (LS03 PR A)
    pub fn connect(_port: u16, _timeout_ms: u64) -> Result<Self, String> {
        Err("VmConnection::connect: not yet implemented (LS03 PR A)".into())
    }

    /// Send CONTINUE to the VM.
    pub fn send_continue(&mut self) -> Result<(), String> {
        Err("not yet implemented".into())
    }

    /// Send a single step_instruction command.
    pub fn step_instruction(&mut self) -> Result<(), String> {
        Err("not yet implemented".into())
    }

    /// Query the current call stack.
    pub fn get_call_stack(&mut self) -> Result<Vec<VmFrame>, String> {
        Err("not yet implemented".into())
    }

    /// Query a variable slot's value.
    pub fn get_slot(&mut self, _slot: u32) -> Result<String, String> {
        Err("not yet implemented".into())
    }
}

/// One frame in the VM's call stack.
#[derive(Debug, Clone)]
pub struct VmFrame {
    /// Name of the function at this frame.
    pub function_name: String,
    /// Current instruction offset within the function.
    pub offset: u64,
}
