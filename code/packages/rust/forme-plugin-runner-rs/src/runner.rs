use crate::context::ContextStreams;
use crate::peer::{object, safe_id, Peer, RpcFault};
use crate::stage::{FromWire, Stage, StageInput, StageOutput, StreamItem, StreamTerminal};
use crate::{
    CancellationToken, FrameDecoder, InputStream, ProtocolError, StageContext, StageError,
    WireValue,
};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::sync::{mpsc, oneshot, Mutex, Semaphore};
use tokio::time::{timeout, Duration};

pub const PROTOCOL_VERSION: i64 = 1;
pub const RUNNER_NAME: &str = "forme-plugin-runner-rs";
pub const RUNNER_VERSION: &str = "0.1.0";

#[derive(Clone, Debug)]
pub struct RunnerOptions {
    pub stage_id: String,
    pub config_schema_hash: Option<String>,
    pub max_frame_bytes: usize,
    pub max_header_bytes: usize,
    pub max_inflight_requests: usize,
    pub max_pending_requests: usize,
    pub max_pending_writes: usize,
    pub max_buffered_stream_values: usize,
    pub max_buffered_stream_bytes: usize,
}

impl RunnerOptions {
    pub fn from_args() -> Self {
        let mut args = std::env::args().skip(1);
        Self {
            stage_id: args.next().unwrap_or_else(|| "stage".into()),
            config_schema_hash: args.next(),
            ..Self::default()
        }
    }

    fn validate(&self) -> Result<(), ProtocolError> {
        let bounds = [
            self.max_frame_bytes,
            self.max_header_bytes,
            self.max_inflight_requests,
            self.max_pending_requests,
            self.max_pending_writes,
            self.max_buffered_stream_values,
            self.max_buffered_stream_bytes,
        ];
        if bounds.into_iter().any(|value| value == 0) {
            return Err(ProtocolError::ResourceLimit(
                "runner bounds must be positive".into(),
            ));
        }
        if [
            self.max_inflight_requests,
            self.max_pending_requests,
            self.max_pending_writes,
            self.max_buffered_stream_values,
            self.max_buffered_stream_bytes,
        ]
        .into_iter()
        .any(|value| value > Semaphore::MAX_PERMITS)
        {
            return Err(ProtocolError::ResourceLimit(
                "runner concurrency bounds exceed the runtime limit".into(),
            ));
        }
        self.max_header_bytes
            .checked_add(4)
            .and_then(|value| value.checked_add(self.max_frame_bytes))
            .ok_or_else(|| {
                ProtocolError::ResourceLimit("runner wire bounds exceed platform capacity".into())
            })?;
        Ok(())
    }
}

impl Default for RunnerOptions {
    fn default() -> Self {
        Self {
            stage_id: "stage".into(),
            config_schema_hash: None,
            max_frame_bytes: 4096,
            max_header_bytes: 512,
            max_inflight_requests: 32,
            max_pending_requests: 64,
            max_pending_writes: 64,
            max_buffered_stream_values: 4,
            max_buffered_stream_bytes: 1024,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Spawned,
    Handshaken,
    Announced,
    Initialized,
    Failed,
    Disposed,
}

struct Active {
    id: i64,
    cancellation: CancellationToken,
}

pub(crate) struct InputSender {
    pub(crate) sender: mpsc::Sender<StreamItem>,
    pub(crate) bytes: Arc<Semaphore>,
    pub(crate) max_bytes: usize,
    pub(crate) terminal: Arc<StreamTerminal>,
}

impl Clone for InputSender {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            bytes: self.bytes.clone(),
            max_bytes: self.max_bytes,
            terminal: self.terminal.clone(),
        }
    }
}

pub(crate) type InputSenders = HashMap<i64, InputSender>;

struct Runtime<S: Stage> {
    stage: Arc<S>,
    metadata: crate::StageMetadata,
    options: RunnerOptions,
    peer: Arc<Peer>,
    phase: Mutex<Phase>,
    control: Mutex<()>,
    active: Mutex<Option<Active>>,
    active_done: tokio::sync::Notify,
    inputs: Arc<Mutex<InputSenders>>,
    capability_inputs: Arc<Mutex<HashSet<i64>>>,
    last_config: Mutex<WireValue>,
}

struct Outcome {
    value: WireValue,
    stop: bool,
}

struct DispatchReady(Option<oneshot::Sender<()>>);

impl DispatchReady {
    fn signal(&mut self) {
        if let Some(sender) = self.0.take() {
            let _ = sender.send(());
        }
    }
}

impl Drop for DispatchReady {
    fn drop(&mut self) {
        self.signal();
    }
}

impl<S: Stage> Runtime<S> {
    fn context_streams(&self) -> ContextStreams {
        ContextStreams {
            inputs: self.inputs.clone(),
            capability_inputs: self.capability_inputs.clone(),
            max_buffered_values: self.options.max_buffered_stream_values,
            max_buffered_bytes: self.options.max_buffered_stream_bytes,
        }
    }

    async fn request(
        self: &Arc<Self>,
        id: i64,
        method: &str,
        raw_params: WireValue,
        ready: Option<oneshot::Sender<()>>,
    ) -> Result<Outcome, RpcFault> {
        let params = params(raw_params, method)?;
        match method {
            "handshake" => self.handshake(params).await,
            "announce" => self.announce().await,
            "stage.init" => self.init(params).await,
            "stage.run" => self.run(id, params, ready).await,
            "stage.dispose" => self.dispose().await,
            _ => Err(RpcFault::new(
                -32601,
                "METHOD_NOT_FOUND",
                Some(object([("method", method.into())])),
            )),
        }
    }

    async fn notification(&self, method: &str, raw_params: WireValue) -> Result<(), ProtocolError> {
        let params = raw_params.into_object()?;
        if method == "$/cancelRequest" {
            let id = safe_id(params.get("id"), "cancel request id")?;
            let cancellation = self
                .active
                .lock()
                .await
                .as_ref()
                .filter(|active| active.id == id)
                .map(|active| active.cancellation.clone());
            if let Some(cancellation) = cancellation {
                let reason = params
                    .get("reason")
                    .and_then(WireValue::as_str)
                    .unwrap_or("operation cancelled");
                cancellation.cancel(reason);
                let error = StageError::new("CANCELLED", reason);
                let mut inputs = self.inputs.lock().await;
                for sender in inputs.values() {
                    sender.terminal.fail(error.clone());
                }
                inputs.clear();
                drop(inputs);
                self.capability_inputs.lock().await.clear();
            }
            return Ok(());
        }
        let stream_id = safe_id(params.get("streamId"), "streamId")?;
        let sender = self
            .inputs
            .lock()
            .await
            .get(&stream_id)
            .cloned()
            .ok_or_else(|| {
                ProtocolError::InvalidMessage("notification targets an unknown stream".into())
            })?;
        match method {
            "stream.value" => {
                let value = params.get("value").cloned().unwrap_or(WireValue::Null);
                let size = value.encoded_size(sender.max_bytes)?;
                let permits = u32::try_from(size.max(1)).map_err(|_| {
                    ProtocolError::ResourceLimit("stream value exceeds configured bound".into())
                })?;
                let byte_permit = sender
                    .bytes
                    .clone()
                    .try_acquire_many_owned(permits)
                    .map_err(|error| match error {
                        tokio::sync::TryAcquireError::NoPermits => ProtocolError::ResourceLimit(
                            "stream buffer exceeds configured byte bound".into(),
                        ),
                        tokio::sync::TryAcquireError::Closed => {
                            ProtocolError::Closed("stream is already complete".into())
                        }
                    })?;
                sender
                    .sender
                    .try_send(StreamItem {
                        value: Ok(value),
                        _bytes: Some(byte_permit),
                    })
                    .map_err(|error| match error {
                        mpsc::error::TrySendError::Full(_) => ProtocolError::ResourceLimit(
                            "stream buffer exceeds configured value bound".into(),
                        ),
                        mpsc::error::TrySendError::Closed(_) => {
                            ProtocolError::Closed("stream is already complete".into())
                        }
                    })
            }
            "stream.error" => {
                sender.terminal.fail(StageError::new(
                    "UPSTREAM_STREAM_ERROR",
                    "host input stream failed",
                ));
                self.inputs.lock().await.remove(&stream_id);
                Ok(())
            }
            "stream.end" => {
                self.inputs.lock().await.remove(&stream_id);
                Ok(())
            }
            _ => Err(ProtocolError::InvalidMessage(
                "unknown runner notification".into(),
            )),
        }
    }

    async fn handshake(&self, params: BTreeMap<String, WireValue>) -> Result<Outcome, RpcFault> {
        let mut phase = self.phase.lock().await;
        if *phase != Phase::Spawned {
            return Err(phase_fault(*phase));
        }
        let expected = [
            ("pluginName", self.metadata.name.clone().into()),
            ("pluginVersion", self.metadata.version.clone().into()),
            ("apiVersion", (self.metadata.api_version as i64).into()),
            ("protocolVersion", PROTOCOL_VERSION.into()),
        ];
        for (key, value) in &expected {
            if params.get(*key) != Some(value) {
                return Err(RpcFault::new(
                    -32006,
                    "MANIFEST_MISMATCH",
                    Some(object([("field", (*key).into())])),
                ));
            }
        }
        *phase = Phase::Handshaken;
        Ok(Outcome {
            value: WireValue::Object(BTreeMap::from([
                ("pluginName".into(), self.metadata.name.clone().into()),
                ("pluginVersion".into(), self.metadata.version.clone().into()),
                (
                    "apiVersion".into(),
                    (self.metadata.api_version as i64).into(),
                ),
                ("protocolVersion".into(), PROTOCOL_VERSION.into()),
                ("runner".into(), RUNNER_NAME.into()),
                ("runnerVersion".into(), RUNNER_VERSION.into()),
            ])),
            stop: false,
        })
    }

    async fn announce(&self) -> Result<Outcome, RpcFault> {
        let mut phase = self.phase.lock().await;
        if *phase != Phase::Handshaken {
            return Err(phase_fault(*phase));
        }
        *phase = Phase::Announced;
        let capabilities = self
            .metadata
            .capabilities
            .iter()
            .cloned()
            .map(Into::into)
            .collect();
        Ok(Outcome {
            value: object([(
                "stage",
                WireValue::Object(BTreeMap::from([
                    ("id".into(), self.options.stage_id.clone().into()),
                    ("consumes".into(), self.metadata.consumes.clone().into()),
                    ("produces".into(), self.metadata.produces.clone().into()),
                    ("capabilities".into(), WireValue::Array(capabilities)),
                    (
                        "configSchemaHash".into(),
                        self.options
                            .config_schema_hash
                            .clone()
                            .map(Into::into)
                            .unwrap_or(WireValue::Null),
                    ),
                ])),
            )]),
            stop: false,
        })
    }

    async fn init(&self, params: BTreeMap<String, WireValue>) -> Result<Outcome, RpcFault> {
        let _control = self.control.lock().await;
        {
            let phase = self.phase.lock().await;
            if *phase == Phase::Initialized {
                return Ok(Outcome {
                    value: WireValue::Null,
                    stop: false,
                });
            }
            if *phase != Phase::Announced {
                return Err(phase_fault(*phase));
            }
        }
        let config = params.get("config").cloned().unwrap_or(WireValue::Null);
        *self.last_config.lock().await = config.clone();
        let context = StageContext::new(
            self.peer.clone(),
            0,
            CancellationToken::new(),
            config.clone(),
            self.context_streams(),
        );
        let stage = self.stage.clone();
        let init = tokio::spawn(async move { stage.init(config, &context).await })
            .await
            .map_err(|_| RpcFault::new(-32603, "INTERNAL_ERROR", None))?;
        if let Err(error) = init {
            *self.phase.lock().await = Phase::Failed;
            return Err(stage_fault(error));
        }
        *self.phase.lock().await = Phase::Initialized;
        Ok(Outcome {
            value: WireValue::Null,
            stop: false,
        })
    }

    async fn run(
        self: &Arc<Self>,
        id: i64,
        params: BTreeMap<String, WireValue>,
        ready: Option<oneshot::Sender<()>>,
    ) -> Result<Outcome, RpcFault> {
        let control = self.control.lock().await;
        if *self.phase.lock().await != Phase::Initialized {
            return Err(phase_fault(*self.phase.lock().await));
        }
        let stream_id = safe_id(params.get("streamId"), "streamId")
            .map_err(|_| RpcFault::new(-32004, "PROTOCOL_VIOLATION", None))?;
        if stream_id <= 0 {
            return Err(RpcFault::new(-32004, "PROTOCOL_VIOLATION", None));
        }
        let cancellation = CancellationToken::new();
        {
            let mut active = self.active.lock().await;
            if active.is_some() {
                return Err(RpcFault::new(-32004, "PROTOCOL_VIOLATION", None));
            }
            *active = Some(Active {
                id,
                cancellation: cancellation.clone(),
            });
        }
        drop(control);
        let runtime = self.clone();
        let body_cancellation = cancellation.clone();
        let body = tokio::spawn(async move {
            runtime
                .run_active(params, stream_id, body_cancellation, ready)
                .await
        });
        let response = body
            .await
            .map_err(|_| RpcFault::new(-32603, "INTERNAL_ERROR", None))
            .and_then(std::convert::identity);
        self.close_capability_streams().await;
        self.inputs.lock().await.clear();
        *self.active.lock().await = None;
        self.active_done.notify_waiters();
        let value = response?;
        Ok(Outcome { value, stop: false })
    }

    async fn run_active(
        &self,
        params: BTreeMap<String, WireValue>,
        stream_id: i64,
        cancellation: CancellationToken,
        ready: Option<oneshot::Sender<()>>,
    ) -> Result<WireValue, RpcFault> {
        let mut ready = DispatchReady(ready);
        let config = params.get("config").cloned().unwrap_or(WireValue::Null);
        *self.last_config.lock().await = config.clone();
        let input = if self.metadata.consumes.starts_with("Stream<") {
            let handle = match params.get("input") {
                Some(WireValue::Object(value))
                    if value.len() == 2
                        && value.get("kind").and_then(WireValue::as_str)
                            == Some("stream-handle") =>
                {
                    value.get("streamId")
                }
                _ => None,
            };
            let input_id = safe_id(handle, "input streamId")
                .map_err(|_| RpcFault::new(-32004, "PROTOCOL_VIOLATION", None))?;
            if input_id < 0 {
                return Err(RpcFault::new(-32004, "PROTOCOL_VIOLATION", None));
            }
            let (sender, receiver) = mpsc::channel(self.options.max_buffered_stream_values);
            let terminal = Arc::new(StreamTerminal::new());
            self.inputs.lock().await.insert(
                input_id,
                InputSender {
                    sender,
                    bytes: Arc::new(Semaphore::new(self.options.max_buffered_stream_bytes)),
                    max_bytes: self.options.max_buffered_stream_bytes,
                    terminal: terminal.clone(),
                },
            );
            StageInput::Stream(InputStream::new(receiver, terminal))
        } else {
            let value = params.get("input").cloned().unwrap_or(WireValue::Null);
            StageInput::Single(S::Input::from_wire(value).map_err(stage_fault)?)
        };
        // Preserve wire ordering: the reader may dispatch stream notifications
        // only after this run has installed its input stream (or failed setup).
        ready.signal();
        let context = StageContext::new(
            self.peer.clone(),
            stream_id,
            cancellation.clone(),
            config.clone(),
            self.context_streams(),
        );
        let stage = self.stage.clone();
        let mut task = tokio::spawn(async move { stage.run(input, config, &context).await });
        let output = tokio::select! {
            biased;
            error = cancellation.cancelled() => {
                task.abort();
                return Err(cancelled_fault(error));
            }
            result = &mut task => result.map_err(|_| RpcFault::new(-32603, "INTERNAL_ERROR", None))?,
        };
        cancellation.check().map_err(cancelled_fault)?;
        let output = output.map_err(stage_fault)?;
        match output {
            StageOutput::Single(value) if !self.metadata.produces.starts_with("Stream<") => {
                cancellation.check().map_err(cancelled_fault)?;
                let value = value.into();
                cancellation.check().map_err(cancelled_fault)?;
                Ok(object([("kind", "single".into()), ("value", value)]))
            }
            StageOutput::Stream(mut receiver) if self.metadata.produces.starts_with("Stream<") => {
                let mut produced = 0_i64;
                loop {
                    let value = tokio::select! {
                        biased;
                        error = cancellation.cancelled() => return Err(cancelled_fault(error)),
                        value = receiver.recv() => value,
                    };
                    cancellation.check().map_err(cancelled_fault)?;
                    let Some(value) = value else { break };
                    let notification = object([
                        ("streamId", stream_id.into()),
                        ("value", value.map_err(stage_fault)?.into()),
                    ]);
                    if self
                        .peer
                        .notify_cancellable("stream.value", notification, &cancellation)
                        .await
                        .is_err()
                    {
                        if let Err(error) = cancellation.check() {
                            return Err(cancelled_fault(error));
                        }
                        return Err(RpcFault::new(-32603, "INTERNAL_ERROR", None));
                    }
                    produced += 1;
                }
                cancellation.check().map_err(cancelled_fault)?;
                Ok(object([
                    ("kind", "stream".into()),
                    ("streamId", stream_id.into()),
                    ("produced", produced.into()),
                ]))
            }
            _ => Err(RpcFault::new(-32004, "PROTOCOL_VIOLATION", None)),
        }
    }

    async fn dispose(&self) -> Result<Outcome, RpcFault> {
        let _control = self.control.lock().await;
        if self.active.lock().await.is_some() {
            return Err(RpcFault::new(-32004, "PROTOCOL_VIOLATION", None));
        }
        let phase = *self.phase.lock().await;
        if phase == Phase::Disposed {
            return Ok(Outcome {
                value: WireValue::Null,
                stop: true,
            });
        }
        if phase != Phase::Initialized {
            return Err(phase_fault(phase));
        }
        let config = self.last_config.lock().await.clone();
        let context = StageContext::new(
            self.peer.clone(),
            0,
            CancellationToken::new(),
            config,
            self.context_streams(),
        );
        let stage = self.stage.clone();
        tokio::spawn(async move { stage.dispose(&context).await })
            .await
            .map_err(|_| RpcFault::new(-32603, "INTERNAL_ERROR", None))?
            .map_err(stage_fault)?;
        *self.phase.lock().await = Phase::Disposed;
        Ok(Outcome {
            value: WireValue::Null,
            stop: true,
        })
    }

    async fn shutdown(&self) {
        let cancellation = self
            .active
            .lock()
            .await
            .as_ref()
            .map(|active| active.cancellation.clone());
        if let Some(cancellation) = cancellation {
            cancellation.cancel("plugin process received termination signal");
            loop {
                let notified = self.active_done.notified();
                tokio::pin!(notified);
                notified.as_mut().enable();
                if self.active.lock().await.is_none() {
                    break;
                }
                notified.await;
            }
        }
        self.close_capability_streams().await;
        self.inputs.lock().await.clear();
        if *self.phase.lock().await == Phase::Initialized && self.active.lock().await.is_none() {
            let _ = self.dispose().await;
        }
        self.peer.stop();
    }

    async fn close_capability_streams(&self) {
        let stream_ids = {
            let mut active = self.capability_inputs.lock().await;
            active.drain().collect::<Vec<_>>()
        };
        for stream_id in stream_ids {
            self.inputs.lock().await.remove(&stream_id);
            let _ = self
                .peer
                .notify("stream.cancel", object([("streamId", stream_id.into())]))
                .await;
        }
    }
}

pub async fn run_plugin<S: Stage>(stage: S, options: RunnerOptions) -> Result<(), ProtocolError> {
    options.validate()?;
    let metadata = stage.metadata();
    if metadata.name.is_empty() || metadata.version.is_empty() || metadata.api_version == 0 {
        return Err(ProtocolError::InvalidMessage(
            "stage metadata is invalid".into(),
        ));
    }
    let peer = Peer::new(
        options.max_frame_bytes,
        options.max_pending_requests,
        options.max_pending_writes,
    );
    let runtime = Arc::new(Runtime {
        stage: Arc::new(stage),
        metadata,
        options: options.clone(),
        peer: peer.clone(),
        phase: Mutex::new(Phase::Spawned),
        control: Mutex::new(()),
        active: Mutex::new(None),
        active_done: tokio::sync::Notify::new(),
        inputs: Arc::new(Mutex::new(HashMap::new())),
        capability_inputs: Arc::new(Mutex::new(HashSet::new())),
        last_config: Mutex::new(WireValue::Null),
    });
    let request_limit = Arc::new(Semaphore::new(options.max_inflight_requests));
    let mut decoder = FrameDecoder::new(options.max_frame_bytes, options.max_header_bytes)?;
    let mut input = tokio::io::stdin();
    let mut chunk = [0_u8; 65536];
    loop {
        let read_capacity = decoder.remaining_capacity().min(chunk.len());
        tokio::select! {
            result = input.read(&mut chunk[..read_capacity]) => {
                let count = result.map_err(|_| ProtocolError::Closed("runner stdin failed".into()))?;
                if count == 0 {
                    decoder.finish()?;
                    return Err(ProtocolError::Closed("host wire closed unexpectedly".into()));
                }
                for message in decoder.push(&chunk[..count])? {
                    receive(message, runtime.clone(), peer.clone(), request_limit.clone()).await?;
                }
            }
            _ = peer.stopped() => return Ok(()),
            _ = termination_signal() => {
                if timeout(Duration::from_millis(250), runtime.shutdown()).await.is_err() {
                    std::process::exit(1);
                }
                return Ok(());
            }
        }
    }
}

async fn receive<S: Stage>(
    message: WireValue,
    runtime: Arc<Runtime<S>>,
    peer: Arc<Peer>,
    request_limit: Arc<Semaphore>,
) -> Result<(), ProtocolError> {
    let message = message.into_object()?;
    if message.get("jsonrpc").and_then(WireValue::as_str) != Some("2.0") {
        return Err(ProtocolError::InvalidMessage(
            "jsonrpc must equal 2.0".into(),
        ));
    }
    if let Some(method) = message.get("method").and_then(WireValue::as_str) {
        let params = message
            .get("params")
            .cloned()
            .unwrap_or(WireValue::Object(BTreeMap::new()));
        if !message.contains_key("id") {
            return runtime.notification(method, params).await;
        }
        let id = safe_id(message.get("id"), "host request id")?;
        if id <= 0 {
            return Err(ProtocolError::InvalidMessage(
                "host request ids must be positive safe integers".into(),
            ));
        }
        let permit = request_limit.clone().try_acquire_owned().map_err(|_| {
            ProtocolError::ResourceLimit("too many concurrent host requests".into())
        })?;
        let method = method.to_owned();
        let needs_stream_barrier =
            method == "stage.run" && runtime.metadata.consumes.starts_with("Stream<");
        let (ready_sender, ready_receiver) = if needs_stream_barrier {
            let (sender, receiver) = oneshot::channel();
            (Some(sender), Some(receiver))
        } else {
            (None, None)
        };
        tokio::spawn(async move {
            let _permit = permit;
            let result = runtime.request(id, &method, params, ready_sender).await;
            let stop = result.as_ref().map(|outcome| outcome.stop).unwrap_or(false);
            let response = result.map(|outcome| outcome.value);
            let _ = peer.response(id, response).await;
            if stop {
                peer.stop();
            }
        });
        if let Some(receiver) = ready_receiver {
            let _ = receiver.await;
        }
        Ok(())
    } else {
        peer.complete_response(&message).await
    }
}

fn params(value: WireValue, method: &str) -> Result<BTreeMap<String, WireValue>, RpcFault> {
    match value {
        WireValue::Object(value) => Ok(value),
        WireValue::Null => Ok(BTreeMap::new()),
        _ => Err(RpcFault::new(
            -32602,
            "INVALID_PARAMS",
            Some(object([("method", method.into())])),
        )),
    }
}

fn phase_fault(phase: Phase) -> RpcFault {
    RpcFault::new(
        -32004,
        "PROTOCOL_VIOLATION",
        Some(object([(
            "phase",
            format!("{phase:?}").to_lowercase().into(),
        )])),
    )
}

fn stage_fault(error: StageError) -> RpcFault {
    if error.code == "CANCELLED" {
        RpcFault::new(-32800, "REQUEST_CANCELLED", Some(error.wire_fields()))
    } else {
        RpcFault::new(-32900, error.to_string(), Some(error.wire_fields()))
    }
}

fn cancelled_fault(error: impl ToString) -> RpcFault {
    RpcFault::new(
        -32800,
        "REQUEST_CANCELLED",
        Some(object([("reason", error.to_string().into())])),
    )
}

#[cfg(unix)]
async fn termination_signal() {
    use tokio::signal::unix::{signal, SignalKind};
    let mut terminate = signal(SignalKind::terminate()).expect("install SIGTERM listener");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = terminate.recv() => {}
    }
}

#[cfg(not(unix))]
async fn termination_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
