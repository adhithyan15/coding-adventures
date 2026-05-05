//! `twig-vm` CLI — runs Twig source files, optionally under a debug server.
//!
//! ## Usage
//!
//! ```text
//! twig-vm <FILE>                          # run normally
//! twig-vm --debug-port <PORT> <FILE>      # run under DAP debug server
//! ```
//!
//! ## Debug-port mode
//!
//! When `--debug-port N` is passed, the VM:
//! 1. Compiles the source file into an `IIRModule`.
//! 2. Binds a TCP listener on `127.0.0.1:N` and accepts ONE connection.
//! 3. Sends an `entry` stop event and blocks waiting for `continue`.
//! 4. Runs the program with a [`twig_vm::debug_server::DebugServer`] hook
//!    — set/clear breakpoints, single-step, pause, get-stack, get-slot
//!    are all honoured.
//! 5. Sends an `exited` event with the resulting exit code.
//!
//! The wire protocol matches `dap-adapter-core::vm_conn::TcpVmConnection`,
//! so `twig-dap` (the editor-facing DAP adapter) connects directly.

use std::io::Read;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::ExitCode;

use twig_vm::debug_server::DebugServer;
use twig_vm::dispatch::{run_with_debug, run_with_globals, Globals, ICTable, ProfileTable};
use twig_vm::TwigVM;

fn main() -> ExitCode {
    // ── Parse args ─────────────────────────────────────────────────
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut debug_port: Option<u16> = None;
    let mut path: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--debug-port" => {
                let v = match args.get(i + 1) {
                    Some(v) => v,
                    None => { eprintln!("--debug-port requires a port number"); return ExitCode::from(2); }
                };
                debug_port = Some(match v.parse() {
                    Ok(n) => n,
                    Err(e) => { eprintln!("invalid port: {e}"); return ExitCode::from(2); }
                });
                i += 2;
            }
            "--help" | "-h" => {
                eprintln!("usage: twig-vm [--debug-port <PORT>] <FILE>");
                return ExitCode::SUCCESS;
            }
            other if !other.starts_with("--") && path.is_none() => {
                path = Some(PathBuf::from(other));
                i += 1;
            }
            other => {
                eprintln!("unknown argument: {other}");
                return ExitCode::from(2);
            }
        }
    }
    let path = match path {
        Some(p) => p,
        None    => { eprintln!("missing source file"); return ExitCode::from(2); }
    };

    // ── Load source ────────────────────────────────────────────────
    let source = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => { eprintln!("failed to read {}: {e}", path.display()); return ExitCode::from(1); }
    };

    // ── Compile ────────────────────────────────────────────────────
    let vm = TwigVM::new();
    let module = match vm.compile(&source) {
        Ok(m) => m,
        Err(e) => { eprintln!("compile error: {e}"); return ExitCode::from(1); }
    };

    // ── Branch: debug vs. plain run ────────────────────────────────
    match debug_port {
        Some(port) => run_under_debug_server(port, &module),
        None       => run_plain(&module),
    }
}

/// Plain (non-debug) execution.
fn run_plain(module: &interpreter_ir::IIRModule) -> ExitCode {
    let mut globals = Globals::new();
    match run_with_globals(module, &mut globals) {
        Ok(_)  => ExitCode::SUCCESS,
        Err(e) => { eprintln!("runtime error: {e}"); ExitCode::from(1) }
    }
}

/// Debug-mode execution: bind TCP, accept one adapter, run under hooks.
fn run_under_debug_server(port: u16, module: &interpreter_ir::IIRModule) -> ExitCode {
    let listener = match TcpListener::bind(("127.0.0.1", port)) {
        Ok(l) => l,
        Err(e) => { eprintln!("failed to bind 127.0.0.1:{port}: {e}"); return ExitCode::from(1); }
    };
    // Echo the actual bound address — useful when port=0 and the OS
    // assigns one — so the spawning DAP adapter can read stdout to
    // discover the port if it set it to 0.
    if let Ok(addr) = listener.local_addr() {
        // stderr keeps stdout clean for any future protocol use.
        eprintln!("twig-vm: listening on {addr}");
    }

    let (stream, _) = match listener.accept() {
        Ok(p) => p,
        Err(e) => { eprintln!("accept failed: {e}"); return ExitCode::from(1); }
    };
    drop(listener); // we only ever serve one adapter

    let mut server = match DebugServer::new(stream) {
        Ok(s) => s,
        Err(e) => { eprintln!("debug server init failed: {e}"); return ExitCode::from(1); }
    };

    // Wait for the adapter to install breakpoints and send `continue`.
    if let Err(e) = server.await_initial_continue() {
        eprintln!("debug server: {e}");
        return ExitCode::from(1);
    }

    // Run the module under the debug hook.
    let mut globals  = Globals::new();
    let mut ic_table = ICTable::new();
    let mut profile  = ProfileTable::new();
    let exit_code = match run_with_debug(module, &mut globals, &mut ic_table, &mut profile, &mut server) {
        Ok(_)  => 0,
        Err(_) => 1,
    };

    // Tell the adapter we're done.
    let _ = server.emit_exited(exit_code);

    // Drain any final commands the adapter might send (disconnect).
    // Best-effort — don't gate exit on it.
    let mut sink = [0u8; 1024];
    let _ = std::io::stdin().read(&mut sink);

    ExitCode::from(exit_code as u8)
}
