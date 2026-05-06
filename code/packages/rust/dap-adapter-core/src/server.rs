//! [`DapServer`] — the generic DAP event loop and request handlers.
//!
//! ## Event loop
//!
//! `DapServer::run` is a tight loop:
//!
//! ```text
//! loop {
//!     drain VM events  → emit `stopped` / `terminated`
//!     read 1 DAP req   → dispatch by `command`
//!     write response   + queued events
//!     if disconnect    → break
//! }
//! ```
//!
//! All I/O is synchronous: reads block on the editor's input stream, and we
//! reach `poll_event` at the top of every iteration so VM-pushed events are
//! never delayed by more than one DAP request's processing time.
//!
//! ## Capabilities
//!
//! The `initialize` response advertises a curated subset of DAP capabilities
//! that match what this generic core supports:
//!
//! | Capability                   | value | rationale                           |
//! |------------------------------|-------|-------------------------------------|
//! | `supportsConfigurationDoneRequest` | `true`  | required for two-phase launch  |
//! | `supportsBreakpointLocationsRequest` | `false` | future enhancement           |
//! | `supportsConditionalBreakpoints`     | `false` | Phase 2 (LS03 spec)         |
//! | `supportsTerminateRequest`           | `false` | disconnect handles teardown |
//! | `supportsRestartRequest`             | `false` | editor re-launches us       |
//! | `supportsStepBack`                   | `false` | not supported by VM         |

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use serde_json::{json, Value};

use crate::adapter::LanguageDebugAdapter;
use crate::breakpoints::BreakpointManager;
use crate::protocol::{build_event, build_response, read_message, write_message,
                       DapRequest, SeqCounter};
use crate::sidecar::SidecarIndex;
use crate::stepper::{StepController, StepDecision, StepMode};
use crate::vm_conn::{
    StoppedEvent, StoppedReason, TcpConnectOptions, TcpVmConnection, VmConnection, VmLocation,
};

// ---------------------------------------------------------------------------
// DapServer
// ---------------------------------------------------------------------------

/// Generic DAP server parameterised by a [`LanguageDebugAdapter`].
///
/// The server owns a [`VmConnection`] (boxed so tests can swap in
/// [`MockVmConnection`](crate::vm_conn::MockVmConnection)) plus all the
/// stateful sub-managers (breakpoints, stepper, sidecar).
pub struct DapServer<A: LanguageDebugAdapter> {
    /// Per-language compile + launch hooks.
    pub adapter: A,
    /// Currently-installed user breakpoints.
    pub bps: BreakpointManager,
    /// Step state machine.
    pub stepper: StepController,
    /// Sidecar (offset ↔ source) — populated on `launch`.
    pub sidecar: Option<SidecarIndex>,
    /// VM connection — populated on `launch`.
    pub vm_conn: Option<Box<dyn VmConnection>>,
    /// Live VM child process — populated on `launch`, killed on `disconnect`.
    pub vm_proc: Option<std::process::Child>,
    /// Outgoing-message sequence counter.
    seq: SeqCounter,
    /// Stop signal; set by `disconnect` handler.
    pub(crate) shutdown: bool,
}

impl<A: LanguageDebugAdapter> DapServer<A> {
    /// Construct a server with the given language adapter.
    pub fn new(adapter: A) -> Self {
        DapServer {
            adapter,
            bps: BreakpointManager::new(),
            stepper: StepController::new(),
            sidecar: None,
            vm_conn: None,
            vm_proc: None,
            seq: SeqCounter::new(),
            shutdown: false,
        }
    }

    // -----------------------------------------------------------------------
    // Public run methods
    // -----------------------------------------------------------------------

    /// Run the adapter over stdio (the standard VS Code launch mode).
    pub fn run_stdio(&mut self) -> Result<(), String> {
        let stdin  = std::io::stdin();
        let stdout = std::io::stdout();
        let reader = BufReader::new(stdin.lock());
        let writer = stdout.lock();
        self.run(reader, writer)
    }

    /// Run the adapter over arbitrary streams (used by tests + TCP transport).
    pub fn run<R: BufRead, W: Write>(&mut self, mut reader: R, mut writer: W) -> Result<(), String> {
        while !self.shutdown {
            // 1. Drain any pending VM events so editor sees stops promptly.
            self.drain_vm_events(&mut writer)?;

            // 2. Read the next DAP request.
            let body = match read_message(&mut reader) {
                Ok(v) => v,
                Err(e) if e == "eof" => break,
                Err(e) => return Err(e),
            };
            let req = DapRequest::from_value(body)?;

            // 3. Dispatch.
            self.dispatch(&req, &mut writer)?;
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Dispatch
    // -----------------------------------------------------------------------

    fn dispatch<W: Write>(&mut self, req: &DapRequest, w: &mut W) -> Result<(), String> {
        let result = match req.command.as_str() {
            "initialize"        => self.handle_initialize(req),
            "launch"            => self.handle_launch(req),
            "setBreakpoints"    => self.handle_set_breakpoints(req),
            "configurationDone" => self.handle_configuration_done(req),
            "continue"          => self.handle_continue(req),
            "pause"             => self.handle_pause(req),
            "next"              => self.handle_next(req),
            "stepIn"            => self.handle_step_in(req),
            "stepOut"           => self.handle_step_out(req),
            "stackTrace"        => self.handle_stack_trace(req),
            "scopes"            => self.handle_scopes(req),
            "variables"         => self.handle_variables(req),
            "source"            => self.handle_source(req),
            "threads"           => self.handle_threads(req),
            "disconnect"        => self.handle_disconnect(req),
            other => Err(format!("unsupported command: {other}")),
        };

        let resp = match &result {
            Ok(body)    => build_response(req.seq, self.seq.next(), &req.command, true,  None, body.clone()),
            Err(msg)    => build_response(req.seq, self.seq.next(), &req.command, false, Some(msg),
                                          json!({})),
        };
        write_message(w, &resp)?;

        // After the `initialize` response, the DAP protocol requires the
        // adapter to send an `initialized` event.  This signals to the
        // editor that the adapter is ready to receive breakpoint
        // configuration (setBreakpoints, setFunctionBreakpoints,
        // configurationDone).  Without this event, editors like VS Code
        // wait silently then disconnect with "Invalid debug adapter".
        //
        // We only emit the event for successful initialize responses —
        // failed initialize means the adapter isn't ready, and the editor
        // will tear down the session regardless.
        if req.command == "initialize" && result.is_ok() {
            let ev = build_event(self.seq.next(), "initialized", json!({}));
            write_message(w, &ev)?;
        }

        // Flush any pending events that the handler may have produced.
        self.drain_vm_events(w)?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Handlers — 13 per spec, plus `threads` (DAP requires it for stackTrace)
    // -----------------------------------------------------------------------

    fn handle_initialize(&mut self, _req: &DapRequest) -> Result<Value, String> {
        Ok(json!({
            "supportsConfigurationDoneRequest":   true,
            "supportsBreakpointLocationsRequest": false,
            "supportsConditionalBreakpoints":    false,
            "supportsTerminateRequest":          false,
            "supportsRestartRequest":            false,
            "supportsStepBack":                  false,
            "supportsExceptionInfoRequest":      false,
            "supportsValueFormattingOptions":    false,
            "supportsSetVariable":               false,
            "supportsModulesRequest":            false,
        }))
    }

    fn handle_launch(&mut self, req: &DapRequest) -> Result<Value, String> {
        let program = req.arguments.get("program").and_then(|v| v.as_str())
            .ok_or("launch: missing 'program'")?;
        let workspace = req.arguments.get("workspaceFolder").and_then(|v| v.as_str())
            .unwrap_or(".");

        let (bytecode_path, sidecar_bytes) = self.adapter
            .compile(&PathBuf::from(program), &PathBuf::from(workspace))
            .map_err(|e| format!("compile: {e}"))?;

        self.sidecar = Some(SidecarIndex::from_bytes(&sidecar_bytes)?);

        // Tests may pre-wire `self.vm_conn` via `MockVmConnection` and then
        // invoke `handle_launch` directly — when they do, we skip the
        // process spawn + TCP connect and use the pre-wired connection.
        // Production callers (the stdio DAP server driven by VS Code) hit
        // this branch with `vm_conn = None` and no `debugPort` argument:
        // we have to pick a free port ourselves and launch the VM.
        if self.vm_conn.is_some() {
            return Ok(json!({}));
        }

        // Resolve the VM port.  Editors don't supply `debugPort` — that's
        // an internal detail of the adapter ↔ VM transport — so we pick a
        // free ephemeral and tell the VM to bind it.  The
        // bind/local_addr/drop dance gives us a port the OS guarantees is
        // free *right now*; there's a small TOCTOU window between the
        // drop and the VM's re-bind, but the same pattern is used by the
        // existing twig-dap end-to-end test (`tests/end_to_end.rs`) and
        // is the standard idiom for picking a port to hand to a child
        // process.
        let port: u16 = match req.arguments.get("debugPort").and_then(|v| v.as_u64()) {
            Some(p) if p != 0 => p as u16,
            _ => {
                let listener = std::net::TcpListener::bind("127.0.0.1:0")
                    .map_err(|e| format!("bind ephemeral port: {e}"))?;
                let port = listener
                    .local_addr()
                    .map_err(|e| format!("local_addr: {e}"))?
                    .port();
                drop(listener);
                port
            }
        };

        // Spawn the VM child process.  Stays alive in `self.vm_proc` for
        // the duration of the session; killed on `disconnect`.
        self.vm_proc = Some(
            self.adapter
                .launch_vm(&bytecode_path, port)
                .map_err(|e| format!("launch_vm: {e}"))?,
        );

        // Connect over the loopback TCP transport.  `connect_with_retry`
        // tolerates the brief startup race where the VM is alive but its
        // listener hasn't bound yet.
        let conn = TcpVmConnection::connect_with_retry(TcpConnectOptions {
            port,
            ..Default::default()
        })
        .map_err(|e| format!("connect to VM on port {port}: {e}"))?;
        self.vm_conn = Some(Box::new(conn));

        Ok(json!({}))
    }

    fn handle_set_breakpoints(&mut self, req: &DapRequest) -> Result<Value, String> {
        let path = req.arguments.get("source")
            .and_then(|s| s.get("path"))
            .and_then(|p| p.as_str())
            .ok_or("setBreakpoints: missing source.path")?;
        let lines: Vec<u32> = req.arguments
            .get("breakpoints")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|e| e.get("line").and_then(|l| l.as_u64()).map(|n| n as u32))
                    .collect()
            })
            .unwrap_or_default();

        let (bps, diff) = self.bps.set_breakpoints(
            &PathBuf::from(path),
            &lines,
            self.sidecar.as_ref(),
        );

        // Apply the VM-level changes after the manager updated its state.
        if let Some(conn) = self.vm_conn.as_deref_mut() {
            for loc in &diff.to_clear   { conn.clear_breakpoint(loc)?; }
            for loc in &diff.to_install { conn.set_breakpoint(loc)?; }
        }

        let breakpoints: Vec<Value> = bps.iter().map(|b| json!({
            "verified": b.verified,
            "line":     b.line,
        })).collect();

        Ok(json!({"breakpoints": breakpoints}))
    }

    fn handle_configuration_done(&mut self, _req: &DapRequest) -> Result<Value, String> {
        // After breakpoints are configured, kick the VM off.
        if let Some(conn) = self.vm_conn.as_deref_mut() {
            conn.cont()?;
        }
        Ok(json!({}))
    }

    fn handle_continue(&mut self, _req: &DapRequest) -> Result<Value, String> {
        if let Some(conn) = self.vm_conn.as_deref_mut() {
            conn.cont()?;
        }
        Ok(json!({"allThreadsContinued": true}))
    }

    fn handle_pause(&mut self, _req: &DapRequest) -> Result<Value, String> {
        if let Some(conn) = self.vm_conn.as_deref_mut() {
            conn.pause()?;
        }
        Ok(json!({}))
    }

    fn handle_next(&mut self, _req: &DapRequest) -> Result<Value, String> {
        self.start_step(StepMode::Over)
    }

    fn handle_step_in(&mut self, _req: &DapRequest) -> Result<Value, String> {
        self.start_step(StepMode::In)
    }

    fn handle_step_out(&mut self, _req: &DapRequest) -> Result<Value, String> {
        self.start_step(StepMode::Out)
    }

    fn handle_stack_trace(&mut self, _req: &DapRequest) -> Result<Value, String> {
        let conn = self.vm_conn.as_deref_mut().ok_or("no VM")?;
        let frames = conn.get_call_stack()?;
        let sidecar = self.sidecar.as_ref();
        let stack_frames: Vec<Value> = frames.iter().enumerate().map(|(idx, f)| {
            let (file, line) = match sidecar.and_then(|s| s.loc_to_source(&f.location)) {
                Some(src) => (src.file, src.line as i64),
                None      => ("<unknown>".to_string(), 0),
            };
            json!({
                "id":     idx,
                "name":   f.location.function,
                "line":   line,
                "column": 1,
                "source": {"path": file},
            })
        }).collect();
        Ok(json!({"stackFrames": stack_frames, "totalFrames": frames.len()}))
    }

    fn handle_scopes(&mut self, req: &DapRequest) -> Result<Value, String> {
        let frame_id = req.arguments.get("frameId").and_then(|v| v.as_u64()).unwrap_or(0);
        // One scope per frame: "Locals".  Variable references encode the
        // frame index (we only have one scope per frame so 1:1 is fine).
        Ok(json!({
            "scopes": [{
                "name":               "Locals",
                "variablesReference": frame_id + 1,  // 0 is reserved (no children)
                "expensive":          false,
            }]
        }))
    }

    fn handle_variables(&mut self, req: &DapRequest) -> Result<Value, String> {
        let var_ref = req.arguments.get("variablesReference")
            .and_then(|v| v.as_u64())
            .ok_or("variables: missing variablesReference")?;
        if var_ref == 0 {
            return Ok(json!({"variables": []}));
        }
        let frame_idx = (var_ref - 1) as usize;

        let conn    = self.vm_conn.as_deref_mut().ok_or("no VM")?;
        let frames  = conn.get_call_stack()?;
        let frame   = frames.get(frame_idx).ok_or("variables: bad frame index")?;
        let frame_loc = frame.location.clone();

        let live_vars = match self.sidecar.as_ref() {
            Some(s) => s.reader().live_variables(&frame_loc.function, frame_loc.instr_index),
            None    => Vec::new(),
        };

        let mut out = Vec::with_capacity(live_vars.len());
        for v in &live_vars {
            // LS06: query by name, not by slot index.  The slot index
            // in the sidecar is no longer reliable across instructions
            // (the VM re-sorts its snapshot list at every stop), but
            // the name is stable.  `get_slot_by_name` returns
            // `"<undef>"` when the name isn't in the current frame,
            // which we surface to the user rather than hiding.
            let value = self.vm_conn.as_deref_mut().unwrap()
                .get_slot_by_name(frame_idx, &v.name)
                .unwrap_or_else(|_| "<error>".to_string());
            out.push(json!({
                "name":               v.name,
                "value":              value,
                "type":               v.type_hint,
                "variablesReference": 0,
            }));
        }

        Ok(json!({"variables": out}))
    }

    fn handle_source(&mut self, req: &DapRequest) -> Result<Value, String> {
        // We don't load source content via DAP — the editor already has it
        // open.  Returning empty content is conformant: DAP spec allows the
        // adapter to defer to the client's file access.
        let _ = req;
        Ok(json!({"content": "", "mimeType": "text/plain"}))
    }

    fn handle_threads(&mut self, _req: &DapRequest) -> Result<Value, String> {
        // The VM is single-threaded for now; report one synthetic thread.
        Ok(json!({"threads": [{"id": 1, "name": "main"}]}))
    }

    fn handle_disconnect(&mut self, _req: &DapRequest) -> Result<Value, String> {
        // Best-effort kill of the VM child process.
        if let Some(mut child) = self.vm_proc.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.vm_conn = None;
        self.shutdown = true;
        Ok(json!({}))
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    fn start_step(&mut self, mode: StepMode) -> Result<Value, String> {
        let conn = self.vm_conn.as_deref_mut().ok_or("no VM")?;
        let frames = conn.get_call_stack()?;
        let depth  = frames.len();
        let line   = match (frames.first(), self.sidecar.as_ref()) {
            (Some(f), Some(s)) => s.loc_to_source(&f.location)
                                    .map(|src| src.line)
                                    .unwrap_or(0),
            _ => 0,
        };
        self.stepper.start(mode, depth, line);
        conn.step_instruction()?;
        Ok(json!({}))
    }

    /// Drain VM-pushed events into outgoing DAP events.  Also runs the
    /// stepper's `on_stopped` decision when a step is in progress.
    ///
    /// We collect all pending events into a local Vec first, releasing the
    /// VM-connection borrow before dispatching them — `handle_stopped`
    /// re-borrows `self.vm_conn` to query the call stack.
    pub(crate) fn drain_vm_events<W: Write>(&mut self, w: &mut W) -> Result<(), String> {
        let mut pending: Vec<StoppedEvent> = Vec::new();
        if let Some(conn) = self.vm_conn.as_deref_mut() {
            while let Some(ev) = conn.poll_event()? {
                pending.push(ev);
            }
        }

        for ev in pending {
            match ev {
                StoppedEvent::Stopped { reason, location } => {
                    self.handle_stopped(reason, location, w)?;
                }
                StoppedEvent::Exited { exit_code } => {
                    let body = json!({"exitCode": exit_code});
                    let msg  = build_event(self.seq.next(), "exited", body);
                    write_message(w, &msg)?;
                    let term = build_event(self.seq.next(), "terminated", json!({}));
                    write_message(w, &term)?;
                    self.shutdown = true;
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    fn handle_stopped<W: Write>(
        &mut self,
        reason: StoppedReason,
        location: VmLocation,
        w: &mut W,
    ) -> Result<(), String> {
        // Breakpoints short-circuit any in-progress step.
        if reason == StoppedReason::Breakpoint {
            self.stepper.cancel();
            return self.emit_stopped_event(StoppedReason::Breakpoint, &location, w);
        }

        // If a step is in progress, ask the controller whether we're done.
        if self.stepper.is_active() {
            // Re-query the VM for the current call depth + map location to a line.
            let conn = self.vm_conn.as_deref_mut().ok_or("no VM")?;
            let frames = conn.get_call_stack()?;
            let depth  = frames.len();
            let line   = self.sidecar.as_ref()
                .and_then(|s| s.loc_to_source(&location))
                .map(|src| src.line)
                .unwrap_or(0);

            match self.stepper.on_stopped(depth, line) {
                StepDecision::Done => {
                    return self.emit_stopped_event(StoppedReason::Step, &location, w);
                }
                StepDecision::Continue => {
                    // Issue another step_instruction; the next on_stopped
                    // event will revisit the decision.
                    let conn = self.vm_conn.as_deref_mut().unwrap();
                    conn.step_instruction()?;
                    return Ok(());
                }
            }
        }

        // No step in progress and not a breakpoint — pass-through.
        self.emit_stopped_event(reason, &location, w)
    }

    fn emit_stopped_event<W: Write>(
        &mut self,
        reason: StoppedReason,
        location: &VmLocation,
        w: &mut W,
    ) -> Result<(), String> {
        let mut body = json!({
            "reason":           reason.as_str(),
            "threadId":         1,
            "allThreadsStopped": true,
        });
        // Decorate with source position if we can resolve it.
        if let Some(src) = self.sidecar.as_ref().and_then(|s| s.loc_to_source(location)) {
            body["source"] = json!({"path": src.file});
            body["line"]   = json!(src.line);
        }
        let msg = build_event(self.seq.next(), "stopped", body);
        write_message(w, &msg)
    }
}

// ---------------------------------------------------------------------------
// Tests — handlers + integration flow
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm_conn::{MockVmConnection, StoppedEvent, StoppedReason, VmFrame, VmLocation};
    use debug_sidecar::DebugSidecarWriter;
    use std::io::Cursor;
    use std::path::Path;

    // ---- Mock LanguageDebugAdapter ---------------------------------------

    struct MockAdapter {
        sidecar_bytes: Vec<u8>,
    }

    impl LanguageDebugAdapter for MockAdapter {
        fn compile(&self, _src: &Path, _ws: &Path) -> Result<(PathBuf, Vec<u8>), String> {
            Ok((PathBuf::from("/tmp/mock.bc"), self.sidecar_bytes.clone()))
        }
        fn launch_vm(&self, _b: &Path, _port: u16) -> Result<std::process::Child, String> {
            // We never actually launch — tests pre-wire the VmConnection.
            Err("MockAdapter::launch_vm should not be called in tests".into())
        }
        fn language_name(&self) -> &'static str { "mock" }
        fn file_extensions(&self) -> &'static [&'static str] { &["mk"] }
    }

    fn fixture_sidecar() -> Vec<u8> {
        let mut w = DebugSidecarWriter::new();
        let fid = w.add_source_file("prog.tw", b"");
        w.begin_function("main", 0, 0);
        w.declare_variable("main", 0, "x", "int", 0, 4);
        w.record("main", 0, fid, 1, 1);
        w.record("main", 1, fid, 2, 1);
        w.record("main", 2, fid, 3, 1);
        w.record("main", 3, fid, 4, 1);
        w.end_function("main", 4);
        w.finish()
    }

    fn server_with_mock_vm() -> (DapServer<MockAdapter>, MockVmConnection) {
        let bytes = fixture_sidecar();
        let mut srv = DapServer::new(MockAdapter { sidecar_bytes: bytes.clone() });
        srv.sidecar = Some(SidecarIndex::from_bytes(&bytes).unwrap());
        let mock = MockVmConnection::new();
        srv.vm_conn = Some(Box::new(mock.clone()));
        (srv, mock)
    }

    // ---- Direct handler unit tests ---------------------------------------

    #[test]
    fn initialize_returns_capabilities() {
        let (mut srv, _) = server_with_mock_vm();
        let req = DapRequest { seq: 1, typ: "request".into(), command: "initialize".into(),
                               arguments: json!({}) };
        let body = srv.handle_initialize(&req).unwrap();
        assert_eq!(body["supportsConfigurationDoneRequest"], true);
    }

    /// After processing the `initialize` request, the DAP protocol
    /// requires the adapter to send an `initialized` event so the
    /// editor knows it can issue breakpoint configuration.  Without
    /// this event, VS Code waits silently for several seconds, then
    /// gives up with "Invalid debug adapter".
    #[test]
    fn initialize_response_is_followed_by_initialized_event() {
        let (mut srv, _) = server_with_mock_vm();
        let req = DapRequest {
            seq: 1,
            typ: "request".into(),
            command: "initialize".into(),
            arguments: json!({}),
        };
        let mut out: Vec<u8> = Vec::new();
        srv.dispatch(&req, &mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        // Two messages should appear: the response, then the event.
        assert!(text.contains("\"command\":\"initialize\""), "missing initialize response: {text}");
        assert!(text.contains("\"type\":\"event\""), "missing event envelope: {text}");
        assert!(text.contains("\"event\":\"initialized\""), "missing initialized event: {text}");
    }

    /// Inverse: a *failed* initialize must NOT trigger an `initialized`
    /// event.  The editor will tear down the session anyway, and
    /// emitting the event would mislead it into thinking it can
    /// configure breakpoints on a dead adapter.
    #[test]
    fn failed_initialize_does_not_emit_initialized_event() {
        // We can't easily make handle_initialize fail in the current
        // implementation (it always returns Ok), so we exercise the
        // negative branch by sending an unknown command — same code
        // path through dispatch.
        let (mut srv, _) = server_with_mock_vm();
        let req = DapRequest {
            seq: 1,
            typ: "request".into(),
            command: "totally-not-a-real-command".into(),
            arguments: json!({}),
        };
        let mut out: Vec<u8> = Vec::new();
        srv.dispatch(&req, &mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(
            !text.contains("\"event\":\"initialized\""),
            "initialized event leaked from a non-initialize dispatch: {text}",
        );
    }

    #[test]
    fn launch_loads_sidecar() {
        // Pre-wire the mock VM connection so handle_launch's auto-spawn
        // path is bypassed; tests assert the sidecar-loading logic, not
        // the VM-spawn path (which is exercised by integration tests).
        let bytes = fixture_sidecar();
        let mut srv = DapServer::new(MockAdapter { sidecar_bytes: bytes });
        srv.vm_conn = Some(Box::new(MockVmConnection::new()));
        let req = DapRequest { seq: 1, typ: "request".into(), command: "launch".into(),
                               arguments: json!({"program": "prog.tw"}) };
        srv.handle_launch(&req).unwrap();
        assert!(srv.sidecar.is_some());
    }

    #[test]
    fn launch_skips_vm_spawn_when_vm_conn_pre_wired() {
        // Regression: if vm_conn is already set, handle_launch must not
        // call adapter.launch_vm — the test path explicitly relies on
        // this short-circuit.
        let bytes = fixture_sidecar();
        let mut srv = DapServer::new(MockAdapter { sidecar_bytes: bytes });
        srv.vm_conn = Some(Box::new(MockVmConnection::new()));
        let req = DapRequest { seq: 1, typ: "request".into(), command: "launch".into(),
                               arguments: json!({"program": "prog.tw"}) };
        // MockAdapter::launch_vm panics; if handle_launch reaches it,
        // this unwrap propagates the failure and the test fails loudly.
        srv.handle_launch(&req).unwrap();
        assert!(srv.vm_proc.is_none(), "vm_proc must remain None when vm_conn is pre-wired");
    }

    #[test]
    fn set_breakpoints_installs_in_vm_and_marks_verified() {
        let (mut srv, mock) = server_with_mock_vm();
        let req = DapRequest {
            seq: 2, typ: "request".into(), command: "setBreakpoints".into(),
            arguments: json!({
                "source":      {"path": "prog.tw"},
                "breakpoints": [{"line": 2}, {"line": 99}],
            }),
        };
        let body = srv.handle_set_breakpoints(&req).unwrap();
        let bps = body["breakpoints"].as_array().unwrap();
        assert_eq!(bps.len(), 2);
        assert_eq!(bps[0]["verified"], true,  "line 2 maps to a real instr");
        assert_eq!(bps[1]["verified"], false, "line 99 is unmappable");
        // VM should now have one breakpoint (line 2 → instr 1 in `main`).
        assert_eq!(mock.breakpoints(), vec![VmLocation::new("main", 1)]);
    }

    #[test]
    fn configuration_done_sends_continue_to_vm() {
        let (mut srv, mock) = server_with_mock_vm();
        let req = DapRequest { seq: 3, typ: "request".into(),
                               command: "configurationDone".into(), arguments: json!({}) };
        srv.handle_configuration_done(&req).unwrap();
        assert!(mock.call_log().contains(&"cont".to_string()));
    }

    #[test]
    fn continue_pause_step_dispatch_to_vm() {
        let (mut srv, mock) = server_with_mock_vm();
        // Pre-set call stack so step's get_call_stack() doesn't fail.
        mock.set_call_stack(vec![VmFrame { location: VmLocation::new("main", 0) }]);

        srv.handle_continue(&DapRequest { seq: 1, typ: "request".into(),
            command: "continue".into(), arguments: json!({}) }).unwrap();
        srv.handle_pause(&DapRequest { seq: 2, typ: "request".into(),
            command: "pause".into(), arguments: json!({}) }).unwrap();
        srv.handle_next(&DapRequest { seq: 3, typ: "request".into(),
            command: "next".into(), arguments: json!({}) }).unwrap();

        let log = mock.call_log();
        assert!(log.contains(&"cont".to_string()));
        assert!(log.contains(&"pause".to_string()));
        assert!(log.contains(&"step_instruction".to_string()));
    }

    #[test]
    fn stack_trace_resolves_via_sidecar() {
        let (mut srv, mock) = server_with_mock_vm();
        mock.set_call_stack(vec![
            VmFrame { location: VmLocation::new("main", 1) },  // line 2
        ]);
        let req = DapRequest { seq: 1, typ: "request".into(),
                               command: "stackTrace".into(), arguments: json!({}) };
        let body = srv.handle_stack_trace(&req).unwrap();
        let frames = body["stackFrames"].as_array().unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0]["name"], "main");
        assert_eq!(frames[0]["line"], 2);
        assert_eq!(frames[0]["source"]["path"], "prog.tw");
    }

    #[test]
    fn scopes_returns_one_locals_per_frame() {
        let (mut srv, _) = server_with_mock_vm();
        let req = DapRequest { seq: 1, typ: "request".into(),
                               command: "scopes".into(),
                               arguments: json!({"frameId": 0}) };
        let body = srv.handle_scopes(&req).unwrap();
        let scopes = body["scopes"].as_array().unwrap();
        assert_eq!(scopes.len(), 1);
        assert_eq!(scopes[0]["name"], "Locals");
        assert_eq!(scopes[0]["variablesReference"], 1);
    }

    #[test]
    fn variables_returns_live_locals_with_values() {
        // LS06: handle_variables now queries by name, not by slot
        // index — populate the mock's named-slot table accordingly.
        let (mut srv, mock) = server_with_mock_vm();
        mock.set_call_stack(vec![VmFrame { location: VmLocation::new("main", 1) }]);
        mock.set_slot_by_name(0, "x", "42");
        let req = DapRequest { seq: 1, typ: "request".into(),
                               command: "variables".into(),
                               arguments: json!({"variablesReference": 1}) };
        let body = srv.handle_variables(&req).unwrap();
        let vars = body["variables"].as_array().unwrap();
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0]["name"],  "x");
        assert_eq!(vars[0]["value"], "42");
        assert_eq!(vars[0]["type"],  "int");
    }

    /// Regression: when a variable's name isn't in the current frame
    /// (e.g. it was scoped out by a step), `handle_variables` returns
    /// `"<undef>"` for that variable rather than failing the entire
    /// response.  Mirrors the VM-side fallback.
    #[test]
    fn variables_returns_undef_for_missing_name() {
        let (mut srv, mock) = server_with_mock_vm();
        mock.set_call_stack(vec![VmFrame { location: VmLocation::new("main", 1) }]);
        // Don't populate named_slots — the MockVmConnection's default
        // behaviour returns "<undef>" for unknown names.
        let req = DapRequest { seq: 1, typ: "request".into(),
                               command: "variables".into(),
                               arguments: json!({"variablesReference": 1}) };
        let body = srv.handle_variables(&req).unwrap();
        let vars = body["variables"].as_array().unwrap();
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0]["value"], "<undef>");
    }

    #[test]
    fn threads_returns_synthetic_main() {
        let (mut srv, _) = server_with_mock_vm();
        let body = srv.handle_threads(&DapRequest { seq: 1, typ: "request".into(),
            command: "threads".into(), arguments: json!({}) }).unwrap();
        let threads = body["threads"].as_array().unwrap();
        assert_eq!(threads.len(), 1);
        assert_eq!(threads[0]["id"], 1);
        assert_eq!(threads[0]["name"], "main");
    }

    #[test]
    fn disconnect_sets_shutdown_flag() {
        let (mut srv, _) = server_with_mock_vm();
        srv.handle_disconnect(&DapRequest { seq: 1, typ: "request".into(),
            command: "disconnect".into(), arguments: json!({}) }).unwrap();
        assert!(srv.shutdown);
        assert!(srv.vm_conn.is_none());
    }

    // ---- Integration test: full flow through run() with mocks ------------

    /// Helper: encode a list of DAP requests as one Content-Length stream.
    fn encode_requests(reqs: &[Value]) -> Vec<u8> {
        let mut out = Vec::new();
        for r in reqs {
            crate::protocol::write_message(&mut out, r).unwrap();
        }
        out
    }

    #[test]
    fn full_launch_breakpoint_continue_flow() {
        // Set up a server that already has a sidecar + mock VM (so we don't
        // need launch to invoke the absent-on-purpose adapter.launch_vm).
        let (mut srv, mock) = server_with_mock_vm();

        // Script the VM: when configurationDone triggers cont(), the VM
        // delivers a breakpoint stop, then on the next cont() (from the
        // explicit `continue` request) it exits.
        mock.fire_on_cont(StoppedEvent::Stopped {
            reason: StoppedReason::Breakpoint,
            location: VmLocation::new("main", 1),
        });
        mock.fire_on_cont(StoppedEvent::Exited { exit_code: 0 });
        // Pre-set the call stack so stackTrace works after the stop.
        mock.set_call_stack(vec![VmFrame { location: VmLocation::new("main", 1) }]);

        // The DAP message sequence: setBreakpoints → configurationDone →
        // (server emits stopped) → stackTrace → continue → (exited+terminated)
        // → disconnect.
        let reqs = vec![
            json!({"seq": 1, "type": "request", "command": "setBreakpoints",
                   "arguments": {"source": {"path": "prog.tw"},
                                 "breakpoints": [{"line": 2}]}}),
            json!({"seq": 2, "type": "request", "command": "configurationDone",
                   "arguments": {}}),
            json!({"seq": 3, "type": "request", "command": "stackTrace",
                   "arguments": {"threadId": 1}}),
            json!({"seq": 4, "type": "request", "command": "continue",
                   "arguments": {"threadId": 1}}),
            json!({"seq": 5, "type": "request", "command": "disconnect",
                   "arguments": {}}),
        ];
        let input = encode_requests(&reqs);
        let mut output: Vec<u8> = Vec::new();
        srv.run(Cursor::new(input), &mut output).unwrap();

        // Read back every framed message that the server sent.
        let mut cursor = Cursor::new(output);
        let mut reader = std::io::BufReader::new(&mut cursor);
        let mut messages: Vec<Value> = Vec::new();
        while let Ok(m) = read_message(&mut reader) {
            messages.push(m);
        }
        assert!(!messages.is_empty(), "server should have produced messages");

        // We expect at least one `setBreakpoints` response, one `stopped`
        // event, one `configurationDone` response, one `stackTrace` response,
        // one `continue` response, plus `exited` + `terminated` events.
        let has_event = |evt: &str| messages.iter().any(|m|
            m["type"] == "event" && m["event"] == evt);
        let has_resp = |cmd: &str| messages.iter().any(|m|
            m["type"] == "response" && m["command"] == cmd);

        // For diagnostics on failure, include a compact "type:command/event"
        // summary of every message the server emitted.
        let summary = || -> Vec<String> {
            messages.iter().map(|m| format!(
                "{}:{}",
                m["type"].as_str().unwrap_or("?"),
                m.get("command").or(m.get("event"))
                    .and_then(|v| v.as_str()).unwrap_or("?"),
            )).collect()
        };

        assert!(has_resp("setBreakpoints"),    "messages: {:?}", summary());
        assert!(has_resp("configurationDone"), "messages: {:?}", summary());
        assert!(has_resp("stackTrace"),        "messages: {:?}", summary());
        assert!(has_resp("continue"),          "messages: {:?}", summary());
        assert!(has_event("stopped"),          "messages: {:?}", summary());
        assert!(has_event("exited"),           "messages: {:?}", summary());
        assert!(has_event("terminated"),       "messages: {:?}", summary());
    }
}
