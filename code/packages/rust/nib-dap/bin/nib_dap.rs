//! `nib-dap` — Nib Debug Adapter entry point.
//!
//! VS Code (and other DAP editors) launch this binary as a subprocess
//! and communicate via stdin/stdout using the Debug Adapter Protocol.
//!
//! Configure in `.vscode/launch.json`:
//!
//! ```json
//! {
//!   "type": "nib",
//!   "request": "launch",
//!   "name": "Debug Nib file",
//!   "program": "${file}"
//! }
//! ```

use dap_adapter_core::DapServer;
use nib_dap::NibDebugAdapter;

fn main() {
    let mut server = DapServer::new(NibDebugAdapter);
    if let Err(e) = server.run_stdio() {
        eprintln!("nib-dap: {e}");
        std::process::exit(1);
    }
}
