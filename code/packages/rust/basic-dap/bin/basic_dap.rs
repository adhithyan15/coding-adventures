//! `basic-dap` — Dartmouth BASIC Debug Adapter entry point.
//!
//! VS Code (and other DAP editors) launch this binary as a subprocess
//! and communicate via stdin/stdout using the Debug Adapter Protocol.
//!
//! ## Usage
//!
//! ```text
//! basic-dap
//! ```
//!
//! Configure in `.vscode/launch.json`:
//!
//! ```json
//! {
//!   "type": "basic",
//!   "request": "launch",
//!   "name": "Debug BASIC file",
//!   "program": "${file}"
//! }
//! ```
//!
//! ## How it's wired
//!
//! ```text
//! main()
//!   │
//!   ▼  DapServer::new(BasicDebugAdapter)
//!   │
//!   ▼  server.run_stdio()    ← blocks until editor sends `disconnect`
//! ```

use basic_dap::BasicDebugAdapter;
use dap_adapter_core::DapServer;

fn main() {
    let mut server = DapServer::new(BasicDebugAdapter);
    if let Err(e) = server.run_stdio() {
        eprintln!("basic-dap: {e}");
        std::process::exit(1);
    }
}
