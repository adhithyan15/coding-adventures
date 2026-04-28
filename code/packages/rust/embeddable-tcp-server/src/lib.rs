//! # embeddable-tcp-server
//!
//! `embeddable-tcp-server` is the language-neutral bridge between the Rust TCP
//! runtime and an embedded application worker.
//!
//! The crate deliberately avoids naming Python, Ruby, Lua, Objective-C, or any
//! one application protocol in its public API. Rust owns TCP sockets, native
//! event-loop integration, connection lifecycle, and socket writes. The embedded
//! worker owns application protocol framing, application semantics, and
//! responses through `generic-job-protocol` frames. A Python Mini Redis worker
//! is used only in tests as one concrete consumer of the same raw-byte seam
//! other language bridges can implement.
//!
//! Mailbox mode routes requests through `generic-job-runtime` so TCP callbacks
//! can submit jobs and return immediately. Worker responses are delivered later
//! through the TCP mailbox owned by the Rust runtime.

use std::collections::BTreeMap;
use std::fmt;
use std::io::{self, BufRead, BufReader, Write};
use std::marker::PhantomData;
use std::net::SocketAddr;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use generic_job_protocol::{
    decode_response_json_line, encode_request_json_line, JobCodecError, JobMetadata, JobRequest,
    JobResponse, JobResult,
};
use generic_job_runtime::{
    ExecutorLimits, RustThreadPool, RustThreadPoolOptions, StdioProcessPool,
    StdioProcessPoolOptions, StdioWorkerCommand, StdioWorkerRestartPolicy, SubmitError,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tcp_runtime::{
    ConnectionId, PlatformError, ShardedStopHandle, ShardedTcpRuntime, StopHandle,
    TcpHandlerResult, TcpMailbox,
};
use tcp_runtime::{TcpConnectionInfo, TcpRuntime, TcpRuntimeOptions};

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
type PlatformTcpRuntime<S> = TcpRuntime<transport_platform::bsd::KqueueTransportPlatform, S>;

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
type PlatformShardedTcpRuntime<S> =
    ShardedTcpRuntime<transport_platform::bsd::KqueueTransportPlatform, S>;

#[cfg(target_os = "linux")]
type PlatformTcpRuntime<S> = TcpRuntime<transport_platform::linux::EpollTransportPlatform, S>;

#[cfg(target_os = "linux")]
type PlatformShardedTcpRuntime<S> =
    ShardedTcpRuntime<transport_platform::linux::EpollTransportPlatform, S>;

#[cfg(target_os = "windows")]
type PlatformTcpRuntime<S> = TcpRuntime<transport_platform::windows::WindowsTransportPlatform, S>;

#[cfg(target_os = "windows")]
type PlatformShardedTcpRuntime<S> =
    ShardedTcpRuntime<transport_platform::windows::WindowsTransportPlatform, S>;

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
    pub event_loop_threads: usize,
    pub worker_processes: usize,
    pub worker_queue_depth: usize,
    pub worker_job_timeout: Option<Duration>,
    pub worker_restart_policy: StdioWorkerRestartPolicy,
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
            event_loop_threads: 1,
            worker_processes: 1,
            worker_queue_depth: ExecutorLimits::default().max_queue_depth,
            worker_job_timeout: None,
            worker_restart_policy: StdioWorkerRestartPolicy::default(),
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

pub struct StdioJobSubmitter<Request, Response> {
    pool: StdioProcessPool<Request, Response>,
    routes: Arc<Mutex<BTreeMap<String, ConnectionId>>>,
    next_job_id: Arc<AtomicU64>,
    _types: PhantomData<fn() -> (Request, Response)>,
}

impl<Request, Response> Clone for StdioJobSubmitter<Request, Response> {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            routes: Arc::clone(&self.routes),
            next_job_id: Arc::clone(&self.next_job_id),
            _types: PhantomData,
        }
    }
}

impl<Request, Response> StdioJobSubmitter<Request, Response>
where
    Request: Serialize + Send + 'static,
    Response: DeserializeOwned + Send + 'static,
{
    pub fn submit(
        &self,
        connection_id: ConnectionId,
        payload: Request,
    ) -> Result<String, WorkerError> {
        self.submit_with_metadata(connection_id, JobMetadata::default(), payload)
    }

    pub fn submit_with_metadata(
        &self,
        connection_id: ConnectionId,
        metadata: JobMetadata,
        payload: Request,
    ) -> Result<String, WorkerError> {
        let id = format!("job-{}", self.next_job_id.fetch_add(1, Ordering::SeqCst));
        let request = JobRequest::new(id.clone(), payload)
            .with_metadata(metadata.with_affinity_key(connection_id.0.to_string()));

        self.routes
            .lock()
            .expect("stdio worker route table mutex poisoned")
            .insert(id.clone(), connection_id);

        if let Err(error) = self.pool.submit(request) {
            self.routes
                .lock()
                .expect("stdio worker route table mutex poisoned")
                .remove(&id);
            return Err(error.into());
        }

        Ok(id)
    }
}

trait MailboxJobSubmitter<Request, Response> {
    fn submit(&self, connection_id: ConnectionId, payload: Request) -> Result<String, WorkerError>;
}

impl<Request, Response> MailboxJobSubmitter<Request, Response>
    for StdioJobSubmitter<Request, Response>
where
    Request: Serialize + Send + 'static,
    Response: DeserializeOwned + Send + 'static,
{
    fn submit(&self, connection_id: ConnectionId, payload: Request) -> Result<String, WorkerError> {
        self.submit(connection_id, payload)
    }
}

/// In-process job submitter backed by the Rust thread-pool runtime.
pub struct RustThreadPoolJobSubmitter<Request, Response> {
    pool: RustThreadPool<Request, Response>,
    routes: Arc<Mutex<BTreeMap<String, ConnectionId>>>,
    next_job_id: Arc<AtomicU64>,
    _types: PhantomData<fn() -> (Request, Response)>,
}

impl<Request, Response> Clone for RustThreadPoolJobSubmitter<Request, Response> {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            routes: Arc::clone(&self.routes),
            next_job_id: Arc::clone(&self.next_job_id),
            _types: PhantomData,
        }
    }
}

impl<Request, Response> RustThreadPoolJobSubmitter<Request, Response>
where
    Request: Send + 'static,
    Response: Send + 'static,
{
    pub fn submit(
        &self,
        connection_id: ConnectionId,
        payload: Request,
    ) -> Result<String, WorkerError> {
        self.submit_with_metadata(connection_id, JobMetadata::default(), payload)
    }

    pub fn submit_with_metadata(
        &self,
        connection_id: ConnectionId,
        metadata: JobMetadata,
        payload: Request,
    ) -> Result<String, WorkerError> {
        let id = format!("job-{}", self.next_job_id.fetch_add(1, Ordering::SeqCst));
        let request = JobRequest::new(id.clone(), payload)
            .with_metadata(metadata.with_affinity_key(connection_id.0.to_string()));

        self.routes
            .lock()
            .expect("thread pool worker route table mutex poisoned")
            .insert(id.clone(), connection_id);

        if let Err(error) = self.pool.submit(request) {
            self.routes
                .lock()
                .expect("thread pool worker route table mutex poisoned")
                .remove(&id);
            return Err(error.into());
        }

        Ok(id)
    }
}

impl<Request, Response> MailboxJobSubmitter<Request, Response>
    for RustThreadPoolJobSubmitter<Request, Response>
where
    Request: Send + 'static,
    Response: Send + 'static,
{
    fn submit(&self, connection_id: ConnectionId, payload: Request) -> Result<String, WorkerError> {
        self.submit(connection_id, payload)
    }
}

pub struct TcpMailboxFrame {
    pub writes: Vec<Vec<u8>>,
    pub close: bool,
}

impl TcpMailboxFrame {
    pub fn write(bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            writes: vec![bytes.into()],
            close: false,
        }
    }

    pub fn write_many(writes: impl Into<Vec<Vec<u8>>>) -> Self {
        Self {
            writes: writes.into(),
            close: false,
        }
    }

    pub fn close() -> Self {
        Self {
            writes: Vec::new(),
            close: true,
        }
    }

    pub fn write_and_close(bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            writes: vec![bytes.into()],
            close: true,
        }
    }

    fn flatten_writes(self) -> Vec<u8> {
        let total = self.writes.iter().map(Vec::len).sum();
        let mut output = Vec::with_capacity(total);
        for write in self.writes {
            output.extend(write);
        }
        output
    }
}

struct WorkerPoolGuard {
    stop: Arc<AtomicBool>,
    response_thread: Option<thread::JoinHandle<()>>,
}

impl Drop for WorkerPoolGuard {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.response_thread.take() {
            let _ = thread.join();
        }
    }
}

enum RuntimeInstance<State> {
    Single(PlatformTcpRuntime<State>),
    Sharded(PlatformShardedTcpRuntime<State>),
}

impl<State> RuntimeInstance<State>
where
    State: Send + 'static,
{
    fn local_addr(&self) -> SocketAddr {
        match self {
            Self::Single(runtime) => runtime.local_addr(),
            Self::Sharded(runtime) => runtime.local_addr(),
        }
    }

    fn mailbox(&self) -> TcpMailbox {
        match self {
            Self::Single(runtime) => runtime.mailbox(),
            Self::Sharded(runtime) => runtime.mailbox(),
        }
    }

    fn stop_handle(&self) -> RuntimeStopHandle {
        match self {
            Self::Single(runtime) => RuntimeStopHandle::Single(runtime.stop_handle()),
            Self::Sharded(runtime) => RuntimeStopHandle::Sharded(runtime.stop_handle()),
        }
    }

    fn serve(&mut self) -> Result<(), PlatformError> {
        match self {
            Self::Single(runtime) => runtime.serve(),
            Self::Sharded(runtime) => runtime.serve(),
        }
    }
}

#[derive(Clone)]
enum RuntimeStopHandle {
    Single(StopHandle),
    Sharded(ShardedStopHandle),
}

impl RuntimeStopHandle {
    fn stop(&self) {
        match self {
            Self::Single(stop) => stop.stop(),
            Self::Sharded(stop) => stop.stop(),
        }
    }
}

impl<Request, Response> StdioJobWorker<Request, Response>
where
    Request: Serialize,
    Response: DeserializeOwned,
{
    pub fn spawn(command: &WorkerCommand) -> io::Result<Self> {
        let stderr = if cfg!(target_os = "windows") {
            Stdio::null()
        } else {
            Stdio::inherit()
        };
        let mut child = Command::new(&command.program)
            .args(&command.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(stderr)
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
    Submit(SubmitError),
    Protocol(String),
}

impl fmt::Display for WorkerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "worker I/O error: {error}"),
            Self::Codec(error) => write!(f, "worker protocol error: {error}"),
            Self::Submit(error) => write!(f, "worker submit error: {error}"),
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

impl From<SubmitError> for WorkerError {
    fn from(value: SubmitError) -> Self {
        Self::Submit(value)
    }
}

/// TCP server that embeds a generic job worker behind the Rust TCP runtime.
#[derive(Clone)]
pub struct EmbeddableTcpServer<State> {
    runtime: Arc<Mutex<Option<RuntimeInstance<State>>>>,
    _worker_pool: Arc<Mutex<Option<WorkerPoolGuard>>>,
    local_addr: SocketAddr,
    stop_handle: RuntimeStopHandle,
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
            _worker_pool: Arc::new(Mutex::new(None)),
            local_addr,
            stop_handle,
            serving: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn new_mailbox<Request, Response, Init, Handler, Close, MapResponse>(
        options: EmbeddableTcpServerOptions,
        init: Init,
        handler: Handler,
        on_close: Close,
        map_response: MapResponse,
    ) -> io::Result<Self>
    where
        Request: Serialize + Send + 'static,
        Response: DeserializeOwned + Send + 'static,
        Init: Fn(TcpConnectionInfo) -> State + Send + Sync + 'static,
        Handler: Fn(
                TcpConnectionInfo,
                &mut State,
                &[u8],
                &StdioJobSubmitter<Request, Response>,
            ) -> TcpHandlerResult
            + Send
            + Sync
            + 'static,
        Close: Fn(TcpConnectionInfo, State) + Send + Sync + 'static,
        MapResponse: Fn(Response) -> Result<TcpMailboxFrame, WorkerError> + Send + Sync + 'static,
    {
        let pool = StdioProcessPool::<Request, Response>::spawn(
            StdioWorkerCommand::new(options.worker.program.clone(), options.worker.args.clone()),
            StdioProcessPoolOptions {
                worker_count: options.worker_processes.max(1),
                limits: ExecutorLimits {
                    max_queue_depth: options.worker_queue_depth.max(1),
                    ..ExecutorLimits::default()
                },
                default_job_timeout: options.worker_job_timeout,
                restart_policy: options.worker_restart_policy.clone(),
            },
        )
        .map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("spawn stdio worker process pool: {error}"),
            )
        })?;
        let routes = Arc::new(Mutex::new(BTreeMap::new()));
        let submitter = StdioJobSubmitter {
            pool: pool.clone(),
            routes: Arc::clone(&routes),
            next_job_id: Arc::new(AtomicU64::new(1)),
            _types: PhantomData,
        };
        let runtime = build_runtime_mailbox(&options, submitter, init, handler, on_close).map_err(
            |error| {
                let io_error = into_io_error(error.clone());
                io::Error::new(
                    io_error.kind(),
                    format!("bind mailbox TCP runtime: {error}"),
                )
            },
        )?;
        let local_addr = runtime.local_addr();
        let stop_handle = runtime.stop_handle();
        let mailbox = runtime.mailbox();
        let response_stop = Arc::new(AtomicBool::new(false));
        let response_thread = spawn_response_router(
            pool.clone(),
            routes,
            mailbox,
            Arc::new(map_response),
            Arc::clone(&response_stop),
        );

        Ok(Self {
            runtime: Arc::new(Mutex::new(Some(runtime))),
            _worker_pool: Arc::new(Mutex::new(Some(WorkerPoolGuard {
                stop: response_stop,
                response_thread: Some(response_thread),
            }))),
            local_addr,
            stop_handle,
            serving: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn new_inprocess_mailbox<Request, Response, Init, Handler, Close, MapResponse, WorkerFn>(
        options: EmbeddableTcpServerOptions,
        init: Init,
        handler: Handler,
        on_close: Close,
        map_response: MapResponse,
        worker_fn: WorkerFn,
    ) -> io::Result<Self>
    where
        Request: Send + 'static,
        Response: Send + 'static,
        Init: Fn(TcpConnectionInfo) -> State + Send + Sync + 'static,
        Handler: Fn(
                TcpConnectionInfo,
                &mut State,
                &[u8],
                &RustThreadPoolJobSubmitter<Request, Response>,
            ) -> TcpHandlerResult
            + Send
            + Sync
            + 'static,
        Close: Fn(TcpConnectionInfo, State) + Send + Sync + 'static,
        MapResponse: Fn(Response) -> Result<TcpMailboxFrame, WorkerError> + Send + Sync + 'static,
        WorkerFn: Fn(
                generic_job_protocol::JobRequest<Request>,
            ) -> generic_job_protocol::JobResult<Response>
            + Send
            + Sync
            + 'static,
    {
        let pool = RustThreadPool::spawn(
            RustThreadPoolOptions {
                worker_count: options.worker_processes.max(1),
                limits: ExecutorLimits {
                    max_queue_depth: options.worker_queue_depth.max(1),
                    ..ExecutorLimits::default()
                },
                default_job_timeout: options.worker_job_timeout,
            },
            worker_fn,
        );
        let routes = Arc::new(Mutex::new(BTreeMap::new()));
        let submitter = RustThreadPoolJobSubmitter {
            pool: pool.clone(),
            routes: Arc::clone(&routes),
            next_job_id: Arc::new(AtomicU64::new(1)),
            _types: PhantomData,
        };
        let runtime = build_runtime_mailbox(&options, submitter, init, handler, on_close)
            .map_err(into_io_error)?;
        let local_addr = runtime.local_addr();
        let stop_handle = runtime.stop_handle();
        let mailbox = runtime.mailbox();
        let response_stop = Arc::new(AtomicBool::new(false));
        let response_thread = spawn_rust_thread_pool_response_router(
            pool,
            routes,
            mailbox,
            Arc::new(map_response),
            Arc::clone(&response_stop),
        );

        Ok(Self {
            runtime: Arc::new(Mutex::new(Some(runtime))),
            _worker_pool: Arc::new(Mutex::new(Some(WorkerPoolGuard {
                stop: response_stop,
                response_thread: Some(response_thread),
            }))),
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

fn spawn_response_router<Request, Response, MapResponse>(
    pool: StdioProcessPool<Request, Response>,
    routes: Arc<Mutex<BTreeMap<String, ConnectionId>>>,
    mailbox: TcpMailbox,
    map_response: Arc<MapResponse>,
    stop: Arc<AtomicBool>,
) -> thread::JoinHandle<()>
where
    Request: Serialize + Send + 'static,
    Response: DeserializeOwned + Send + 'static,
    MapResponse: Fn(Response) -> Result<TcpMailboxFrame, WorkerError> + Send + Sync + 'static,
{
    thread::spawn(move || loop {
        if stop.load(Ordering::SeqCst) {
            break;
        }
        let Some(response) = pool
            .recv_response_timeout(Duration::from_millis(10))
            .unwrap_or(None)
        else {
            continue;
        };
        mailbox.resume_all_reads();
        let Some(connection_id) = routes
            .lock()
            .expect("stdio worker route table mutex poisoned")
            .remove(&response.id)
        else {
            continue;
        };

        match response.result {
            JobResult::Ok { payload } => match map_response(payload) {
                Ok(frame) => {
                    let close = frame.close;
                    let bytes = frame.flatten_writes();
                    if close {
                        mailbox.send_and_close(connection_id, bytes);
                    } else if !bytes.is_empty() {
                        mailbox.send(connection_id, bytes);
                    }
                }
                Err(_) => mailbox.close(connection_id),
            },
            JobResult::Error { .. } | JobResult::Cancelled { .. } | JobResult::TimedOut { .. } => {
                mailbox.close(connection_id)
            }
        }
    })
}

fn spawn_rust_thread_pool_response_router<Request, Response, MapResponse>(
    pool: RustThreadPool<Request, Response>,
    routes: Arc<Mutex<BTreeMap<String, ConnectionId>>>,
    mailbox: TcpMailbox,
    map_response: Arc<MapResponse>,
    stop: Arc<AtomicBool>,
) -> thread::JoinHandle<()>
where
    Request: Send + 'static,
    Response: Send + 'static,
    MapResponse: Fn(Response) -> Result<TcpMailboxFrame, WorkerError> + Send + Sync + 'static,
{
    thread::spawn(move || loop {
        if stop.load(Ordering::SeqCst) {
            break;
        }
        let Some(response) = pool
            .recv_response_timeout(Duration::from_millis(10))
            .unwrap_or(None)
        else {
            continue;
        };
        mailbox.resume_all_reads();
        let Some(connection_id) = routes
            .lock()
            .expect("thread pool worker route table mutex poisoned")
            .remove(&response.id)
        else {
            continue;
        };

        match response.result {
            JobResult::Ok { payload } => match map_response(payload) {
                Ok(frame) => {
                    let close = frame.close;
                    let bytes = frame.flatten_writes();
                    if close {
                        mailbox.send_and_close(connection_id, bytes);
                    } else if !bytes.is_empty() {
                        mailbox.send(connection_id, bytes);
                    }
                }
                Err(_) => mailbox.close(connection_id),
            },
            JobResult::Error { .. } | JobResult::Cancelled { .. } | JobResult::TimedOut { .. } => {
                mailbox.close(connection_id)
            }
        }
    })
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
) -> Result<RuntimeInstance<State>, PlatformError>
where
    State: Send + 'static,
    Request: Send + 'static,
    Response: Send + 'static,
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
    let runtime_options = runtime_options(options);
    let worker_count = options.event_loop_threads.max(1);
    if worker_count > 1 {
        TcpRuntime::bind_kqueue_sharded_with_state(
            (host.as_str(), options.port),
            runtime_options,
            worker_count,
            init,
            move |info, state, bytes| {
                let mut worker = worker.lock().expect("embedded worker mutex poisoned");
                handler(info, state, bytes, &mut worker)
            },
            on_close,
        )
        .map(RuntimeInstance::Sharded)
    } else {
        TcpRuntime::bind_kqueue_with_state(
            (host.as_str(), options.port),
            runtime_options,
            init,
            move |info, state, bytes| {
                let mut worker = worker.lock().expect("embedded worker mutex poisoned");
                handler(info, state, bytes, &mut worker)
            },
            on_close,
        )
        .map(RuntimeInstance::Single)
    }
}

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
fn build_runtime_mailbox<State, Request, Response, Init, Handler, Close, Submitter>(
    options: &EmbeddableTcpServerOptions,
    submitter: Submitter,
    init: Init,
    handler: Handler,
    on_close: Close,
) -> Result<RuntimeInstance<State>, PlatformError>
where
    State: Send + 'static,
    Request: Send + 'static,
    Response: Send + 'static,
    Init: Fn(TcpConnectionInfo) -> State + Send + Sync + 'static,
    Handler: Fn(TcpConnectionInfo, &mut State, &[u8], &Submitter) -> TcpHandlerResult
        + Send
        + Sync
        + 'static,
    Close: Fn(TcpConnectionInfo, State) + Send + Sync + 'static,
    Submitter: MailboxJobSubmitter<Request, Response> + Send + Sync + 'static,
{
    let host = options.host.clone();
    let runtime_options = runtime_options(options);
    let worker_count = options.event_loop_threads.max(1);
    if worker_count > 1 {
        TcpRuntime::bind_kqueue_sharded_with_state(
            (host.as_str(), options.port),
            runtime_options,
            worker_count,
            init,
            move |info, state, bytes| handler(info, state, bytes, &submitter),
            on_close,
        )
        .map(RuntimeInstance::Sharded)
    } else {
        TcpRuntime::bind_kqueue_with_state(
            (host.as_str(), options.port),
            runtime_options,
            init,
            move |info, state, bytes| handler(info, state, bytes, &submitter),
            on_close,
        )
        .map(RuntimeInstance::Single)
    }
}

#[cfg(target_os = "linux")]
fn build_runtime<State, Request, Response, Init, Handler, Close>(
    options: &EmbeddableTcpServerOptions,
    worker: Arc<Mutex<StdioJobWorker<Request, Response>>>,
    init: Init,
    handler: Handler,
    on_close: Close,
) -> Result<RuntimeInstance<State>, PlatformError>
where
    State: Send + 'static,
    Request: Send + 'static,
    Response: Send + 'static,
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
    let runtime_options = runtime_options(options);
    let worker_count = options.event_loop_threads.max(1);
    if worker_count > 1 {
        TcpRuntime::bind_epoll_sharded_with_state(
            (host.as_str(), options.port),
            runtime_options,
            worker_count,
            init,
            move |info, state, bytes| {
                let mut worker = worker.lock().expect("embedded worker mutex poisoned");
                handler(info, state, bytes, &mut worker)
            },
            on_close,
        )
        .map(RuntimeInstance::Sharded)
    } else {
        TcpRuntime::bind_epoll_with_state(
            (host.as_str(), options.port),
            runtime_options,
            init,
            move |info, state, bytes| {
                let mut worker = worker.lock().expect("embedded worker mutex poisoned");
                handler(info, state, bytes, &mut worker)
            },
            on_close,
        )
        .map(RuntimeInstance::Single)
    }
}

#[cfg(target_os = "linux")]
fn build_runtime_mailbox<State, Request, Response, Init, Handler, Close, Submitter>(
    options: &EmbeddableTcpServerOptions,
    submitter: Submitter,
    init: Init,
    handler: Handler,
    on_close: Close,
) -> Result<RuntimeInstance<State>, PlatformError>
where
    State: Send + 'static,
    Request: Send + 'static,
    Response: Send + 'static,
    Init: Fn(TcpConnectionInfo) -> State + Send + Sync + 'static,
    Handler: Fn(TcpConnectionInfo, &mut State, &[u8], &Submitter) -> TcpHandlerResult
        + Send
        + Sync
        + 'static,
    Close: Fn(TcpConnectionInfo, State) + Send + Sync + 'static,
    Submitter: MailboxJobSubmitter<Request, Response> + Send + Sync + 'static,
{
    let host = options.host.clone();
    let runtime_options = runtime_options(options);
    let worker_count = options.event_loop_threads.max(1);
    if worker_count > 1 {
        TcpRuntime::bind_epoll_sharded_with_state(
            (host.as_str(), options.port),
            runtime_options,
            worker_count,
            init,
            move |info, state, bytes| handler(info, state, bytes, &submitter),
            on_close,
        )
        .map(RuntimeInstance::Sharded)
    } else {
        TcpRuntime::bind_epoll_with_state(
            (host.as_str(), options.port),
            runtime_options,
            init,
            move |info, state, bytes| handler(info, state, bytes, &submitter),
            on_close,
        )
        .map(RuntimeInstance::Single)
    }
}

#[cfg(target_os = "windows")]
fn build_runtime<State, Request, Response, Init, Handler, Close>(
    options: &EmbeddableTcpServerOptions,
    worker: Arc<Mutex<StdioJobWorker<Request, Response>>>,
    init: Init,
    handler: Handler,
    on_close: Close,
) -> Result<RuntimeInstance<State>, PlatformError>
where
    State: Send + 'static,
    Request: Send + 'static,
    Response: Send + 'static,
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
    let runtime_options = runtime_options(options);
    let worker_count = options.event_loop_threads.max(1);
    if worker_count > 1 {
        TcpRuntime::bind_windows_sharded_with_state(
            (host.as_str(), options.port),
            runtime_options,
            worker_count,
            init,
            move |info, state, bytes| {
                let mut worker = worker.lock().expect("embedded worker mutex poisoned");
                handler(info, state, bytes, &mut worker)
            },
            on_close,
        )
        .map(RuntimeInstance::Sharded)
    } else {
        TcpRuntime::bind_windows_with_state(
            (host.as_str(), options.port),
            runtime_options,
            init,
            move |info, state, bytes| {
                let mut worker = worker.lock().expect("embedded worker mutex poisoned");
                handler(info, state, bytes, &mut worker)
            },
            on_close,
        )
        .map(RuntimeInstance::Single)
    }
}

#[cfg(target_os = "windows")]
fn build_runtime_mailbox<State, Request, Response, Init, Handler, Close, Submitter>(
    options: &EmbeddableTcpServerOptions,
    submitter: Submitter,
    init: Init,
    handler: Handler,
    on_close: Close,
) -> Result<RuntimeInstance<State>, PlatformError>
where
    State: Send + 'static,
    Request: Send + 'static,
    Response: Send + 'static,
    Init: Fn(TcpConnectionInfo) -> State + Send + Sync + 'static,
    Handler: Fn(TcpConnectionInfo, &mut State, &[u8], &Submitter) -> TcpHandlerResult
        + Send
        + Sync
        + 'static,
    Close: Fn(TcpConnectionInfo, State) + Send + Sync + 'static,
    Submitter: MailboxJobSubmitter<Request, Response> + Send + Sync + 'static,
{
    let host = options.host.clone();
    let runtime_options = runtime_options(options);
    let worker_count = options.event_loop_threads.max(1);
    if worker_count > 1 {
        TcpRuntime::bind_windows_sharded_with_state(
            (host.as_str(), options.port),
            runtime_options,
            worker_count,
            init,
            move |info, state, bytes| handler(info, state, bytes, &submitter),
            on_close,
        )
        .map(RuntimeInstance::Sharded)
    } else {
        TcpRuntime::bind_windows_with_state(
            (host.as_str(), options.port),
            runtime_options,
            init,
            move |info, state, bytes| handler(info, state, bytes, &submitter),
            on_close,
        )
        .map(RuntimeInstance::Single)
    }
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
    use resp_protocol::{decode, encode, RespValue};
    use serde::{Deserialize, Serialize};
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::path::{Path, PathBuf};
    use std::thread;
    use std::time::Duration;

    const MAX_TCP_CHUNK_BYTES: usize = 1024 * 1024;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct TcpInputJob {
        stream_id: String,
        bytes_hex: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct TcpOutputFrame {
        writes_hex: Vec<String>,
        close: bool,
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

    fn execute_scripted_worker(script: &str) -> Result<TcpOutputFrame, WorkerError> {
        let worker_command = scripted_worker(script).expect("python interpreter");
        let mut worker = StdioJobWorker::<TcpInputJob, TcpOutputFrame>::spawn(&worker_command)
            .expect("spawn scripted worker");
        worker.execute(TcpInputJob {
            stream_id: "test-stream".to_string(),
            bytes_hex: hex_encode(command(&[b"PING"])),
        })
    }

    fn handle_tcp_bytes(
        info: TcpConnectionInfo,
        _state: &mut (),
        data: &[u8],
        worker: &mut StdioJobWorker<TcpInputJob, TcpOutputFrame>,
    ) -> TcpHandlerResult {
        if data.len() > MAX_TCP_CHUNK_BYTES {
            return TcpHandlerResult::close();
        }

        match worker.execute(TcpInputJob {
            stream_id: info.id.0.to_string(),
            bytes_hex: hex_encode(data.to_vec()),
        }) {
            Ok(frame) => match decode_writes(&frame.writes_hex) {
                Ok(bytes) if frame.close => TcpHandlerResult::write_and_close(bytes),
                Ok(bytes) if bytes.is_empty() => TcpHandlerResult::default(),
                Ok(bytes) => TcpHandlerResult::write(bytes),
                Err(_) => TcpHandlerResult::close(),
            },
            Err(_) => TcpHandlerResult::close(),
        }
    }

    fn handle_tcp_bytes_mailbox(
        info: TcpConnectionInfo,
        _state: &mut (),
        data: &[u8],
        submitter: &StdioJobSubmitter<TcpInputJob, TcpOutputFrame>,
    ) -> TcpHandlerResult {
        handle_tcp_bytes_mailbox_with_submitter(info, _state, data, submitter)
    }

    fn handle_tcp_bytes_mailbox_with_submitter<S>(
        info: TcpConnectionInfo,
        _state: &mut (),
        data: &[u8],
        submitter: &S,
    ) -> TcpHandlerResult
    where
        S: MailboxJobSubmitter<TcpInputJob, TcpOutputFrame>,
    {
        if data.len() > MAX_TCP_CHUNK_BYTES {
            return TcpHandlerResult::close();
        }

        match submitter.submit(
            info.id,
            TcpInputJob {
                stream_id: info.id.0.to_string(),
                bytes_hex: hex_encode(data.to_vec()),
            },
        ) {
            Ok(_) => TcpHandlerResult::default(),
            Err(WorkerError::Submit(SubmitError::QueueFull)) => TcpHandlerResult::defer_read(),
            Err(_) => TcpHandlerResult::close(),
        }
    }

    fn map_tcp_output_frame(frame: TcpOutputFrame) -> Result<TcpMailboxFrame, WorkerError> {
        let mut writes = Vec::with_capacity(frame.writes_hex.len());
        for item in frame.writes_hex {
            writes.push(hex_decode(&item).map_err(WorkerError::Protocol)?);
        }
        Ok(TcpMailboxFrame {
            writes,
            close: frame.close,
        })
    }

    fn decode_writes(writes_hex: &[String]) -> Result<Vec<u8>, String> {
        let mut output = Vec::new();
        for item in writes_hex {
            output.extend(hex_decode(item)?);
        }
        Ok(output)
    }

    fn read_responses(stream: &mut TcpStream, count: usize) -> io::Result<Vec<RespValue>> {
        let mut values = Vec::with_capacity(count);
        let mut buffer = Vec::new();
        let mut chunk = [0u8; 1024];
        while values.len() < count {
            match decode(&buffer) {
                Ok(Some((value, consumed))) => {
                    buffer.drain(..consumed);
                    values.push(value);
                }
                Ok(None) => {
                    let n = stream.read(&mut chunk)?;
                    if n == 0 {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "server closed before all responses",
                        ));
                    }
                    buffer.extend_from_slice(&chunk[..n]);
                }
                Err(error) => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("invalid RESP response: {error}"),
                    ));
                }
            }
        }
        Ok(values)
    }

    fn write_command(stream: &mut TcpStream, parts: &[&[u8]]) {
        stream
            .write_all(&command(parts))
            .expect("write RESP command to TCP stream");
    }

    fn write_fragmented_command(stream: &mut TcpStream, parts: &[&[u8]]) {
        let bytes = command(parts);
        let split_at = bytes.len() / 2;
        stream
            .write_all(&bytes[..split_at])
            .expect("write first fragment");
        stream
            .write_all(&bytes[split_at..])
            .expect("write second fragment");
    }

    fn scripted_success_response(id_expression: &str) -> String {
        format!(
            r#"
import json, sys
frame = json.loads(sys.stdin.readline())
body = frame["body"]
payload = {{"writes_hex":["{}"],"close":False}}
print(json.dumps({{"version":1,"kind":"response","body":{{"id":{},"result":{{"status":"ok","payload":payload}},"metadata":body["metadata"]}}}}), flush=True)
"#,
            hex_encode(b"+PONG\r\n".to_vec()),
            id_expression
        )
    }

    fn assert_pong_output(frame: TcpOutputFrame) {
        assert!(!frame.close);
        assert_eq!(
            decode_writes(&frame.writes_hex).expect("decode worker writes"),
            b"+PONG\r\n"
        );
    }

    fn assert_worker_receives_opaque_tcp_bytes() {
        let script = r#"
import json, sys
frame = json.loads(sys.stdin.readline())
body = frame["body"]
payload = body["payload"]
assert set(payload.keys()) == {"stream_id", "bytes_hex"}
assert payload["stream_id"] == "test-stream"
assert payload["bytes_hex"].startswith("2a")
print(json.dumps({"version":1,"kind":"response","body":{"id":body["id"],"result":{"status":"ok","payload":{"writes_hex":[],"close":False}},"metadata":body["metadata"]}}), flush=True)
"#;
        execute_scripted_worker(script).expect("opaque TCP bytes response");
    }

    fn start_test_server(
        worker: WorkerCommand,
    ) -> (EmbeddableTcpServer<()>, thread::JoinHandle<io::Result<()>>) {
        let server = EmbeddableTcpServer::new(
            EmbeddableTcpServerOptions {
                host: "127.0.0.1".to_string(),
                port: 0,
                max_connections: 64,
                event_loop_threads: 1,
                worker_processes: 1,
                worker_queue_depth: ExecutorLimits::default().max_queue_depth,
                worker_job_timeout: None,
                worker_restart_policy: StdioWorkerRestartPolicy::default(),
                worker,
            },
            |_| (),
            handle_tcp_bytes,
            |_, _| {},
        )
        .expect("create server");
        assert!(!server.is_running());
        let background = server.clone();
        let handle = thread::spawn(move || background.serve());
        (server, handle)
    }

    fn connect_client(server: &EmbeddableTcpServer<()>) -> TcpStream {
        let stream = TcpStream::connect(server.local_addr()).expect("connect client");
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .expect("set read timeout");
        stream
    }

    fn stop_server(server: EmbeddableTcpServer<()>, handle: thread::JoinHandle<io::Result<()>>) {
        server.stop();
        handle
            .join()
            .expect("server thread")
            .expect("server result");
    }

    fn assert_two_streams_keep_distinct_redis_sessions(
        first: &mut TcpStream,
        second: &mut TcpStream,
    ) {
        write_command(first, &[b"SET", b"k", b"db0"]);
        assert_eq!(
            read_response(first).expect("read first set"),
            RespValue::SimpleString("OK".to_string())
        );

        write_command(second, &[b"SELECT", b"1"]);
        assert_eq!(
            read_response(second).expect("read second select"),
            RespValue::SimpleString("OK".to_string())
        );

        write_command(second, &[b"GET", b"k"]);
        assert_eq!(
            read_response(second).expect("read second missing"),
            RespValue::BulkString(None)
        );

        write_command(second, &[b"SET", b"k", b"db1"]);
        assert_eq!(
            read_response(second).expect("read second set"),
            RespValue::SimpleString("OK".to_string())
        );

        write_command(first, &[b"GET", b"k"]);
        write_command(second, &[b"GET", b"k"]);
        assert_eq!(
            read_response(first).expect("read first db0"),
            RespValue::BulkString(Some(b"db0".to_vec()))
        );
        assert_eq!(
            read_response(second).expect("read second db1"),
            RespValue::BulkString(Some(b"db1".to_vec()))
        );
    }

    fn assert_pipelined_commands(stream: &mut TcpStream) {
        let mut pipelined = Vec::new();
        pipelined.extend(command(&[b"PING"]));
        pipelined.extend(command(&[b"PING", b"hello"]));
        stream.write_all(&pipelined).expect("write pipeline");
        assert_eq!(
            read_responses(stream, 2).expect("read pipeline"),
            vec![
                RespValue::SimpleString("PONG".to_string()),
                RespValue::BulkString(Some(b"hello".to_vec())),
            ]
        );
    }

    fn assert_fragmented_command(stream: &mut TcpStream) {
        write_fragmented_command(stream, &[b"PING"]);
        assert_eq!(
            read_response(stream).expect("read fragmented ping"),
            RespValue::SimpleString("PONG".to_string())
        );
    }

    fn assert_hash_round_trip(stream: &mut TcpStream) {
        write_command(stream, &[b"HSET", b"user", b"name", b"grace"]);
        assert_eq!(
            read_response(stream).expect("read hset"),
            RespValue::Integer(1)
        );

        write_command(stream, &[b"HGET", b"user", b"name"]);
        assert_eq!(
            read_response(stream).expect("read hget"),
            RespValue::BulkString(Some(b"grace".to_vec()))
        );
    }

    fn assert_string_round_trip(stream: &mut TcpStream) {
        write_command(stream, &[b"SET", b"name", b"ada"]);
        assert_eq!(
            read_response(stream).expect("read set"),
            RespValue::SimpleString("OK".to_string())
        );

        write_command(stream, &[b"GET", b"name"]);
        assert_eq!(
            read_response(stream).expect("read get"),
            RespValue::BulkString(Some(b"ada".to_vec()))
        );
    }

    fn assert_ping(stream: &mut TcpStream) {
        write_command(stream, &[b"PING"]);
        assert_eq!(
            read_response(stream).expect("read ping"),
            RespValue::SimpleString("PONG".to_string())
        );
    }

    fn assert_invalid_worker_write_closes_connection() {
        let Some(command) = scripted_worker(
            r#"
import json, sys
frame = json.loads(sys.stdin.readline())
body = frame["body"]
print(json.dumps({"version":1,"kind":"response","body":{"id":body["id"],"result":{"status":"ok","payload":{"writes_hex":["zz"],"close":False}},"metadata":body["metadata"]}}), flush=True)
"#,
        ) else {
            return;
        };
        let (server, handle) = start_test_server(command);
        let mut stream = connect_client(&server);
        write_command(&mut stream, &[b"PING"]);
        let mut one = [0u8; 1];
        assert_eq!(stream.read(&mut one).expect("read close"), 0);
        stop_server(server, handle);
    }

    fn assert_worker_close_frame_closes_connection() {
        let Some(command) = scripted_worker(
            r#"
import json, sys
frame = json.loads(sys.stdin.readline())
body = frame["body"]
print(json.dumps({"version":1,"kind":"response","body":{"id":body["id"],"result":{"status":"ok","payload":{"writes_hex":[],"close":True}},"metadata":body["metadata"]}}), flush=True)
"#,
        ) else {
            return;
        };
        let (server, handle) = start_test_server(command);
        let mut stream = connect_client(&server);
        write_command(&mut stream, &[b"PING"]);
        let mut one = [0u8; 1];
        assert_eq!(stream.read(&mut one).expect("read close"), 0);
        stop_server(server, handle);
    }

    fn assert_worker_response_can_be_empty() {
        let Some(command) = scripted_worker(
            r#"
import json, sys
frame = json.loads(sys.stdin.readline())
body = frame["body"]
print(json.dumps({"version":1,"kind":"response","body":{"id":body["id"],"result":{"status":"ok","payload":{"writes_hex":[],"close":False}},"metadata":body["metadata"]}}), flush=True)
"#,
        ) else {
            return;
        };
        let (server, handle) = start_test_server(command);
        let mut stream = connect_client(&server);
        stream
            .set_read_timeout(Some(Duration::from_millis(100)))
            .expect("set short read timeout");
        write_command(&mut stream, &[b"PING"]);
        let mut one = [0u8; 1];
        assert!(matches!(
            stream.read(&mut one),
            Err(error)
                if error.kind() == io::ErrorKind::WouldBlock
                    || error.kind() == io::ErrorKind::TimedOut
        ));
        stop_server(server, handle);
    }

    fn assert_worker_error_closes_connection() {
        let Some(command) = scripted_worker(
            r#"
import json, sys
frame = json.loads(sys.stdin.readline())
body = frame["body"]
print(json.dumps({"version":1,"kind":"response","body":{"id":body["id"],"result":{"status":"error","error":{"code":"worker_failed","message":"boom","retryable":False,"origin":"worker","detail":None}},"metadata":body["metadata"]}}), flush=True)
"#,
        ) else {
            return;
        };
        let (server, handle) = start_test_server(command);
        let mut stream = connect_client(&server);
        write_command(&mut stream, &[b"PING"]);
        let mut one = [0u8; 1];
        assert_eq!(stream.read(&mut one).expect("read close"), 0);
        stop_server(server, handle);
    }

    fn assert_scripted_transport_edges() {
        assert_invalid_worker_write_closes_connection();
        assert_worker_close_frame_closes_connection();
        assert_worker_response_can_be_empty();
        assert_worker_error_closes_connection();
    }

    fn assert_python_worker_round_trip(mut stream: TcpStream) {
        assert_ping(&mut stream);
        assert_string_round_trip(&mut stream);
        assert_hash_round_trip(&mut stream);
        assert_pipelined_commands(&mut stream);
        assert_fragmented_command(&mut stream);
    }

    fn assert_python_worker_multi_stream_sessions(server: &EmbeddableTcpServer<()>) {
        let mut first = connect_client(server);
        let mut second = connect_client(server);
        assert_two_streams_keep_distinct_redis_sessions(&mut first, &mut second);
    }

    fn run_python_worker_integration(worker: WorkerCommand) {
        let (server, handle) = start_test_server(worker);
        let stream = connect_client(&server);
        assert_python_worker_round_trip(stream);
        assert_python_worker_multi_stream_sessions(&server);
        stop_server(server, handle);
    }

    fn start_async_test_server(
        worker: WorkerCommand,
    ) -> (EmbeddableTcpServer<()>, thread::JoinHandle<io::Result<()>>) {
        start_async_test_server_with_queue_depth(worker, ExecutorLimits::default().max_queue_depth)
    }

    fn start_async_test_server_with_queue_depth(
        worker: WorkerCommand,
        worker_queue_depth: usize,
    ) -> (EmbeddableTcpServer<()>, thread::JoinHandle<io::Result<()>>) {
        let server = EmbeddableTcpServer::new_mailbox(
            EmbeddableTcpServerOptions {
                host: "127.0.0.1".to_string(),
                port: 0,
                max_connections: 64,
                event_loop_threads: 1,
                worker_processes: 1,
                worker_queue_depth,
                worker_job_timeout: None,
                worker_restart_policy: StdioWorkerRestartPolicy::default(),
                worker,
            },
            |_| (),
            handle_tcp_bytes_mailbox,
            |_, _| {},
            map_tcp_output_frame,
        )
        .expect("create async mailbox server");
        let background = server.clone();
        let handle = thread::spawn(move || background.serve());
        (server, handle)
    }

    fn start_async_inprocess_test_server(
        worker_count: usize,
        worker_queue_depth: usize,
    ) -> (EmbeddableTcpServer<()>, thread::JoinHandle<io::Result<()>>) {
        let server = EmbeddableTcpServer::new_inprocess_mailbox(
            EmbeddableTcpServerOptions {
                host: "127.0.0.1".to_string(),
                port: 0,
                max_connections: 64,
                event_loop_threads: 1,
                worker_processes: worker_count.max(1),
                worker_queue_depth,
                worker_job_timeout: None,
                worker_restart_policy: StdioWorkerRestartPolicy::default(),
                worker: WorkerCommand::new("worker", Vec::<String>::new()),
            },
            |_| (),
            handle_tcp_bytes_mailbox_with_submitter::<
                RustThreadPoolJobSubmitter<TcpInputJob, TcpOutputFrame>,
            >,
            |_, _| {},
            map_tcp_output_frame,
            |request| JobResult::Ok {
                payload: TcpOutputFrame {
                    writes_hex: vec![request.payload.bytes_hex],
                    close: false,
                },
            },
        )
        .expect("create in-process async mailbox server");
        let background = server.clone();
        let handle = thread::spawn(move || background.serve());
        (server, handle)
    }

    fn assert_scripted_worker_maps_control_results() {
        let error_script = r#"
import json, sys
frame = json.loads(sys.stdin.readline())
body = frame["body"]
print(json.dumps({"version":1,"kind":"response","body":{"id":body["id"],"result":{"status":"error","error":{"code":"worker_failed","message":"boom","retryable":False,"origin":"worker","detail":None}},"metadata":body["metadata"]}}), flush=True)
"#;
        let error = execute_scripted_worker(error_script).expect_err("worker error should fail");
        assert!(error.to_string().contains("worker_failed: boom"));

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
        assert_eq!(options.event_loop_threads, 1);
        assert_eq!(options.worker_processes, 1);
        assert_eq!(
            options.worker_queue_depth,
            ExecutorLimits::default().max_queue_depth
        );
        assert_eq!(options.worker_job_timeout, None);
        assert_eq!(
            options.worker_restart_policy,
            StdioWorkerRestartPolicy::Never
        );

        let mut zero_connections = options.clone();
        zero_connections.max_connections = 0;
        assert_eq!(runtime_options(&zero_connections).max_connections, 1);
    }

    #[test]
    fn decode_writes_concatenates_worker_frames() {
        assert_eq!(
            decode_writes(&[
                hex_encode(b"hello ".to_vec()),
                hex_encode(b"world".to_vec())
            ])
            .expect("decode writes"),
            b"hello world"
        );
        assert_eq!(
            decode_writes(&["zz".to_string()]),
            Err("invalid hex byte: 122".to_string())
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

        let codec_error = generic_job_protocol::decode_request_json_line::<TcpInputJob>("not-json")
            .expect_err("invalid JSON should fail");
        let worker_error = WorkerError::from(codec_error);
        assert!(worker_error
            .to_string()
            .starts_with("worker protocol error: job frame JSON error:"));

        let submit_error = WorkerError::from(SubmitError::QueueFull);
        assert_eq!(
            submit_error.to_string(),
            "worker submit error: job queue is full"
        );
    }

    #[test]
    fn stdio_worker_executes_opaque_tcp_byte_jobs() {
        let Some(worker_command) = worker_command() else {
            eprintln!("skipping test because no Python interpreter was found");
            return;
        };
        let mut worker = StdioJobWorker::<TcpInputJob, TcpOutputFrame>::spawn(&worker_command)
            .expect("spawn worker");
        let response = worker
            .execute(TcpInputJob {
                stream_id: "stream-1".to_string(),
                bytes_hex: hex_encode(command(&[b"PING"])),
            })
            .expect("worker response");
        assert_pong_output(response);
    }

    #[test]
    fn stdio_worker_rejects_mismatched_response_ids() {
        let script = scripted_success_response(r#"body["id"] + "-wrong""#);

        let error = execute_scripted_worker(&script).expect_err("mismatched response should fail");
        assert!(error.to_string().contains("response id mismatch"));
    }

    #[test]
    fn stdio_worker_sends_only_opaque_tcp_bytes_to_worker() {
        assert_worker_receives_opaque_tcp_bytes();
    }

    #[test]
    fn stdio_worker_maps_portable_control_results() {
        assert_scripted_worker_maps_control_results();
    }

    #[test]
    fn tcp_server_handles_transport_edge_frames() {
        assert_scripted_transport_edges();
    }

    #[test]
    fn embeddable_tcp_server_delegates_opaque_bytes_to_python_worker() {
        let Some(worker) = worker_command() else {
            eprintln!("skipping test because no Python interpreter was found");
            return;
        };
        run_python_worker_integration(worker);
    }

    #[test]
    fn mailbox_server_does_not_block_tcp_reads_waiting_for_worker_responses() {
        let Some(worker) = scripted_worker(
            r#"
import json, sys

first = json.loads(sys.stdin.readline())
second = json.loads(sys.stdin.readline())

def emit(frame, text):
    body = frame["body"]
    payload = {"writes_hex": [text.encode("utf-8").hex()], "close": False}
    print(json.dumps({"version": 1, "kind": "response", "body": {"id": body["id"], "result": {"status": "ok", "payload": payload}, "metadata": body["metadata"]}}), flush=True)

emit(second, "second")
emit(first, "first")
"#,
        ) else {
            eprintln!("skipping test because no Python interpreter was found");
            return;
        };

        let (server, handle) = start_async_test_server(worker);
        let mut first = connect_client(&server);
        let mut second = connect_client(&server);

        first.write_all(b"one").expect("write first request");
        second.write_all(b"two").expect("write second request");

        let mut second_response = [0u8; 6];
        second
            .read_exact(&mut second_response)
            .expect("read second response");
        assert_eq!(&second_response, b"second");

        let mut first_response = [0u8; 5];
        first
            .read_exact(&mut first_response)
            .expect("read first response");
        assert_eq!(&first_response, b"first");

        stop_server(server, handle);
    }

    #[test]
    fn mailbox_server_replays_deferred_reads_after_worker_queue_pressure() {
        let marker = std::env::temp_dir().join(format!(
            "embeddable-tcp-queue-full-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
        ));
        let marker_literal = format!("{:?}", marker.to_string_lossy());
        let script = format!(
            r#"
import json, sys, time

marker_path = {marker_literal}

def emit(frame, text):
    body = frame["body"]
    payload = {{"writes_hex": [text.encode("utf-8").hex()], "close": False}}
    print(json.dumps({{"version": 1, "kind": "response", "body": {{"id": body["id"], "result": {{"status": "ok", "payload": payload}}, "metadata": body["metadata"]}}}}), flush=True)

first = json.loads(sys.stdin.readline())
open(marker_path, "w").close()
time.sleep(0.2)
emit(first, "first")

second = json.loads(sys.stdin.readline())
emit(second, "second")
"#
        );
        let Some(worker) = scripted_worker(&script) else {
            eprintln!("skipping test because no Python interpreter was found");
            return;
        };

        let (server, handle) = start_async_test_server_with_queue_depth(worker, 1);
        let mut first = connect_client(&server);
        let mut second = connect_client(&server);

        first.write_all(b"one").expect("write first request");
        for _ in 0..100 {
            if marker.exists() {
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
        assert!(
            marker.exists(),
            "worker should have accepted the first job before queue pressure is tested"
        );
        second.write_all(b"two").expect("write second request");

        let mut first_response = [0u8; 5];
        first
            .read_exact(&mut first_response)
            .expect("read first response");
        assert_eq!(&first_response, b"first");

        let mut second_response = [0u8; 6];
        second
            .read_exact(&mut second_response)
            .expect("read replayed second response");
        assert_eq!(&second_response, b"second");

        stop_server(server, handle);
        let _ = std::fs::remove_file(marker);
    }

    #[test]
    fn mailbox_server_processes_jobs_with_rust_thread_pool() {
        let (server, handle) = start_async_inprocess_test_server(4, 1024);
        let mut stream = connect_client(&server);
        let request = command(&[b"PING"]);
        let request_len = request.len();
        stream
            .write_all(&request)
            .expect("write ping through in-process worker");
        let mut response = vec![0u8; request_len];
        stream.read_exact(&mut response).expect("read response");
        assert_eq!(response, request);
        stop_server(server, handle);
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn mailbox_server_can_use_multiple_event_loop_threads() {
        let server = EmbeddableTcpServer::new_inprocess_mailbox(
            EmbeddableTcpServerOptions {
                host: "127.0.0.1".to_string(),
                port: 0,
                max_connections: 64,
                event_loop_threads: 2,
                worker_processes: 4,
                worker_queue_depth: 1024,
                worker_job_timeout: None,
                worker_restart_policy: StdioWorkerRestartPolicy::default(),
                worker: WorkerCommand::new("worker", Vec::<String>::new()),
            },
            |_| (),
            handle_tcp_bytes_mailbox_with_submitter::<
                RustThreadPoolJobSubmitter<TcpInputJob, TcpOutputFrame>,
            >,
            |_, _| {},
            map_tcp_output_frame,
            |request| JobResult::Ok {
                payload: TcpOutputFrame {
                    writes_hex: vec![request.payload.bytes_hex],
                    close: false,
                },
            },
        )
        .expect("create sharded in-process async mailbox server");
        let background = server.clone();
        let handle = thread::spawn(move || background.serve());

        let mut clients = (0..8).map(|_| connect_client(&server)).collect::<Vec<_>>();
        for (index, client) in clients.iter_mut().enumerate() {
            client
                .write_all(format!("client-{index}").as_bytes())
                .expect("write client payload");
        }
        for (index, client) in clients.iter_mut().enumerate() {
            let expected = format!("client-{index}").into_bytes();
            let mut response = vec![0u8; expected.len()];
            client.read_exact(&mut response).expect("read response");
            assert_eq!(response, expected);
        }

        stop_server(server, handle);
    }
}
