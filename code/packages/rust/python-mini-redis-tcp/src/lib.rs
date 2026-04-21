//! # python-mini-redis-tcp
//!
//! Prototype bridge between the Rust `tcp-runtime` data plane and a Python
//! Mini Redis command worker.
//!
//! This crate intentionally keeps the seam visible:
//!
//! - Rust owns TCP sockets, native event-loop integration, connection state,
//!   RESP request framing, and response writes.
//! - Python owns Redis command execution and key/value state.
//! - The process boundary uses one JSON object per line with hex-encoded byte
//!   arrays so the contract is inspectable before we optimize it.
//!
//! The current prototype calls the Python worker synchronously from the TCP
//! read callback. That proves the language boundary and the TCP-runtime
//! consumer shape, but it is not the final job-runtime design. The next layer
//! should route requests through `generic-job-runtime` so worker completions can
//! be applied asynchronously without blocking the reactor.

use std::fmt;
use std::io::{self, BufRead, BufReader, Write};
use std::net::SocketAddr;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use resp_protocol::{decode, encode, RespError, RespValue};
use serde::{Deserialize, Serialize};
use tcp_runtime::{ConnectionId, PlatformError, StopHandle, TcpHandlerResult, TcpRuntime};
use tcp_runtime::{TcpConnectionInfo, TcpRuntimeOptions};

const MAX_BUFFERED_REQUEST_BYTES: usize = 1024 * 1024;

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
type PythonRedisRuntime =
    TcpRuntime<transport_platform::bsd::KqueueTransportPlatform, RedisConnectionState>;

#[cfg(target_os = "linux")]
type PythonRedisRuntime =
    TcpRuntime<transport_platform::linux::EpollTransportPlatform, RedisConnectionState>;

#[cfg(target_os = "windows")]
type PythonRedisRuntime =
    TcpRuntime<transport_platform::windows::WindowsTransportPlatform, RedisConnectionState>;

/// Command used to start the Python Mini Redis worker process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonWorkerCommand {
    pub program: String,
    pub args: Vec<String>,
}

impl PythonWorkerCommand {
    pub fn new(program: impl Into<String>, args: impl Into<Vec<String>>) -> Self {
        Self {
            program: program.into(),
            args: args.into(),
        }
    }
}

/// TCP listener and worker options for the prototype server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonMiniRedisOptions {
    pub host: String,
    pub port: u16,
    pub max_connections: usize,
    pub worker: PythonWorkerCommand,
}

impl PythonMiniRedisOptions {
    pub fn new(worker: PythonWorkerCommand) -> Self {
        Self {
            worker,
            ..Self::default()
        }
    }
}

impl Default for PythonMiniRedisOptions {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 6379,
            max_connections: TcpRuntimeOptions::default().max_connections,
            worker: PythonWorkerCommand::new("python", Vec::<String>::new()),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct RedisConnectionState {
    read_buffer: Vec<u8>,
    sequence: u64,
}

/// A TCP server that delegates Mini Redis command execution to Python.
#[derive(Clone)]
pub struct PythonMiniRedisServer {
    runtime: Arc<Mutex<Option<PythonRedisRuntime>>>,
    local_addr: SocketAddr,
    stop_handle: StopHandle,
    serving: Arc<AtomicBool>,
}

impl PythonMiniRedisServer {
    pub fn new(options: PythonMiniRedisOptions) -> io::Result<Self> {
        let worker = Arc::new(Mutex::new(PythonWorkerProcess::spawn(&options.worker)?));
        let runtime = build_runtime(&options, worker).map_err(into_io_error)?;
        let local_addr = runtime.local_addr();
        let stop_handle = runtime.stop_handle();

        Ok(Self {
            runtime: Arc::new(Mutex::new(Some(runtime))),
            local_addr,
            stop_handle,
            serving: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn serve(&self) -> io::Result<()> {
        let mut runtime = self
            .runtime
            .lock()
            .expect("python mini redis runtime mutex poisoned")
            .take()
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "python mini redis runtime is already serving or has already served",
                )
            })?;

        self.serving.store(true, Ordering::SeqCst);
        let result = runtime.serve().map_err(into_io_error);
        self.serving.store(false, Ordering::SeqCst);
        result
    }

    pub fn stop(&self) {
        self.stop_handle.stop();
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub fn is_running(&self) -> bool {
        self.serving.load(Ordering::SeqCst)
    }
}

#[derive(Debug)]
struct PythonWorkerProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_job_id: u64,
}

impl PythonWorkerProcess {
    fn spawn(command: &PythonWorkerCommand) -> io::Result<Self> {
        let mut child = Command::new(&command.program)
            .args(&command.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;

        let stdin = child.stdin.take().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::BrokenPipe,
                "python worker did not expose stdin",
            )
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::BrokenPipe,
                "python worker did not expose stdout",
            )
        })?;

        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            next_job_id: 1,
        })
    }

    fn execute(
        &mut self,
        connection_id: ConnectionId,
        sequence: u64,
        argv: Vec<Vec<u8>>,
    ) -> Result<Vec<u8>, WorkerError> {
        let id = format!("job-{}", self.next_job_id);
        self.next_job_id += 1;
        let expected_connection_id = connection_id.0.to_string();

        let request = WorkerRequest {
            id: id.clone(),
            connection_id: expected_connection_id.clone(),
            sequence,
            argv_hex: argv.into_iter().map(hex_encode).collect(),
        };

        let encoded = serde_json::to_string(&request)?;
        self.stdin.write_all(encoded.as_bytes())?;
        self.stdin.write_all(b"\n")?;
        self.stdin.flush()?;

        let mut line = String::new();
        let read = self.stdout.read_line(&mut line)?;
        if read == 0 {
            return Err(WorkerError::Protocol(
                "python worker closed stdout before responding".to_string(),
            ));
        }

        let response: WorkerResponse = serde_json::from_str(line.trim_end())?;
        if response.id != id {
            return Err(WorkerError::Protocol(format!(
                "python worker response id mismatch: expected {}, got {}",
                id, response.id
            )));
        }
        if response.connection_id != expected_connection_id {
            return Err(WorkerError::Protocol(format!(
                "python worker connection id mismatch: expected {}, got {}",
                expected_connection_id, response.connection_id
            )));
        }
        if response.sequence != sequence {
            return Err(WorkerError::Protocol(format!(
                "python worker sequence mismatch: expected {}, got {}",
                sequence, response.sequence
            )));
        }
        if !response.ok {
            return Err(WorkerError::Protocol(
                response
                    .error
                    .unwrap_or_else(|| "python worker returned an error".to_string()),
            ));
        }
        let Some(resp_hex) = response.resp_hex else {
            return Err(WorkerError::Protocol(
                "python worker omitted resp_hex".to_string(),
            ));
        };
        hex_decode(&resp_hex).map_err(WorkerError::Protocol)
    }
}

impl Drop for PythonWorkerProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[derive(Debug, Serialize)]
struct WorkerRequest {
    id: String,
    connection_id: String,
    sequence: u64,
    argv_hex: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct WorkerResponse {
    id: String,
    connection_id: String,
    sequence: u64,
    ok: bool,
    resp_hex: Option<String>,
    error: Option<String>,
}

#[derive(Debug)]
enum WorkerError {
    Io(io::Error),
    Json(serde_json::Error),
    Protocol(String),
}

impl fmt::Display for WorkerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "worker I/O error: {error}"),
            Self::Json(error) => write!(f, "worker JSON error: {error}"),
            Self::Protocol(message) => f.write_str(message),
        }
    }
}

impl From<io::Error> for WorkerError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for WorkerError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

fn handle_connection_data(
    worker: &Arc<Mutex<PythonWorkerProcess>>,
    info: TcpConnectionInfo,
    state: &mut RedisConnectionState,
    data: &[u8],
) -> TcpHandlerResult {
    if state.read_buffer.len().saturating_add(data.len()) > MAX_BUFFERED_REQUEST_BYTES {
        state.read_buffer.clear();
        return TcpHandlerResult::write_and_close(protocol_error_response(
            "ERR protocol error: request exceeds maximum buffered size",
        ));
    }

    state.read_buffer.extend_from_slice(data);
    let mut responses = Vec::new();

    loop {
        match decode(&state.read_buffer) {
            Ok(Some((value, consumed))) => {
                state.read_buffer.drain(..consumed);
                let Some(argv) = argv_from_resp(value) else {
                    responses.extend(protocol_error_response(
                        "ERR protocol error: expected array of bulk strings",
                    ));
                    continue;
                };

                state.sequence += 1;
                let worker_response = worker
                    .lock()
                    .expect("python worker mutex poisoned")
                    .execute(info.id, state.sequence, argv);

                match worker_response {
                    Ok(bytes) => responses.extend(bytes),
                    Err(error) => responses.extend(protocol_error_response(&format!(
                        "ERR python worker failed: {error}"
                    ))),
                }
            }
            Ok(None) => break,
            Err(err) => {
                state.read_buffer.clear();
                responses.extend(protocol_error_response(&format!("ERR {err}")));
                break;
            }
        }
    }

    if responses.is_empty() {
        TcpHandlerResult::default()
    } else {
        TcpHandlerResult::write(responses)
    }
}

fn argv_from_resp(value: RespValue) -> Option<Vec<Vec<u8>>> {
    let RespValue::Array(Some(values)) = value else {
        return None;
    };
    let mut argv = Vec::with_capacity(values.len());
    for value in values {
        match value {
            RespValue::BulkString(Some(bytes)) => argv.push(bytes),
            RespValue::SimpleString(text) => argv.push(text.into_bytes()),
            _ => return None,
        }
    }
    Some(argv)
}

fn protocol_error_response(message: &str) -> Vec<u8> {
    encode(RespValue::Error(RespError::new(message))).expect("RESP error responses should encode")
}

fn runtime_options(options: &PythonMiniRedisOptions) -> TcpRuntimeOptions {
    let mut runtime_options = TcpRuntimeOptions::default();
    runtime_options.max_connections = options.max_connections.max(1);
    runtime_options
}

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
fn build_runtime(
    options: &PythonMiniRedisOptions,
    worker: Arc<Mutex<PythonWorkerProcess>>,
) -> Result<PythonRedisRuntime, PlatformError> {
    let host = options.host.clone();
    TcpRuntime::bind_kqueue_with_state(
        (host.as_str(), options.port),
        runtime_options(options),
        |_| RedisConnectionState::default(),
        move |info, state, bytes| handle_connection_data(&worker, info, state, bytes),
        |_, _| {},
    )
}

#[cfg(target_os = "linux")]
fn build_runtime(
    options: &PythonMiniRedisOptions,
    worker: Arc<Mutex<PythonWorkerProcess>>,
) -> Result<PythonRedisRuntime, PlatformError> {
    let host = options.host.clone();
    TcpRuntime::bind_epoll_with_state(
        (host.as_str(), options.port),
        runtime_options(options),
        |_| RedisConnectionState::default(),
        move |info, state, bytes| handle_connection_data(&worker, info, state, bytes),
        |_, _| {},
    )
}

#[cfg(target_os = "windows")]
fn build_runtime(
    options: &PythonMiniRedisOptions,
    worker: Arc<Mutex<PythonWorkerProcess>>,
) -> Result<PythonRedisRuntime, PlatformError> {
    let host = options.host.clone();
    TcpRuntime::bind_windows_with_state(
        (host.as_str(), options.port),
        runtime_options(options),
        |_| RedisConnectionState::default(),
        move |info, state, bytes| handle_connection_data(&worker, info, state, bytes),
        |_, _| {},
    )
}

fn into_io_error(error: PlatformError) -> io::Error {
    use io::ErrorKind;

    let kind = match error {
        PlatformError::AddressInUse => ErrorKind::AddrInUse,
        PlatformError::AddressNotAvailable => ErrorKind::AddrNotAvailable,
        PlatformError::PermissionDenied => ErrorKind::PermissionDenied,
        PlatformError::ConnectionRefused => ErrorKind::ConnectionRefused,
        PlatformError::ConnectionReset => ErrorKind::ConnectionReset,
        PlatformError::BrokenPipe => ErrorKind::BrokenPipe,
        PlatformError::TimedOut => ErrorKind::TimedOut,
        PlatformError::Interrupted => ErrorKind::Interrupted,
        PlatformError::InvalidResource | PlatformError::ResourceClosed => ErrorKind::InvalidInput,
        PlatformError::Unsupported(_) => ErrorKind::Unsupported,
        PlatformError::ProviderFault(_) | PlatformError::Io(_) => ErrorKind::Other,
    };
    io::Error::new(kind, error.to_string())
}

fn hex_encode(bytes: Vec<u8>) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn hex_decode(input: &str) -> Result<Vec<u8>, String> {
    if input.len() % 2 != 0 {
        return Err("hex input has odd length".to_string());
    }
    let mut output = Vec::with_capacity(input.len() / 2);
    let bytes = input.as_bytes();
    for pair in bytes.chunks_exact(2) {
        let high = hex_value(pair[0])?;
        let low = hex_value(pair[1])?;
        output.push((high << 4) | low);
    }
    Ok(output)
}

fn hex_value(byte: u8) -> Result<u8, String> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(format!("invalid hex byte: {byte}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::path::{Path, PathBuf};
    use std::thread;
    use std::time::Duration;

    fn bulk(value: &[u8]) -> RespValue {
        RespValue::BulkString(Some(value.to_vec()))
    }

    fn command(parts: &[&[u8]]) -> Vec<u8> {
        let values = parts.iter().map(|part| bulk(part)).collect::<Vec<_>>();
        encode(RespValue::Array(Some(values))).expect("test command should encode")
    }

    fn read_response(stream: &mut TcpStream) -> io::Result<RespValue> {
        let mut buffer = Vec::new();
        let mut chunk = [0u8; 1024];
        loop {
            match decode(&buffer) {
                Ok(Some((value, _))) => return Ok(value),
                Ok(None) => {}
                Err(error) => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("invalid RESP response: {error}"),
                    ));
                }
            }
            let n = stream.read(&mut chunk)?;
            if n == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "server closed before response",
                ));
            }
            buffer.extend_from_slice(&chunk[..n]);
        }
    }

    fn python_command() -> Option<PythonWorkerCommand> {
        let script = worker_script();
        for candidate in python_candidates() {
            if python_can_run_worker(&candidate) {
                return Some(PythonWorkerCommand::new(
                    candidate,
                    vec![script.to_string_lossy().into_owned()],
                ));
            }
        }
        None
    }

    fn python_can_run_worker(candidate: &str) -> bool {
        Command::new(candidate)
            .arg("-c")
            .arg("import sys; raise SystemExit(0 if sys.version_info >= (3, 12) else 1)")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn python_candidates() -> Vec<String> {
        let mut candidates = Vec::new();
        if let Ok(value) = std::env::var("PYTHON") {
            candidates.push(value);
        }
        if cfg!(windows) {
            candidates.push("python3.12".to_string());
            candidates.push("python".to_string());
        } else {
            candidates.push("python3.12".to_string());
            candidates.push("python3".to_string());
            candidates.push("python".to_string());
        }
        candidates.dedup();
        candidates
    }

    fn worker_script() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../../python/mini-redis-python-worker/src/mini_redis_python_worker/stdio_worker.py",
        )
    }

    #[test]
    fn hex_round_trips_binary_bytes() {
        let bytes = vec![0, 1, 2, 15, 16, 255];
        assert_eq!(hex_decode(&hex_encode(bytes.clone())).unwrap(), bytes);
    }

    #[test]
    fn argv_from_resp_accepts_resp_arrays() {
        let value = RespValue::Array(Some(vec![bulk(b"PING"), bulk(b"hello")]));
        assert_eq!(
            argv_from_resp(value),
            Some(vec![b"PING".to_vec(), b"hello".to_vec()])
        );
    }

    #[test]
    fn python_worker_process_executes_command_jobs() {
        let Some(command) = python_command() else {
            eprintln!("skipping test because no Python interpreter was found");
            return;
        };
        let mut worker = PythonWorkerProcess::spawn(&command).expect("spawn worker");
        let response = worker
            .execute(ConnectionId(7), 1, vec![b"PING".to_vec()])
            .expect("worker response");
        assert_eq!(response, b"+PONG\r\n");
    }

    #[test]
    fn tcp_runtime_delegates_redis_commands_to_python_worker() {
        let Some(worker) = python_command() else {
            eprintln!("skipping test because no Python interpreter was found");
            return;
        };
        let server = PythonMiniRedisServer::new(PythonMiniRedisOptions {
            host: "127.0.0.1".to_string(),
            port: 0,
            max_connections: 64,
            worker,
        })
        .expect("create server");
        let addr = server.local_addr();
        let background = server.clone();
        let handle = thread::spawn(move || background.serve());

        let mut stream = TcpStream::connect(addr).expect("connect client");
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .expect("set read timeout");

        stream.write_all(&command(&[b"PING"])).expect("write ping");
        assert_eq!(
            read_response(&mut stream).expect("read ping"),
            RespValue::SimpleString("PONG".to_string())
        );

        stream
            .write_all(&command(&[b"SET", b"name", b"ada"]))
            .expect("write set");
        assert_eq!(
            read_response(&mut stream).expect("read set"),
            RespValue::SimpleString("OK".to_string())
        );

        stream
            .write_all(&command(&[b"GET", b"name"]))
            .expect("write get");
        assert_eq!(
            read_response(&mut stream).expect("read get"),
            RespValue::BulkString(Some(b"ada".to_vec()))
        );

        stream
            .write_all(&command(&[b"HSET", b"user", b"name", b"grace"]))
            .expect("write hset");
        assert_eq!(
            read_response(&mut stream).expect("read hset"),
            RespValue::Integer(1)
        );

        stream
            .write_all(&command(&[b"HGET", b"user", b"name"]))
            .expect("write hget");
        assert_eq!(
            read_response(&mut stream).expect("read hget"),
            RespValue::BulkString(Some(b"grace".to_vec()))
        );

        server.stop();
        handle
            .join()
            .expect("server thread")
            .expect("server result");
    }
}
