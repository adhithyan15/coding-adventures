//! `oct-dap` — Oct Debug Adapter entry point.
//!
//! VS Code (and other DAP editors) launch this binary as a subprocess
//! and communicate via stdin/stdout using the Debug Adapter Protocol.
//!
//! Configure in `.vscode/launch.json`:
//!
//! ```json
//! {
//!   "type": "oct",
//!   "request": "launch",
//!   "name": "Debug Oct file",
//!   "program": "${file}"
//! }
//! ```

use dap_adapter_core::DapServer;
use oct_dap::OctDebugAdapter;

fn main() {
    let mut server = DapServer::new(OctDebugAdapter);
    if let Err(e) = server.run_stdio() {
        eprintln!("oct-dap: {e}");
        std::process::exit(1);
    }
}
