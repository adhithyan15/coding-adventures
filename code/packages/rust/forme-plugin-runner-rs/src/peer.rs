use crate::wire::{encode_frame, ProtocolError, WireValue};
use crate::CancellationToken;
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Weak};
use tokio::io::{AsyncWriteExt, BufWriter, Stdout};
use tokio::sync::{oneshot, Mutex, Notify, OwnedSemaphorePermit, Semaphore};

const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;

#[derive(Debug)]
pub(crate) struct RemoteError {
    pub code: i64,
    pub message: String,
    pub data: WireValue,
}

#[derive(Clone, Debug)]
pub(crate) struct RpcFault {
    pub code: i64,
    pub message: String,
    pub data: Option<WireValue>,
}

impl RpcFault {
    pub fn new(code: i64, message: impl Into<String>, data: Option<WireValue>) -> Self {
        Self {
            code,
            message: message.into(),
            data,
        }
    }
}

struct Pending {
    sender: oneshot::Sender<Result<WireValue, RemoteError>>,
    _permit: OwnedSemaphorePermit,
}

struct PendingCleanup {
    peer: Weak<Peer>,
    id: i64,
}

impl Drop for PendingCleanup {
    fn drop(&mut self) {
        if let Some(peer) = self.peer.upgrade() {
            let id = self.id;
            tokio::spawn(async move {
                peer.pending.lock().await.remove(&id);
            });
        }
    }
}

pub(crate) struct Peer {
    output: Mutex<BufWriter<Stdout>>,
    max_frame_bytes: usize,
    next_id: AtomicI64,
    pending: Mutex<HashMap<i64, Pending>>,
    pending_limit: Arc<Semaphore>,
    write_limit: Arc<Semaphore>,
    stopped: AtomicBool,
    shutdown: Notify,
}

impl Peer {
    pub fn new(max_frame_bytes: usize, max_pending: usize, max_writes: usize) -> Arc<Self> {
        Arc::new(Self {
            output: Mutex::new(BufWriter::new(tokio::io::stdout())),
            max_frame_bytes,
            next_id: AtomicI64::new(-1),
            pending: Mutex::new(HashMap::new()),
            pending_limit: Arc::new(Semaphore::new(max_pending)),
            write_limit: Arc::new(Semaphore::new(max_writes)),
            stopped: AtomicBool::new(false),
            shutdown: Notify::new(),
        })
    }

    pub async fn request_cancellable(
        self: &Arc<Self>,
        method: &str,
        params: WireValue,
        cancellation: &CancellationToken,
    ) -> Result<WireValue, ProtocolErrorOrRemote> {
        self.assert_open()?;
        let permit = self
            .pending_limit
            .clone()
            .try_acquire_owned()
            .map_err(|_| ProtocolError::ResourceLimit("too many pending plugin requests".into()))?;
        let id = self.next_id.fetch_sub(1, Ordering::Relaxed);
        if id <= -MAX_SAFE_INTEGER {
            return Err(ProtocolError::ResourceLimit("plugin request id exhausted".into()).into());
        }
        let (sender, receiver) = oneshot::channel();
        self.pending.lock().await.insert(
            id,
            Pending {
                sender,
                _permit: permit,
            },
        );
        let _cleanup = PendingCleanup {
            peer: Arc::downgrade(self),
            id,
        };
        let message = object([
            ("jsonrpc", "2.0".into()),
            ("id", id.into()),
            ("method", method.into()),
            ("params", params),
        ]);
        if let Err(error) = self.send_cancellable(&message, cancellation).await {
            self.pending.lock().await.remove(&id);
            return Err(error.into());
        }
        tokio::select! {
            biased;
            _ = cancellation.cancelled() => Err(ProtocolErrorOrRemote::Protocol),
            result = receiver => result
                .map_err(|_| ProtocolErrorOrRemote::Protocol)?
                .map_err(Into::into),
        }
    }

    pub async fn notify(&self, method: &str, params: WireValue) -> Result<(), ProtocolError> {
        self.send(&object([
            ("jsonrpc", "2.0".into()),
            ("method", method.into()),
            ("params", params),
        ]))
        .await
    }

    pub async fn notify_cancellable(
        &self,
        method: &str,
        params: WireValue,
        cancellation: &CancellationToken,
    ) -> Result<(), ProtocolError> {
        self.send_cancellable(
            &object([
                ("jsonrpc", "2.0".into()),
                ("method", method.into()),
                ("params", params),
            ]),
            cancellation,
        )
        .await
    }

    pub async fn response(
        &self,
        id: i64,
        result: Result<WireValue, RpcFault>,
    ) -> Result<(), ProtocolError> {
        let message = match result {
            Ok(value) => object([
                ("jsonrpc", "2.0".into()),
                ("id", id.into()),
                ("result", value),
            ]),
            Err(error) => {
                let mut fault = BTreeMap::from([
                    ("code".into(), error.code.into()),
                    ("message".into(), error.message.into()),
                ]);
                if let Some(data) = error.data {
                    fault.insert("data".into(), data);
                }
                object([
                    ("jsonrpc", "2.0".into()),
                    ("id", id.into()),
                    ("error", WireValue::Object(fault)),
                ])
            }
        };
        self.send(&message).await
    }

    pub async fn complete_response(
        &self,
        message: &BTreeMap<String, WireValue>,
    ) -> Result<(), ProtocolError> {
        let id = safe_id(message.get("id"), "plugin response id")?;
        if id >= 0 {
            return Err(ProtocolError::InvalidMessage(
                "plugin response ids must be negative safe integers".into(),
            ));
        }
        let pending = self.pending.lock().await.remove(&id).ok_or_else(|| {
            ProtocolError::InvalidMessage("response has an unknown request id".into())
        })?;
        let has_result = message.contains_key("result");
        let has_error = message.contains_key("error");
        if has_result == has_error {
            let _ = pending.sender.send(Err(RemoteError {
                code: -32603,
                message: "malformed JSON-RPC response".into(),
                data: WireValue::Null,
            }));
            return Err(ProtocolError::InvalidMessage(
                "response must contain exactly one result or error".into(),
            ));
        }
        if let Some(result) = message.get("result") {
            let _ = pending.sender.send(Ok(result.clone()));
            return Ok(());
        }
        let error = message
            .get("error")
            .and_then(|value| match value {
                WireValue::Object(value) => Some(value),
                _ => None,
            })
            .ok_or_else(|| ProtocolError::InvalidMessage("malformed JSON-RPC error".into()))?;
        let code = safe_id(error.get("code"), "error code")?;
        let message = error
            .get("message")
            .and_then(WireValue::as_str)
            .ok_or_else(|| ProtocolError::InvalidMessage("malformed JSON-RPC error".into()))?;
        let _ = pending.sender.send(Err(RemoteError {
            code,
            message: message.to_owned(),
            data: error.get("data").cloned().unwrap_or(WireValue::Null),
        }));
        Ok(())
    }

    async fn send(&self, value: &WireValue) -> Result<(), ProtocolError> {
        self.assert_open()?;
        let _permit = self
            .write_limit
            .clone()
            .try_acquire_owned()
            .map_err(|_| ProtocolError::ResourceLimit("too many pending wire writes".into()))?;
        let frame = encode_frame(value, self.max_frame_bytes)?;
        let mut output = self.output.lock().await;
        output
            .write_all(&frame)
            .await
            .map_err(|_| ProtocolError::Closed("runner stdout failed".into()))?;
        output
            .flush()
            .await
            .map_err(|_| ProtocolError::Closed("runner stdout failed".into()))
    }

    async fn send_cancellable(
        &self,
        value: &WireValue,
        cancellation: &CancellationToken,
    ) -> Result<(), ProtocolError> {
        self.assert_open()?;
        let _permit = self
            .write_limit
            .clone()
            .try_acquire_owned()
            .map_err(|_| ProtocolError::ResourceLimit("too many pending wire writes".into()))?;
        let frame = encode_frame(value, self.max_frame_bytes)?;
        let mut output = tokio::select! {
            biased;
            _ = cancellation.cancelled() => {
                return Err(ProtocolError::Closed("operation cancelled".into()));
            }
            output = self.output.lock() => output,
        };
        cancellation
            .check()
            .map_err(|_| ProtocolError::Closed("operation cancelled".into()))?;
        // Once the frame starts, finish it without cancellation so stdout can never
        // contain a partial frame. Cancellation is linearized by the check above.
        output
            .write_all(&frame)
            .await
            .map_err(|_| ProtocolError::Closed("runner stdout failed".into()))?;
        output
            .flush()
            .await
            .map_err(|_| ProtocolError::Closed("runner stdout failed".into()))
    }

    fn assert_open(&self) -> Result<(), ProtocolError> {
        if self.stopped.load(Ordering::Acquire) {
            Err(ProtocolError::Closed("runner wire is closed".into()))
        } else {
            Ok(())
        }
    }

    pub fn stop(self: &Arc<Self>) {
        if !self.stopped.swap(true, Ordering::AcqRel) {
            self.shutdown.notify_waiters();
            let peer = self.clone();
            tokio::spawn(async move {
                peer.pending.lock().await.clear();
            });
        }
    }

    pub async fn stopped(&self) {
        if !self.stopped.load(Ordering::Acquire) {
            self.shutdown.notified().await;
        }
    }
}

#[derive(Debug)]
pub(crate) enum ProtocolErrorOrRemote {
    Protocol,
    Remote(RemoteError),
}

impl From<ProtocolError> for ProtocolErrorOrRemote {
    fn from(_value: ProtocolError) -> Self {
        Self::Protocol
    }
}

impl From<RemoteError> for ProtocolErrorOrRemote {
    fn from(value: RemoteError) -> Self {
        Self::Remote(value)
    }
}

pub(crate) fn safe_id(value: Option<&WireValue>, label: &str) -> Result<i64, ProtocolError> {
    let value = value
        .and_then(WireValue::as_i64)
        .ok_or_else(|| ProtocolError::InvalidMessage(format!("{label} must be a safe integer")))?;
    if value.abs() > MAX_SAFE_INTEGER {
        return Err(ProtocolError::InvalidMessage(format!(
            "{label} must be a safe integer"
        )));
    }
    Ok(value)
}

pub(crate) fn object<const N: usize>(entries: [(&str, WireValue); N]) -> WireValue {
    WireValue::Object(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
}
