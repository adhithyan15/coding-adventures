//! # generic-job-runtime
//!
//! Bounded executors for `generic-job-protocol` jobs.
//!
//! The runtime is deliberately transport-neutral. TCP, UI event handling,
//! parsing, image work, and future FFI bridges can all submit typed
//! `JobRequest<T>` values and drain typed `JobResponse<U>` values without the
//! executor knowing what the payload means.

use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::io::{self, BufRead, BufReader, Write};
use std::marker::PhantomData;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

use generic_job_protocol::{
    decode_response_json_line_with_limit, encode_request_json_line, JobCodecError, JobRequest,
    JobResponse,
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
}

impl Default for StdioProcessPoolOptions {
    fn default() -> Self {
        Self {
            worker_count: 1,
            limits: ExecutorLimits::default(),
        }
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
    fn drain_responses(&self, max: usize) -> Vec<JobResponse<Response>>;
    fn shutdown(&self);
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
    workers: Vec<WorkerProcess>,
    responses: Mutex<mpsc::Receiver<JobResponse<Response>>>,
    next_worker: AtomicUsize,
    in_flight: Arc<AtomicUsize>,
    shutting_down: AtomicBool,
    limits: ExecutorLimits,
}

struct WorkerProcess {
    stdin: Arc<Mutex<ChildStdin>>,
    child: Arc<Mutex<Child>>,
    reader: Option<thread::JoinHandle<()>>,
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
        let in_flight = Arc::new(AtomicUsize::new(0));

        for _ in 0..worker_count {
            workers.push(spawn_worker::<Response>(
                &command,
                sender.clone(),
                Arc::clone(&in_flight),
                options.limits.max_response_bytes,
            )?);
        }

        Ok(Self {
            inner: Arc::new(PoolInner {
                workers,
                responses: Mutex::new(receiver),
                next_worker: AtomicUsize::new(0),
                in_flight,
                shutting_down: AtomicBool::new(false),
                limits: options.limits,
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
            return Ok((hasher.finish() as usize) % worker_len);
        }
        Ok(self.inner.next_worker.fetch_add(1, Ordering::SeqCst) % worker_len)
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
            supports_timeouts: false,
            supports_affinity: true,
            supports_ordered_responses: false,
            requires_serializable_payloads: true,
            max_workers: self.inner.workers.len(),
            max_queue_depth: self.inner.limits.max_queue_depth,
            max_payload_bytes: self.inner.limits.max_payload_bytes,
        }
    }

    fn try_submit(&self, request: JobRequest<Request>) -> Result<(), SubmitError> {
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

        self.reserve_slot()?;
        let worker_index = self.worker_index_for(&request)?;
        let write_result = {
            let mut stdin = self.inner.workers[worker_index]
                .stdin
                .lock()
                .expect("stdio worker stdin mutex poisoned");
            stdin
                .write_all(encoded.as_bytes())
                .and_then(|_| stdin.flush())
        };

        if let Err(error) = write_result {
            self.inner.in_flight.fetch_sub(1, Ordering::SeqCst);
            return Err(error.into());
        }

        Ok(())
    }

    fn drain_responses(&self, max: usize) -> Vec<JobResponse<Response>> {
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
    sender: mpsc::Sender<JobResponse<Response>>,
    in_flight: Arc<AtomicUsize>,
    max_response_bytes: usize,
) -> io::Result<WorkerProcess>
where
    Response: DeserializeOwned + Send + 'static,
{
    let mut child = Command::new(&command.program)
        .args(&command.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "worker did not expose stdin"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "worker did not expose stdout"))?;

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
                    decrement_in_flight(&in_flight);
                    if sender.send(response).is_err() {
                        break;
                    }
                }
                Err(JobCodecError::OversizedFrame { .. }) => break,
                Err(_) if line.len() > max_response_bytes => break,
                Err(_) => {}
            }
        }
    });

    Ok(WorkerProcess {
        stdin: Arc::new(Mutex::new(stdin)),
        child,
        reader: Some(reader),
    })
}

fn decrement_in_flight(in_flight: &AtomicUsize) {
    let _ = in_flight.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
        Some(current.saturating_sub(1))
    });
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
}
