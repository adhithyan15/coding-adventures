//! # generic-job-runtime
//!
//! Bounded executors for `generic-job-protocol` jobs.
//!
//! The runtime is deliberately transport-neutral. TCP, UI event handling,
//! parsing, image work, and future FFI bridges can all submit typed
//! `JobRequest<T>` values and drain typed `JobResponse<U>` values without the
//! executor knowing what the payload means.

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::io::{self, BufRead, BufReader, Write};
use std::marker::PhantomData;
use std::panic::AssertUnwindSafe;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Condvar, Mutex, Weak};
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use generic_job_protocol::{
    decode_response_json_line_with_limit, encode_request_json_line, JobCancellation, JobCodecError,
    JobError, JobErrorOrigin, JobMetadata, JobRequest, JobResponse, JobResult, JobTimeout,
};
use serde::de::DeserializeOwned;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutorCapabilities {
    pub supports_parallel_execution: bool,
    pub supports_parallel_callbacks: bool,
    pub requires_vm_lock: bool,
    pub supports_process_isolation: bool,
    pub supports_cancellation: bool,
    pub supports_timeouts: bool,
    pub supports_affinity: bool,
    pub supports_ordered_responses: bool,
    pub requires_serializable_payloads: bool,
    pub max_workers: usize,
    pub max_queue_depth: usize,
    pub max_payload_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutorLimits {
    pub max_queue_depth: usize,
    pub max_payload_bytes: usize,
    pub max_response_bytes: usize,
}

impl Default for ExecutorLimits {
    fn default() -> Self {
        Self {
            max_queue_depth: 1024,
            max_payload_bytes: 1024 * 1024,
            max_response_bytes: 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StdioWorkerCommand {
    pub program: String,
    pub args: Vec<String>,
}

impl StdioWorkerCommand {
    pub fn new(program: impl Into<String>, args: impl Into<Vec<String>>) -> Self {
        Self {
            program: program.into(),
            args: args.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StdioProcessPoolOptions {
    pub worker_count: usize,
    pub limits: ExecutorLimits,
    pub default_job_timeout: Option<Duration>,
    pub restart_policy: StdioWorkerRestartPolicy,
}

impl Default for StdioProcessPoolOptions {
    fn default() -> Self {
        Self {
            worker_count: 1,
            limits: ExecutorLimits::default(),
            default_job_timeout: None,
            restart_policy: StdioWorkerRestartPolicy::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StdioWorkerRestartPolicy {
    Never,
    Always,
    Bounded {
        max_restarts: usize,
        window: Duration,
    },
}

impl Default for StdioWorkerRestartPolicy {
    fn default() -> Self {
        Self::Never
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubmitError {
    QueueFull,
    ShuttingDown,
    WorkerUnavailable,
    PayloadTooLarge { actual: usize, max: usize },
    Codec(String),
    Io(String),
}

impl fmt::Display for SubmitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::QueueFull => f.write_str("job queue is full"),
            Self::ShuttingDown => f.write_str("job executor is shutting down"),
            Self::WorkerUnavailable => f.write_str("no worker process is available"),
            Self::PayloadTooLarge { actual, max } => {
                write!(f, "job payload frame has {actual} bytes, exceeding {max}")
            }
            Self::Codec(message) => write!(f, "job codec error: {message}"),
            Self::Io(message) => write!(f, "job I/O error: {message}"),
        }
    }
}

impl std::error::Error for SubmitError {}

impl From<JobCodecError> for SubmitError {
    fn from(value: JobCodecError) -> Self {
        Self::Codec(value.to_string())
    }
}

impl From<io::Error> for SubmitError {
    fn from(value: io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelResult {
    Cancelled,
    AlreadyCancelled,
    AlreadyCompleted,
    ShuttingDown,
    Unsupported,
}

#[derive(Debug)]
pub enum RuntimeError {
    Io(io::Error),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "job runtime I/O error: {error}"),
        }
    }
}

impl std::error::Error for RuntimeError {}

pub trait JobExecutor<Request, Response> {
    fn capabilities(&self) -> ExecutorCapabilities;
    fn try_submit(&self, request: JobRequest<Request>) -> Result<(), SubmitError>;
    fn cancel(&self, id: &str) -> CancelResult;
    fn drain_responses(&self, max: usize) -> Vec<JobResponse<Response>>;
    fn shutdown(&self);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustThreadPoolOptions {
    pub worker_count: usize,
    pub limits: ExecutorLimits,
    pub default_job_timeout: Option<Duration>,
}

impl Default for RustThreadPoolOptions {
    fn default() -> Self {
        Self {
            worker_count: thread::available_parallelism()
                .map(usize::from)
                .unwrap_or(1),
            limits: ExecutorLimits::default(),
            default_job_timeout: None,
        }
    }
}

type RustJobHandler<Request, Response> =
    dyn Fn(JobRequest<Request>) -> JobResult<Response> + Send + Sync + 'static;

pub struct RustThreadPool<Request, Response> {
    inner: Arc<ThreadPoolInner<Request, Response>>,
    guard: Arc<ThreadPoolGuard<Request, Response>>,
}

impl<Request, Response> Clone for RustThreadPool<Request, Response> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            guard: Arc::clone(&self.guard),
        }
    }
}

struct ThreadPoolInner<Request, Response> {
    queue: Mutex<ThreadPoolQueue<Request>>,
    queue_ready: Condvar,
    responses: Mutex<mpsc::Receiver<JobResponse<Response>>>,
    response_sender: mpsc::Sender<JobResponse<Response>>,
    handler: Arc<RustJobHandler<Request, Response>>,
    in_flight: AtomicUsize,
    shutting_down: AtomicBool,
    worker_count: usize,
    limits: ExecutorLimits,
    default_job_timeout: Option<Duration>,
}

struct ThreadPoolGuard<Request, Response> {
    inner: Weak<ThreadPoolInner<Request, Response>>,
    workers: Mutex<Vec<JoinHandle<()>>>,
}

struct ThreadPoolQueue<Request> {
    jobs: VecDeque<ThreadPoolJob<Request>>,
    pending: HashMap<String, ThreadPoolPendingJob>,
    closed: bool,
}

struct ThreadPoolJob<Request> {
    request: JobRequest<Request>,
}

struct ThreadPoolPendingJob {
    metadata: JobMetadata,
    deadline_at_ms: Option<u64>,
    state: ThreadPoolJobState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ThreadPoolJobState {
    Queued,
    Running {
        terminal: Option<ThreadPoolTerminal>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ThreadPoolTerminal {
    Cancelled,
    TimedOut,
}

impl<Request, Response> RustThreadPool<Request, Response>
where
    Request: Send + 'static,
    Response: Send + 'static,
{
    pub fn spawn<Handler>(options: RustThreadPoolOptions, handler: Handler) -> Self
    where
        Handler: Fn(JobRequest<Request>) -> JobResult<Response> + Send + Sync + 'static,
    {
        let worker_count = options.worker_count.max(1);
        let (sender, receiver) = mpsc::channel();
        let inner = Arc::new(ThreadPoolInner {
            queue: Mutex::new(ThreadPoolQueue {
                jobs: VecDeque::new(),
                pending: HashMap::new(),
                closed: false,
            }),
            queue_ready: Condvar::new(),
            responses: Mutex::new(receiver),
            response_sender: sender,
            handler: Arc::new(handler),
            in_flight: AtomicUsize::new(0),
            shutting_down: AtomicBool::new(false),
            worker_count,
            limits: options.limits,
            default_job_timeout: options.default_job_timeout,
        });

        let workers = (0..worker_count)
            .map(|_| spawn_thread_pool_worker(Arc::clone(&inner)))
            .collect::<Vec<_>>();
        let guard = Arc::new(ThreadPoolGuard {
            inner: Arc::downgrade(&inner),
            workers: Mutex::new(workers),
        });

        Self { inner, guard }
    }

    pub fn submit(&self, request: JobRequest<Request>) -> Result<(), SubmitError> {
        self.try_submit(request)
    }

    pub fn recv_response_timeout(
        &self,
        timeout: Duration,
    ) -> Result<Option<JobResponse<Response>>, RuntimeError> {
        self.expire_timed_out_jobs();
        let receiver = self
            .inner
            .responses
            .lock()
            .expect("job response receiver mutex poisoned");
        match receiver.recv_timeout(timeout) {
            Ok(response) => Ok(Some(response)),
            Err(mpsc::RecvTimeoutError::Timeout) => Ok(None),
            Err(mpsc::RecvTimeoutError::Disconnected) => Ok(None),
        }
    }

    fn reserve_slot(&self) -> Result<(), SubmitError> {
        loop {
            let current = self.inner.in_flight.load(Ordering::SeqCst);
            if current >= self.inner.limits.max_queue_depth {
                return Err(SubmitError::QueueFull);
            }
            if self
                .inner
                .in_flight
                .compare_exchange(current, current + 1, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                return Ok(());
            }
        }
    }

    fn expire_timed_out_jobs(&self) {
        let now = current_time_millis();
        let mut timed_out = Vec::new();
        {
            let mut queue = self
                .inner
                .queue
                .lock()
                .expect("thread pool queue mutex poisoned");
            let expired_ids = queue
                .pending
                .iter()
                .filter_map(|(id, pending)| {
                    pending
                        .deadline_at_ms
                        .filter(|deadline| *deadline <= now)
                        .map(|_| id.clone())
                })
                .collect::<Vec<_>>();
            for id in expired_ids {
                let Some(pending) = queue.pending.get_mut(&id) else {
                    continue;
                };
                match pending.state {
                    ThreadPoolJobState::Queued => {
                        if let Some(position) =
                            queue.jobs.iter().position(|job| job.request.id == id)
                        {
                            queue.jobs.remove(position);
                        }
                        if let Some(pending) = queue.pending.remove(&id) {
                            decrement_in_flight(&self.inner.in_flight);
                            timed_out.push(timeout_response(id, pending.metadata));
                        }
                    }
                    ThreadPoolJobState::Running { ref mut terminal } => {
                        if terminal.is_none() {
                            *terminal = Some(ThreadPoolTerminal::TimedOut);
                        }
                    }
                }
            }
        }

        for response in timed_out {
            let _ = self.inner.response_sender.send(response);
        }
    }
}

impl<Request, Response> JobExecutor<Request, Response> for RustThreadPool<Request, Response>
where
    Request: Send + 'static,
    Response: Send + 'static,
{
    fn capabilities(&self) -> ExecutorCapabilities {
        ExecutorCapabilities {
            supports_parallel_execution: self.inner.worker_count > 1,
            supports_parallel_callbacks: true,
            requires_vm_lock: false,
            supports_process_isolation: false,
            supports_cancellation: true,
            supports_timeouts: true,
            supports_affinity: false,
            supports_ordered_responses: false,
            requires_serializable_payloads: false,
            max_workers: self.inner.worker_count,
            max_queue_depth: self.inner.limits.max_queue_depth,
            max_payload_bytes: self.inner.limits.max_payload_bytes,
        }
    }

    fn try_submit(&self, request: JobRequest<Request>) -> Result<(), SubmitError> {
        self.expire_timed_out_jobs();
        if self.inner.shutting_down.load(Ordering::SeqCst) {
            return Err(SubmitError::ShuttingDown);
        }

        self.reserve_slot()?;
        let id = request.id.clone();
        let metadata = request.metadata.clone();
        let deadline_at_ms = metadata.deadline_at_ms.or_else(|| {
            self.inner
                .default_job_timeout
                .map(|timeout| current_time_millis().saturating_add(timeout.as_millis() as u64))
        });
        {
            let mut queue = self
                .inner
                .queue
                .lock()
                .expect("thread pool queue mutex poisoned");
            if queue.closed || self.inner.shutting_down.load(Ordering::SeqCst) {
                decrement_in_flight(&self.inner.in_flight);
                return Err(SubmitError::ShuttingDown);
            }
            queue.pending.insert(
                id,
                ThreadPoolPendingJob {
                    metadata,
                    deadline_at_ms,
                    state: ThreadPoolJobState::Queued,
                },
            );
            queue.jobs.push_back(ThreadPoolJob { request });
        }
        self.inner.queue_ready.notify_one();
        Ok(())
    }

    fn cancel(&self, id: &str) -> CancelResult {
        if self.inner.shutting_down.load(Ordering::SeqCst) {
            return CancelResult::ShuttingDown;
        }

        let mut cancelled = None;
        {
            let mut queue = self
                .inner
                .queue
                .lock()
                .expect("thread pool queue mutex poisoned");
            let Some(pending) = queue.pending.get_mut(id) else {
                return CancelResult::AlreadyCompleted;
            };
            match pending.state {
                ThreadPoolJobState::Queued => {
                    if let Some(position) = queue.jobs.iter().position(|job| job.request.id == id) {
                        queue.jobs.remove(position);
                    }
                    if let Some(pending) = queue.pending.remove(id) {
                        decrement_in_flight(&self.inner.in_flight);
                        cancelled = Some(cancelled_response(id.to_string(), pending.metadata));
                    }
                }
                ThreadPoolJobState::Running { ref mut terminal } => match terminal {
                    Some(ThreadPoolTerminal::Cancelled) => return CancelResult::AlreadyCancelled,
                    Some(ThreadPoolTerminal::TimedOut) => return CancelResult::AlreadyCompleted,
                    None => *terminal = Some(ThreadPoolTerminal::Cancelled),
                },
            }
        }

        if let Some(response) = cancelled {
            let _ = self.inner.response_sender.send(response);
        }
        CancelResult::Cancelled
    }

    fn drain_responses(&self, max: usize) -> Vec<JobResponse<Response>> {
        self.expire_timed_out_jobs();
        let receiver = self
            .inner
            .responses
            .lock()
            .expect("job response receiver mutex poisoned");
        let mut responses = Vec::new();
        for _ in 0..max {
            match receiver.try_recv() {
                Ok(response) => responses.push(response),
                Err(mpsc::TryRecvError::Empty | mpsc::TryRecvError::Disconnected) => break,
            }
        }
        responses
    }

    fn shutdown(&self) {
        self.inner.shutting_down.store(true, Ordering::SeqCst);
        let mut queue = self
            .inner
            .queue
            .lock()
            .expect("thread pool queue mutex poisoned");
        queue.closed = true;
        self.inner.queue_ready.notify_all();
    }
}

impl<Request, Response> Drop for ThreadPoolGuard<Request, Response> {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.upgrade() {
            inner.shutting_down.store(true, Ordering::SeqCst);
            if let Ok(mut queue) = inner.queue.lock() {
                queue.closed = true;
                inner.queue_ready.notify_all();
            }
        };
        if let Ok(mut workers) = self.workers.lock() {
            for worker in workers.drain(..) {
                let _ = worker.join();
            }
        }
    }
}

fn spawn_thread_pool_worker<Request, Response>(
    inner: Arc<ThreadPoolInner<Request, Response>>,
) -> JoinHandle<()>
where
    Request: Send + 'static,
    Response: Send + 'static,
{
    thread::spawn(move || loop {
        let Some(request) = take_next_thread_pool_job(&inner) else {
            break;
        };
        let id = request.id.clone();
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| (inner.handler)(request)))
            .unwrap_or_else(|panic| panic_job_result(panic_message(panic)));
        let response = complete_thread_pool_job(&inner, id, result);
        if let Some(response) = response {
            if inner.response_sender.send(response).is_err() {
                break;
            }
        }
    })
}

fn take_next_thread_pool_job<Request, Response>(
    inner: &ThreadPoolInner<Request, Response>,
) -> Option<JobRequest<Request>> {
    let mut queue = inner
        .queue
        .lock()
        .expect("thread pool queue mutex poisoned");
    loop {
        if queue.closed || inner.shutting_down.load(Ordering::SeqCst) {
            return None;
        }
        let Some(job) = queue.jobs.pop_front() else {
            queue = inner
                .queue_ready
                .wait(queue)
                .expect("thread pool queue mutex poisoned");
            continue;
        };
        if let Some(pending) = queue.pending.get_mut(&job.request.id) {
            pending.state = ThreadPoolJobState::Running { terminal: None };
            return Some(job.request);
        }
    }
}

fn complete_thread_pool_job<Request, Response>(
    inner: &ThreadPoolInner<Request, Response>,
    id: String,
    result: JobResult<Response>,
) -> Option<JobResponse<Response>> {
    let pending = {
        let mut queue = inner
            .queue
            .lock()
            .expect("thread pool queue mutex poisoned");
        queue.pending.remove(&id)
    };
    let Some(pending) = pending else {
        return None;
    };
    decrement_in_flight(&inner.in_flight);

    match pending.state {
        ThreadPoolJobState::Running {
            terminal: Some(ThreadPoolTerminal::Cancelled),
        } => Some(cancelled_response(id, pending.metadata)),
        ThreadPoolJobState::Running {
            terminal: Some(ThreadPoolTerminal::TimedOut),
        } => Some(timeout_response(id, pending.metadata)),
        _ => Some(JobResponse {
            id,
            result,
            metadata: pending.metadata,
        }),
    }
}

pub struct StdioProcessPool<Request, Response> {
    inner: Arc<PoolInner<Response>>,
    _request: PhantomData<fn() -> Request>,
}

impl<Request, Response> Clone for StdioProcessPool<Request, Response> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            _request: PhantomData,
        }
    }
}

struct PoolInner<Response> {
    command: StdioWorkerCommand,
    workers: Vec<WorkerSlot>,
    responses: Mutex<mpsc::Receiver<JobResponse<Response>>>,
    response_sender: mpsc::Sender<JobResponse<Response>>,
    pending: Arc<Mutex<HashMap<String, PendingJob>>>,
    next_worker: AtomicUsize,
    in_flight: Arc<AtomicUsize>,
    shutting_down: AtomicBool,
    worker_alive: Vec<Arc<AtomicBool>>,
    limits: ExecutorLimits,
    default_job_timeout: Option<Duration>,
    restart_policy: StdioWorkerRestartPolicy,
}

struct WorkerSlot {
    process: Mutex<Option<WorkerProcess>>,
    alive: Arc<AtomicBool>,
    restart_state: Mutex<RestartState>,
}

struct WorkerProcess {
    stdin: Arc<Mutex<ChildStdin>>,
    child: Arc<Mutex<Child>>,
    reader: Option<thread::JoinHandle<()>>,
}

#[derive(Debug, Default)]
struct RestartState {
    restart_timestamps_ms: VecDeque<u64>,
}

impl Drop for WorkerProcess {
    fn drop(&mut self) {
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}

#[derive(Debug, Clone)]
struct PendingJob {
    worker_index: usize,
    metadata: JobMetadata,
    deadline_at_ms: Option<u64>,
}

impl<Response> Drop for PoolInner<Response> {
    fn drop(&mut self) {
        self.shutting_down.store(true, Ordering::SeqCst);
    }
}

impl<Request, Response> StdioProcessPool<Request, Response>
where
    Request: Serialize + Send + 'static,
    Response: DeserializeOwned + Send + 'static,
{
    pub fn spawn(
        command: StdioWorkerCommand,
        options: StdioProcessPoolOptions,
    ) -> io::Result<Self> {
        let worker_count = options.worker_count.max(1);
        let (sender, receiver) = mpsc::channel();
        let mut workers = Vec::with_capacity(worker_count);
        let mut worker_alive = Vec::with_capacity(worker_count);
        let in_flight = Arc::new(AtomicUsize::new(0));
        let pending = Arc::new(Mutex::new(HashMap::new()));

        for worker_index in 0..worker_count {
            let alive = Arc::new(AtomicBool::new(false));
            let process = spawn_worker::<Response>(
                &command,
                worker_index,
                sender.clone(),
                Arc::clone(&pending),
                Arc::clone(&in_flight),
                Arc::clone(&alive),
                options.limits.max_response_bytes,
            )?;
            worker_alive.push(Arc::clone(&alive));
            workers.push(WorkerSlot {
                process: Mutex::new(Some(process)),
                alive,
                restart_state: Mutex::new(RestartState::default()),
            });
        }

        Ok(Self {
            inner: Arc::new(PoolInner {
                command,
                workers,
                responses: Mutex::new(receiver),
                response_sender: sender,
                pending,
                next_worker: AtomicUsize::new(0),
                in_flight,
                shutting_down: AtomicBool::new(false),
                worker_alive,
                limits: options.limits,
                default_job_timeout: options.default_job_timeout,
                restart_policy: options.restart_policy,
            }),
            _request: PhantomData,
        })
    }

    pub fn submit(&self, request: JobRequest<Request>) -> Result<(), SubmitError> {
        self.try_submit(request)
    }

    pub fn recv_response_timeout(
        &self,
        timeout: Duration,
    ) -> Result<Option<JobResponse<Response>>, RuntimeError> {
        self.expire_timed_out_jobs();
        let receiver = self
            .inner
            .responses
            .lock()
            .expect("job response receiver mutex poisoned");
        match receiver.recv_timeout(timeout) {
            Ok(response) => Ok(Some(response)),
            Err(mpsc::RecvTimeoutError::Timeout) => Ok(None),
            Err(mpsc::RecvTimeoutError::Disconnected) => Ok(None),
        }
    }

    fn worker_index_for(&self, request: &JobRequest<Request>) -> Result<usize, SubmitError> {
        let worker_len = self.inner.workers.len();
        if worker_len == 0 {
            return Err(SubmitError::WorkerUnavailable);
        }
        if let Some(affinity_key) = request.metadata.affinity_key.as_deref() {
            let mut hasher = DefaultHasher::new();
            affinity_key.hash(&mut hasher);
            let start = (hasher.finish() as usize) % worker_len;
            return self.first_live_worker_from(start);
        }
        self.first_live_worker_from(self.inner.next_worker.fetch_add(1, Ordering::SeqCst))
    }

    fn first_live_worker_from(&self, start: usize) -> Result<usize, SubmitError> {
        let worker_len = self.inner.worker_alive.len();
        let mut restart_error = None;
        for offset in 0..worker_len {
            let index = (start + offset) % worker_len;
            if self.inner.worker_alive[index].load(Ordering::SeqCst) {
                return Ok(index);
            }
            match self.restart_worker_if_allowed(index) {
                Ok(true) => return Ok(index),
                Ok(false) => {}
                Err(error) => restart_error = Some(error),
            }
        }
        Err(restart_error.unwrap_or(SubmitError::WorkerUnavailable))
    }

    fn restart_worker_if_allowed(&self, worker_index: usize) -> Result<bool, SubmitError> {
        if self.inner.shutting_down.load(Ordering::SeqCst)
            || self.inner.worker_alive[worker_index].load(Ordering::SeqCst)
        {
            return Ok(self.inner.worker_alive[worker_index].load(Ordering::SeqCst));
        }

        let slot = &self.inner.workers[worker_index];
        let mut process = slot.process.lock().expect("worker slot mutex poisoned");
        if slot.alive.load(Ordering::SeqCst) {
            return Ok(true);
        }
        if !restart_allowed(&self.inner.restart_policy, &slot.restart_state) {
            return Ok(false);
        }

        let previous = process.take();
        drop(previous);

        let restarted = spawn_worker::<Response>(
            &self.inner.command,
            worker_index,
            self.inner.response_sender.clone(),
            Arc::clone(&self.inner.pending),
            Arc::clone(&self.inner.in_flight),
            Arc::clone(&slot.alive),
            self.inner.limits.max_response_bytes,
        )?;
        *process = Some(restarted);
        Ok(true)
    }

    fn reserve_slot(&self) -> Result<(), SubmitError> {
        loop {
            let current = self.inner.in_flight.load(Ordering::SeqCst);
            if current >= self.inner.limits.max_queue_depth {
                return Err(SubmitError::QueueFull);
            }
            if self
                .inner
                .in_flight
                .compare_exchange(current, current + 1, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                return Ok(());
            }
        }
    }

    fn expire_timed_out_jobs(&self) {
        let now = current_time_millis();
        let expired = {
            let mut pending = self
                .inner
                .pending
                .lock()
                .expect("pending job table mutex poisoned");
            let expired_ids = pending
                .iter()
                .filter_map(|(id, job)| {
                    job.deadline_at_ms
                        .filter(|deadline| *deadline <= now)
                        .map(|_| id.clone())
                })
                .collect::<Vec<_>>();
            let mut expired = Vec::with_capacity(expired_ids.len());
            for id in expired_ids {
                if let Some(job) = pending.remove(&id) {
                    decrement_in_flight(&self.inner.in_flight);
                    expired.push((id, job.metadata));
                }
            }
            expired
        };

        for (id, metadata) in expired {
            let _ = self
                .inner
                .response_sender
                .send(timeout_response(id, metadata));
        }
    }
}

impl<Request, Response> JobExecutor<Request, Response> for StdioProcessPool<Request, Response>
where
    Request: Serialize + Send + 'static,
    Response: DeserializeOwned + Send + 'static,
{
    fn capabilities(&self) -> ExecutorCapabilities {
        ExecutorCapabilities {
            supports_parallel_execution: self.inner.workers.len() > 1,
            supports_parallel_callbacks: true,
            requires_vm_lock: false,
            supports_process_isolation: true,
            supports_cancellation: false,
            supports_timeouts: true,
            supports_affinity: true,
            supports_ordered_responses: false,
            requires_serializable_payloads: true,
            max_workers: self.inner.workers.len(),
            max_queue_depth: self.inner.limits.max_queue_depth,
            max_payload_bytes: self.inner.limits.max_payload_bytes,
        }
    }

    fn try_submit(&self, request: JobRequest<Request>) -> Result<(), SubmitError> {
        self.expire_timed_out_jobs();
        if self.inner.shutting_down.load(Ordering::SeqCst) {
            return Err(SubmitError::ShuttingDown);
        }

        let encoded = encode_request_json_line(&request)?;
        if encoded.len() > self.inner.limits.max_payload_bytes {
            return Err(SubmitError::PayloadTooLarge {
                actual: encoded.len(),
                max: self.inner.limits.max_payload_bytes,
            });
        }

        let worker_index = self.worker_index_for(&request)?;
        self.reserve_slot()?;
        let id = request.id.clone();
        let metadata = request.metadata.clone();
        let deadline_at_ms = metadata.deadline_at_ms.or_else(|| {
            self.inner
                .default_job_timeout
                .map(|timeout| current_time_millis().saturating_add(timeout.as_millis() as u64))
        });
        self.inner
            .pending
            .lock()
            .expect("pending job table mutex poisoned")
            .insert(
                id.clone(),
                PendingJob {
                    worker_index,
                    metadata,
                    deadline_at_ms,
                },
            );
        let write_result = {
            let process = self.inner.workers[worker_index]
                .process
                .lock()
                .expect("worker slot mutex poisoned");
            match process.as_ref() {
                Some(process) => {
                    let mut stdin = process
                        .stdin
                        .lock()
                        .expect("stdio worker stdin mutex poisoned");
                    stdin
                        .write_all(encoded.as_bytes())
                        .and_then(|_| stdin.flush())
                }
                None => Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "worker process is unavailable",
                )),
            }
        };

        if let Err(error) = write_result {
            self.inner
                .pending
                .lock()
                .expect("pending job table mutex poisoned")
                .remove(&id);
            decrement_in_flight(&self.inner.in_flight);
            self.inner.worker_alive[worker_index].store(false, Ordering::SeqCst);
            self.inner.workers[worker_index]
                .process
                .lock()
                .expect("worker slot mutex poisoned")
                .take();
            fail_pending_jobs_for_worker(
                &self.inner.pending,
                &self.inner.in_flight,
                worker_index,
                &self.inner.response_sender,
            );
            return Err(error.into());
        }

        Ok(())
    }

    fn cancel(&self, id: &str) -> CancelResult {
        if self.inner.shutting_down.load(Ordering::SeqCst) {
            return CancelResult::ShuttingDown;
        }
        if self
            .inner
            .pending
            .lock()
            .expect("pending job table mutex poisoned")
            .contains_key(id)
        {
            CancelResult::Unsupported
        } else {
            CancelResult::AlreadyCompleted
        }
    }

    fn drain_responses(&self, max: usize) -> Vec<JobResponse<Response>> {
        self.expire_timed_out_jobs();
        let receiver = self
            .inner
            .responses
            .lock()
            .expect("job response receiver mutex poisoned");
        let mut responses = Vec::new();
        for _ in 0..max {
            match receiver.try_recv() {
                Ok(response) => responses.push(response),
                Err(mpsc::TryRecvError::Empty | mpsc::TryRecvError::Disconnected) => break,
            }
        }
        responses
    }

    fn shutdown(&self) {
        self.inner.shutting_down.store(true, Ordering::SeqCst);
    }
}

fn spawn_worker<Response>(
    command: &StdioWorkerCommand,
    worker_index: usize,
    sender: mpsc::Sender<JobResponse<Response>>,
    pending: Arc<Mutex<HashMap<String, PendingJob>>>,
    in_flight: Arc<AtomicUsize>,
    alive: Arc<AtomicBool>,
    max_response_bytes: usize,
) -> io::Result<WorkerProcess>
where
    Response: DeserializeOwned + Send + 'static,
{
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
        .spawn()
        .map_err(|error| io::Error::new(error.kind(), format!("spawn worker process: {error}")))?;

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "worker did not expose stdin"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "worker did not expose stdout"))?;

    alive.store(true, Ordering::SeqCst);
    let child = Arc::new(Mutex::new(child));
    let reader = thread::spawn(move || {
        let mut stdout = BufReader::new(stdout);
        loop {
            let mut line = String::new();
            match stdout.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {}
                Err(_) => break,
            }

            match decode_response_json_line_with_limit::<Response>(&line, max_response_bytes) {
                Ok(response) => {
                    let metadata = remove_pending_job(&pending, &in_flight, &response.id);
                    if metadata.is_none() {
                        continue;
                    }
                    if sender.send(response).is_err() {
                        break;
                    }
                }
                Err(JobCodecError::OversizedFrame { .. }) => break,
                Err(_) if line.len() > max_response_bytes => break,
                Err(_) => {}
            }
        }
        alive.store(false, Ordering::SeqCst);
        fail_pending_jobs_for_worker(&pending, &in_flight, worker_index, &sender);
    });

    Ok(WorkerProcess {
        stdin: Arc::new(Mutex::new(stdin)),
        child,
        reader: Some(reader),
    })
}

fn restart_allowed(policy: &StdioWorkerRestartPolicy, restart_state: &Mutex<RestartState>) -> bool {
    match policy {
        StdioWorkerRestartPolicy::Never => false,
        StdioWorkerRestartPolicy::Always => true,
        StdioWorkerRestartPolicy::Bounded {
            max_restarts,
            window,
        } => {
            if *max_restarts == 0 {
                return false;
            }
            let now = current_time_millis();
            let window_ms = window.as_millis() as u64;
            let cutoff = now.saturating_sub(window_ms);
            let mut state = restart_state
                .lock()
                .expect("worker restart state mutex poisoned");
            while state
                .restart_timestamps_ms
                .front()
                .is_some_and(|timestamp| *timestamp <= cutoff)
            {
                state.restart_timestamps_ms.pop_front();
            }
            if state.restart_timestamps_ms.len() >= *max_restarts {
                return false;
            }
            state.restart_timestamps_ms.push_back(now);
            true
        }
    }
}

fn decrement_in_flight(in_flight: &AtomicUsize) {
    let _ = in_flight.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
        Some(current.saturating_sub(1))
    });
}

fn remove_pending_job(
    pending: &Mutex<HashMap<String, PendingJob>>,
    in_flight: &AtomicUsize,
    id: &str,
) -> Option<JobMetadata> {
    let job = pending
        .lock()
        .expect("pending job table mutex poisoned")
        .remove(id)?;
    decrement_in_flight(in_flight);
    Some(job.metadata)
}

fn fail_pending_jobs_for_worker<Response>(
    pending: &Mutex<HashMap<String, PendingJob>>,
    in_flight: &AtomicUsize,
    worker_index: usize,
    sender: &mpsc::Sender<JobResponse<Response>>,
) {
    let failed = {
        let mut pending = pending.lock().expect("pending job table mutex poisoned");
        let failed_ids = pending
            .iter()
            .filter_map(|(id, job)| (job.worker_index == worker_index).then(|| id.clone()))
            .collect::<Vec<_>>();
        let mut failed = Vec::with_capacity(failed_ids.len());
        for id in failed_ids {
            if let Some(job) = pending.remove(&id) {
                decrement_in_flight(in_flight);
                failed.push((id, job.metadata));
            }
        }
        failed
    };

    for (id, metadata) in failed {
        if sender
            .send(worker_unavailable_response(id, metadata))
            .is_err()
        {
            break;
        }
    }
}

fn current_time_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn cancelled_response<Response>(id: String, metadata: JobMetadata) -> JobResponse<Response> {
    JobResponse {
        id,
        result: JobResult::Cancelled {
            cancellation: JobCancellation {
                message: "job was cancelled by the executor".to_string(),
            },
        },
        metadata,
    }
}

fn timeout_response<Response>(id: String, metadata: JobMetadata) -> JobResponse<Response> {
    JobResponse {
        id,
        result: JobResult::TimedOut {
            timeout: JobTimeout {
                message: "job exceeded its executor deadline".to_string(),
            },
        },
        metadata,
    }
}

fn worker_unavailable_response<Response>(
    id: String,
    metadata: JobMetadata,
) -> JobResponse<Response> {
    JobResponse::error(
        id,
        JobError::new(
            "worker_unavailable",
            "worker exited before completing the job",
            JobErrorOrigin::Executor,
        ),
    )
    .with_metadata(metadata)
}

fn panic_job_result<Response>(message: String) -> JobResult<Response> {
    JobResult::Error {
        error: JobError::new("worker_panic", message, JobErrorOrigin::PanicOrException),
    }
}

fn panic_message(panic: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = panic.downcast_ref::<&str>() {
        format!("job handler panicked: {message}")
    } else if let Some(message) = panic.downcast_ref::<String>() {
        format!("job handler panicked: {message}")
    } else {
        "job handler panicked".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use generic_job_protocol::{JobMetadata, JobResult};
    use serde::{Deserialize, Serialize};
    use std::process::Command;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct EchoJob {
        stream_id: String,
        text: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct EchoResponse {
        stream_id: String,
        counter: u64,
        text: String,
    }

    fn python_candidates() -> Vec<String> {
        let mut candidates = Vec::new();
        if let Ok(value) = std::env::var("PYTHON") {
            candidates.push(value);
        }
        if cfg!(windows) {
            candidates.push("python".to_string());
        } else {
            candidates.push("python3".to_string());
            candidates.push("python".to_string());
        }
        candidates.dedup();
        candidates
    }

    fn python_interpreter() -> Option<String> {
        python_candidates().into_iter().find(|candidate| {
            Command::new(candidate)
                .arg("-c")
                .arg("import json, sys; raise SystemExit(0)")
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false)
        })
    }

    fn scripted_worker(script: &str) -> Option<StdioWorkerCommand> {
        python_interpreter().map(|program| {
            StdioWorkerCommand::new(program, vec!["-c".to_string(), script.to_string()])
        })
    }

    fn echo_worker_script(extra: &str) -> String {
        format!(
            r#"
import json, sys, time
counts = {{}}
{extra}
for line in sys.stdin:
    frame = json.loads(line)
    body = frame["body"]
    payload = body["payload"]
    stream_id = payload["stream_id"]
    counts[stream_id] = counts.get(stream_id, 0) + 1
    out = {{"stream_id": stream_id, "counter": counts[stream_id], "text": payload["text"]}}
    print(json.dumps({{"version": 1, "kind": "response", "body": {{"id": body["id"], "result": {{"status": "ok", "payload": out}}, "metadata": body["metadata"]}}}}), flush=True)
"#
        )
    }

    #[test]
    fn thread_pool_runs_jobs_without_transport_knowledge() {
        let executions = Arc::new(AtomicUsize::new(0));
        let seen = Arc::clone(&executions);
        let pool = RustThreadPool::spawn(
            RustThreadPoolOptions {
                worker_count: 2,
                limits: ExecutorLimits {
                    max_queue_depth: 4,
                    ..ExecutorLimits::default()
                },
                default_job_timeout: None,
            },
            move |request: JobRequest<EchoJob>| {
                let counter = seen.fetch_add(1, Ordering::SeqCst) as u64 + 1;
                JobResult::Ok {
                    payload: EchoResponse {
                        stream_id: request.payload.stream_id,
                        counter,
                        text: request.payload.text,
                    },
                }
            },
        );

        pool.try_submit(JobRequest::new(
            "job-1",
            EchoJob {
                stream_id: "stream".to_string(),
                text: "hello".to_string(),
            },
        ))
        .expect("submit Rust job");

        let response = wait_for_thread_response(&pool).expect("thread response");
        match response.result {
            JobResult::Ok { payload } => {
                assert_eq!(payload.stream_id, "stream");
                assert_eq!(payload.text, "hello");
                assert_eq!(executions.load(Ordering::SeqCst), 1);
            }
            other => panic!("expected thread-pool success, got {other:?}"),
        }
    }

    #[test]
    fn thread_pool_rejects_jobs_when_in_flight_limit_is_full() {
        let entered = Arc::new(AtomicUsize::new(0));
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        let handler_entered = Arc::clone(&entered);
        let handler_gate = Arc::clone(&gate);
        let pool = RustThreadPool::spawn(
            RustThreadPoolOptions {
                worker_count: 1,
                limits: ExecutorLimits {
                    max_queue_depth: 1,
                    ..ExecutorLimits::default()
                },
                default_job_timeout: None,
            },
            move |request: JobRequest<EchoJob>| {
                handler_entered.fetch_add(1, Ordering::SeqCst);
                wait_for_gate(&handler_gate);
                JobResult::Ok {
                    payload: EchoResponse {
                        stream_id: request.payload.stream_id,
                        counter: 1,
                        text: request.payload.text,
                    },
                }
            },
        );

        pool.try_submit(JobRequest::new(
            "job-1",
            EchoJob {
                stream_id: "stream".to_string(),
                text: "one".to_string(),
            },
        ))
        .expect("first job is accepted");
        wait_until(|| entered.load(Ordering::SeqCst) == 1);

        let err = pool
            .try_submit(JobRequest::new(
                "job-2",
                EchoJob {
                    stream_id: "stream".to_string(),
                    text: "two".to_string(),
                },
            ))
            .expect_err("second job should hit backpressure");
        assert_eq!(err, SubmitError::QueueFull);
        open_gate(&gate);
    }

    #[test]
    fn thread_pool_cancels_queued_job_and_releases_capacity() {
        let entered = Arc::new(AtomicUsize::new(0));
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        let handler_entered = Arc::clone(&entered);
        let handler_gate = Arc::clone(&gate);
        let pool = RustThreadPool::spawn(
            RustThreadPoolOptions {
                worker_count: 1,
                limits: ExecutorLimits {
                    max_queue_depth: 2,
                    ..ExecutorLimits::default()
                },
                default_job_timeout: None,
            },
            move |request: JobRequest<EchoJob>| {
                if request.payload.text == "block" {
                    handler_entered.fetch_add(1, Ordering::SeqCst);
                    wait_for_gate(&handler_gate);
                }
                JobResult::Ok {
                    payload: EchoResponse {
                        stream_id: request.payload.stream_id,
                        counter: 1,
                        text: request.payload.text,
                    },
                }
            },
        );

        pool.try_submit(JobRequest::new(
            "job-1",
            EchoJob {
                stream_id: "stream".to_string(),
                text: "block".to_string(),
            },
        ))
        .expect("submit running job");
        wait_until(|| entered.load(Ordering::SeqCst) == 1);
        pool.try_submit(JobRequest::new(
            "job-2",
            EchoJob {
                stream_id: "stream".to_string(),
                text: "queued".to_string(),
            },
        ))
        .expect("submit queued job");

        assert_eq!(pool.cancel("job-2"), CancelResult::Cancelled);
        let response = wait_for_thread_response(&pool).expect("queued cancellation response");
        assert!(matches!(response.result, JobResult::Cancelled { .. }));

        pool.try_submit(JobRequest::new(
            "job-3",
            EchoJob {
                stream_id: "stream".to_string(),
                text: "after-cancel".to_string(),
            },
        ))
        .expect("queued cancellation should release capacity");
        open_gate(&gate);
    }

    #[test]
    fn thread_pool_cancels_running_job_when_handler_returns() {
        let entered = Arc::new(AtomicUsize::new(0));
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        let handler_entered = Arc::clone(&entered);
        let handler_gate = Arc::clone(&gate);
        let pool = RustThreadPool::spawn(
            RustThreadPoolOptions {
                worker_count: 1,
                limits: ExecutorLimits {
                    max_queue_depth: 1,
                    ..ExecutorLimits::default()
                },
                default_job_timeout: None,
            },
            move |request: JobRequest<EchoJob>| {
                handler_entered.fetch_add(1, Ordering::SeqCst);
                wait_for_gate(&handler_gate);
                JobResult::Ok {
                    payload: EchoResponse {
                        stream_id: request.payload.stream_id,
                        counter: 1,
                        text: "stale-success".to_string(),
                    },
                }
            },
        );

        pool.try_submit(JobRequest::new(
            "job-1",
            EchoJob {
                stream_id: "stream".to_string(),
                text: "running".to_string(),
            },
        ))
        .expect("submit running job");
        wait_until(|| entered.load(Ordering::SeqCst) == 1);

        assert_eq!(pool.cancel("job-1"), CancelResult::Cancelled);
        assert!(
            pool.recv_response_timeout(Duration::from_millis(25))
                .expect("check for immediate response")
                .is_none(),
            "running cancellation should not claim capacity until the worker returns"
        );

        open_gate(&gate);
        let response = wait_for_thread_response(&pool).expect("running cancellation response");
        assert!(matches!(response.result, JobResult::Cancelled { .. }));
    }

    #[test]
    fn thread_pool_converts_panics_to_job_errors() {
        let pool = RustThreadPool::spawn(
            RustThreadPoolOptions {
                worker_count: 1,
                limits: ExecutorLimits::default(),
                default_job_timeout: None,
            },
            |_request: JobRequest<EchoJob>| -> JobResult<EchoResponse> {
                panic!("boom");
            },
        );

        pool.try_submit(JobRequest::new(
            "job-1",
            EchoJob {
                stream_id: "stream".to_string(),
                text: "panic".to_string(),
            },
        ))
        .expect("submit panicking job");

        let response = wait_for_thread_response(&pool).expect("panic response");
        match response.result {
            JobResult::Error { error } => {
                assert_eq!(error.code, "worker_panic");
                assert_eq!(error.origin, JobErrorOrigin::PanicOrException);
                assert!(error.message.contains("boom"));
            }
            other => panic!("expected panic error, got {other:?}"),
        }
    }

    #[test]
    fn thread_pool_times_out_running_job_when_handler_returns() {
        let entered = Arc::new(AtomicUsize::new(0));
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        let handler_entered = Arc::clone(&entered);
        let handler_gate = Arc::clone(&gate);
        let pool = RustThreadPool::spawn(
            RustThreadPoolOptions {
                worker_count: 1,
                limits: ExecutorLimits {
                    max_queue_depth: 1,
                    ..ExecutorLimits::default()
                },
                default_job_timeout: Some(Duration::from_millis(25)),
            },
            move |request: JobRequest<EchoJob>| {
                handler_entered.fetch_add(1, Ordering::SeqCst);
                wait_for_gate(&handler_gate);
                JobResult::Ok {
                    payload: EchoResponse {
                        stream_id: request.payload.stream_id,
                        counter: 1,
                        text: "stale-success".to_string(),
                    },
                }
            },
        );

        pool.try_submit(JobRequest::new(
            "job-1",
            EchoJob {
                stream_id: "stream".to_string(),
                text: "running".to_string(),
            },
        ))
        .expect("submit running job");
        wait_until(|| entered.load(Ordering::SeqCst) == 1);
        thread::sleep(Duration::from_millis(50));
        assert!(
            pool.recv_response_timeout(Duration::from_millis(25))
                .expect("check for timeout response")
                .is_none(),
            "running timeout should not claim capacity until the worker returns"
        );

        open_gate(&gate);
        let response = wait_for_thread_response(&pool).expect("timeout response");
        assert!(matches!(response.result, JobResult::TimedOut { .. }));
    }

    #[test]
    fn process_pool_routes_same_affinity_to_same_worker() {
        let Some(command) = scripted_worker(&echo_worker_script("")) else {
            eprintln!("skipping test because no Python interpreter was found");
            return;
        };
        let pool = StdioProcessPool::<EchoJob, EchoResponse>::spawn(
            command,
            StdioProcessPoolOptions {
                worker_count: 2,
                limits: ExecutorLimits {
                    max_queue_depth: 8,
                    ..ExecutorLimits::default()
                },
                default_job_timeout: None,
                restart_policy: StdioWorkerRestartPolicy::Never,
            },
        )
        .expect("spawn process pool");

        for index in 0..2 {
            pool.try_submit(
                JobRequest::new(
                    format!("job-{index}"),
                    EchoJob {
                        stream_id: "stream-7".to_string(),
                        text: format!("message-{index}"),
                    },
                )
                .with_metadata(JobMetadata::default().with_affinity_key("stream-7")),
            )
            .expect("submit job");
        }

        let mut responses = Vec::new();
        for _ in 0..100 {
            responses.extend(pool.drain_responses(8));
            if responses.len() == 2 {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }

        assert_eq!(responses.len(), 2);
        let counters = responses
            .into_iter()
            .map(|response| match response.result {
                JobResult::Ok { payload } => payload.counter,
                other => panic!("unexpected response: {other:?}"),
            })
            .collect::<Vec<_>>();
        assert_eq!(counters, vec![1, 2]);
    }

    #[test]
    fn process_pool_rejects_jobs_when_in_flight_limit_is_full() {
        let Some(command) = scripted_worker(&echo_worker_script("time.sleep(0.2)")) else {
            eprintln!("skipping test because no Python interpreter was found");
            return;
        };
        let pool = StdioProcessPool::<EchoJob, EchoResponse>::spawn(
            command,
            StdioProcessPoolOptions {
                worker_count: 1,
                limits: ExecutorLimits {
                    max_queue_depth: 1,
                    ..ExecutorLimits::default()
                },
                default_job_timeout: None,
                restart_policy: StdioWorkerRestartPolicy::Never,
            },
        )
        .expect("spawn process pool");

        pool.try_submit(JobRequest::new(
            "job-1",
            EchoJob {
                stream_id: "stream".to_string(),
                text: "one".to_string(),
            },
        ))
        .expect("first job is accepted");

        let err = pool
            .try_submit(JobRequest::new(
                "job-2",
                EchoJob {
                    stream_id: "stream".to_string(),
                    text: "two".to_string(),
                },
            ))
            .expect_err("second job should hit backpressure");
        assert_eq!(err, SubmitError::QueueFull);
    }

    #[test]
    fn process_pool_times_out_jobs_and_releases_capacity() {
        let Some(command) = scripted_worker(
            r#"
import json, sys, time
for line in sys.stdin:
    frame = json.loads(line)
    body = frame["body"]
    time.sleep(0.4)
    out = {"stream_id": "late", "counter": 1, "text": "late"}
    print(json.dumps({"version": 1, "kind": "response", "body": {"id": body["id"], "result": {"status": "ok", "payload": out}, "metadata": body["metadata"]}}), flush=True)
"#,
        ) else {
            eprintln!("skipping test because no Python interpreter was found");
            return;
        };
        let pool = StdioProcessPool::<EchoJob, EchoResponse>::spawn(
            command,
            StdioProcessPoolOptions {
                worker_count: 1,
                limits: ExecutorLimits {
                    max_queue_depth: 1,
                    ..ExecutorLimits::default()
                },
                default_job_timeout: Some(Duration::from_millis(50)),
                restart_policy: StdioWorkerRestartPolicy::Never,
            },
        )
        .expect("spawn process pool");

        pool.try_submit(JobRequest::new(
            "job-1",
            EchoJob {
                stream_id: "stream".to_string(),
                text: "one".to_string(),
            },
        ))
        .expect("submit slow job");

        let response = wait_for_response(&pool).expect("timeout response");
        match response.result {
            JobResult::TimedOut { timeout } => {
                assert!(timeout.message.contains("deadline"));
            }
            other => panic!("expected timed out response, got {other:?}"),
        }

        pool.try_submit(JobRequest::new(
            "job-2",
            EchoJob {
                stream_id: "stream".to_string(),
                text: "two".to_string(),
            },
        ))
        .expect("timeout should release in-flight capacity");
    }

    #[test]
    fn process_pool_reports_worker_exit_and_stops_routing_to_dead_worker() {
        let Some(command) = scripted_worker(
            r#"
import sys
sys.stdin.readline()
"#,
        ) else {
            eprintln!("skipping test because no Python interpreter was found");
            return;
        };
        let pool = StdioProcessPool::<EchoJob, EchoResponse>::spawn(
            command,
            StdioProcessPoolOptions {
                worker_count: 1,
                limits: ExecutorLimits {
                    max_queue_depth: 1,
                    ..ExecutorLimits::default()
                },
                default_job_timeout: None,
                restart_policy: StdioWorkerRestartPolicy::Never,
            },
        )
        .expect("spawn process pool");

        pool.try_submit(JobRequest::new(
            "job-1",
            EchoJob {
                stream_id: "stream".to_string(),
                text: "one".to_string(),
            },
        ))
        .expect("submit job to exiting worker");

        let response = wait_for_response(&pool).expect("worker failure response");
        match response.result {
            JobResult::Error { error } => {
                assert_eq!(error.code, "worker_unavailable");
                assert_eq!(error.origin, JobErrorOrigin::Executor);
            }
            other => panic!("expected worker error response, got {other:?}"),
        }

        let err = pool
            .try_submit(JobRequest::new(
                "job-2",
                EchoJob {
                    stream_id: "stream".to_string(),
                    text: "two".to_string(),
                },
            ))
            .expect_err("dead worker should be unavailable");
        assert_eq!(err, SubmitError::WorkerUnavailable);
    }

    #[test]
    fn process_pool_restarts_dead_worker_when_policy_allows() {
        let marker = std::env::temp_dir().join(format!(
            "generic-job-runtime-restart-once-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
        ));
        let marker_literal = format!("{:?}", marker.to_string_lossy());
        let script = format!(
            r#"
import json, os, sys
marker_path = {marker_literal}

for line in sys.stdin:
    frame = json.loads(line)
    body = frame["body"]
    payload = body["payload"]
    if not os.path.exists(marker_path):
        open(marker_path, "w").close()
        sys.exit(0)
    out = {{"stream_id": payload["stream_id"], "counter": 1, "text": payload["text"]}}
    print(json.dumps({{"version": 1, "kind": "response", "body": {{"id": body["id"], "result": {{"status": "ok", "payload": out}}, "metadata": body["metadata"]}}}}), flush=True)
"#
        );
        let Some(command) = scripted_worker(&script) else {
            eprintln!("skipping test because no Python interpreter was found");
            return;
        };
        let pool = StdioProcessPool::<EchoJob, EchoResponse>::spawn(
            command,
            StdioProcessPoolOptions {
                worker_count: 1,
                limits: ExecutorLimits {
                    max_queue_depth: 1,
                    ..ExecutorLimits::default()
                },
                default_job_timeout: None,
                restart_policy: StdioWorkerRestartPolicy::Always,
            },
        )
        .expect("spawn process pool");

        pool.try_submit(JobRequest::new(
            "job-1",
            EchoJob {
                stream_id: "stream".to_string(),
                text: "one".to_string(),
            },
        ))
        .expect("submit job to worker that exits");

        let response = wait_for_response(&pool).expect("worker failure response");
        match response.result {
            JobResult::Error { error } => assert_eq!(error.code, "worker_unavailable"),
            other => panic!("expected worker failure before restart, got {other:?}"),
        }

        pool.try_submit(JobRequest::new(
            "job-2",
            EchoJob {
                stream_id: "stream".to_string(),
                text: "two".to_string(),
            },
        ))
        .expect("submit job to restarted worker");

        let response = wait_for_response(&pool).expect("restarted worker response");
        match response.result {
            JobResult::Ok { payload } => {
                assert_eq!(payload.stream_id, "stream");
                assert_eq!(payload.text, "two");
            }
            other => panic!("expected restarted worker success, got {other:?}"),
        }

        let _ = std::fs::remove_file(marker);
    }

    #[test]
    fn process_pool_stops_restarting_after_bounded_policy_is_exhausted() {
        let Some(command) = scripted_worker(
            r#"
import sys
for line in sys.stdin:
    sys.exit(0)
"#,
        ) else {
            eprintln!("skipping test because no Python interpreter was found");
            return;
        };
        let pool = StdioProcessPool::<EchoJob, EchoResponse>::spawn(
            command,
            StdioProcessPoolOptions {
                worker_count: 1,
                limits: ExecutorLimits {
                    max_queue_depth: 1,
                    ..ExecutorLimits::default()
                },
                default_job_timeout: None,
                restart_policy: StdioWorkerRestartPolicy::Bounded {
                    max_restarts: 1,
                    window: Duration::from_secs(60),
                },
            },
        )
        .expect("spawn process pool");

        for job_id in ["job-1", "job-2"] {
            pool.try_submit(JobRequest::new(
                job_id,
                EchoJob {
                    stream_id: "stream".to_string(),
                    text: job_id.to_string(),
                },
            ))
            .expect("submit job before restart budget is exhausted");

            let response = wait_for_response(&pool).expect("worker failure response");
            match response.result {
                JobResult::Error { error } => assert_eq!(error.code, "worker_unavailable"),
                other => panic!("expected worker failure response, got {other:?}"),
            }
        }

        let err = pool
            .try_submit(JobRequest::new(
                "job-3",
                EchoJob {
                    stream_id: "stream".to_string(),
                    text: "three".to_string(),
                },
            ))
            .expect_err("restart budget should be exhausted");
        assert_eq!(err, SubmitError::WorkerUnavailable);
    }

    fn wait_for_response(
        pool: &StdioProcessPool<EchoJob, EchoResponse>,
    ) -> Option<JobResponse<EchoResponse>> {
        for _ in 0..100 {
            if let Some(response) = pool.drain_responses(1).into_iter().next() {
                return Some(response);
            }
            thread::sleep(Duration::from_millis(10));
        }
        None
    }

    fn wait_for_thread_response(
        pool: &RustThreadPool<EchoJob, EchoResponse>,
    ) -> Option<JobResponse<EchoResponse>> {
        for _ in 0..100 {
            if let Some(response) = pool.drain_responses(1).into_iter().next() {
                return Some(response);
            }
            thread::sleep(Duration::from_millis(10));
        }
        None
    }

    fn wait_until(mut predicate: impl FnMut() -> bool) {
        for _ in 0..100 {
            if predicate() {
                return;
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!("condition was not met before timeout");
    }

    fn wait_for_gate(gate: &Arc<(Mutex<bool>, Condvar)>) {
        let (lock, cvar) = &**gate;
        let mut open = lock.lock().expect("gate mutex poisoned");
        while !*open {
            open = cvar.wait(open).expect("gate mutex poisoned");
        }
    }

    fn open_gate(gate: &Arc<(Mutex<bool>, Condvar)>) {
        let (lock, cvar) = &**gate;
        *lock.lock().expect("gate mutex poisoned") = true;
        cvar.notify_all();
    }
}
