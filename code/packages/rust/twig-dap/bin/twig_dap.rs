//! `twig-dap` — Twig Debug Adapter entry point.
//!
//! VS Code (and other DAP editors) launch this binary as a subprocess and
//! communicate via stdin/stdout using the Debug Adapter Protocol.
//!
//! ## Usage
//!
//! ```text
//! twig-dap
//! ```
//!
//! Configure in `.vscode/launch.json`:
//!
//! ```json
//! {
//!   "type": "twig",
//!   "request": "launch",
//!   "name": "Debug Twig file",
//!   "program": "${file}"
//! }
//! ```
//!
//! ## How it's wired
//!
//! ```text
//! main()
//!   │
//!   ▼  DapServer::new(TwigDebugAdapter)
//!   │
//!   ▼  server.run_stdio()    ← blocks until editor sends `disconnect`
//! ```

use dap_adapter_core::DapServer;
use twig_dap::TwigDebugAdapter;

fn main() {
    let mut server = DapServer::new(TwigDebugAdapter);
    if let Err(e) = server.run_stdio() {
        eprintln!("twig-dap: {e}");
        std::process::exit(1);
    }
}
