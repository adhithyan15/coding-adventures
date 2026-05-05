//! `twig-dap` — Twig Debug Adapter entry point.
//!
//! VS Code (and other DAP editors) launch this binary as a subprocess
//! and communicate via stdin/stdout using the Debug Adapter Protocol.
//!
//! ## Usage
//!
//! ```text
//! twig-dap
//! ```
//!
//! Configure in `.vscode/launch.json`:
//! ```json
//! {
//!   "type": "twig",
//!   "request": "launch",
//!   "name": "Debug Twig file",
//!   "program": "${file}"
//! }
//! ```
//!
//! ## Implementation (LS03 PR B)
//!
//! ```rust,ignore
//! use dap_adapter_core::DapServer;
//! use twig_dap::TwigDebugAdapter;
//!
//! fn main() {
//!     DapServer::new(TwigDebugAdapter)
//!         .run_stdio()
//!         .expect("DAP server error");
//! }
//! ```
//!
//! TODO: uncomment once LS03 PR A (dap-adapter-core) is implemented.

fn main() {
    eprintln!("twig-dap: LS03 PR B not yet implemented.");
    eprintln!("Implement dap-adapter-core (LS03 PR A) first, then wire here.");
    std::process::exit(1);
}
