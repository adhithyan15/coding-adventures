//! TCP-backed [`DebugHooks`] implementation — the production debug server.
//!
//! Speaks the newline-delimited JSON wire protocol documented at the top of
//! [`dap_adapter_core::vm_conn`]:
//!
//! ```text
//! → { "cmd": "set_breakpoint", "fn": "foo", "offset": 5 }
//! ← { "ok": true }
//! ⇐ { "event": "stopped", "reason": "breakpoint", "fn": "foo", "offset": 5 }
//! ```
//!
//! ## How it plugs into dispatch
//!
//! [`DebugServer`] implements [`DebugHooks::before_instruction`].  Each
//! safepoint:
//! 1. Drains pending TCP commands (`set_breakpoint`, `clear_breakpoint`,
//!    `pause`, etc.) — non-blocking.
//! 2. Updates internal state (breakpoints, single-step flag).
//! 3. If the new state says "stop here" (breakpoint hit, step pending,
//!    pause requested), emits a `stopped` event and **blocks** reading the
//!    socket until `continue` or `step_instruction` arrives.
//!
//! ## Lifecycle
//!
//! ```text
//! TwigVM CLI sees `--debug-port N`
//!   ┃
//!   ▼  bind TCP listener; accept ONE connection
//! DebugServer::new(stream)
//!   ┃
//!   ▼  send {event: "stopped", reason: "entry"} and wait for `continue`
//! run_with_debug(module, …, &mut server)   // dispatcher takes over
//!   ┃
//!   ▼  on every safepoint → DebugServer::before_instruction
//!   ┃
//!   ▼  dispatch returns Ok(value) or Err(RunError)
//! send {event: "exited", exit_code}
//! ```

use std::collections::{HashSet, VecDeque};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;

use serde_json::{json, Value};

use crate::debug::{DebugHooks, FrameView};

// ---------------------------------------------------------------------------
// DoS guard
// ---------------------------------------------------------------------------

/// Maximum bytes a single newline-terminated command line may contain.
///
/// Commands and event payloads are small structured JSON; 1 MiB is far
/// above any legitimate value and well below memory exhaustion.  Without
/// this cap, a peer that streams bytes without a newline would force
/// unbounded `String` growth.
pub const MAX_LINE_BYTES: usize = 1024 * 1024;

/// Read one line of at most [`MAX_LINE_BYTES`] bytes (inclusive of `\n`)
/// from a `BufRead`, returning the line as a `String`.
///
/// Behaves like `BufRead::read_line` but with a hard cap: if more than
/// [`MAX_LINE_BYTES`] bytes are consumed before a newline, returns an
/// `InvalidData` error and the partially-read bytes are discarded.
fn read_line_capped(reader: &mut impl BufRead, line: &mut String) -> std::io::Result<usize> {
    let mut limited = reader.take(MAX_LINE_BYTES as u64);
    let n = limited.read_line(line)?;
    // If we hit the limit AND no newline arrived, signal overflow.
    // (`read_line` succeeds at EOF too — that's `n == 0` and we let it
    // through unchanged.)
    if n == MAX_LINE_BYTES && !line.ends_with('\n') {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("debug-server line exceeded {MAX_LINE_BYTES} bytes"),
        ));
    }
    Ok(n)
}

// ---------------------------------------------------------------------------
// Reasons the VM stopped — wire-format strings match dap-adapter-core
// ---------------------------------------------------------------------------

/// Why the VM is stopped at a safepoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    /// User-set breakpoint at this `(fn, pc)`.
    Breakpoint,
    /// Single-step completed.
    Step,
    /// Async pause request.
    Pause,
    /// Initial entry stop (always sent right after the VM starts).
    Entry,
}

impl StopReason {
    fn as_str(self) -> &'static str {
        match self {
            StopReason::Breakpoint => "breakpoint",
            StopReason::Step       => "step",
            StopReason::Pause      => "pause",
            StopReason::Entry      => "entry",
        }
    }
}

// ---------------------------------------------------------------------------
// DebugServer
// ---------------------------------------------------------------------------

/// One-connection-per-VM debug server.
///
/// Owns the TCP stream, breakpoint set, single-step flag, pause flag, and a
/// snapshot of the live call stack.  Created by the `twig-vm` CLI when
/// `--debug-port` is supplied; passed to [`crate::dispatch::run_with_debug`].
pub struct DebugServer {
    reader: BufReader<TcpStream>,
    writer: TcpStream,
    breakpoints: HashSet<(String, usize)>,
    single_step: bool,
    pause_requested: bool,
    /// Reconstructed call stack — `[(fn_name, pc), …]`, depth-indexed.
    /// Updated from `before_instruction`'s `(fn, depth, pc)` triple.
    call_stack: Vec<(String, usize)>,
    /// `true` once the initial-entry stop has been sent.
    started: bool,
    /// Last frame's register values, for `get_slot` queries.  Cleared on
    /// every `before_instruction` and refilled from the FrameView.
    last_frame_registers: Vec<(String, String)>,
}

impl DebugServer {
    /// Wrap a connected `TcpStream` as a debug server.
    ///
    /// The stream's `try_clone` must succeed (used to split read/write).
    pub fn new(stream: TcpStream) -> std::io::Result<Self> {
        let writer = stream.try_clone()?;
        let reader = BufReader::new(stream);
        Ok(DebugServer {
            reader,
            writer,
            breakpoints:           HashSet::new(),
            single_step:           false,
            pause_requested:       false,
            call_stack:            Vec::new(),
            started:               false,
            last_frame_registers:  Vec::new(),
        })
    }

    // -------------------------------------------------------------------
    // Wire I/O
    // -------------------------------------------------------------------

    /// Send a single JSON line to the adapter.
    fn write_json(&mut self, v: Value) -> std::io::Result<()> {
        let mut s = serde_json::to_string(&v)?;
        s.push('\n');
        self.writer.write_all(s.as_bytes())?;
        self.writer.flush()
    }

    /// Read the next JSON line, blocking.  Returns `None` on EOF.
    ///
    /// Uses the [`MAX_LINE_BYTES`]-capped reader so a malicious peer
    /// cannot OOM us with a never-newline-terminated stream.
    fn read_json_blocking(&mut self) -> std::io::Result<Option<Value>> {
        let mut line = String::new();
        if read_line_capped(&mut self.reader, &mut line)? == 0 {
            return Ok(None); // EOF — adapter disconnected
        }
        let v: Value = serde_json::from_str(line.trim_end())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData,
                                             format!("bad JSON: {e}")))?;
        Ok(Some(v))
    }

    /// Drain *any number of pending* commands without blocking.
    ///
    /// Uses `set_nonblocking(true)` for the duration of the call.  Returns
    /// every pending command parsed as JSON; commands that don't parse are
    /// silently dropped (the adapter sends only well-formed JSON).
    fn drain_pending(&mut self) -> std::io::Result<VecDeque<Value>> {
        let mut out = VecDeque::new();
        // Switch the underlying TcpStream to non-blocking, peek lines.
        self.writer.set_nonblocking(true)?;
        loop {
            let mut line = String::new();
            match read_line_capped(&mut self.reader, &mut line) {
                Ok(0) => break,                 // EOF
                Ok(_) => {
                    if let Ok(v) = serde_json::from_str::<Value>(line.trim_end()) {
                        out.push_back(v);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => {
                    self.writer.set_nonblocking(false)?;
                    return Err(e);
                }
            }
        }
        self.writer.set_nonblocking(false)?;
        Ok(out)
    }

    // -------------------------------------------------------------------
    // Command handling
    // -------------------------------------------------------------------

    /// Process one command; return `true` if the command unblocks the VM
    /// (i.e. `continue`, `step_instruction`).
    ///
    /// Synchronous commands (`set_breakpoint`, `pause`, `get_call_stack`,
    /// `get_slot`) are handled inline and **do not** unblock — caller
    /// keeps reading until a continue/step arrives.
    fn handle_command(&mut self, v: Value) -> std::io::Result<bool> {
        let cmd = v.get("cmd").and_then(|c| c.as_str()).unwrap_or("");
        match cmd {
            "set_breakpoint" => {
                if let (Some(f), Some(o)) = (v.get("fn").and_then(|x| x.as_str()),
                                              v.get("offset").and_then(|x| x.as_u64())) {
                    self.breakpoints.insert((f.to_string(), o as usize));
                }
                self.write_json(json!({"ok": true}))?;
                Ok(false)
            }
            "clear_breakpoint" => {
                if let (Some(f), Some(o)) = (v.get("fn").and_then(|x| x.as_str()),
                                              v.get("offset").and_then(|x| x.as_u64())) {
                    self.breakpoints.remove(&(f.to_string(), o as usize));
                }
                self.write_json(json!({"ok": true}))?;
                Ok(false)
            }
            "continue" => {
                self.single_step = false;
                self.pause_requested = false;
                self.write_json(json!({"ok": true}))?;
                Ok(true)
            }
            "pause" => {
                self.pause_requested = true;
                self.write_json(json!({"ok": true}))?;
                Ok(false)
            }
            "step_instruction" => {
                self.single_step = true;
                self.pause_requested = false;
                self.write_json(json!({"ok": true}))?;
                Ok(true)
            }
            "get_call_stack" => {
                let frames: Vec<Value> = self.call_stack.iter()
                    .rev() // deepest frame first per protocol
                    .map(|(f, o)| json!({"fn": f, "offset": o}))
                    .collect();
                self.write_json(json!({"frames": frames}))?;
                Ok(false)
            }
            "get_slot" => {
                let slot = v.get("slot").and_then(|s| s.as_u64()).unwrap_or(0) as usize;
                let repr = self.last_frame_registers.get(slot)
                    .map(|(_, r)| r.clone())
                    .unwrap_or_else(|| "<undef>".to_string());
                self.write_json(json!({"kind": "any", "repr": repr}))?;
                Ok(false)
            }
            other => {
                self.write_json(json!({"ok": false, "error": format!("unknown cmd: {other}")}))?;
                Ok(false)
            }
        }
    }

    // -------------------------------------------------------------------
    // Stop coordination
    // -------------------------------------------------------------------

    fn emit_stopped(&mut self, reason: StopReason, fn_name: &str, pc: usize) -> std::io::Result<()> {
        self.write_json(json!({
            "event":  "stopped",
            "reason": reason.as_str(),
            "fn":     fn_name,
            "offset": pc,
        }))
    }

    /// Block reading commands until one returns `true` (continue / step).
    fn block_until_resume(&mut self, fn_name: &str, pc: usize) -> std::io::Result<()> {
        // Always announce the stop first.
        // (caller decides which reason — entry/breakpoint/step/pause)
        let _ = (fn_name, pc); // captured implicitly in `emit_stopped` callers above
        loop {
            match self.read_json_blocking()? {
                Some(v) => {
                    if self.handle_command(v)? {
                        return Ok(());
                    }
                }
                None => {
                    // Adapter disconnected; let dispatch continue freely.
                    return Ok(());
                }
            }
        }
    }

    // -------------------------------------------------------------------
    // Stack-tracking
    // -------------------------------------------------------------------

    /// Update `call_stack` based on the new `(fn_name, depth, pc)` triple.
    ///
    /// `depth` is the number of *parent* frames so the array is sized to
    /// `depth + 1` after this call.
    fn track_stack(&mut self, fn_name: &str, depth: usize, pc: usize) {
        // Truncate to drop any frames at deeper depths (a return).
        self.call_stack.truncate(depth);
        // Pad with placeholders if we somehow skipped frames (shouldn't
        // happen, but defensive).
        while self.call_stack.len() < depth {
            self.call_stack.push(("<unknown>".to_string(), 0));
        }
        // Push or update the current frame.
        if self.call_stack.len() == depth {
            self.call_stack.push((fn_name.to_string(), pc));
        } else {
            self.call_stack[depth] = (fn_name.to_string(), pc);
        }
    }

    /// Snapshot the FrameView's registers into `last_frame_registers`.
    fn snapshot_frame(&mut self, frame: &FrameView<'_>) {
        let mut names = frame.register_names();
        names.sort(); // stable order for indexed slot access
        self.last_frame_registers.clear();
        for n in names {
            let r = frame.read_register(&n).unwrap_or_else(|| "<undef>".to_string());
            self.last_frame_registers.push((n, r));
        }
    }

    // -------------------------------------------------------------------
    // Public lifecycle (called by the CLI)
    // -------------------------------------------------------------------

    /// Send the initial "entry" stop and block until the adapter replies
    /// with `continue` (or `step_instruction`).
    ///
    /// The CLI calls this once *before* invoking `run_with_debug` so the
    /// adapter has a chance to install breakpoints before any code runs.
    pub fn await_initial_continue(&mut self) -> std::io::Result<()> {
        // Process any pre-execution commands the adapter sends (e.g. it
        // might setBreakpoints right after connecting before we even
        // emit the first stopped).  We also emit an "entry" stopped to
        // mark "the VM is paused at the start, ready to receive
        // breakpoints."  The wire model: VM emits stopped, adapter
        // installs breakpoints (synchronous), adapter sends continue.
        self.emit_stopped(StopReason::Entry, "<entry>", 0)?;
        self.started = true;
        self.block_until_resume("<entry>", 0)
    }

    /// Send the final "exited" event when the VM finishes.
    pub fn emit_exited(&mut self, exit_code: i32) -> std::io::Result<()> {
        self.write_json(json!({
            "event":     "exited",
            "exit_code": exit_code,
        }))
    }
}

// ---------------------------------------------------------------------------
// DebugHooks impl
// ---------------------------------------------------------------------------

impl DebugHooks for DebugServer {
    fn before_instruction(
        &mut self,
        fn_name: &str,
        depth: usize,
        pc: usize,
        frame: &FrameView<'_>,
    ) {
        self.track_stack(fn_name, depth, pc);
        self.snapshot_frame(frame);

        // 1. Drain any pending commands (set_breakpoint, etc.) that
        //    arrived while the VM was running.
        if let Ok(pending) = self.drain_pending() {
            for cmd in pending {
                let _ = self.handle_command(cmd);
            }
        }

        // 2. Determine whether this safepoint warrants a stop.
        let stop_reason = if self.breakpoints.contains(&(fn_name.to_string(), pc)) {
            Some(StopReason::Breakpoint)
        } else if self.single_step {
            self.single_step = false;
            Some(StopReason::Step)
        } else if self.pause_requested {
            self.pause_requested = false;
            Some(StopReason::Pause)
        } else {
            None
        };

        if let Some(reason) = stop_reason {
            let _ = self.emit_stopped(reason, fn_name, pc);
            let _ = self.block_until_resume(fn_name, pc);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;

    /// Spawn a TcpListener on a random localhost port and return
    /// (server-side stream, client-side stream).
    fn loopback_pair() -> (TcpStream, TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let client_handle = thread::spawn(move || TcpStream::connect(addr).unwrap());
        let (server, _) = listener.accept().unwrap();
        let client = client_handle.join().unwrap();
        (server, client)
    }

    fn read_line(s: &mut TcpStream) -> String {
        let mut buf = [0u8; 4096];
        let n = std::io::Read::read(s, &mut buf).unwrap();
        String::from_utf8_lossy(&buf[..n]).to_string()
    }

    #[test]
    fn server_constructs_from_stream() {
        let (server_side, _client) = loopback_pair();
        let _ = DebugServer::new(server_side).expect("ok");
    }

    #[test]
    fn set_breakpoint_acks() {
        let (server_side, mut client) = loopback_pair();
        let mut server = DebugServer::new(server_side).unwrap();
        // Client sends a set_breakpoint command.
        client.write_all(b"{\"cmd\":\"set_breakpoint\",\"fn\":\"main\",\"offset\":5}\n").unwrap();
        // Server reads + processes.
        let cmd = server.read_json_blocking().unwrap().unwrap();
        let unblocks = server.handle_command(cmd).unwrap();
        assert!(!unblocks, "set_breakpoint must not unblock");
        assert_eq!(server.breakpoints.len(), 1);
        assert!(server.breakpoints.contains(&("main".to_string(), 5)));
        // Client receives ack.
        let resp = read_line(&mut client);
        assert!(resp.contains("\"ok\":true"), "got: {resp}");
    }

    #[test]
    fn continue_unblocks() {
        let (server_side, mut client) = loopback_pair();
        let mut server = DebugServer::new(server_side).unwrap();
        client.write_all(b"{\"cmd\":\"continue\"}\n").unwrap();
        let cmd = server.read_json_blocking().unwrap().unwrap();
        let unblocks = server.handle_command(cmd).unwrap();
        assert!(unblocks);
        let _ = read_line(&mut client); // drain ack
    }

    #[test]
    fn step_instruction_unblocks_and_sets_flag() {
        let (server_side, mut client) = loopback_pair();
        let mut server = DebugServer::new(server_side).unwrap();
        client.write_all(b"{\"cmd\":\"step_instruction\"}\n").unwrap();
        let cmd = server.read_json_blocking().unwrap().unwrap();
        let unblocks = server.handle_command(cmd).unwrap();
        assert!(unblocks);
        assert!(server.single_step);
        let _ = read_line(&mut client);
    }

    #[test]
    fn track_stack_pushes_and_pops() {
        let (s, _c) = loopback_pair();
        let mut srv = DebugServer::new(s).unwrap();
        srv.track_stack("main", 0, 0);
        srv.track_stack("main", 0, 1);
        srv.track_stack("foo", 1, 0);
        assert_eq!(srv.call_stack, vec![
            ("main".to_string(), 1),
            ("foo".to_string(),  0),
        ]);
        srv.track_stack("main", 0, 2);
        assert_eq!(srv.call_stack, vec![("main".to_string(), 2)]);
    }

    #[test]
    fn read_line_capped_rejects_overlong_input() {
        // Build a buffer of MAX_LINE_BYTES + 1 bytes, NO newline.  The
        // helper must return InvalidData rather than allocate the lot.
        use std::io::Cursor;
        let bytes = vec![b'A'; MAX_LINE_BYTES + 1];
        let mut reader = std::io::BufReader::new(Cursor::new(bytes));
        let mut line = String::new();
        let err = read_line_capped(&mut reader, &mut line).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn read_line_capped_accepts_legitimate_line() {
        use std::io::Cursor;
        let raw = b"{\"cmd\":\"continue\"}\n".to_vec();
        let mut reader = std::io::BufReader::new(Cursor::new(raw));
        let mut line = String::new();
        let n = read_line_capped(&mut reader, &mut line).unwrap();
        assert_eq!(n, line.len());
        assert!(line.ends_with('\n'));
    }

    #[test]
    fn get_call_stack_returns_deepest_first() {
        let (server_side, mut client) = loopback_pair();
        let mut server = DebugServer::new(server_side).unwrap();
        server.track_stack("main", 0, 0);
        server.track_stack("foo",  1, 5);
        client.write_all(b"{\"cmd\":\"get_call_stack\"}\n").unwrap();
        let cmd = server.read_json_blocking().unwrap().unwrap();
        let _ = server.handle_command(cmd).unwrap();
        let resp = read_line(&mut client);
        // Deepest frame ("foo") must appear first.
        let idx_foo = resp.find("\"foo\"").expect("foo present");
        let idx_main = resp.find("\"main\"").expect("main present");
        assert!(idx_foo < idx_main, "deepest first: {resp}");
    }
}
