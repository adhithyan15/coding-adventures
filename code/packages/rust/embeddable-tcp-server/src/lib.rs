//! # embeddable-tcp-server
//!
//! `embeddable-tcp-server` is the language-neutral bridge between the Rust TCP
//! runtime and an embedded application worker.
//!
//! The crate deliberately avoids naming Python, Ruby, Lua, Objective-C, or any
//! one application protocol in its public API. Rust owns TCP sockets, native
//! event-loop integration, connection lifecycle, and socket writes. The embedded
//! worker owns application semantics and responds through `generic-job-protocol`
//! frames. A Python Mini Redis worker is used only in tests as one concrete
//! consumer of the same seam other language bridges can implement.
//!
//! The current implementation is still synchronous: the TCP read callback waits
//! for a worker response before returning bytes to write. That is enough to
//! prove the portable embedding boundary. The next layer should route requests
//! through the generic job runtime so worker completion can happen
//! asynchronously without blocking the reactor.

use std::fmt;
use std::io::{self, BufRead, BufReader, Write};
use std::marker::PhantomData;
use std::net::SocketAddr;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use generic_job_protocol::{
    decode_response_json_line, encode_request_json_line, JobCodecError, JobMetadata, JobRequest,
    JobResponse, JobResult,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tcp_runtime::{PlatformError, StopHandle, TcpHandlerResult};
use tcp_runtime::{TcpConnectionInfo, TcpRuntime, TcpRuntimeOptions};

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
type PlatformTcpRuntime<S> = TcpRuntime<transport_platform::bsd::KqueueTransportPlatform, S>;

#[cfg(target_os = "linux")]
type PlatformTcpRuntime<S> = TcpRuntime<transport_platform::linux::EpollTransportPlatform, S>;

#[cfg(target_os = "windows")]
type PlatformTcpRuntime<S> = TcpRuntime<transport_platform::windows::WindowsTransportPlatform, S>;

/// Command used to start an embedded language/application worker process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerCommand {
    pub program: String,
    pub args: Vec<String>,
}

impl WorkerCommand {
    pub fn new(program: impl Into<String>, args: impl Into<Vec<String>>) -> Self {
        Self {
            program: program.into(),
            args: args.into(),
        }
    }
}

/// TCP listener and worker options for an embedded application server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddableTcpServerOptions {
    pub host: String,
    pub port: u16,
    pub max_connections: usize,
    pub worker: WorkerCommand,
}

impl EmbeddableTcpServerOptions {
    pub fn new(worker: WorkerCommand) -> Self {
        Self {
            worker,
            ..Self::default()
        }
    }
}

impl Default for EmbeddableTcpServerOptions {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 0,
            max_connections: TcpRuntimeOptions::default().max_connections,
            worker: WorkerCommand::new("worker", Vec::<String>::new()),
        }
    }
}

/// Language-neutral stdio worker that speaks `generic-job-protocol` JSON lines.
#[derive(Debug)]
pub struct StdioJobWorker<Request, Response> {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_job_id: u64,
    _request: PhantomData<Request>,
    _response: PhantomData<Response>,
}

impl<Request, Response> StdioJobWorker<Request, Response>
where
    Request: Serialize,
    Response: DeserializeOwned,
{
    pub fn spawn(command: &WorkerCommand) -> io::Result<Self> {
        let mut child = Command::new(&command.program)
            .args(&command.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;

        let stdin = child.stdin.take().ok_or_else(|| {
            io::Error::new(io::ErrorKind::BrokenPipe, "worker did not expose stdin")
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            io::Error::new(io::ErrorKind::BrokenPipe, "worker did not expose stdout")
        })?;

        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            next_job_id: 1,
            _request: PhantomData,
            _response: PhantomData,
        })
    }

    pub fn execute(&mut self, payload: Request) -> Result<Response, WorkerError> {
        self.execute_with_metadata(JobMetadata::default(), payload)
    }

    pub fn execute_with_affinity(
        &mut self,
        affinity_key: String,
        sequence: u64,
        payload: Request,
    ) -> Result<Response, WorkerError> {
        let metadata = JobMetadata::default()
            .with_affinity_key(affinity_key)
            .with_sequence(sequence);
        self.execute_with_metadata(metadata, payload)
    }

    pub fn execute_with_metadata(
        &mut self,
        metadata: JobMetadata,
        payload: Request,
    ) -> Result<Response, WorkerError> {
        let id = format!("job-{}", self.next_job_id);
        self.next_job_id += 1;

        let request = JobRequest::new(id.clone(), payload).with_metadata(metadata);

        let encoded = encode_request_json_line(&request)?;
        self.stdin.write_all(encoded.as_bytes())?;
        self.stdin.flush()?;

        let mut line = String::new();
        let read = self.stdout.read_line(&mut line)?;
        if read == 0 {
            return Err(WorkerError::Protocol(
                "worker closed stdout before responding".to_string(),
            ));
        }

        let response: JobResponse<Response> = decode_response_json_line(line.trim_end())?;
        validate_response_id(&response, &id)?;

        match response.result {
            JobResult::Ok { payload } => Ok(payload),
            JobResult::Error { error } => Err(WorkerError::Protocol(format!(
                "{}: {}",
                error.code, error.message
            ))),
            JobResult::Cancelled { cancellation } => Err(WorkerError::Protocol(format!(
                "worker cancelled job: {}",
                cancellation.message
            ))),
            JobResult::TimedOut { timeout } => Err(WorkerError::Protocol(format!(
                "worker timed out job: {}",
                timeout.message
            ))),
        }
    }
}

impl<Request, Response> Drop for StdioJobWorker<Request, Response> {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Errors returned by an embedded worker process or the protocol boundary.
#[derive(Debug)]
pub enum WorkerError {
    Io(io::Error),
    Codec(JobCodecError),
    Protocol(String),
}

impl fmt::Display for WorkerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "worker I/O error: {error}"),
            Self::Codec(error) => write!(f, "worker protocol error: {error}"),
            Self::Protocol(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for WorkerError {}

impl From<io::Error> for WorkerError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<JobCodecError> for WorkerError {
    fn from(value: JobCodecError) -> Self {
        Self::Codec(value)
    }
}

/// TCP server that embeds a generic job worker behind the Rust TCP runtime.
#[derive(Clone)]
pub struct EmbeddableTcpServer<State> {
    runtime: Arc<Mutex<Option<PlatformTcpRuntime<State>>>>,
    local_addr: SocketAddr,
    stop_handle: StopHandle,
    serving: Arc<AtomicBool>,
}

impl<State> EmbeddableTcpServer<State>
where
    State: Send + 'static,
{
    pub fn new<Request, Response, Init, Handler, Close>(
        options: EmbeddableTcpServerOptions,
        init: Init,
        handler: Handler,
        on_close: Close,
    ) -> io::Result<Self>
    where
        Request: Serialize + Send + 'static,
        Response: DeserializeOwned + Send + 'static,
        Init: Fn(TcpConnectionInfo) -> State + Send + Sync + 'static,
        Handler: Fn(
                TcpConnectionInfo,
                &mut State,
                &[u8],
                &mut StdioJobWorker<Request, Response>,
            ) -> TcpHandlerResult
            + Send
            + Sync
            + 'static,
        Close: Fn(TcpConnectionInfo, State) + Send + Sync + 'static,
    {
        let worker = Arc::new(Mutex::new(StdioJobWorker::<Request, Response>::spawn(
            &options.worker,
        )?));
        let runtime =
            build_runtime(&options, worker, init, handler, on_close).map_err(into_io_error)?;
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
            .expect("embeddable tcp server runtime mutex poisoned")
            .take()
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "embeddable tcp server runtime is already serving or has already served",
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

fn validate_response_id<Response>(
    response: &JobResponse<Response>,
    expected_id: &str,
) -> Result<(), WorkerError> {
    if response.id != expected_id {
        return Err(WorkerError::Protocol(format!(
            "worker response id mismatch: expected {}, got {}",
            expected_id, response.id
        )));
    }
    Ok(())
}

fn runtime_options(options: &EmbeddableTcpServerOptions) -> TcpRuntimeOptions {
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
fn build_runtime<State, Request, Response, Init, Handler, Close>(
    options: &EmbeddableTcpServerOptions,
    worker: Arc<Mutex<StdioJobWorker<Request, Response>>>,
    init: Init,
    handler: Handler,
    on_close: Close,
) -> Result<PlatformTcpRuntime<State>, PlatformError>
where
    State: Send + 'static,
    Request: Serialize + Send + 'static,
    Response: DeserializeOwned + Send + 'static,
    Init: Fn(TcpConnectionInfo) -> State + Send + Sync + 'static,
    Handler: Fn(
            TcpConnectionInfo,
            &mut State,
            &[u8],
            &mut StdioJobWorker<Request, Response>,
        ) -> TcpHandlerResult
        + Send
        + Sync
        + 'static,
    Close: Fn(TcpConnectionInfo, State) + Send + Sync + 'static,
{
    let host = options.host.clone();
    TcpRuntime::bind_kqueue_with_state(
        (host.as_str(), options.port),
        runtime_options(options),
        init,
        move |info, state, bytes| {
            let mut worker = worker.lock().expect("embedded worker mutex poisoned");
            handler(info, state, bytes, &mut worker)
        },
        on_close,
    )
}

#[cfg(target_os = "linux")]
fn build_runtime<State, Request, Response, Init, Handler, Close>(
    options: &EmbeddableTcpServerOptions,
    worker: Arc<Mutex<StdioJobWorker<Request, Response>>>,
    init: Init,
    handler: Handler,
    on_close: Close,
) -> Result<PlatformTcpRuntime<State>, PlatformError>
where
    State: Send + 'static,
    Request: Serialize + Send + 'static,
    Response: DeserializeOwned + Send + 'static,
    Init: Fn(TcpConnectionInfo) -> State + Send + Sync + 'static,
    Handler: Fn(
            TcpConnectionInfo,
            &mut State,
            &[u8],
            &mut StdioJobWorker<Request, Response>,
        ) -> TcpHandlerResult
        + Send
        + Sync
        + 'static,
    Close: Fn(TcpConnectionInfo, State) + Send + Sync + 'static,
{
    let host = options.host.clone();
    TcpRuntime::bind_epoll_with_state(
        (host.as_str(), options.port),
        runtime_options(options),
        init,
        move |info, state, bytes| {
            let mut worker = worker.lock().expect("embedded worker mutex poisoned");
            handler(info, state, bytes, &mut worker)
        },
        on_close,
    )
}

#[cfg(target_os = "windows")]
fn build_runtime<State, Request, Response, Init, Handler, Close>(
    options: &EmbeddableTcpServerOptions,
    worker: Arc<Mutex<StdioJobWorker<Request, Response>>>,
    init: Init,
    handler: Handler,
    on_close: Close,
) -> Result<PlatformTcpRuntime<State>, PlatformError>
where
    State: Send + 'static,
    Request: Serialize + Send + 'static,
    Response: DeserializeOwned + Send + 'static,
    Init: Fn(TcpConnectionInfo) -> State + Send + Sync + 'static,
    Handler: Fn(
            TcpConnectionInfo,
            &mut State,
            &[u8],
            &mut StdioJobWorker<Request, Response>,
        ) -> TcpHandlerResult
        + Send
        + Sync
        + 'static,
    Close: Fn(TcpConnectionInfo, State) + Send + Sync + 'static,
{
    let host = options.host.clone();
    TcpRuntime::bind_windows_with_state(
        (host.as_str(), options.port),
        runtime_options(options),
        init,
        move |info, state, bytes| {
            let mut worker = worker.lock().expect("embedded worker mutex poisoned");
            handler(info, state, bytes, &mut worker)
        },
        on_close,
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

#[cfg(test)]
mod tests {
    use super::*;
    use resp_protocol::{decode, encode, RespError, RespValue};
    use serde::{Deserialize, Serialize};
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::path::{Path, PathBuf};
    use std::thread;
    use std::time::Duration;

    const MAX_BUFFERED_REQUEST_BYTES: usize = 1024 * 1024;

    #[derive(Debug, Default, Clone, PartialEq, Eq)]
    struct RedisConnectionState {
        read_buffer: Vec<u8>,
        selected_db: usize,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct RedisCommandJob {
        selected_db: usize,
        command: String,
        args_hex: Vec<String>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct RedisResponseJob {
        selected_db: usize,
        response: RedisEngineResponseJob,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(tag = "kind", rename_all = "snake_case")]
    enum RedisEngineResponseJob {
        SimpleString { value: String },
        Error { message: String },
        Integer { value: i64 },
        BulkString { value_hex: Option<String> },
        Array { values: Option<Vec<RedisEngineResponseJob>> },
    }

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

    fn worker_command() -> Option<WorkerCommand> {
        let script = worker_script();
        for candidate in python_candidates() {
            if python_can_run_worker(&candidate) {
                return Some(WorkerCommand::new(
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

    fn python_interpreter() -> Option<String> {
        python_candidates()
            .into_iter()
            .find(|candidate| python_can_run_worker(candidate))
    }

    fn scripted_worker(script: &str) -> Option<WorkerCommand> {
        python_interpreter()
            .map(|program| WorkerCommand::new(program, vec!["-c".to_string(), script.to_string()]))
    }

    fn execute_scripted_worker(script: &str) -> Result<RedisResponseJob, WorkerError> {
        let command = scripted_worker(script).expect("python interpreter");
        let mut worker = StdioJobWorker::<RedisCommandJob, RedisResponseJob>::spawn(&command)
            .expect("spawn scripted worker");
        worker.execute(
            RedisCommandJob {
                selected_db: 0,
                command: "PING".to_string(),
                args_hex: Vec::new(),
            },
        )
    }

    fn handle_redis_connection_data(
        _info: TcpConnectionInfo,
        state: &mut RedisConnectionState,
        data: &[u8],
        worker: &mut StdioJobWorker<RedisCommandJob, RedisResponseJob>,
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
                    let Some(job) = command_job_from_resp(value, state.selected_db) else {
                        responses.extend(protocol_error_response(
                            "ERR protocol error: expected array command frame",
                        ));
                        continue;
                    };

                    let worker_response = worker.execute(job);

                    match worker_response {
                        Ok(payload) => {
                            state.selected_db = payload.selected_db;
                            match engine_response_to_resp(payload.response) {
                                Ok(value) => responses.extend(
                                    encode(value).expect("engine responses should encode"),
                                ),
                                Err(error) => responses.extend(protocol_error_response(&format!(
                                    "ERR worker returned invalid response payload: {error}"
                                ))),
                            }
                        },
                        Err(error) => responses.extend(protocol_error_response(&format!(
                            "ERR worker failed: {error}"
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

    fn command_job_from_resp(value: RespValue, selected_db: usize) -> Option<RedisCommandJob> {
        let RespValue::Array(Some(values)) = value else {
            return None;
        };
        let mut parts = Vec::with_capacity(values.len());
        for value in values {
            match value {
                RespValue::BulkString(Some(bytes)) => parts.push(bytes),
                RespValue::SimpleString(text) => parts.push(text.into_bytes()),
                RespValue::Integer(value) => parts.push(value.to_string().into_bytes()),
                _ => return None,
            }
        }
        let (command, args) = parts.split_first()?;
        Some(RedisCommandJob {
            selected_db,
            command: ascii_upper(command),
            args_hex: args.iter().cloned().map(hex_encode).collect(),
        })
    }

    fn engine_response_to_resp(response: RedisEngineResponseJob) -> Result<RespValue, String> {
        match response {
            RedisEngineResponseJob::SimpleString { value } => Ok(RespValue::SimpleString(value)),
            RedisEngineResponseJob::Error { message } => {
                Ok(RespValue::Error(RespError::new(message)))
            }
            RedisEngineResponseJob::Integer { value } => Ok(RespValue::Integer(value)),
            RedisEngineResponseJob::BulkString { value_hex } => {
                let value = value_hex.map(|encoded| hex_decode(&encoded)).transpose()?;
                Ok(RespValue::BulkString(value))
            }
            RedisEngineResponseJob::Array { values } => {
                let values = values
                    .map(|items| {
                        items
                            .into_iter()
                            .map(engine_response_to_resp)
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .transpose()?;
                Ok(RespValue::Array(values))
            }
        }
    }

    fn ascii_upper(bytes: &[u8]) -> String {
        bytes
            .iter()
            .map(|byte| byte.to_ascii_uppercase() as char)
            .collect()
    }

    fn protocol_error_response(message: &str) -> Vec<u8> {
        encode(RespValue::Error(RespError::new(message)))
            .expect("RESP error responses should encode")
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

    #[test]
    fn hex_round_trips_binary_bytes() {
        let bytes = vec![0, 1, 2, 15, 16, 255];
        assert_eq!(hex_decode(&hex_encode(bytes.clone())).unwrap(), bytes);
    }

    #[test]
    fn hex_decode_rejects_malformed_input() {
        assert_eq!(
            hex_decode("abc"),
            Err("hex input has odd length".to_string())
        );
        assert_eq!(hex_decode("zz"), Err("invalid hex byte: 122".to_string()));
    }

    #[test]
    fn options_default_and_runtime_options_are_stable() {
        let worker = WorkerCommand::new("python3.12", vec!["worker.py".to_string()]);
        let options = EmbeddableTcpServerOptions::new(worker.clone());
        assert_eq!(options.host, "127.0.0.1");
        assert_eq!(options.port, 0);
        assert_eq!(options.worker, worker);

        let mut zero_connections = options.clone();
        zero_connections.max_connections = 0;
        assert_eq!(runtime_options(&zero_connections).max_connections, 1);
    }

    #[test]
    fn command_job_from_resp_accepts_resp_arrays() {
        let value = RespValue::Array(Some(vec![bulk(b"PING"), bulk(b"hello")]));
        assert_eq!(
            command_job_from_resp(value, 3),
            Some(RedisCommandJob {
                selected_db: 3,
                command: "PING".to_string(),
                args_hex: vec![hex_encode(b"hello".to_vec())],
            })
        );
    }

    #[test]
    fn command_job_from_resp_rejects_non_array_commands() {
        assert_eq!(command_job_from_resp(RespValue::Integer(1), 0), None);
        assert_eq!(command_job_from_resp(RespValue::Array(None), 0), None);
        assert_eq!(
            command_job_from_resp(RespValue::Array(Some(vec![RespValue::Integer(1)])), 0),
            Some(RedisCommandJob {
                selected_db: 0,
                command: "1".to_string(),
                args_hex: Vec::new(),
            })
        );
    }

    #[test]
    fn platform_errors_map_to_io_kinds() {
        assert_eq!(
            into_io_error(PlatformError::AddressInUse).kind(),
            io::ErrorKind::AddrInUse
        );
        assert_eq!(
            into_io_error(PlatformError::InvalidResource).kind(),
            io::ErrorKind::InvalidInput
        );
        assert_eq!(
            into_io_error(PlatformError::Unsupported("nope")).kind(),
            io::ErrorKind::Unsupported
        );
        assert_eq!(
            into_io_error(PlatformError::ProviderFault("boom".to_string())).kind(),
            io::ErrorKind::Other
        );
    }

    #[test]
    fn worker_error_display_includes_error_origin() {
        let io_error = WorkerError::from(io::Error::new(io::ErrorKind::BrokenPipe, "closed"));
        assert_eq!(io_error.to_string(), "worker I/O error: closed");

        let codec_error =
            generic_job_protocol::decode_request_json_line::<RedisCommandJob>("not-json")
                .expect_err("invalid JSON should fail");
        let worker_error = WorkerError::from(codec_error);
        assert!(worker_error
            .to_string()
            .starts_with("worker protocol error: job frame JSON error:"));
    }

    #[test]
    fn stdio_worker_executes_generic_command_jobs() {
        let Some(command) = worker_command() else {
            eprintln!("skipping test because no Python interpreter was found");
            return;
        };
        let mut worker = StdioJobWorker::<RedisCommandJob, RedisResponseJob>::spawn(&command)
            .expect("spawn worker");
        let response = worker
            .execute(
                RedisCommandJob {
                    selected_db: 0,
                    command: "PING".to_string(),
                    args_hex: Vec::new(),
                },
            )
            .expect("worker response");
        assert_eq!(response.selected_db, 0);
        assert_eq!(
            response.response,
            RedisEngineResponseJob::SimpleString {
                value: "PONG".to_string()
            }
        );
    }

    #[test]
    fn stdio_worker_rejects_mismatched_response_ids() {
        let script = r#"
import json, sys
frame = json.loads(sys.stdin.readline())
body = frame["body"]
print(json.dumps({"version":1,"kind":"response","body":{"id":body["id"] + "-wrong","result":{"status":"ok","payload":{"selected_db":0,"response":{"kind":"simple_string","value":"PONG"}}},"metadata":body["metadata"]}}), flush=True)
"#;

        let error = execute_scripted_worker(script).expect_err("mismatched response should fail");
        assert!(error.to_string().contains("response id mismatch"));
    }

    #[test]
    fn stdio_worker_maps_portable_error_results() {
        let script = r#"
import json, sys
frame = json.loads(sys.stdin.readline())
body = frame["body"]
print(json.dumps({"version":1,"kind":"response","body":{"id":body["id"],"result":{"status":"error","error":{"code":"worker_failed","message":"boom","retryable":False,"origin":"worker","detail":None}},"metadata":body["metadata"]}}), flush=True)
"#;

        let error = execute_scripted_worker(script).expect_err("worker error should fail");
        assert!(error.to_string().contains("worker_failed: boom"));
    }

    #[test]
    fn stdio_worker_maps_cancelled_and_timed_out_results() {
        let cancelled = r#"
import json, sys
frame = json.loads(sys.stdin.readline())
body = frame["body"]
print(json.dumps({"version":1,"kind":"response","body":{"id":body["id"],"result":{"status":"cancelled","cancellation":{"message":"stopped"}},"metadata":body["metadata"]}}), flush=True)
"#;
        let error = execute_scripted_worker(cancelled).expect_err("cancelled job should fail");
        assert!(error.to_string().contains("cancelled job: stopped"));

        let timed_out = r#"
import json, sys
frame = json.loads(sys.stdin.readline())
body = frame["body"]
print(json.dumps({"version":1,"kind":"response","body":{"id":body["id"],"result":{"status":"timed_out","timeout":{"message":"too slow"}},"metadata":body["metadata"]}}), flush=True)
"#;
        let error = execute_scripted_worker(timed_out).expect_err("timed out job should fail");
        assert!(error.to_string().contains("timed out job: too slow"));
    }

    #[test]
    fn embeddable_tcp_server_delegates_redis_commands_to_python_worker() {
        let Some(worker) = worker_command() else {
            eprintln!("skipping test because no Python interpreter was found");
            return;
        };
        let server = EmbeddableTcpServer::new(
            EmbeddableTcpServerOptions {
                host: "127.0.0.1".to_string(),
                port: 0,
                max_connections: 64,
                worker,
            },
            |_| RedisConnectionState::default(),
            handle_redis_connection_data,
            |_, _| {},
        )
        .expect("create server");
        let addr = server.local_addr();
        assert!(!server.is_running());
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
