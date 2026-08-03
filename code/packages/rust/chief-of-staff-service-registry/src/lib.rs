//! Durable host intent and observation registry for the D18 Chief orchestrator.
//!
//! The registry is deliberately a cache, not the live process authority. It
//! persists enough immutable package identity and restart intent to reconcile
//! after a crash, plus the orchestrator's last bounded observation. The process
//! supervisor must still verify whether a cached PID is alive, and channel
//! membership remains authoritative in `chief-of-staff-channel-endpoints`.

#![forbid(unsafe_code)]

use core::fmt::{self, Display, Formatter};

use chief_of_staff_channel_crypto::ChannelId;
use coding_adventures_json_value::JsonValue;
use storage_core::{
    Revision, StorageBackend, StorageError, StorageListOptions, StoragePutInput, StorageRecord,
};

const REGISTRY_NAMESPACE: &str = "chief-service-registry";
const HOST_PREFIX: &str = "hosts/";
const HOST_CONTENT_TYPE: &str = "application/vnd.coding-adventures.chief-host-v1";
const MAGIC: &[u8; 4] = b"D18R";
const VERSION: u8 = 1;
const MAX_HOST_NAME_BYTES: usize = 64;
const MAX_PACKAGE_PATH_BYTES: usize = 4096;
const MAX_REASON_BYTES: usize = 512;
const MAX_RECORD_BYTES: usize = 8192;
const MAX_HOSTS: usize = 4096;

/// Stable lowercase host name used as the registry key.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HostName(String);

impl HostName {
    /// Validate the agent-manifest host-name grammar.
    pub fn new(value: impl Into<String>) -> Result<Self, RegistryError> {
        let value = value.into();
        let bytes = value.as_bytes();
        if bytes.len() < 2 || bytes.len() > MAX_HOST_NAME_BYTES {
            return Err(RegistryError::invalid(
                "host_name",
                "must contain between 2 and 64 bytes",
            ));
        }
        if !bytes[0].is_ascii_lowercase()
            || !bytes
                .iter()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
        {
            return Err(RegistryError::invalid(
                "host_name",
                "must start with a lowercase letter and contain only lowercase letters, digits, or hyphens",
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for HostName {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Portable UTF-8 package path persisted independently of the local OS path ABI.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackagePath(String);

impl PackagePath {
    pub fn new(value: impl Into<String>) -> Result<Self, RegistryError> {
        let value = value.into();
        if value.is_empty() || value.len() > MAX_PACKAGE_PATH_BYTES {
            return Err(RegistryError::invalid(
                "package_path",
                "must contain between 1 and 4096 UTF-8 bytes",
            ));
        }
        if value.chars().any(char::is_control) {
            return Err(RegistryError::invalid(
                "package_path",
                "must not contain control characters",
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Durable relaunch policy copied from the signed package manifest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestartPolicy {
    Always,
    OnFailure,
    Never,
}

/// Durable operator intent. Observed state may temporarily disagree during reconciliation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesiredState {
    Running,
    Stopped,
}

/// Last lifecycle observation recorded by the orchestrator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostStatus {
    Starting,
    Running,
    Restarting,
    Stopping,
    Stopped,
    Crashed { exit_code: Option<i32> },
    Quarantined { until_ns: u64, reason: String },
}

/// Immutable package identity supplied at registration time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostRegistration {
    host_name: HostName,
    package_path: PackagePath,
    package_hash: [u8; 32],
    restart_policy: RestartPolicy,
}

impl HostRegistration {
    pub fn new(
        host_name: HostName,
        package_path: PackagePath,
        package_hash: [u8; 32],
        restart_policy: RestartPolicy,
    ) -> Self {
        Self {
            host_name,
            package_path,
            package_hash,
            restart_policy,
        }
    }

    pub fn host_name(&self) -> &HostName {
        &self.host_name
    }

    pub fn package_path(&self) -> &PackagePath {
        &self.package_path
    }

    pub fn package_hash(&self) -> &[u8; 32] {
        &self.package_hash
    }

    pub fn restart_policy(&self) -> RestartPolicy {
        self.restart_policy
    }
}

/// Cached process observation. Every field is evidence to be reverified after restart.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostObservation {
    status: HostStatus,
    process_id: Option<u32>,
    started_at_ns: Option<u64>,
    last_heartbeat_ns: Option<u64>,
    control_channel_id: Option<ChannelId>,
    restart_count: u32,
    last_restart_ns: Option<u64>,
}

impl HostObservation {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        status: HostStatus,
        process_id: Option<u32>,
        started_at_ns: Option<u64>,
        last_heartbeat_ns: Option<u64>,
        control_channel_id: Option<ChannelId>,
        restart_count: u32,
        last_restart_ns: Option<u64>,
    ) -> Result<Self, RegistryError> {
        if process_id == Some(0) {
            return Err(RegistryError::invalid("process_id", "must be non-zero"));
        }
        if last_heartbeat_ns.is_some() && started_at_ns.is_none() {
            return Err(RegistryError::invalid(
                "last_heartbeat_ns",
                "requires started_at_ns",
            ));
        }
        if let (Some(started), Some(heartbeat)) = (started_at_ns, last_heartbeat_ns) {
            if heartbeat < started {
                return Err(RegistryError::invalid(
                    "last_heartbeat_ns",
                    "must not precede started_at_ns",
                ));
            }
        }
        if restart_count == 0 && last_restart_ns.is_some()
            || restart_count > 0 && last_restart_ns.is_none()
        {
            return Err(RegistryError::invalid(
                "last_restart_ns",
                "must be present exactly when restart_count is non-zero",
            ));
        }
        if let Some(channel_id) = control_channel_id {
            validate_uuid_v7(channel_id)?;
        }
        match &status {
            HostStatus::Running => {
                if process_id.is_none()
                    || started_at_ns.is_none()
                    || last_heartbeat_ns.is_none()
                    || control_channel_id.is_none()
                {
                    return Err(RegistryError::invalid(
                        "status",
                        "running requires process_id, start, heartbeat, and control channel",
                    ));
                }
            }
            HostStatus::Stopped | HostStatus::Crashed { .. } | HostStatus::Quarantined { .. } => {
                if process_id.is_some() || control_channel_id.is_some() {
                    return Err(RegistryError::invalid(
                        "status",
                        "inactive status must not retain a process ID or control channel",
                    ));
                }
            }
            HostStatus::Starting | HostStatus::Restarting | HostStatus::Stopping => {}
        }
        if let HostStatus::Quarantined { reason, .. } = &status {
            validate_reason(reason)?;
        }
        Ok(Self {
            status,
            process_id,
            started_at_ns,
            last_heartbeat_ns,
            control_channel_id,
            restart_count,
            last_restart_ns,
        })
    }

    pub fn stopped() -> Self {
        Self {
            status: HostStatus::Stopped,
            process_id: None,
            started_at_ns: None,
            last_heartbeat_ns: None,
            control_channel_id: None,
            restart_count: 0,
            last_restart_ns: None,
        }
    }

    pub fn status(&self) -> &HostStatus {
        &self.status
    }

    pub fn process_id(&self) -> Option<u32> {
        self.process_id
    }

    pub fn started_at_ns(&self) -> Option<u64> {
        self.started_at_ns
    }

    pub fn last_heartbeat_ns(&self) -> Option<u64> {
        self.last_heartbeat_ns
    }

    pub fn control_channel_id(&self) -> Option<ChannelId> {
        self.control_channel_id
    }

    pub fn restart_count(&self) -> u32 {
        self.restart_count
    }

    pub fn last_restart_ns(&self) -> Option<u64> {
        self.last_restart_ns
    }
}

/// One durable registry entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostEntry {
    registration: HostRegistration,
    desired_state: DesiredState,
    observation: HostObservation,
}

impl HostEntry {
    pub fn registered(registration: HostRegistration, desired_state: DesiredState) -> Self {
        Self {
            registration,
            desired_state,
            observation: HostObservation::stopped(),
        }
    }

    pub fn new(
        registration: HostRegistration,
        desired_state: DesiredState,
        observation: HostObservation,
    ) -> Self {
        Self {
            registration,
            desired_state,
            observation,
        }
    }

    pub fn registration(&self) -> &HostRegistration {
        &self.registration
    }

    pub fn desired_state(&self) -> DesiredState {
        self.desired_state
    }

    pub fn observation(&self) -> &HostObservation {
        &self.observation
    }

    pub fn with_desired_state(mut self, desired_state: DesiredState) -> Self {
        self.desired_state = desired_state;
        self
    }

    pub fn with_observation(mut self, observation: HostObservation) -> Self {
        self.observation = observation;
        self
    }
}

/// One loaded entry plus the revision required for a safe update or delete.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadedHost {
    entry: HostEntry,
    revision: Revision,
}

impl LoadedHost {
    pub fn entry(&self) -> &HostEntry {
        &self.entry
    }

    pub fn revision(&self) -> &Revision {
        &self.revision
    }
}

/// Registry failures. Diagnostics never contain record bodies.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegistryError {
    Storage(StorageError),
    InvalidField {
        field: &'static str,
        message: String,
    },
    HostNotFound(HostName),
    ConflictingRegistration(HostName),
    ConcurrentUpdate(HostName),
    CorruptRecord(String),
    TooManyHosts,
}

impl RegistryError {
    fn invalid(field: &'static str, message: impl Into<String>) -> Self {
        Self::InvalidField {
            field,
            message: message.into(),
        }
    }
}

impl Display for RegistryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Storage(error) => write!(formatter, "{error}"),
            Self::InvalidField { field, message } => {
                write!(formatter, "invalid {field}: {message}")
            }
            Self::HostNotFound(name) => write!(formatter, "host not found: {name}"),
            Self::ConflictingRegistration(name) => {
                write!(formatter, "different package already registered as {name}")
            }
            Self::ConcurrentUpdate(name) => {
                write!(formatter, "host changed concurrently: {name}")
            }
            Self::CorruptRecord(message) => write!(formatter, "corrupt host record: {message}"),
            Self::TooManyHosts => formatter.write_str("registry exceeds the 4096-host bound"),
        }
    }
}

impl std::error::Error for RegistryError {}

impl From<StorageError> for RegistryError {
    fn from(error: StorageError) -> Self {
        Self::Storage(error)
    }
}

/// CAS-backed service registry over an injected repository storage backend.
pub struct ServiceRegistry<'a> {
    backend: &'a dyn StorageBackend,
}

impl<'a> ServiceRegistry<'a> {
    pub fn new(backend: &'a dyn StorageBackend) -> Self {
        Self { backend }
    }

    /// Atomically register a host. Repeating the same package identity is idempotent.
    pub fn register(&self, entry: &HostEntry) -> Result<LoadedHost, RegistryError> {
        self.backend.initialize()?;
        let name = entry.registration.host_name.clone();
        let key = host_record_key(&name);
        let input = host_put(&key, encode_host_entry(entry))?.with_if_absent();
        match self.backend.put(input) {
            Ok(record) => decode_storage_record(record, &name),
            Err(StorageError::Conflict { .. }) => {
                let existing = self
                    .load(&name)?
                    .ok_or_else(|| RegistryError::HostNotFound(name.clone()))?;
                if existing.entry.registration == entry.registration {
                    Ok(existing)
                } else {
                    Err(RegistryError::ConflictingRegistration(name))
                }
            }
            Err(error) => Err(error.into()),
        }
    }

    pub fn load(&self, name: &HostName) -> Result<Option<LoadedHost>, RegistryError> {
        self.backend.initialize()?;
        self.backend
            .get(REGISTRY_NAMESPACE, &host_record_key(name))?
            .map(|record| decode_storage_record(record, name))
            .transpose()
    }

    /// List every entry in stable host-name order.
    pub fn list(&self) -> Result<Vec<LoadedHost>, RegistryError> {
        self.backend.initialize()?;
        let page = self.backend.list(
            REGISTRY_NAMESPACE,
            StorageListOptions {
                prefix: Some(HOST_PREFIX.to_string()),
                recursive: true,
                page_size: Some(MAX_HOSTS),
                cursor: None,
            },
        )?;
        if page.next_cursor.is_some() || page.records.len() > MAX_HOSTS {
            return Err(RegistryError::TooManyHosts);
        }
        page.records
            .into_iter()
            .map(|record| {
                let suffix = record.key.strip_prefix(HOST_PREFIX).ok_or_else(|| {
                    RegistryError::CorruptRecord("host key has the wrong prefix".to_string())
                })?;
                let name = HostName::new(suffix.to_string()).map_err(as_corrupt)?;
                decode_storage_record(record, &name)
            })
            .collect()
    }

    /// Replace one entry if it still has the caller's loaded revision.
    pub fn update(
        &self,
        loaded: &LoadedHost,
        replacement: &HostEntry,
    ) -> Result<LoadedHost, RegistryError> {
        let old_name = &loaded.entry.registration.host_name;
        if replacement.registration.host_name != *old_name {
            return Err(RegistryError::invalid(
                "host_name",
                "cannot change during an update",
            ));
        }
        if replacement.registration != loaded.entry.registration {
            return Err(RegistryError::invalid(
                "registration",
                "immutable package identity cannot change during an update",
            ));
        }
        let key = host_record_key(old_name);
        let input = host_put(&key, encode_host_entry(replacement))?
            .with_if_revision(Some(loaded.revision.clone()));
        match self.backend.put(input) {
            Ok(record) => decode_storage_record(record, old_name),
            Err(StorageError::Conflict { .. }) => {
                Err(RegistryError::ConcurrentUpdate(old_name.clone()))
            }
            Err(error) => Err(error.into()),
        }
    }

    /// Deregister exactly the revision the caller inspected.
    pub fn deregister(&self, loaded: &LoadedHost) -> Result<(), RegistryError> {
        let name = &loaded.entry.registration.host_name;
        match self.backend.delete(
            REGISTRY_NAMESPACE,
            &host_record_key(name),
            Some(&loaded.revision),
        ) {
            Ok(()) => Ok(()),
            Err(StorageError::Conflict { .. }) => {
                Err(RegistryError::ConcurrentUpdate(name.clone()))
            }
            Err(error) => Err(error.into()),
        }
    }
}

/// Stable storage key for one host.
pub fn host_record_key(name: &HostName) -> String {
    format!("{HOST_PREFIX}{}", name.as_str())
}

/// Encode one bounded version-1 host record.
pub fn encode_host_entry(entry: &HostEntry) -> Vec<u8> {
    let mut output = Vec::with_capacity(192);
    output.extend_from_slice(MAGIC);
    output.push(VERSION);
    push_string_u16(&mut output, entry.registration.host_name.as_str());
    push_string_u16(&mut output, entry.registration.package_path.as_str());
    output.extend_from_slice(&entry.registration.package_hash);
    output.push(restart_policy_tag(entry.registration.restart_policy));
    output.push(match entry.desired_state {
        DesiredState::Running => 1,
        DesiredState::Stopped => 2,
    });
    encode_status(&mut output, &entry.observation.status);
    push_option_u32(&mut output, entry.observation.process_id);
    push_option_u64(&mut output, entry.observation.started_at_ns);
    push_option_u64(&mut output, entry.observation.last_heartbeat_ns);
    match entry.observation.control_channel_id {
        Some(channel_id) => {
            output.push(1);
            output.extend_from_slice(&channel_id.0);
        }
        None => output.push(0),
    }
    output.extend_from_slice(&entry.observation.restart_count.to_be_bytes());
    push_option_u64(&mut output, entry.observation.last_restart_ns);
    output
}

/// Decode one strict version-1 host record.
pub fn decode_host_entry(bytes: &[u8]) -> Result<HostEntry, RegistryError> {
    if bytes.len() > MAX_RECORD_BYTES {
        return Err(RegistryError::CorruptRecord(
            "record exceeds the 8192-byte bound".to_string(),
        ));
    }
    let mut reader = Reader::new(bytes);
    if reader.take(4)? != MAGIC {
        return Err(RegistryError::CorruptRecord("invalid magic".to_string()));
    }
    if reader.u8()? != VERSION {
        return Err(RegistryError::CorruptRecord(
            "unsupported version".to_string(),
        ));
    }
    let host_name = HostName::new(reader.string_u16(MAX_HOST_NAME_BYTES)?).map_err(as_corrupt)?;
    let package_path =
        PackagePath::new(reader.string_u16(MAX_PACKAGE_PATH_BYTES)?).map_err(as_corrupt)?;
    let mut package_hash = [0u8; 32];
    package_hash.copy_from_slice(reader.take(32)?);
    let restart_policy = match reader.u8()? {
        1 => RestartPolicy::Always,
        2 => RestartPolicy::OnFailure,
        3 => RestartPolicy::Never,
        _ => {
            return Err(RegistryError::CorruptRecord(
                "invalid restart policy".to_string(),
            ))
        }
    };
    let desired_state = match reader.u8()? {
        1 => DesiredState::Running,
        2 => DesiredState::Stopped,
        _ => {
            return Err(RegistryError::CorruptRecord(
                "invalid desired state".to_string(),
            ))
        }
    };
    let status = decode_status(&mut reader)?;
    let process_id = reader.option_u32()?;
    let started_at_ns = reader.option_u64()?;
    let last_heartbeat_ns = reader.option_u64()?;
    let control_channel_id = match reader.u8()? {
        0 => None,
        1 => {
            let mut bytes = [0u8; 16];
            bytes.copy_from_slice(reader.take(16)?);
            Some(ChannelId(bytes))
        }
        _ => {
            return Err(RegistryError::CorruptRecord(
                "invalid channel option".to_string(),
            ))
        }
    };
    let restart_count = reader.u32()?;
    let last_restart_ns = reader.option_u64()?;
    reader.finish()?;
    let observation = HostObservation::new(
        status,
        process_id,
        started_at_ns,
        last_heartbeat_ns,
        control_channel_id,
        restart_count,
        last_restart_ns,
    )
    .map_err(as_corrupt)?;
    Ok(HostEntry::new(
        HostRegistration::new(host_name, package_path, package_hash, restart_policy),
        desired_state,
        observation,
    ))
}

fn host_put(key: &str, body: Vec<u8>) -> Result<StoragePutInput, StorageError> {
    StoragePutInput::new(
        REGISTRY_NAMESPACE,
        key,
        HOST_CONTENT_TYPE,
        JsonValue::Object(Vec::new()),
        body,
    )
}

fn decode_storage_record(
    record: StorageRecord,
    expected: &HostName,
) -> Result<LoadedHost, RegistryError> {
    if record.namespace != REGISTRY_NAMESPACE {
        return Err(RegistryError::CorruptRecord("wrong namespace".to_string()));
    }
    if record.key != host_record_key(expected) {
        return Err(RegistryError::CorruptRecord(
            "host key mismatch".to_string(),
        ));
    }
    if record.content_type != HOST_CONTENT_TYPE {
        return Err(RegistryError::CorruptRecord(
            "wrong content type".to_string(),
        ));
    }
    let entry = decode_host_entry(&record.body)?;
    if entry.registration.host_name != *expected {
        return Err(RegistryError::CorruptRecord(
            "body host name disagrees with storage key".to_string(),
        ));
    }
    Ok(LoadedHost {
        entry,
        revision: record.revision,
    })
}

fn validate_uuid_v7(channel_id: ChannelId) -> Result<(), RegistryError> {
    if channel_id.0[6] >> 4 != 7 || channel_id.0[8] & 0xc0 != 0x80 {
        return Err(RegistryError::invalid(
            "control_channel_id",
            "must be a canonical UUID v7",
        ));
    }
    Ok(())
}

fn validate_reason(reason: &str) -> Result<(), RegistryError> {
    if reason.is_empty() || reason.len() > MAX_REASON_BYTES {
        return Err(RegistryError::invalid(
            "quarantine_reason",
            "must contain between 1 and 512 UTF-8 bytes",
        ));
    }
    if reason.chars().any(char::is_control) {
        return Err(RegistryError::invalid(
            "quarantine_reason",
            "must not contain control characters",
        ));
    }
    Ok(())
}

fn as_corrupt(error: RegistryError) -> RegistryError {
    RegistryError::CorruptRecord(error.to_string())
}

fn restart_policy_tag(policy: RestartPolicy) -> u8 {
    match policy {
        RestartPolicy::Always => 1,
        RestartPolicy::OnFailure => 2,
        RestartPolicy::Never => 3,
    }
}

fn encode_status(output: &mut Vec<u8>, status: &HostStatus) {
    match status {
        HostStatus::Starting => output.push(1),
        HostStatus::Running => output.push(2),
        HostStatus::Restarting => output.push(3),
        HostStatus::Stopping => output.push(4),
        HostStatus::Stopped => output.push(5),
        HostStatus::Crashed { exit_code } => {
            output.push(6);
            match exit_code {
                Some(code) => {
                    output.push(1);
                    output.extend_from_slice(&code.to_be_bytes());
                }
                None => output.push(0),
            }
        }
        HostStatus::Quarantined { until_ns, reason } => {
            output.push(7);
            output.extend_from_slice(&until_ns.to_be_bytes());
            push_string_u16(output, reason);
        }
    }
}

fn decode_status(reader: &mut Reader<'_>) -> Result<HostStatus, RegistryError> {
    match reader.u8()? {
        1 => Ok(HostStatus::Starting),
        2 => Ok(HostStatus::Running),
        3 => Ok(HostStatus::Restarting),
        4 => Ok(HostStatus::Stopping),
        5 => Ok(HostStatus::Stopped),
        6 => {
            let exit_code = match reader.u8()? {
                0 => None,
                1 => Some(reader.i32()?),
                _ => {
                    return Err(RegistryError::CorruptRecord(
                        "invalid exit-code option".to_string(),
                    ))
                }
            };
            Ok(HostStatus::Crashed { exit_code })
        }
        7 => Ok(HostStatus::Quarantined {
            until_ns: reader.u64()?,
            reason: reader.string_u16(MAX_REASON_BYTES)?,
        }),
        _ => Err(RegistryError::CorruptRecord(
            "invalid host status".to_string(),
        )),
    }
}

fn push_string_u16(output: &mut Vec<u8>, value: &str) {
    output.extend_from_slice(&(value.len() as u16).to_be_bytes());
    output.extend_from_slice(value.as_bytes());
}

fn push_option_u32(output: &mut Vec<u8>, value: Option<u32>) {
    match value {
        Some(value) => {
            output.push(1);
            output.extend_from_slice(&value.to_be_bytes());
        }
        None => output.push(0),
    }
}

fn push_option_u64(output: &mut Vec<u8>, value: Option<u64>) {
    match value {
        Some(value) => {
            output.push(1);
            output.extend_from_slice(&value.to_be_bytes());
        }
        None => output.push(0),
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], RegistryError> {
        let end = self
            .position
            .checked_add(length)
            .ok_or_else(|| RegistryError::CorruptRecord("record length overflow".to_string()))?;
        if end > self.bytes.len() {
            return Err(RegistryError::CorruptRecord("truncated record".to_string()));
        }
        let slice = &self.bytes[self.position..end];
        self.position = end;
        Ok(slice)
    }

    fn u8(&mut self) -> Result<u8, RegistryError> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, RegistryError> {
        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(self.take(4)?);
        Ok(u32::from_be_bytes(bytes))
    }

    fn i32(&mut self) -> Result<i32, RegistryError> {
        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(self.take(4)?);
        Ok(i32::from_be_bytes(bytes))
    }

    fn u64(&mut self) -> Result<u64, RegistryError> {
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(self.take(8)?);
        Ok(u64::from_be_bytes(bytes))
    }

    fn option_u32(&mut self) -> Result<Option<u32>, RegistryError> {
        match self.u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.u32()?)),
            _ => Err(RegistryError::CorruptRecord(
                "invalid u32 option".to_string(),
            )),
        }
    }

    fn option_u64(&mut self) -> Result<Option<u64>, RegistryError> {
        match self.u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.u64()?)),
            _ => Err(RegistryError::CorruptRecord(
                "invalid u64 option".to_string(),
            )),
        }
    }

    fn string_u16(&mut self, maximum: usize) -> Result<String, RegistryError> {
        let mut length = [0u8; 2];
        length.copy_from_slice(self.take(2)?);
        let length = usize::from(u16::from_be_bytes(length));
        if length > maximum {
            return Err(RegistryError::CorruptRecord(
                "string exceeds its field bound".to_string(),
            ));
        }
        let value = core::str::from_utf8(self.take(length)?)
            .map_err(|_| RegistryError::CorruptRecord("invalid UTF-8".to_string()))?;
        Ok(value.to_string())
    }

    fn finish(self) -> Result<(), RegistryError> {
        if self.position != self.bytes.len() {
            return Err(RegistryError::CorruptRecord(
                "unexpected trailing bytes".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use storage_core::InMemoryStorageBackend;
    use storage_local_folder::LocalFolderStorageBackend;

    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn name(value: &str) -> HostName {
        HostName::new(value).unwrap()
    }

    fn channel_id() -> ChannelId {
        let mut bytes = [0u8; 16];
        bytes[0..6].copy_from_slice(&[1, 2, 3, 4, 5, 6]);
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        ChannelId(bytes)
    }

    fn registration(host: &str, hash_byte: u8) -> HostRegistration {
        HostRegistration::new(
            name(host),
            PackagePath::new(format!("agents/{host}.agent")).unwrap(),
            [hash_byte; 32],
            RestartPolicy::OnFailure,
        )
    }

    fn running_entry(host: &str) -> HostEntry {
        let observation = HostObservation::new(
            HostStatus::Running,
            Some(42),
            Some(100),
            Some(110),
            Some(channel_id()),
            1,
            Some(90),
        )
        .unwrap();
        HostEntry::new(registration(host, 7), DesiredState::Running, observation)
    }

    fn temporary_directory(label: &str) -> std::path::PathBuf {
        let nonce = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "chief-service-registry-{label}-{}-{now}-{nonce}",
            std::process::id()
        ))
    }

    #[test]
    fn names_paths_and_observations_fail_closed() {
        for invalid in ["", "a", "Upper", "-host", "host_name"] {
            assert!(HostName::new(invalid).is_err(), "accepted {invalid:?}");
        }
        assert!(PackagePath::new("").is_err());
        assert!(PackagePath::new("agent\npath").is_err());
        assert!(HostObservation::new(
            HostStatus::Running,
            Some(0),
            Some(1),
            Some(1),
            Some(channel_id()),
            0,
            None,
        )
        .is_err());
        assert!(HostObservation::new(
            HostStatus::Running,
            Some(1),
            Some(2),
            Some(1),
            Some(channel_id()),
            0,
            None,
        )
        .is_err());
        assert!(
            HostObservation::new(HostStatus::Stopped, Some(1), None, None, None, 0, None,).is_err()
        );
    }

    #[test]
    fn codec_round_trips_every_status_shape() {
        let statuses = vec![
            HostStatus::Starting,
            HostStatus::Running,
            HostStatus::Restarting,
            HostStatus::Stopping,
            HostStatus::Stopped,
            HostStatus::Crashed { exit_code: None },
            HostStatus::Crashed {
                exit_code: Some(-9),
            },
            HostStatus::Quarantined {
                until_ns: 999,
                reason: "panic threshold".to_string(),
            },
        ];
        for status in statuses {
            let active = matches!(status, HostStatus::Running);
            let observation = HostObservation::new(
                status,
                active.then_some(42),
                active.then_some(100),
                active.then_some(110),
                active.then_some(channel_id()),
                0,
                None,
            )
            .unwrap();
            let entry = HostEntry::new(
                registration("mail-host", 3),
                DesiredState::Running,
                observation,
            );
            assert_eq!(
                decode_host_entry(&encode_host_entry(&entry)).unwrap(),
                entry
            );
        }
    }

    #[test]
    fn codec_rejects_every_truncated_prefix_and_trailing_bytes() {
        let encoded = encode_host_entry(&running_entry("mail-host"));
        for length in 0..encoded.len() {
            assert!(
                decode_host_entry(&encoded[..length]).is_err(),
                "accepted {length}"
            );
        }
        let mut padded = encoded;
        padded.push(0);
        assert!(decode_host_entry(&padded).is_err());
    }

    #[test]
    fn codec_rejects_magic_version_uuid_and_size_corruption() {
        let entry = running_entry("mail-host");
        let mut encoded = encode_host_entry(&entry);
        encoded[0] ^= 1;
        assert!(decode_host_entry(&encoded).is_err());
        let mut encoded = encode_host_entry(&entry);
        encoded[4] = 2;
        assert!(decode_host_entry(&encoded).is_err());
        let mut encoded = encode_host_entry(&entry);
        let channel_offset = encoded
            .windows(16)
            .position(|window| window == channel_id().0)
            .unwrap();
        encoded[channel_offset + 6] = 0x40;
        assert!(decode_host_entry(&encoded).is_err());
        assert!(decode_host_entry(&vec![0; MAX_RECORD_BYTES + 1]).is_err());
    }

    #[test]
    fn registration_lookup_and_listing_are_stable() {
        let backend = InMemoryStorageBackend::new();
        let registry = ServiceRegistry::new(&backend);
        registry
            .register(&HostEntry::registered(
                registration("zeta-host", 1),
                DesiredState::Running,
            ))
            .unwrap();
        registry
            .register(&HostEntry::registered(
                registration("alpha-host", 2),
                DesiredState::Stopped,
            ))
            .unwrap();
        let listed = registry.list().unwrap();
        let names: Vec<_> = listed
            .iter()
            .map(|loaded| loaded.entry().registration().host_name().as_str())
            .collect();
        assert_eq!(names, vec!["alpha-host", "zeta-host"]);
        assert!(registry.load(&name("missing-host")).unwrap().is_none());
    }

    #[test]
    fn registration_is_idempotent_after_observation_changes() {
        let backend = InMemoryStorageBackend::new();
        let registry = ServiceRegistry::new(&backend);
        let initial = HostEntry::registered(registration("mail-host", 7), DesiredState::Running);
        let loaded = registry.register(&initial).unwrap();
        let updated = registry
            .update(&loaded, &running_entry("mail-host"))
            .unwrap();
        let retried = registry.register(&initial).unwrap();
        assert_eq!(retried, updated);
    }

    #[test]
    fn conflicting_package_identity_is_rejected() {
        let backend = InMemoryStorageBackend::new();
        let registry = ServiceRegistry::new(&backend);
        registry
            .register(&HostEntry::registered(
                registration("mail-host", 1),
                DesiredState::Running,
            ))
            .unwrap();
        let error = registry
            .register(&HostEntry::registered(
                registration("mail-host", 2),
                DesiredState::Running,
            ))
            .unwrap_err();
        assert!(matches!(error, RegistryError::ConflictingRegistration(_)));
    }

    #[test]
    fn stale_updates_and_deletes_fail_cas() {
        let backend = InMemoryStorageBackend::new();
        let registry = ServiceRegistry::new(&backend);
        let loaded = registry.register(&running_entry("mail-host")).unwrap();
        let stopped = loaded
            .entry()
            .clone()
            .with_desired_state(DesiredState::Stopped)
            .with_observation(HostObservation::stopped());
        registry.update(&loaded, &stopped).unwrap();
        assert!(matches!(
            registry.update(&loaded, &stopped),
            Err(RegistryError::ConcurrentUpdate(_))
        ));
        assert!(matches!(
            registry.deregister(&loaded),
            Err(RegistryError::ConcurrentUpdate(_))
        ));
    }

    #[test]
    fn deregistration_removes_the_loaded_revision() {
        let backend = InMemoryStorageBackend::new();
        let registry = ServiceRegistry::new(&backend);
        let loaded = registry.register(&running_entry("mail-host")).unwrap();
        registry.deregister(&loaded).unwrap();
        assert!(registry.load(&name("mail-host")).unwrap().is_none());
    }

    #[test]
    fn immutable_registration_cannot_change_during_update() {
        let backend = InMemoryStorageBackend::new();
        let registry = ServiceRegistry::new(&backend);
        let loaded = registry.register(&running_entry("mail-host")).unwrap();
        let replacement =
            HostEntry::registered(registration("mail-host", 99), DesiredState::Running);
        assert!(matches!(
            registry.update(&loaded, &replacement),
            Err(RegistryError::InvalidField { .. })
        ));
    }

    #[test]
    fn local_folder_restart_recovers_intent_and_observation() {
        let root = temporary_directory("restart");
        let expected = running_entry("mail-host");
        {
            let backend = LocalFolderStorageBackend::new(&root);
            let registry = ServiceRegistry::new(&backend);
            registry.register(&expected).unwrap();
        }
        {
            let backend = LocalFolderStorageBackend::new(&root);
            let registry = ServiceRegistry::new(&backend);
            let recovered = registry.load(&name("mail-host")).unwrap().unwrap();
            assert_eq!(recovered.entry(), &expected);
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn storage_content_type_and_body_name_are_verified() {
        let backend = InMemoryStorageBackend::new();
        backend.initialize().unwrap();
        let entry = running_entry("mail-host");
        let key = host_record_key(&name("mail-host"));
        backend
            .put(
                StoragePutInput::new(
                    REGISTRY_NAMESPACE,
                    &key,
                    "application/octet-stream",
                    JsonValue::Object(Vec::new()),
                    encode_host_entry(&entry),
                )
                .unwrap(),
            )
            .unwrap();
        let registry = ServiceRegistry::new(&backend);
        assert!(matches!(
            registry.load(&name("mail-host")),
            Err(RegistryError::CorruptRecord(_))
        ));

        let backend = InMemoryStorageBackend::new();
        backend.initialize().unwrap();
        backend
            .put(host_put(&key, encode_host_entry(&running_entry("other-host"))).unwrap())
            .unwrap();
        let registry = ServiceRegistry::new(&backend);
        assert!(matches!(
            registry.load(&name("mail-host")),
            Err(RegistryError::CorruptRecord(_))
        ));
    }

    #[test]
    fn host_key_is_stable_and_namespaced() {
        assert_eq!(host_record_key(&name("mail-host")), "hosts/mail-host");
    }

    #[test]
    fn public_views_expose_the_persisted_contract() {
        let entry = running_entry("mail-host");
        let registration = entry.registration();
        assert_eq!(registration.host_name().to_string(), "mail-host");
        assert_eq!(
            registration.package_path().as_str(),
            "agents/mail-host.agent"
        );
        assert_eq!(registration.package_hash(), &[7; 32]);
        assert_eq!(registration.restart_policy(), RestartPolicy::OnFailure);
        assert_eq!(entry.desired_state(), DesiredState::Running);

        let observation = entry.observation();
        assert_eq!(observation.status(), &HostStatus::Running);
        assert_eq!(observation.process_id(), Some(42));
        assert_eq!(observation.started_at_ns(), Some(100));
        assert_eq!(observation.last_heartbeat_ns(), Some(110));
        assert_eq!(observation.control_channel_id(), Some(channel_id()));
        assert_eq!(observation.restart_count(), 1);
        assert_eq!(observation.last_restart_ns(), Some(90));

        let backend = InMemoryStorageBackend::new();
        let loaded = ServiceRegistry::new(&backend).register(&entry).unwrap();
        assert!(!loaded.revision().as_str().is_empty());
    }

    #[test]
    fn observation_validation_covers_restart_uuid_running_and_quarantine_rules() {
        assert!(
            HostObservation::new(HostStatus::Starting, None, None, Some(1), None, 0, None,)
                .is_err()
        );
        assert!(
            HostObservation::new(HostStatus::Starting, None, None, None, None, 1, None,).is_err()
        );
        assert!(
            HostObservation::new(HostStatus::Starting, None, None, None, None, 0, Some(1),)
                .is_err()
        );
        assert!(HostObservation::new(
            HostStatus::Starting,
            None,
            None,
            None,
            Some(ChannelId([0; 16])),
            0,
            None,
        )
        .is_err());
        assert!(
            HostObservation::new(HostStatus::Running, None, None, None, None, 0, None,).is_err()
        );
        assert!(HostObservation::new(
            HostStatus::Quarantined {
                until_ns: 5,
                reason: String::new(),
            },
            None,
            None,
            None,
            None,
            0,
            None,
        )
        .is_err());
        assert!(HostObservation::new(
            HostStatus::Quarantined {
                until_ns: 5,
                reason: "bad\nreason".to_string(),
            },
            None,
            None,
            None,
            None,
            0,
            None,
        )
        .is_err());
    }

    #[test]
    fn codec_rejects_invalid_tags_options_utf8_and_lengths() {
        let entry = running_entry("mail-host");
        let encoded = encode_host_entry(&entry);
        let host_length_offset = 5;
        let host_offset = host_length_offset + 2;
        let path_length_offset = host_offset + "mail-host".len();
        let path_offset = path_length_offset + 2;
        let restart_offset = path_offset + "agents/mail-host.agent".len() + 32;
        let desired_offset = restart_offset + 1;
        let status_offset = desired_offset + 1;
        let process_option_offset = status_offset + 1;
        let started_option_offset = process_option_offset + 1 + 4;
        let heartbeat_option_offset = started_option_offset + 1 + 8;
        let channel_option_offset = heartbeat_option_offset + 1 + 8;
        let last_restart_option_offset = channel_option_offset + 1 + 16 + 4;

        for (offset, invalid) in [
            (restart_offset, 9),
            (desired_offset, 9),
            (status_offset, 9),
            (process_option_offset, 2),
            (started_option_offset, 2),
            (heartbeat_option_offset, 2),
            (channel_option_offset, 2),
            (last_restart_option_offset, 2),
        ] {
            let mut malformed = encoded.clone();
            malformed[offset] = invalid;
            assert!(
                decode_host_entry(&malformed).is_err(),
                "accepted offset {offset}"
            );
        }

        let mut invalid_utf8 = encoded.clone();
        invalid_utf8[host_offset] = 0xff;
        assert!(decode_host_entry(&invalid_utf8).is_err());

        let mut oversized_path = encoded;
        oversized_path[path_length_offset..path_length_offset + 2]
            .copy_from_slice(&u16::MAX.to_be_bytes());
        assert!(decode_host_entry(&oversized_path).is_err());

        let crashed = HostEntry::new(
            registration("mail-host", 7),
            DesiredState::Running,
            HostObservation::new(
                HostStatus::Crashed { exit_code: Some(9) },
                None,
                None,
                None,
                None,
                0,
                None,
            )
            .unwrap(),
        );
        let mut malformed_exit = encode_host_entry(&crashed);
        malformed_exit[status_offset + 1] = 2;
        assert!(decode_host_entry(&malformed_exit).is_err());
    }

    #[test]
    fn codec_round_trips_all_restart_and_desired_state_tags() {
        for policy in [
            RestartPolicy::Always,
            RestartPolicy::OnFailure,
            RestartPolicy::Never,
        ] {
            for desired in [DesiredState::Running, DesiredState::Stopped] {
                let registration = HostRegistration::new(
                    name("mail-host"),
                    PackagePath::new("agents/mail-host.agent").unwrap(),
                    [1; 32],
                    policy,
                );
                let entry = HostEntry::registered(registration, desired);
                assert_eq!(
                    decode_host_entry(&encode_host_entry(&entry)).unwrap(),
                    entry
                );
            }
        }
    }

    #[test]
    fn registry_errors_have_bounded_actionable_diagnostics() {
        let errors = vec![
            RegistryError::from(StorageError::Unavailable {
                message: "offline".to_string(),
            }),
            RegistryError::invalid("field", "bad"),
            RegistryError::HostNotFound(name("mail-host")),
            RegistryError::ConflictingRegistration(name("mail-host")),
            RegistryError::ConcurrentUpdate(name("mail-host")),
            RegistryError::CorruptRecord("bad bytes".to_string()),
            RegistryError::TooManyHosts,
        ];
        for error in errors {
            assert!(!error.to_string().is_empty());
        }
    }

    #[test]
    fn update_rejects_a_different_host_name() {
        let backend = InMemoryStorageBackend::new();
        let registry = ServiceRegistry::new(&backend);
        let loaded = registry.register(&running_entry("mail-host")).unwrap();
        assert!(matches!(
            registry.update(&loaded, &running_entry("other-host")),
            Err(RegistryError::InvalidField {
                field: "host_name",
                ..
            })
        ));
    }

    #[test]
    fn reader_detects_position_overflow() {
        let mut reader = Reader {
            bytes: &[],
            position: usize::MAX,
        };
        assert!(reader.take(1).is_err());
    }
}
