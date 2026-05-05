//! VM Debug Protocol — connection trait + mock implementation.
//!
//! The [`VmConnection`] trait abstracts the wire protocol between the DAP
//! adapter and the running VM.  Making this a trait (rather than a concrete
//! TCP client) lets us:
//!
//! 1. Unit-test the entire `DapServer` against a scripted [`MockVmConnection`]
//!    without spawning real processes.
//! 2. Defer the concrete TCP implementation to a later PR — twig-vm does not
//!    yet have a debug server, and there is no reason for the generic crate
//!    to ship a wire-format frozen against a VM that does not exist.
//!
//! ## Wire-protocol-elect (informational)
//!
//! When `twig-vm` grows a debug server, it will speak newline-delimited JSON
//! over TCP, with the following commands and events.  This document is the
//! contract; a future PR will provide the concrete `TcpVmConnection`
//! implementing it:
//!
//! ```text
//! → { "cmd": "set_breakpoint", "fn": "foo", "offset": 5 }
//! ← { "ok": true }
//!
//! → { "cmd": "clear_breakpoint", "fn": "foo", "offset": 5 }
//! ← { "ok": true }
//!
//! → { "cmd": "continue" }
//! ← { "ok": true }
//!
//! → { "cmd": "pause" }
//! ← { "ok": true }
//!
//! → { "cmd": "step_instruction" }
//! ← { "ok": true }
//!
//! → { "cmd": "get_call_stack" }
//! ← { "frames": [{ "fn": "foo", "offset": 5 }, …] }
//!
//! → { "cmd": "get_slot", "frame": 0, "slot": 1 }
//! ← { "kind": "integer", "repr": "42" }
//!
//! Async events (no request, server-pushed):
//! ⇐ { "event": "stopped", "reason": "breakpoint", "fn": "foo", "offset": 5 }
//! ⇐ { "event": "exited",  "exit_code": 0 }
//! ```

use std::collections::VecDeque;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tcp_client::{connect, ConnectOptions, TcpConnection, TcpError};

// ---------------------------------------------------------------------------
// DoS / SSRF guards
// ---------------------------------------------------------------------------

/// Maximum bytes accepted for a single line on the VM TCP socket.
///
/// A buggy or malicious VM that streams data without ever emitting `\n` would
/// otherwise drive `read_line` into an unbounded `String` allocation.  64 KiB
/// is far above any legitimate VM message and well below memory exhaustion.
pub const MAX_VM_LINE_BYTES: usize = 64 * 1024;

/// Maximum number of [`StoppedEvent`]s buffered between `poll_event` calls.
///
/// If a VM streams events faster than the adapter can drain them, `rpc()`
/// would queue them indefinitely.  Capping protects against memory blow-up
/// and surfaces a clear error to the user.
pub const MAX_PENDING_EVENTS: usize = 10_000;

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// A point of execution in the VM — `(function, instruction_index)`.
///
/// We use `(fn, idx)` rather than a flat `u64` offset because that's what
/// `debug-sidecar` already indexes against, and it's the natural shape for
/// the wire protocol (each VM frame already carries `fn` + `offset`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VmLocation {
    /// Function name as registered with the sidecar.
    pub function: String,
    /// 0-based instruction index within that function.
    pub instr_index: usize,
}

impl VmLocation {
    /// Build a new location.
    pub fn new(function: impl Into<String>, instr_index: usize) -> Self {
        VmLocation { function: function.into(), instr_index }
    }
}

/// One frame in the VM's call stack.
///
/// The deepest currently-executing frame is at index 0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmFrame {
    /// Where this frame is executing.
    pub location: VmLocation,
}

/// Why the VM stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoppedReason {
    /// Hit a user breakpoint.
    Breakpoint,
    /// Completed a step (next/stepIn/stepOut).
    Step,
    /// Pause was requested.
    Pause,
    /// Some other condition (e.g. entry, exception).
    Other,
}

impl StoppedReason {
    /// DAP wire string for this reason.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Breakpoint => "breakpoint",
            Self::Step       => "step",
            Self::Pause      => "pause",
            Self::Other      => "entry",
        }
    }
}

/// An event pushed by the VM (asynchronous).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoppedEvent {
    /// Execution paused at `location` for `reason`.
    Stopped {
        /// The reason for stopping.
        reason: StoppedReason,
        /// Where execution paused.
        location: VmLocation,
    },
    /// VM process exited with `exit_code`.
    Exited {
        /// Process exit code.
        exit_code: i32,
    },
}

// ---------------------------------------------------------------------------
// VmConnection trait
// ---------------------------------------------------------------------------

/// VM debug-protocol client — synchronous request/response API.
///
/// Implementations must be `Send` so the `DapServer` can hand the connection
/// to a poller thread if needed.  Async event delivery is handled via
/// [`VmConnection::poll_event`].
pub trait VmConnection: Send {
    /// Install a breakpoint at `loc`.  Idempotent.
    fn set_breakpoint(&mut self, loc: &VmLocation) -> Result<(), String>;

    /// Remove a breakpoint at `loc` (no-op if not present).
    fn clear_breakpoint(&mut self, loc: &VmLocation) -> Result<(), String>;

    /// Resume execution.  The VM will run until it hits a breakpoint, is
    /// paused, completes a step, or exits.
    fn cont(&mut self) -> Result<(), String>;

    /// Request the VM to pause at the next safepoint.
    fn pause(&mut self) -> Result<(), String>;

    /// Execute one instruction and return.
    fn step_instruction(&mut self) -> Result<(), String>;

    /// Retrieve the current call stack (deepest frame first).
    fn get_call_stack(&mut self) -> Result<Vec<VmFrame>, String>;

    /// Read slot `slot` in frame `frame` (0 = top of stack).
    /// Returns a printable representation.
    fn get_slot(&mut self, frame: usize, slot: u32) -> Result<String, String>;

    /// Non-blocking poll for a server-pushed event.
    ///
    /// Returns `Ok(None)` if no event is ready, `Ok(Some(event))` if one was
    /// dequeued, or `Err(_)` on connection failure.
    fn poll_event(&mut self) -> Result<Option<StoppedEvent>, String>;
}

// ---------------------------------------------------------------------------
// MockVmConnection
// ---------------------------------------------------------------------------

/// Test double for [`VmConnection`].
///
/// Maintains an in-memory model:
/// - A breakpoint set the test can read.
/// - A scripted call stack the test can mutate.
/// - A scripted slot table for `get_slot`.
/// - A queue of [`StoppedEvent`]s the test can push; `poll_event` drains it.
///
/// The mock does **not** simulate stepping internally — tests drive
/// stepping by pushing `Stopped { reason: Step, … }` events into the queue
/// at the right moments.
#[derive(Debug, Clone)]
pub struct MockVmConnection {
    inner: Arc<Mutex<MockState>>,
}

#[derive(Debug)]
struct MockState {
    breakpoints:  Vec<VmLocation>,
    call_stack:   Vec<VmFrame>,
    slots:        std::collections::HashMap<(usize, u32), String>,
    events:       VecDeque<StoppedEvent>,
    /// Events to enqueue the *next* time `cont()` is called.  Lets tests
    /// model the realistic "continue, then VM hits a breakpoint" flow
    /// without the events being delivered before the first DAP request is
    /// processed.
    cont_triggered_events: VecDeque<StoppedEvent>,
    /// Sequence of method names captured for assertions.
    call_log:     Vec<String>,
}

impl Default for MockVmConnection {
    fn default() -> Self {
        Self::new()
    }
}

impl MockVmConnection {
    /// Build a fresh mock with empty state.
    pub fn new() -> Self {
        MockVmConnection {
            inner: Arc::new(Mutex::new(MockState {
                breakpoints: Vec::new(),
                call_stack:  Vec::new(),
                slots:       std::collections::HashMap::new(),
                events:      VecDeque::new(),
                cont_triggered_events: VecDeque::new(),
                call_log:    Vec::new(),
            })),
        }
    }

    /// Queue an event that will be fired when `cont()` is next called.
    pub fn fire_on_cont(&self, ev: StoppedEvent) {
        self.inner.lock().unwrap().cont_triggered_events.push_back(ev);
    }

    /// Set the call stack the next `get_call_stack()` will return.
    pub fn set_call_stack(&self, frames: Vec<VmFrame>) {
        self.inner.lock().unwrap().call_stack = frames;
    }

    /// Set the value `get_slot(frame, slot)` will return.
    pub fn set_slot(&self, frame: usize, slot: u32, value: impl Into<String>) {
        self.inner.lock().unwrap().slots.insert((frame, slot), value.into());
    }

    /// Push an event to be returned by the next `poll_event()` call.
    pub fn push_event(&self, ev: StoppedEvent) {
        self.inner.lock().unwrap().events.push_back(ev);
    }

    /// Inspect the breakpoint set (for tests).
    pub fn breakpoints(&self) -> Vec<VmLocation> {
        self.inner.lock().unwrap().breakpoints.clone()
    }

    /// Return the recorded call log (for tests).
    pub fn call_log(&self) -> Vec<String> {
        self.inner.lock().unwrap().call_log.clone()
    }

    fn record(&mut self, name: &str) {
        self.inner.lock().unwrap().call_log.push(name.to_string());
    }
}

impl VmConnection for MockVmConnection {
    fn set_breakpoint(&mut self, loc: &VmLocation) -> Result<(), String> {
        self.record("set_breakpoint");
        let mut s = self.inner.lock().unwrap();
        if !s.breakpoints.contains(loc) {
            s.breakpoints.push(loc.clone());
        }
        Ok(())
    }

    fn clear_breakpoint(&mut self, loc: &VmLocation) -> Result<(), String> {
        self.record("clear_breakpoint");
        let mut s = self.inner.lock().unwrap();
        s.breakpoints.retain(|b| b != loc);
        Ok(())
    }

    fn cont(&mut self) -> Result<(), String> {
        self.record("cont");
        let mut s = self.inner.lock().unwrap();
        // Fire ONE waiting event per `cont()` so each user "continue" yields
        // exactly one VM stop or exit event (the realistic 1:1 wire model).
        if let Some(ev) = s.cont_triggered_events.pop_front() {
            s.events.push_back(ev);
        }
        Ok(())
    }

    fn pause(&mut self) -> Result<(), String> {
        self.record("pause");
        Ok(())
    }

    fn step_instruction(&mut self) -> Result<(), String> {
        self.record("step_instruction");
        Ok(())
    }

    fn get_call_stack(&mut self) -> Result<Vec<VmFrame>, String> {
        self.record("get_call_stack");
        Ok(self.inner.lock().unwrap().call_stack.clone())
    }

    fn get_slot(&mut self, frame: usize, slot: u32) -> Result<String, String> {
        self.record("get_slot");
        let s = self.inner.lock().unwrap();
        Ok(s.slots
            .get(&(frame, slot))
            .cloned()
            .unwrap_or_else(|| "<undef>".to_string()))
    }

    fn poll_event(&mut self) -> Result<Option<StoppedEvent>, String> {
        // Don't log poll — it's called in a loop and would flood the log.
        Ok(self.inner.lock().unwrap().events.pop_front())
    }
}

// ---------------------------------------------------------------------------
// TcpVmConnection — concrete client over the `tcp-client` crate
// ---------------------------------------------------------------------------

/// Real [`VmConnection`] backed by a TCP socket to the VM debug server.
///
/// Wraps [`tcp_client::TcpConnection`] (NET01) with the newline-delimited JSON
/// protocol described at the top of this module.
///
/// ## Threading model
///
/// `TcpVmConnection` is **single-threaded**.  Each public method writes one
/// command line and reads response lines until a non-event response is seen.
/// Async events that arrive between commands are buffered in
/// `pending_events`; `poll_event` drains that buffer first, then attempts one
/// short-timeout read to pick up any newly-arrived events.
///
/// This avoids the complexity of a reader-thread + channel demultiplexer at
/// the cost of slightly less responsive event delivery.  In practice the
/// editor calls `poll_event` after every command anyway, so the latency is
/// bounded by the user's typing speed.
///
/// ## Read timeout
///
/// The underlying [`TcpConnection`] is constructed with a short read timeout
/// (100 ms by default).  This makes `poll_event` non-blocking in the common
/// "no event yet" case.  Synchronous command responses are expected to arrive
/// well within this window; if a response takes longer the read loop simply
/// retries until the configured response deadline.
#[derive(Debug)]
pub struct TcpVmConnection {
    conn: TcpConnection,
    pending_events: VecDeque<StoppedEvent>,
    /// Maximum total time to wait for a single command's response.
    response_deadline: Duration,
}

/// Tunable parameters for [`TcpVmConnection::connect_with_retry`].
#[derive(Debug, Clone)]
pub struct TcpConnectOptions {
    /// VM-side TCP port (typically auto-allocated by the adapter).
    pub port: u16,
    /// Hostname / IP — almost always `"127.0.0.1"`.
    pub host: String,
    /// Total time budget for *establishing* the connection (with retries).
    pub connect_budget_ms: u64,
    /// Per-attempt connect timeout.
    pub per_attempt_ms: u64,
    /// Read timeout used for `poll_event` non-blocking reads.
    pub poll_read_ms: u64,
    /// Maximum time to wait for a single RPC response.
    pub response_deadline_ms: u64,
}

impl Default for TcpConnectOptions {
    fn default() -> Self {
        TcpConnectOptions {
            port:                 0, // caller must set
            host:                 "127.0.0.1".to_string(),
            connect_budget_ms:    5_000,
            per_attempt_ms:       250,
            poll_read_ms:         100,
            response_deadline_ms: 5_000,
        }
    }
}

impl TcpVmConnection {
    /// Connect to the VM debug server on `opts.port`, retrying with
    /// exponential backoff up to `opts.connect_budget_ms`.
    ///
    /// VMs typically need a few hundred milliseconds to fully initialise
    /// their debug server after `launch_vm` returns; this method hides that
    /// race from the DAP server's `launch` handler.
    ///
    /// ## SSRF guard — loopback only
    ///
    /// The VM debug server **must** listen on a loopback address.  A
    /// non-loopback `host` (e.g. `"169.254.169.254"`, an internal hostname,
    /// or a public IP) is rejected before any DNS or connect attempt — this
    /// prevents a malicious or buggy [`LanguageDebugAdapter`] from steering
    /// the adapter into making outbound connections that would leak the
    /// debugged program's variable values to a third party.
    pub fn connect_with_retry(opts: TcpConnectOptions) -> Result<Self, String> {
        // Reject anything that doesn't parse as a loopback IP.  We do not
        // accept hostnames here on purpose — DNS introduces a TOCTOU window
        // where a hostname could resolve to a different address on each
        // attempt.
        let ip: IpAddr = opts.host.parse()
            .map_err(|e| format!("vm host must be a literal IP, got '{}': {e}", opts.host))?;
        if !ip.is_loopback() {
            return Err(format!("vm host must be loopback, got {ip}"));
        }

        let deadline = Instant::now() + Duration::from_millis(opts.connect_budget_ms);
        let mut backoff = Duration::from_millis(50);

        let connect_opts = ConnectOptions {
            connect_timeout: Duration::from_millis(opts.per_attempt_ms),
            read_timeout:    Some(Duration::from_millis(opts.poll_read_ms)),
            write_timeout:   Some(Duration::from_secs(5)),
            buffer_size:     8192,
        };

        loop {
            match connect(&opts.host, opts.port, connect_opts.clone()) {
                Ok(c) => {
                    return Ok(TcpVmConnection {
                        conn: c,
                        pending_events: VecDeque::new(),
                        response_deadline: Duration::from_millis(opts.response_deadline_ms),
                    });
                }
                Err(e) => {
                    if Instant::now() >= deadline {
                        return Err(format!("vm connect: gave up after {}ms: {e}",
                                           opts.connect_budget_ms));
                    }
                    std::thread::sleep(backoff);
                    backoff = (backoff * 2).min(Duration::from_millis(500));
                }
            }
        }
    }

    /// Read one line of at most [`MAX_VM_LINE_BYTES`] bytes (newline included).
    ///
    /// Reads one byte at a time from the underlying buffered TCP connection.
    /// The per-byte cost is dominated by `tcp-client`'s 8 KiB `BufReader`,
    /// so legitimate lines complete in well under a millisecond — the cost
    /// of the loop is only paid when a peer is misbehaving.
    fn read_line_capped(&mut self) -> Result<String, TcpError> {
        let mut buf: Vec<u8> = Vec::with_capacity(256);
        loop {
            if buf.len() >= MAX_VM_LINE_BYTES {
                // Synthesise an IO error so callers can match on it.
                return Err(TcpError::IoError(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("vm line exceeded {MAX_VM_LINE_BYTES} bytes"),
                )));
            }
            let one = self.conn.read_exact(1)?;
            buf.push(one[0]);
            if one[0] == b'\n' {
                break;
            }
            if one[0] == 0 {
                // Treat NUL early-terminator from a closed pipe.
                break;
            }
        }
        String::from_utf8(buf)
            .map_err(|e| TcpError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidData, format!("vm line not utf-8: {e}"),
            )))
    }

    /// Send one JSON command and return the first non-event response.
    ///
    /// Events that arrive on the wire while we wait for the response are
    /// queued in `pending_events` and surface through `poll_event` later.
    ///
    /// ## DoS protections
    /// - The deadline is checked at the top of *every* iteration, regardless
    ///   of whether the previous read succeeded — a malicious VM cannot
    ///   starve the response by streaming events forever.
    /// - `pending_events` is capped at [`MAX_PENDING_EVENTS`].
    /// - Each line read is capped at [`MAX_VM_LINE_BYTES`].
    fn rpc(&mut self, cmd: Value) -> Result<Value, String> {
        // ----- 1. Send command line ----------------------------------------
        let mut line = serde_json::to_string(&cmd)
            .map_err(|e| format!("encode cmd: {e}"))?;
        line.push('\n');
        self.conn.write_all(line.as_bytes()).map_err(|e| format!("vm write: {e}"))?;
        self.conn.flush().map_err(|e| format!("vm flush: {e}"))?;

        // ----- 2. Read until a non-event response arrives ------------------
        let deadline = Instant::now() + self.response_deadline;
        loop {
            // Deadline is checked unconditionally each iteration so a
            // malicious VM cannot starve our response with infinite events.
            if Instant::now() >= deadline {
                return Err("vm response timeout".into());
            }

            let raw = match self.read_line_capped() {
                Ok(s) => s,
                Err(TcpError::Timeout { .. }) => continue,
                Err(e) => return Err(format!("vm read: {e}")),
            };
            if raw.is_empty() {
                return Err("vm closed connection".into());
            }
            let v: Value = serde_json::from_str(raw.trim_end())
                .map_err(|e| format!("vm sent invalid JSON: {e}"))?;
            if v.get("event").is_some() {
                if let Some(ev) = parse_event_value(&v) {
                    if self.pending_events.len() >= MAX_PENDING_EVENTS {
                        return Err(format!(
                            "vm flooded {MAX_PENDING_EVENTS} pending events; closing"
                        ));
                    }
                    self.pending_events.push_back(ev);
                }
                continue;
            }
            return Ok(v);
        }
    }

    /// Helper — assert the response carries `{"ok": true}`.
    fn rpc_ok(&mut self, cmd: Value) -> Result<(), String> {
        let resp = self.rpc(cmd)?;
        if resp.get("ok") == Some(&Value::Bool(true)) {
            Ok(())
        } else {
            Err(format!("vm refused command: {resp}"))
        }
    }
}

impl VmConnection for TcpVmConnection {
    fn set_breakpoint(&mut self, loc: &VmLocation) -> Result<(), String> {
        self.rpc_ok(json!({
            "cmd":    "set_breakpoint",
            "fn":     loc.function,
            "offset": loc.instr_index,
        }))
    }

    fn clear_breakpoint(&mut self, loc: &VmLocation) -> Result<(), String> {
        self.rpc_ok(json!({
            "cmd":    "clear_breakpoint",
            "fn":     loc.function,
            "offset": loc.instr_index,
        }))
    }

    fn cont(&mut self) -> Result<(), String> {
        self.rpc_ok(json!({"cmd": "continue"}))
    }

    fn pause(&mut self) -> Result<(), String> {
        self.rpc_ok(json!({"cmd": "pause"}))
    }

    fn step_instruction(&mut self) -> Result<(), String> {
        self.rpc_ok(json!({"cmd": "step_instruction"}))
    }

    fn get_call_stack(&mut self) -> Result<Vec<VmFrame>, String> {
        let resp = self.rpc(json!({"cmd": "get_call_stack"}))?;
        let arr = resp.get("frames")
            .and_then(|v| v.as_array())
            .ok_or_else(|| format!("malformed get_call_stack response: {resp}"))?;
        let mut out = Vec::with_capacity(arr.len());
        for f in arr {
            let function = f.get("fn").and_then(|v| v.as_str())
                .ok_or_else(|| format!("frame missing 'fn': {f}"))?
                .to_string();
            let instr_index = f.get("offset").and_then(|v| v.as_u64())
                .ok_or_else(|| format!("frame missing 'offset': {f}"))? as usize;
            out.push(VmFrame { location: VmLocation { function, instr_index } });
        }
        Ok(out)
    }

    fn get_slot(&mut self, frame: usize, slot: u32) -> Result<String, String> {
        let resp = self.rpc(json!({
            "cmd":   "get_slot",
            "frame": frame,
            "slot":  slot,
        }))?;
        if let Some(repr) = resp.get("repr").and_then(|v| v.as_str()) {
            return Ok(repr.to_string());
        }
        // Some VMs may return the value under a different key — fall back to
        // the whole response stringified rather than fail.
        Ok(resp.to_string())
    }

    fn poll_event(&mut self) -> Result<Option<StoppedEvent>, String> {
        if let Some(ev) = self.pending_events.pop_front() {
            return Ok(Some(ev));
        }
        match self.read_line_capped() {
            // VM closed the socket cleanly between frames.  This is the
            // expected flow when a program runs to completion without
            // any breakpoints — twig-vm finishes, drops the listener,
            // and the FIN arrives.  Surface it as a clean exit so the
            // dispatch loop emits `exited` + `terminated` and the editor
            // ends the session normally.  Without this branch the
            // adapter prints `vm read: unexpected EOF` on stderr and
            // exits 1, which VS Code reports as "Invalid debug adapter".
            Ok(s) if s.is_empty() => Ok(Some(StoppedEvent::Exited { exit_code: 0 })),
            Ok(s) => {
                let v: Value = serde_json::from_str(s.trim_end())
                    .map_err(|e| format!("vm sent invalid JSON: {e}"))?;
                Ok(parse_event_value(&v))
            }
            Err(TcpError::Timeout { .. }) => Ok(None),
            // Same as the empty-line branch above, but the close happened
            // mid-frame (VM exited while we were waiting for the next
            // event byte).  `read_exact(1)` returns `UnexpectedEof` on a
            // half-open socket — that's the most common shape of a
            // graceful VM exit.
            Err(TcpError::UnexpectedEof { .. }) => {
                Ok(Some(StoppedEvent::Exited { exit_code: 0 }))
            }
            Err(e) => Err(format!("vm read: {e}")),
        }
    }
}

/// Parse a `{event, ...}` JSON object into a [`StoppedEvent`].
///
/// Returns `None` for unrecognised events (so the adapter can ignore them
/// gracefully rather than crashing on a forward-compatible VM).
fn parse_event_value(v: &Value) -> Option<StoppedEvent> {
    let event = v.get("event")?.as_str()?;
    match event {
        "stopped" => {
            let reason = match v.get("reason").and_then(|r| r.as_str()) {
                Some("breakpoint") => StoppedReason::Breakpoint,
                Some("step")       => StoppedReason::Step,
                Some("pause")      => StoppedReason::Pause,
                _                  => StoppedReason::Other,
            };
            let function = v.get("fn").and_then(|f| f.as_str())?.to_string();
            let instr_index = v.get("offset").and_then(|o| o.as_u64())? as usize;
            Some(StoppedEvent::Stopped {
                reason,
                location: VmLocation { function, instr_index },
            })
        }
        "exited" => {
            let exit_code = v.get("exit_code").and_then(|c| c.as_i64()).unwrap_or(0) as i32;
            Some(StoppedEvent::Exited { exit_code })
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn loc(f: &str, i: usize) -> VmLocation { VmLocation::new(f, i) }

    #[test]
    fn vm_location_eq_and_hash() {
        let a = loc("foo", 3);
        let b = loc("foo", 3);
        assert_eq!(a, b);
        let mut s: std::collections::HashSet<VmLocation> = std::collections::HashSet::new();
        s.insert(a);
        assert!(s.contains(&b));
    }

    #[test]
    fn stopped_reason_strings() {
        assert_eq!(StoppedReason::Breakpoint.as_str(), "breakpoint");
        assert_eq!(StoppedReason::Step.as_str(),       "step");
        assert_eq!(StoppedReason::Pause.as_str(),      "pause");
        assert_eq!(StoppedReason::Other.as_str(),      "entry");
    }

    #[test]
    fn mock_set_and_clear_breakpoint() {
        let mut m = MockVmConnection::new();
        m.set_breakpoint(&loc("f", 1)).unwrap();
        m.set_breakpoint(&loc("f", 2)).unwrap();
        assert_eq!(m.breakpoints(), vec![loc("f", 1), loc("f", 2)]);
        m.clear_breakpoint(&loc("f", 1)).unwrap();
        assert_eq!(m.breakpoints(), vec![loc("f", 2)]);
    }

    #[test]
    fn mock_set_breakpoint_idempotent() {
        let mut m = MockVmConnection::new();
        m.set_breakpoint(&loc("f", 1)).unwrap();
        m.set_breakpoint(&loc("f", 1)).unwrap();
        assert_eq!(m.breakpoints().len(), 1);
    }

    #[test]
    fn mock_call_stack_round_trip() {
        let mut m = MockVmConnection::new();
        let frames = vec![
            VmFrame { location: loc("foo", 5) },
            VmFrame { location: loc("main", 2) },
        ];
        m.set_call_stack(frames.clone());
        assert_eq!(m.get_call_stack().unwrap(), frames);
    }

    #[test]
    fn mock_get_slot_default_undef() {
        let mut m = MockVmConnection::new();
        assert_eq!(m.get_slot(0, 7).unwrap(), "<undef>");
    }

    #[test]
    fn mock_get_slot_returns_set_value() {
        let mut m = MockVmConnection::new();
        m.set_slot(0, 1, "42");
        assert_eq!(m.get_slot(0, 1).unwrap(), "42");
    }

    #[test]
    fn mock_poll_event_drains_queue() {
        let mut m = MockVmConnection::new();
        let ev = StoppedEvent::Stopped { reason: StoppedReason::Step, location: loc("f", 3) };
        m.push_event(ev.clone());
        assert_eq!(m.poll_event().unwrap(), Some(ev));
        assert_eq!(m.poll_event().unwrap(), None);
    }

    #[test]
    fn mock_records_call_log() {
        let mut m = MockVmConnection::new();
        m.cont().unwrap();
        m.step_instruction().unwrap();
        m.pause().unwrap();
        let log = m.call_log();
        assert_eq!(log, vec!["cont", "step_instruction", "pause"]);
    }

    #[test]
    fn mock_clones_share_state() {
        // Both clones see the same backing Mutex via Arc — needed because
        // tests hold one clone and DapServer holds the other.
        let m1 = MockVmConnection::new();
        let mut m2 = m1.clone();
        m2.set_breakpoint(&loc("f", 0)).unwrap();
        assert_eq!(m1.breakpoints(), vec![loc("f", 0)]);
    }

    // ---- TCP wire-format tests (parse_event_value) ------------------------

    #[test]
    fn parse_event_stopped_breakpoint() {
        let v = json!({"event": "stopped", "reason": "breakpoint",
                       "fn": "main", "offset": 5});
        let ev = parse_event_value(&v).expect("parsed");
        assert_eq!(ev, StoppedEvent::Stopped {
            reason: StoppedReason::Breakpoint,
            location: loc("main", 5),
        });
    }

    #[test]
    fn parse_event_stopped_step() {
        let v = json!({"event": "stopped", "reason": "step",
                       "fn": "f", "offset": 0});
        let ev = parse_event_value(&v).unwrap();
        assert!(matches!(
            ev,
            StoppedEvent::Stopped { reason: StoppedReason::Step, .. }
        ));
    }

    #[test]
    fn parse_event_unknown_reason_is_other() {
        let v = json!({"event": "stopped", "reason": "made_up",
                       "fn": "f", "offset": 0});
        let ev = parse_event_value(&v).unwrap();
        match ev {
            StoppedEvent::Stopped { reason, .. } => assert_eq!(reason, StoppedReason::Other),
            _ => panic!("expected Stopped"),
        }
    }

    #[test]
    fn parse_event_exited() {
        let v = json!({"event": "exited", "exit_code": 7});
        let ev = parse_event_value(&v).unwrap();
        assert_eq!(ev, StoppedEvent::Exited { exit_code: 7 });
    }

    #[test]
    fn parse_event_unknown_returns_none() {
        let v = json!({"event": "fnord"});
        assert!(parse_event_value(&v).is_none());
    }

    #[test]
    fn parse_event_missing_event_field_returns_none() {
        let v = json!({"ok": true});
        assert!(parse_event_value(&v).is_none());
    }

    #[test]
    fn tcp_connect_to_dead_port_fails_within_budget() {
        // Connect to a port nothing is listening on; ensure we give up
        // within the budget (test budget = 250ms).
        let opts = TcpConnectOptions {
            port: 1, // privileged port; never listening for normal users
            host: "127.0.0.1".to_string(),
            connect_budget_ms: 250,
            per_attempt_ms: 50,
            poll_read_ms: 50,
            response_deadline_ms: 250,
        };
        let started = std::time::Instant::now();
        let res = TcpVmConnection::connect_with_retry(opts);
        assert!(res.is_err());
        // Verify we didn't loop forever.
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    // ---- SSRF guard tests -------------------------------------------------

    #[test]
    fn tcp_connect_rejects_non_loopback_ip() {
        // Non-loopback IPv4 — must be rejected before any DNS or connect.
        let opts = TcpConnectOptions {
            host: "169.254.169.254".to_string(), // cloud metadata IP
            port: 80,
            ..TcpConnectOptions::default()
        };
        let started = std::time::Instant::now();
        let err = TcpVmConnection::connect_with_retry(opts).unwrap_err();
        assert!(err.contains("loopback"), "got: {err}");
        // Must reject without ever attempting a connect (within ~10ms).
        assert!(started.elapsed() < Duration::from_millis(100));
    }

    #[test]
    fn tcp_connect_rejects_hostname() {
        // Hostnames are rejected — DNS introduces a TOCTOU window.
        let opts = TcpConnectOptions {
            host: "localhost".to_string(),
            port: 1,
            ..TcpConnectOptions::default()
        };
        let err = TcpVmConnection::connect_with_retry(opts).unwrap_err();
        assert!(err.contains("literal IP"), "got: {err}");
    }

    #[test]
    fn tcp_connect_accepts_ipv6_loopback() {
        // ::1 is loopback and must be accepted (the connect itself will fail
        // since nothing is listening, but the SSRF guard must let it through).
        let opts = TcpConnectOptions {
            host: "::1".to_string(),
            port: 1,
            connect_budget_ms: 100,
            per_attempt_ms: 50,
            ..TcpConnectOptions::default()
        };
        let err = TcpVmConnection::connect_with_retry(opts).unwrap_err();
        // The SSRF guard let us past — the error is now a connect error,
        // not a "must be loopback" error.
        assert!(!err.contains("loopback"), "got: {err}");
    }
}
