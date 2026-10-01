use crate::peer::{object, Peer, ProtocolErrorOrRemote};
use crate::{CancellationToken, StageError, WireValue};
use coding_adventures_base64::{decode, encode, STANDARD};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;

const MAX_MEDIATED_BYTES: usize = 1024 * 1024;

#[derive(Clone)]
pub struct StageContext {
    peer: Arc<Peer>,
    stream_id: i64,
    pub cancellation: CancellationToken,
    pub config: WireValue,
}

impl StageContext {
    pub(crate) fn new(
        peer: Arc<Peer>,
        stream_id: i64,
        cancellation: CancellationToken,
        config: WireValue,
    ) -> Self {
        Self {
            peer,
            stream_id,
            cancellation,
            config,
        }
    }

    pub fn storage(&self) -> StorageApi<'_> {
        StorageApi(self)
    }
    pub fn env(&self) -> EnvApi<'_> {
        EnvApi(self)
    }
    pub fn time(&self) -> TimeApi<'_> {
        TimeApi(self)
    }
    pub fn filesystem(&self) -> FilesystemApi<'_> {
        FilesystemApi(self)
    }
    pub fn network(&self) -> NetworkApi<'_> {
        NetworkApi(self)
    }
    pub fn shell(&self) -> ShellApi<'_> {
        ShellApi(self)
    }
    pub fn logger(&self) -> Logger<'_> {
        Logger(self)
    }

    async fn request(
        &self,
        method: &str,
        params: BTreeMap<String, WireValue>,
    ) -> Result<WireValue, StageError> {
        self.cancellation
            .check()
            .map_err(|error| StageError::new("CANCELLED", error.to_string()))?;
        let mut params = params;
        params.insert("streamId".into(), self.stream_id.into());
        let result = self
            .peer
            .request_cancellable(method, WireValue::Object(params), &self.cancellation)
            .await;
        self.cancellation
            .check()
            .map_err(|error| StageError::new("CANCELLED", error.to_string()))?;
        let value = match result {
            Ok(value) => value,
            Err(ProtocolErrorOrRemote::Protocol) => {
                return Err(StageError::new(
                    "PLUGIN_PROTOCOL_ERROR",
                    "host protocol failed",
                ))
            }
            Err(ProtocolErrorOrRemote::Remote(error)) if error.code == -32800 => {
                return Err(StageError::new("CANCELLED", error.message))
            }
            Err(ProtocolErrorOrRemote::Remote(error)) if error.code == -32001 => {
                let capability = error
                    .data
                    .get("capability")
                    .and_then(WireValue::as_str)
                    .unwrap_or("unknown")
                    .to_owned();
                return Err(StageError::new("CAPABILITY_DENIED", error.message)
                    .with_field("capability", capability.into()));
            }
            Err(ProtocolErrorOrRemote::Remote(error)) => {
                return Err(StageError::new(
                    "PLUGIN_PROTOCOL_ERROR",
                    "host capability request failed",
                )
                .with_field("rpcCode", error.code.into()))
            }
        };
        self.cancellation
            .check()
            .map_err(|error| StageError::new("CANCELLED", error.to_string()))?;
        Ok(value)
    }
}

pub struct StorageApi<'a>(&'a StageContext);

impl StorageApi<'_> {
    pub async fn read(&self, path: &str) -> Result<Vec<u8>, StageError> {
        result_bytes(
            self.0
                .request(
                    "ctx.storage.read",
                    BTreeMap::from([("path".into(), path.into())]),
                )
                .await?,
            "storage.read",
        )
    }

    pub async fn read_bounded(&self, path: &str, max_bytes: usize) -> Result<Vec<u8>, StageError> {
        assert_bound(max_bytes)?;
        let value = self.read(path).await?;
        if value.len() > max_bytes {
            return Err(StageError::new(
                "RESOURCE_LIMIT_EXCEEDED",
                "storage value exceeds requested bound",
            ));
        }
        Ok(value)
    }

    pub async fn write(&self, path: &str, bytes: &[u8]) -> Result<(), StageError> {
        assert_bytes(bytes)?;
        self.0
            .request(
                "ctx.storage.write",
                BTreeMap::from([
                    ("path".into(), path.into()),
                    ("bytes".into(), encode(bytes, &STANDARD).into()),
                ]),
            )
            .await?;
        Ok(())
    }

    pub async fn exists(&self, path: &str) -> Result<bool, StageError> {
        self.0
            .request(
                "ctx.storage.exists",
                BTreeMap::from([("path".into(), path.into())]),
            )
            .await?
            .as_bool()
            .ok_or_else(|| {
                StageError::new(
                    "PLUGIN_PROTOCOL_ERROR",
                    "storage.exists result is malformed",
                )
            })
    }

    pub async fn stat(&self, path: &str) -> Result<WireValue, StageError> {
        require_object(
            self.0
                .request(
                    "ctx.storage.stat",
                    BTreeMap::from([("path".into(), path.into())]),
                )
                .await?,
            "storage.stat",
        )
    }

    pub async fn list(&self, path: &str) -> Result<Vec<WireValue>, StageError> {
        require_array(
            self.0
                .request(
                    "ctx.storage.list",
                    BTreeMap::from([("path".into(), path.into())]),
                )
                .await?,
            "storage.list",
        )
    }

    pub async fn watch(&self, path: &str) -> Result<Vec<WireValue>, StageError> {
        require_array(
            self.0
                .request(
                    "ctx.storage.watch",
                    BTreeMap::from([("path".into(), path.into())]),
                )
                .await?,
            "storage.watch",
        )
    }

    pub async fn remove(&self, path: &str) -> Result<(), StageError> {
        self.0
            .request(
                "ctx.storage.remove",
                BTreeMap::from([("path".into(), path.into())]),
            )
            .await?;
        Ok(())
    }
}

pub struct EnvApi<'a>(&'a StageContext);
impl EnvApi<'_> {
    pub async fn get(&self, name: &str) -> Result<Option<String>, StageError> {
        match self
            .0
            .request(
                "ctx.env.get",
                BTreeMap::from([("name".into(), name.into())]),
            )
            .await?
        {
            WireValue::Null => Ok(None),
            WireValue::String(value) => Ok(Some(value)),
            _ => Err(StageError::new(
                "PLUGIN_PROTOCOL_ERROR",
                "env.get result is malformed",
            )),
        }
    }

    pub async fn get_or_throw(&self, name: &str) -> Result<String, StageError> {
        self.get(name).await?.ok_or_else(|| {
            StageError::new(
                "ENV_MISSING",
                format!("Environment variable {name} is not set"),
            )
        })
    }
}

pub struct TimeApi<'a>(&'a StageContext);
impl TimeApi<'_> {
    pub async fn now_ms(&self) -> Result<WireValue, StageError> {
        let value = self.0.request("ctx.time.nowMs", BTreeMap::new()).await?;
        match value {
            WireValue::Number(_) => Ok(value),
            _ => Err(StageError::new(
                "PLUGIN_PROTOCOL_ERROR",
                "time.nowMs result is malformed",
            )),
        }
    }

    pub async fn now_iso(&self) -> Result<String, StageError> {
        require_string(
            self.0.request("ctx.time.nowIso", BTreeMap::new()).await?,
            "time.nowIso",
        )
    }

    pub fn monotonic_ms(&self) -> u128 {
        static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
        START.get_or_init(Instant::now).elapsed().as_millis()
    }
}

pub struct FilesystemApi<'a>(&'a StageContext);
impl FilesystemApi<'_> {
    pub async fn home_dir(&self) -> Result<String, StageError> {
        require_string(
            self.0
                .request("ctx.filesystem.homeDir", BTreeMap::new())
                .await?,
            "filesystem.homeDir",
        )
    }
    pub async fn temp_dir(&self) -> Result<String, StageError> {
        require_string(
            self.0
                .request("ctx.filesystem.tempDir", BTreeMap::new())
                .await?,
            "filesystem.tempDir",
        )
    }
    pub async fn read_absolute(&self, path: &str) -> Result<Vec<u8>, StageError> {
        result_bytes(
            self.0
                .request(
                    "ctx.filesystem.readAbsolute",
                    BTreeMap::from([("path".into(), path.into())]),
                )
                .await?,
            "filesystem.readAbsolute",
        )
    }
    pub async fn read_absolute_bounded(
        &self,
        path: &str,
        max_bytes: usize,
    ) -> Result<Vec<u8>, StageError> {
        assert_bound(max_bytes)?;
        let value = self.read_absolute(path).await?;
        if value.len() > max_bytes {
            return Err(StageError::new(
                "RESOURCE_LIMIT_EXCEEDED",
                "filesystem value exceeds requested bound",
            ));
        }
        Ok(value)
    }
    pub async fn write_absolute(&self, path: &str, bytes: &[u8]) -> Result<(), StageError> {
        assert_bytes(bytes)?;
        self.0
            .request(
                "ctx.filesystem.writeAbsolute",
                BTreeMap::from([
                    ("path".into(), path.into()),
                    ("bytes".into(), encode(bytes, &STANDARD).into()),
                ]),
            )
            .await?;
        Ok(())
    }
}

pub struct NetworkResponse {
    pub status: i64,
    pub status_text: String,
    pub headers: BTreeMap<String, String>,
    pub url: String,
    bytes: Vec<u8>,
}
impl NetworkResponse {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

pub struct NetworkApi<'a>(&'a StageContext);
impl NetworkApi<'_> {
    pub async fn fetch(&self, url: &str, init: WireValue) -> Result<NetworkResponse, StageError> {
        let value = self
            .0
            .request(
                "ctx.network.fetch",
                BTreeMap::from([("url".into(), url.into()), ("init".into(), init)]),
            )
            .await?;
        let object = match value {
            WireValue::Object(value) => value,
            _ => {
                return Err(StageError::new(
                    "PLUGIN_PROTOCOL_ERROR",
                    "network.fetch result is malformed",
                ))
            }
        };
        let status = object
            .get("status")
            .and_then(WireValue::as_i64)
            .ok_or_else(|| {
                StageError::new("PLUGIN_PROTOCOL_ERROR", "network.fetch result is malformed")
            })?;
        let status_text = object
            .get("statusText")
            .and_then(WireValue::as_str)
            .unwrap_or("")
            .to_owned();
        let response_url = object
            .get("url")
            .and_then(WireValue::as_str)
            .unwrap_or(url)
            .to_owned();
        let headers = match object.get("headers") {
            Some(WireValue::Object(values)) => values
                .iter()
                .map(|(key, value)| {
                    value
                        .as_str()
                        .map(|value| (key.clone(), value.to_owned()))
                        .ok_or_else(|| {
                            StageError::new(
                                "PLUGIN_PROTOCOL_ERROR",
                                "network.fetch headers are malformed",
                            )
                        })
                })
                .collect::<Result<_, _>>()?,
            None => BTreeMap::new(),
            _ => {
                return Err(StageError::new(
                    "PLUGIN_PROTOCOL_ERROR",
                    "network.fetch headers are malformed",
                ))
            }
        };
        let bytes = decode_bytes(object.get("bytes"), "network.fetch bytes")?;
        Ok(NetworkResponse {
            status,
            status_text,
            headers,
            url: response_url,
            bytes,
        })
    }
}

pub struct ShellResult {
    pub exit_code: i64,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}
pub struct ShellApi<'a>(&'a StageContext);
impl ShellApi<'_> {
    pub async fn run(
        &self,
        command: &str,
        args: &[String],
        options: WireValue,
    ) -> Result<ShellResult, StageError> {
        let value = self
            .0
            .request(
                "ctx.shell.run",
                BTreeMap::from([
                    ("command".into(), command.into()),
                    (
                        "args".into(),
                        WireValue::Array(args.iter().cloned().map(Into::into).collect()),
                    ),
                    ("options".into(), options),
                ]),
            )
            .await?;
        let object = match value {
            WireValue::Object(value) => value,
            _ => {
                return Err(StageError::new(
                    "PLUGIN_PROTOCOL_ERROR",
                    "shell.run result is malformed",
                ))
            }
        };
        let exit_code = object
            .get("exitCode")
            .and_then(WireValue::as_i64)
            .ok_or_else(|| {
                StageError::new("PLUGIN_PROTOCOL_ERROR", "shell.run result is malformed")
            })?;
        Ok(ShellResult {
            exit_code,
            stdout: decode_bytes(object.get("stdout"), "shell.run stdout")?,
            stderr: decode_bytes(object.get("stderr"), "shell.run stderr")?,
        })
    }
}

pub struct Logger<'a>(&'a StageContext);
impl Logger<'_> {
    pub async fn info(&self, message: &str, fields: WireValue) -> Result<(), StageError> {
        self.0
            .peer
            .notify(
                "log",
                object([
                    ("level", "info".into()),
                    ("message", message.into()),
                    ("fields", fields),
                ]),
            )
            .await
            .map_err(|_| StageError::new("PLUGIN_PROTOCOL_ERROR", "log notification failed"))
    }
}

fn decode_bytes(value: Option<&WireValue>, label: &str) -> Result<Vec<u8>, StageError> {
    let value = value
        .and_then(WireValue::as_str)
        .ok_or_else(|| StageError::new("PLUGIN_PROTOCOL_ERROR", format!("{label} is malformed")))?;
    let max_encoded = MAX_MEDIATED_BYTES
        .checked_add(2)
        .and_then(|length| length.checked_div(3))
        .and_then(|length| length.checked_mul(4))
        .expect("mediated byte bound fits usize");
    if value.len() > max_encoded {
        return Err(StageError::new(
            "PLUGIN_PROTOCOL_ERROR",
            format!("{label} is malformed"),
        ));
    }
    let bytes = decode(value, &STANDARD)
        .map_err(|_| StageError::new("PLUGIN_PROTOCOL_ERROR", format!("{label} is malformed")))?;
    if bytes.len() > MAX_MEDIATED_BYTES || encode(&bytes, &STANDARD) != value {
        return Err(StageError::new(
            "PLUGIN_PROTOCOL_ERROR",
            format!("{label} is malformed"),
        ));
    }
    Ok(bytes)
}
fn result_bytes(value: WireValue, label: &str) -> Result<Vec<u8>, StageError> {
    match value {
        WireValue::Object(value) => decode_bytes(value.get("bytes"), label),
        _ => Err(StageError::new(
            "PLUGIN_PROTOCOL_ERROR",
            format!("{label} result is malformed"),
        )),
    }
}
fn require_string(value: WireValue, label: &str) -> Result<String, StageError> {
    match value {
        WireValue::String(value) => Ok(value),
        _ => Err(StageError::new(
            "PLUGIN_PROTOCOL_ERROR",
            format!("{label} result is malformed"),
        )),
    }
}
fn require_array(value: WireValue, label: &str) -> Result<Vec<WireValue>, StageError> {
    match value {
        WireValue::Array(value) => Ok(value),
        _ => Err(StageError::new(
            "PLUGIN_PROTOCOL_ERROR",
            format!("{label} result is malformed"),
        )),
    }
}
fn require_object(value: WireValue, label: &str) -> Result<WireValue, StageError> {
    match value {
        WireValue::Object(_) => Ok(value),
        _ => Err(StageError::new(
            "PLUGIN_PROTOCOL_ERROR",
            format!("{label} result is malformed"),
        )),
    }
}
fn assert_bound(value: usize) -> Result<(), StageError> {
    if value > MAX_MEDIATED_BYTES {
        Err(StageError::new(
            "RESOURCE_LIMIT_EXCEEDED",
            "requested bound exceeds mediated byte limit",
        ))
    } else {
        Ok(())
    }
}
fn assert_bytes(value: &[u8]) -> Result<(), StageError> {
    if value.len() > MAX_MEDIATED_BYTES {
        Err(StageError::new(
            "RESOURCE_LIMIT_EXCEEDED",
            "mediated bytes exceed configured bound",
        ))
    } else {
        Ok(())
    }
}
