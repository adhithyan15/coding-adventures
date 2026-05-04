//! [`DapServer`] — the generic DAP event loop.
//!
//! ## Implementation plan (LS03 PR A)
//!
//! The server runs as a synchronous event loop over stdin/stdout (or TCP for tests).
//!
//! ### Event loop
//!
//! ```text
//! loop:
//!   1. Read a Content-Length framed JSON message from input.
//!      See protocol::read_message().
//!
//!   2. Parse as DapMessage { type, seq, command/event, body }.
//!
//!   3. Dispatch:
//!      "initialize"        → handle_initialize()
//!      "launch"            → handle_launch()   (calls adapter.compile + launch_vm)
//!      "setBreakpoints"    → handle_set_breakpoints()
//!      "configurationDone" → handle_configuration_done()
//!      "continue"          → handle_continue()
//!      "pause"             → handle_pause()
//!      "next"              → handle_next()      (step-over)
//!      "stepIn"            → handle_step_in()
//!      "stepOut"           → handle_step_out()
//!      "stackTrace"        → handle_stack_trace()
//!      "scopes"            → handle_scopes()
//!      "variables"         → handle_variables()
//!      "source"            → handle_source()
//!      "disconnect"        → handle_disconnect(); break
//!      unknown             → send error response
//!
//!   4. Write response(s) + any queued events.
//! ```
//!
//! ### VM exit monitoring
//!
//! Spawn a background thread that polls `vm_proc.try_wait()` every 100ms.
//! When the process exits, send a `terminated` event to the editor.
//!
//! ### Stepping (see stepper.rs for algorithms)
//!
//! `next` / `stepIn` / `stepOut` all follow the algorithms from spec 05e.
//! After each step, the stepper sends a `stopped` event with reason "step".

use crate::LanguageDebugAdapter;
use crate::breakpoints::BreakpointManager;
use crate::stepper::StepController;
use crate::sidecar::SidecarIndex;
use crate::vm_conn::VmConnection;

/// Generic DAP server parameterised by a language adapter.
///
/// ## TODO — implement (LS03 PR A)
pub struct DapServer<A: LanguageDebugAdapter> {
    pub(crate) adapter: A,
    pub(crate) bps: BreakpointManager,
    pub(crate) stepper: StepController,
    pub(crate) sidecar: Option<SidecarIndex>,
    pub(crate) vm_conn: Option<VmConnection>,
    pub(crate) vm_proc: Option<std::process::Child>,
}

impl<A: LanguageDebugAdapter> DapServer<A> {
    /// Construct the server.
    pub fn new(adapter: A) -> Self {
        DapServer {
            adapter,
            bps: BreakpointManager::new(),
            stepper: StepController::new(),
            sidecar: None,
            vm_conn: None,
            vm_proc: None,
        }
    }

    /// Run the DAP adapter over stdin/stdout (standard VS Code mode).
    ///
    /// Blocks until the editor sends `disconnect` or the VM exits.
    ///
    /// ## TODO — implement (LS03 PR A)
    pub fn run_stdio(self) -> Result<(), String> {
        // TODO: implement event loop over stdin/stdout.
        eprintln!("[dap-adapter-core] run_stdio: not yet implemented (LS03 PR A)");
        Ok(())
    }

    /// Run the DAP adapter over a TCP stream (used in tests).
    ///
    /// ## TODO — implement (LS03 PR A)
    pub fn run_tcp(self, _stream: std::net::TcpStream) -> Result<(), String> {
        // TODO: implement event loop over TCP stream.
        eprintln!("[dap-adapter-core] run_tcp: not yet implemented (LS03 PR A)");
        Ok(())
    }
}
