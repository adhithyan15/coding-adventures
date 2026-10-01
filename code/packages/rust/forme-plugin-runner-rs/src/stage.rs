use crate::wire::WireValue;
use async_trait::async_trait;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, Notify, OwnedSemaphorePermit};

#[derive(Clone, Debug)]
pub struct StageMetadata {
    pub name: String,
    pub version: String,
    pub api_version: u32,
    pub consumes: String,
    pub produces: String,
    pub capabilities: Vec<String>,
}

impl StageMetadata {
    pub fn new<I, S>(
        name: impl Into<String>,
        version: impl Into<String>,
        api_version: u32,
        consumes: impl Into<String>,
        produces: impl Into<String>,
        capabilities: I,
    ) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            name: name.into(),
            version: version.into(),
            api_version,
            consumes: consumes.into(),
            produces: produces.into(),
            capabilities: capabilities.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct StageError {
    pub code: String,
    message: String,
    pub recoverable: bool,
    pub fields: BTreeMap<String, WireValue>,
}

impl StageError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            recoverable: false,
            fields: BTreeMap::new(),
        }
    }

    pub fn with_field(mut self, key: impl Into<String>, value: WireValue) -> Self {
        self.fields.insert(key.into(), value);
        self
    }

    pub fn recoverable(mut self, recoverable: bool) -> Self {
        self.recoverable = recoverable;
        self
    }

    pub fn wire_data(&self) -> serde_json::Value {
        let fields = self
            .fields
            .iter()
            .map(|(key, value)| (key.clone(), debug_json(value)))
            .collect::<serde_json::Map<_, _>>();
        serde_json::json!({
            "stageErrorCode": self.code,
            "recoverable": self.recoverable,
            "fields": fields,
        })
    }

    pub(crate) fn wire_fields(&self) -> WireValue {
        WireValue::Object(BTreeMap::from([
            ("stageErrorCode".into(), self.code.clone().into()),
            ("recoverable".into(), self.recoverable.into()),
            ("fields".into(), WireValue::Object(self.fields.clone())),
        ]))
    }
}

impl fmt::Display for StageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for StageError {}

fn debug_json(value: &WireValue) -> serde_json::Value {
    match value {
        WireValue::Null => serde_json::Value::Null,
        WireValue::Bool(value) => (*value).into(),
        WireValue::Number(value) => serde_json::Value::Number(value.clone()),
        WireValue::String(value) => value.clone().into(),
        WireValue::Bytes(value) => serde_json::json!({"bytes": value}),
        WireValue::Array(values) => values.iter().map(debug_json).collect(),
        WireValue::Object(values) => values
            .iter()
            .map(|(key, value)| (key.clone(), debug_json(value)))
            .collect(),
    }
}

#[derive(Clone, Debug)]
pub struct CancellationError(String);

impl fmt::Display for CancellationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for CancellationError {}

#[derive(Default)]
struct CancellationState {
    reason: Mutex<Option<String>>,
    notify: Notify,
}

#[derive(Clone, Default)]
pub struct CancellationToken(Arc<CancellationState>);

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self, reason: impl Into<String>) {
        let mut guard = self.0.reason.lock().expect("cancellation mutex poisoned");
        if guard.is_none() {
            *guard = Some(reason.into());
            self.0.notify.notify_waiters();
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.0
            .reason
            .lock()
            .expect("cancellation mutex poisoned")
            .is_some()
    }

    pub fn check(&self) -> Result<(), CancellationError> {
        match self
            .0
            .reason
            .lock()
            .expect("cancellation mutex poisoned")
            .clone()
        {
            Some(reason) => Err(CancellationError(reason)),
            None => Ok(()),
        }
    }

    pub async fn cancelled(&self) -> CancellationError {
        loop {
            let notified = self.0.notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if let Err(error) = self.check() {
                return error;
            }
            notified.await;
        }
    }
}

pub struct InputStream<T> {
    receiver: mpsc::Receiver<StreamItem<T>>,
    terminal: Arc<StreamTerminal>,
    terminal_delivered: bool,
}

impl<T> InputStream<T> {
    pub(crate) fn new(
        receiver: mpsc::Receiver<StreamItem<T>>,
        terminal: Arc<StreamTerminal>,
    ) -> Self {
        Self {
            receiver,
            terminal,
            terminal_delivered: false,
        }
    }

    pub async fn next(&mut self) -> Option<Result<T, StageError>> {
        if let Some(error) = self.terminal_error() {
            return Some(Err(error));
        }
        tokio::select! {
            biased;
            _ = self.terminal.notify.notified() => self.terminal_error().map(Err),
            item = self.receiver.recv() => {
                if let Some(error) = self.terminal_error() {
                    Some(Err(error))
                } else {
                    item.map(|item| item.value)
                }
            }
        }
    }

    fn terminal_error(&mut self) -> Option<StageError> {
        if self.terminal_delivered {
            return None;
        }
        let error = self.terminal.error();
        if error.is_some() {
            self.terminal_delivered = true;
            self.receiver.close();
            while self.receiver.try_recv().is_ok() {}
        }
        error
    }
}

pub(crate) struct StreamTerminal {
    error: Mutex<Option<StageError>>,
    notify: Notify,
}

impl StreamTerminal {
    pub(crate) fn new() -> Self {
        Self {
            error: Mutex::new(None),
            notify: Notify::new(),
        }
    }

    pub(crate) fn fail(&self, error: StageError) {
        let mut guard = self.error.lock().expect("stream terminal mutex poisoned");
        if guard.is_none() {
            *guard = Some(error);
            self.notify.notify_one();
        }
    }

    fn error(&self) -> Option<StageError> {
        self.error
            .lock()
            .expect("stream terminal mutex poisoned")
            .clone()
    }
}

pub(crate) struct StreamItem<T> {
    pub value: Result<T, StageError>,
    pub _bytes: Option<OwnedSemaphorePermit>,
}

pub enum StageInput<T> {
    Single(T),
    Stream(InputStream<T>),
}

pub enum StageOutput<T> {
    Single(T),
    Stream(mpsc::Receiver<Result<T, StageError>>),
}

pub trait FromWire: Sized {
    fn from_wire(value: WireValue) -> Result<Self, StageError>;
}

impl FromWire for WireValue {
    fn from_wire(value: WireValue) -> Result<Self, StageError> {
        Ok(value)
    }
}

#[async_trait]
pub trait Stage: Send + Sync + 'static {
    type Input: FromWire + Send + 'static;
    type Output: Into<WireValue> + Send + 'static;

    fn metadata(&self) -> StageMetadata;

    async fn init(
        &self,
        _config: WireValue,
        _context: &crate::StageContext,
    ) -> Result<(), StageError> {
        Ok(())
    }

    async fn run(
        &self,
        input: StageInput<Self::Input>,
        config: WireValue,
        context: &crate::StageContext,
    ) -> Result<StageOutput<Self::Output>, StageError>;

    async fn dispose(&self, _context: &crate::StageContext) -> Result<(), StageError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn terminal_error_preempts_and_discards_buffered_values() {
        let (sender, receiver) = mpsc::channel(2);
        let terminal = Arc::new(StreamTerminal::new());
        sender
            .send(StreamItem {
                value: Ok(7_u8),
                _bytes: None,
            })
            .await
            .unwrap();
        terminal.fail(StageError::new("UPSTREAM_STREAM_ERROR", "failed"));

        let mut stream = InputStream::new(receiver, terminal);
        let error = stream.next().await.unwrap().unwrap_err();
        assert_eq!(error.code, "UPSTREAM_STREAM_ERROR");
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn cancellation_wakes_active_waiters() {
        let token = CancellationToken::new();
        let waiter = token.clone();
        let task = tokio::spawn(async move { waiter.cancelled().await });
        tokio::task::yield_now().await;
        token.cancel("stop now");
        assert_eq!(task.await.unwrap().to_string(), "stop now");
    }
}
