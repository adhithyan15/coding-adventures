//! Durable, manifest-blind launch authority for D18 Chief pipelines.
//!
//! Host records are revision-CAS protected and exact-package bound. Immutable
//! channel claims prevent a UUID from crossing pipeline boundaries. Both wiring
//! and launch resolution reload authoritative channel membership and fail closed.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use chief_of_staff_channel_crypto::wire::MAX_IDENTITY_BYTES;
use chief_of_staff_channel_crypto::ChannelId;
use chief_of_staff_channel_endpoints::{
    AgentId, ChannelDefinition, ChannelDefinitionStore, ChannelEndpointError, ChannelLifecycle,
};
use chief_of_staff_host_control_protocol::{
    ChannelBinding, ChannelBindingAccess, LaunchBindings, LevelOneModelBinding,
    MAX_LAUNCH_CHANNEL_BINDINGS, MAX_LAUNCH_CHANNEL_NAME_BYTES, MAX_LAUNCH_MODEL_BYTES,
};
use chief_of_staff_service_registry::{
    HostName, HostRegistration, PackagePath, RegistryError, RestartPolicy, ServiceRegistry,
};
use coding_adventures_json_value::JsonValue;
use core::fmt::{self, Display, Formatter};
use storage_core::{Revision, StorageBackend, StorageError, StoragePutInput, StorageRecord};

const NAMESPACE: &str = "chief-pipeline-bindings";
const HOST_PREFIX: &str = "hosts/";
const CHANNEL_PREFIX: &str = "channels/";
const HOST_CONTENT_TYPE: &str = "application/vnd.coding-adventures.chief-host-bindings-v1";
const CLAIM_CONTENT_TYPE: &str = "application/vnd.coding-adventures.chief-channel-claim-v1";
const HOST_MAGIC: &[u8; 4] = b"D18B";
const CLAIM_MAGIC: &[u8; 4] = b"D18P";
const VERSION: u8 = 1;
const MAX_HOST_NAME_BYTES: usize = 64;
const MAX_PACKAGE_PATH_BYTES: usize = 4096;
const MAX_HOST_RECORD_BYTES: usize = 32 * 1024;
const CLAIM_RECORD_BYTES: usize = 37;

/// Canonical UUID-v7 identity for one isolated pipeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PipelineId([u8; 16]);

impl PipelineId {
    /// Validate and own one canonical UUID-v7 pipeline identity.
    pub fn new(bytes: [u8; 16]) -> Result<Self, PipelineBindingError> {
        if !is_uuid_v7(&bytes) {
            return Err(PipelineBindingError::InvalidPipelineId);
        }
        Ok(Self(bytes))
    }

    /// Return the canonical 16-byte identity.
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

/// One exact host package's durable pipeline launch authority.
#[derive(Clone, Debug, PartialEq)]
pub struct HostPipelineBinding {
    pipeline_id: PipelineId,
    registration: HostRegistration,
    agent_id: AgentId,
    launch_bindings: LaunchBindings,
}

impl HostPipelineBinding {
    /// Construct one already bounded binding record.
    pub fn new(
        pipeline_id: PipelineId,
        registration: HostRegistration,
        agent_id: AgentId,
        launch_bindings: LaunchBindings,
    ) -> Self {
        Self {
            pipeline_id,
            registration,
            agent_id,
            launch_bindings,
        }
    }

    /// Return the isolated pipeline identity.
    pub fn pipeline_id(&self) -> PipelineId {
        self.pipeline_id
    }

    /// Return the exact durable host registration.
    pub fn registration(&self) -> &HostRegistration {
        &self.registration
    }

    /// Return the channel-membership identity authorized by pipeline wiring.
    pub fn agent_id(&self) -> &AgentId {
        &self.agent_id
    }

    /// Return the canonical named channels and optional model settings.
    pub fn launch_bindings(&self) -> &LaunchBindings {
        &self.launch_bindings
    }
}

/// One loaded host binding plus its storage revision.
#[derive(Clone, Debug, PartialEq)]
pub struct LoadedHostPipelineBinding {
    binding: HostPipelineBinding,
    revision: Revision,
}

impl LoadedHostPipelineBinding {
    /// Borrow the decoded durable binding.
    pub fn binding(&self) -> &HostPipelineBinding {
        &self.binding
    }

    /// Borrow the revision required for replacement or unwiring.
    pub fn revision(&self) -> &Revision {
        &self.revision
    }
}

/// Stable failure from durable pipeline binding storage or authorization.
#[derive(Debug)]
pub enum PipelineBindingError {
    /// The injected storage backend failed.
    Storage(StorageError),
    /// The durable service registry failed validation or storage.
    Registry(RegistryError),
    /// Authoritative channel definition storage failed.
    Channel(ChannelEndpointError),
    /// A pipeline identity was not a canonical UUID v7.
    InvalidPipelineId,
    /// The host does not have a durable service registration.
    HostNotRegistered,
    /// The requested or loaded binding disagrees with durable registration.
    RegistrationMismatch,
    /// A referenced channel definition or immutable claim is absent.
    ChannelUnavailable,
    /// A channel is destroyed or does not authorize this agent and direction.
    ChannelUnauthorized,
    /// The channel UUID was already claimed by a different pipeline.
    CrossPipelineChannel,
    /// A different binding already exists for the same host name.
    ConflictingHostBinding,
    /// A revision-CAS replacement or unwiring lost a race.
    ConcurrentUpdate,
    /// A persisted record was malformed or inconsistent with its storage key.
    CorruptRecord,
}

impl Display for PipelineBindingError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Storage(_) => "pipeline bindings: storage failure",
            Self::Registry(_) => "pipeline bindings: registry failure",
            Self::Channel(_) => "pipeline bindings: channel lookup failure",
            Self::InvalidPipelineId => "pipeline bindings: invalid pipeline identity",
            Self::HostNotRegistered => "pipeline bindings: host is not registered",
            Self::RegistrationMismatch => "pipeline bindings: host registration mismatch",
            Self::ChannelUnavailable => "pipeline bindings: channel unavailable",
            Self::ChannelUnauthorized => "pipeline bindings: channel direction unauthorized",
            Self::CrossPipelineChannel => "pipeline bindings: channel belongs to another pipeline",
            Self::ConflictingHostBinding => "pipeline bindings: different host binding exists",
            Self::ConcurrentUpdate => "pipeline bindings: concurrent update",
            Self::CorruptRecord => "pipeline bindings: corrupt durable record",
        })
    }
}

impl std::error::Error for PipelineBindingError {}

impl From<StorageError> for PipelineBindingError {
    fn from(error: StorageError) -> Self {
        Self::Storage(error)
    }
}

impl From<RegistryError> for PipelineBindingError {
    fn from(error: RegistryError) -> Self {
        Self::Registry(error)
    }
}

impl From<ChannelEndpointError> for PipelineBindingError {
    fn from(error: ChannelEndpointError) -> Self {
        Self::Channel(error)
    }
}

/// CAS-backed host bindings and immutable cross-pipeline channel claims.
pub struct PipelineBindingStore<'a> {
    backend: &'a dyn StorageBackend,
}

impl<'a> PipelineBindingStore<'a> {
    /// Bind the store to one initialized or uninitialized repository backend.
    pub fn new(backend: &'a dyn StorageBackend) -> Self {
        Self { backend }
    }

    /// Authorize and atomically create one host binding.
    ///
    /// Channel claims are created first and intentionally survive a later host
    /// conflict. That failure mode can deny reuse but cannot grant authority.
    pub fn wire(
        &self,
        binding: &HostPipelineBinding,
    ) -> Result<LoadedHostPipelineBinding, PipelineBindingError> {
        self.backend.initialize()?;
        self.require_registration(binding)?;
        self.require_authorized_channels(binding)?;
        self.claim_channels(binding)?;
        let key = host_key(binding.registration.host_name());
        let body = encode_host_binding(binding);
        let input = put(&key, HOST_CONTENT_TYPE, body.clone())?.with_if_absent();
        match self.backend.put(input) {
            Ok(record) => decode_host_record(record, binding.registration.host_name()),
            Err(StorageError::Conflict { .. }) => {
                let existing = self
                    .load(binding.registration.host_name())?
                    .ok_or(PipelineBindingError::ConflictingHostBinding)?;
                if existing.binding == *binding {
                    Ok(existing)
                } else {
                    Err(PipelineBindingError::ConflictingHostBinding)
                }
            }
            Err(error) => Err(error.into()),
        }
    }

    /// Load one host binding without treating it as current launch authority.
    pub fn load(
        &self,
        host_name: &HostName,
    ) -> Result<Option<LoadedHostPipelineBinding>, PipelineBindingError> {
        self.backend.initialize()?;
        self.backend
            .get(NAMESPACE, &host_key(host_name))?
            .map(|record| decode_host_record(record, host_name))
            .transpose()
    }

    /// Claim any new channels, then revision-CAS replace one host binding.
    pub fn replace(
        &self,
        loaded: &LoadedHostPipelineBinding,
        replacement: &HostPipelineBinding,
    ) -> Result<LoadedHostPipelineBinding, PipelineBindingError> {
        if loaded.binding.registration.host_name() != replacement.registration.host_name() {
            return Err(PipelineBindingError::RegistrationMismatch);
        }
        self.require_registration(replacement)?;
        self.require_authorized_channels(replacement)?;
        self.claim_channels(replacement)?;
        let name = replacement.registration.host_name();
        let input = put(
            &host_key(name),
            HOST_CONTENT_TYPE,
            encode_host_binding(replacement),
        )?
        .with_if_revision(Some(loaded.revision.clone()));
        match self.backend.put(input) {
            Ok(record) => decode_host_record(record, name),
            Err(StorageError::Conflict { .. }) => Err(PipelineBindingError::ConcurrentUpdate),
            Err(error) => Err(error.into()),
        }
    }

    /// Revision-CAS remove one host binding while retaining immutable claims.
    /// Repeating the removal after absence is idempotent.
    pub fn unwire(&self, loaded: &LoadedHostPipelineBinding) -> Result<(), PipelineBindingError> {
        match self.backend.delete(
            NAMESPACE,
            &host_key(loaded.binding.registration.host_name()),
            Some(&loaded.revision),
        ) {
            Ok(()) => Ok(()),
            Err(StorageError::Conflict { .. }) => Err(PipelineBindingError::ConcurrentUpdate),
            Err(error) => Err(error.into()),
        }
    }

    /// Revalidate and return exact launch authority for one durable registration.
    pub fn resolve_launch(
        &self,
        registration: &HostRegistration,
    ) -> Result<LaunchBindings, PipelineBindingError> {
        let loaded = self
            .load(registration.host_name())?
            .ok_or(PipelineBindingError::HostNotRegistered)?;
        if loaded.binding.registration != *registration {
            return Err(PipelineBindingError::RegistrationMismatch);
        }
        self.require_registration(&loaded.binding)?;
        self.require_claims(&loaded.binding)?;
        self.require_authorized_channels(&loaded.binding)?;
        Ok(loaded.binding.launch_bindings.clone())
    }

    fn require_registration(
        &self,
        binding: &HostPipelineBinding,
    ) -> Result<(), PipelineBindingError> {
        let loaded = ServiceRegistry::new(self.backend)
            .load(binding.registration.host_name())?
            .ok_or(PipelineBindingError::HostNotRegistered)?;
        if loaded.entry().registration() != &binding.registration {
            return Err(PipelineBindingError::RegistrationMismatch);
        }
        Ok(())
    }

    fn require_authorized_channels(
        &self,
        binding: &HostPipelineBinding,
    ) -> Result<(), PipelineBindingError> {
        let definitions = ChannelDefinitionStore::new(self.backend);
        for channel in binding.launch_bindings.channels() {
            let definition = definitions
                .load(ChannelId(channel.channel_id()))?
                .ok_or(PipelineBindingError::ChannelUnavailable)?;
            authorize_channel(&definition, binding.agent_id(), channel.access())?;
        }
        Ok(())
    }

    fn claim_channels(&self, binding: &HostPipelineBinding) -> Result<(), PipelineBindingError> {
        for channel in binding.launch_bindings.channels() {
            let channel_id = ChannelId(channel.channel_id());
            let key = channel_key(channel_id);
            let body = encode_claim(binding.pipeline_id, channel_id);
            let input = put(&key, CLAIM_CONTENT_TYPE, body.clone())?.with_if_absent();
            match self.backend.put(input) {
                Ok(record) => require_claim_record(&record, binding.pipeline_id, channel_id)?,
                Err(StorageError::Conflict { .. }) => {
                    let record = self
                        .backend
                        .get(NAMESPACE, &key)?
                        .ok_or(PipelineBindingError::ChannelUnavailable)?;
                    require_claim_record(&record, binding.pipeline_id, channel_id)?;
                }
                Err(error) => return Err(error.into()),
            }
        }
        Ok(())
    }

    fn require_claims(&self, binding: &HostPipelineBinding) -> Result<(), PipelineBindingError> {
        for channel in binding.launch_bindings.channels() {
            let channel_id = ChannelId(channel.channel_id());
            let record = self
                .backend
                .get(NAMESPACE, &channel_key(channel_id))?
                .ok_or(PipelineBindingError::ChannelUnavailable)?;
            require_claim_record(&record, binding.pipeline_id, channel_id)?;
        }
        Ok(())
    }
}

fn authorize_channel(
    definition: &ChannelDefinition,
    agent_id: &AgentId,
    access: ChannelBindingAccess,
) -> Result<(), PipelineBindingError> {
    if definition.lifecycle() != ChannelLifecycle::Active {
        return Err(PipelineBindingError::ChannelUnauthorized);
    }
    let authorized = match access {
        ChannelBindingAccess::Read => definition.receiver(agent_id).is_some(),
        ChannelBindingAccess::Write => definition.originator().agent_id == *agent_id,
    };
    if !authorized {
        return Err(PipelineBindingError::ChannelUnauthorized);
    }
    Ok(())
}

fn host_key(host_name: &HostName) -> String {
    format!("{HOST_PREFIX}{}", host_name.as_str())
}

fn channel_key(channel_id: ChannelId) -> String {
    let mut encoded = String::with_capacity(32);
    for byte in channel_id.0 {
        use core::fmt::Write;
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    format!("{CHANNEL_PREFIX}{encoded}")
}

fn put(key: &str, content_type: &str, body: Vec<u8>) -> Result<StoragePutInput, StorageError> {
    StoragePutInput::new(
        NAMESPACE,
        key,
        content_type,
        JsonValue::Object(Vec::new()),
        body,
    )
}

/// Encode one strict bounded version-1 host binding.
pub fn encode_host_binding(binding: &HostPipelineBinding) -> Vec<u8> {
    let mut output = Vec::with_capacity(512);
    output.extend_from_slice(HOST_MAGIC);
    output.push(VERSION);
    output.extend_from_slice(binding.pipeline_id.as_bytes());
    push_u16_bytes(
        &mut output,
        binding.registration.host_name().as_str().as_bytes(),
    );
    push_u16_bytes(
        &mut output,
        binding.registration.package_path().as_str().as_bytes(),
    );
    output.extend_from_slice(binding.registration.package_hash());
    output.push(restart_policy_tag(binding.registration.restart_policy()));
    push_u16_bytes(&mut output, binding.agent_id.as_bytes());
    output.extend_from_slice(&(binding.launch_bindings.channels().len() as u16).to_be_bytes());
    for channel in binding.launch_bindings.channels() {
        output.push(match channel.access() {
            ChannelBindingAccess::Read => 1,
            ChannelBindingAccess::Write => 2,
        });
        output.push(channel.name().len() as u8);
        output.extend_from_slice(channel.name().as_bytes());
        output.extend_from_slice(&channel.channel_id());
    }
    match binding.launch_bindings.level_one_model() {
        None => output.push(0),
        Some(model) => {
            output.push(1);
            push_u16_bytes(&mut output, model.model().as_bytes());
            output.extend_from_slice(&model.temperature().to_bits().to_be_bytes());
            output.extend_from_slice(&model.max_tokens().to_be_bytes());
        }
    }
    output
}

/// Decode one strict bounded version-1 host binding.
pub fn decode_host_binding(bytes: &[u8]) -> Result<HostPipelineBinding, PipelineBindingError> {
    if bytes.len() > MAX_HOST_RECORD_BYTES {
        return Err(PipelineBindingError::CorruptRecord);
    }
    let mut reader = Reader::new(bytes);
    if reader.take(4)? != HOST_MAGIC || reader.u8()? != VERSION {
        return Err(PipelineBindingError::CorruptRecord);
    }
    let mut pipeline_bytes = [0u8; 16];
    pipeline_bytes.copy_from_slice(reader.take(16)?);
    let pipeline_id =
        PipelineId::new(pipeline_bytes).map_err(|_| PipelineBindingError::CorruptRecord)?;
    let host_name = HostName::new(reader.string_u16(MAX_HOST_NAME_BYTES)?)
        .map_err(|_| PipelineBindingError::CorruptRecord)?;
    let package_path = PackagePath::new(reader.string_u16(MAX_PACKAGE_PATH_BYTES)?)
        .map_err(|_| PipelineBindingError::CorruptRecord)?;
    let mut package_hash = [0u8; 32];
    package_hash.copy_from_slice(reader.take(32)?);
    let restart_policy = match reader.u8()? {
        1 => RestartPolicy::Always,
        2 => RestartPolicy::OnFailure,
        3 => RestartPolicy::Never,
        _ => return Err(PipelineBindingError::CorruptRecord),
    };
    let agent_id = AgentId::new(reader.vec_u16(MAX_IDENTITY_BYTES)?)
        .map_err(|_| PipelineBindingError::CorruptRecord)?;
    let channel_count = usize::from(reader.u16()?);
    if channel_count > MAX_LAUNCH_CHANNEL_BINDINGS {
        return Err(PipelineBindingError::CorruptRecord);
    }
    let mut channels = Vec::with_capacity(channel_count);
    for _ in 0..channel_count {
        let access = match reader.u8()? {
            1 => ChannelBindingAccess::Read,
            2 => ChannelBindingAccess::Write,
            _ => return Err(PipelineBindingError::CorruptRecord),
        };
        let name_length = usize::from(reader.u8()?);
        if name_length == 0 || name_length > MAX_LAUNCH_CHANNEL_NAME_BYTES {
            return Err(PipelineBindingError::CorruptRecord);
        }
        let name = std::str::from_utf8(reader.take(name_length)?)
            .map_err(|_| PipelineBindingError::CorruptRecord)?;
        let mut channel_id = [0u8; 16];
        channel_id.copy_from_slice(reader.take(16)?);
        channels.push(
            ChannelBinding::new(name, access, channel_id)
                .map_err(|_| PipelineBindingError::CorruptRecord)?,
        );
    }
    let model = match reader.u8()? {
        0 => None,
        1 => {
            let selector = reader.string_u16(MAX_LAUNCH_MODEL_BYTES)?;
            let temperature = f32::from_bits(reader.u32()?);
            let max_tokens = reader.u32()?;
            Some(
                LevelOneModelBinding::new(selector, temperature, max_tokens)
                    .map_err(|_| PipelineBindingError::CorruptRecord)?,
            )
        }
        _ => return Err(PipelineBindingError::CorruptRecord),
    };
    reader.finish()?;
    let launch_bindings =
        LaunchBindings::new(channels, model).map_err(|_| PipelineBindingError::CorruptRecord)?;
    Ok(HostPipelineBinding::new(
        pipeline_id,
        HostRegistration::new(host_name, package_path, package_hash, restart_policy),
        agent_id,
        launch_bindings,
    ))
}

fn decode_host_record(
    record: StorageRecord,
    expected_name: &HostName,
) -> Result<LoadedHostPipelineBinding, PipelineBindingError> {
    if record.namespace != NAMESPACE
        || record.key != host_key(expected_name)
        || record.content_type != HOST_CONTENT_TYPE
    {
        return Err(PipelineBindingError::CorruptRecord);
    }
    let binding = decode_host_binding(&record.body)?;
    if binding.registration.host_name() != expected_name {
        return Err(PipelineBindingError::CorruptRecord);
    }
    Ok(LoadedHostPipelineBinding {
        binding,
        revision: record.revision,
    })
}

fn encode_claim(pipeline_id: PipelineId, channel_id: ChannelId) -> Vec<u8> {
    let mut output = Vec::with_capacity(CLAIM_RECORD_BYTES);
    output.extend_from_slice(CLAIM_MAGIC);
    output.push(VERSION);
    output.extend_from_slice(pipeline_id.as_bytes());
    output.extend_from_slice(&channel_id.0);
    output
}

fn require_claim_record(
    record: &StorageRecord,
    expected_pipeline: PipelineId,
    expected_channel: ChannelId,
) -> Result<(), PipelineBindingError> {
    if record.namespace != NAMESPACE
        || record.key != channel_key(expected_channel)
        || record.content_type != CLAIM_CONTENT_TYPE
        || record.body.len() != CLAIM_RECORD_BYTES
        || &record.body[..4] != CLAIM_MAGIC
        || record.body[4] != VERSION
        || &record.body[21..] != expected_channel.0.as_slice()
    {
        return Err(PipelineBindingError::CorruptRecord);
    }
    if &record.body[5..21] != expected_pipeline.as_bytes() {
        return Err(PipelineBindingError::CrossPipelineChannel);
    }
    Ok(())
}

fn push_u16_bytes(output: &mut Vec<u8>, bytes: &[u8]) {
    output.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
    output.extend_from_slice(bytes);
}

fn restart_policy_tag(policy: RestartPolicy) -> u8 {
    match policy {
        RestartPolicy::Always => 1,
        RestartPolicy::OnFailure => 2,
        RestartPolicy::Never => 3,
    }
}

fn is_uuid_v7(bytes: &[u8; 16]) -> bool {
    bytes[6] >> 4 == 7 && bytes[8] & 0xc0 == 0x80
}

struct Reader<'a> {
    remaining: &'a [u8],
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { remaining: bytes }
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], PipelineBindingError> {
        if self.remaining.len() < length {
            return Err(PipelineBindingError::CorruptRecord);
        }
        let (value, rest) = self.remaining.split_at(length);
        self.remaining = rest;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, PipelineBindingError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, PipelineBindingError> {
        let bytes = self
            .take(2)?
            .try_into()
            .map_err(|_| PipelineBindingError::CorruptRecord)?;
        Ok(u16::from_be_bytes(bytes))
    }

    fn u32(&mut self) -> Result<u32, PipelineBindingError> {
        let bytes = self
            .take(4)?
            .try_into()
            .map_err(|_| PipelineBindingError::CorruptRecord)?;
        Ok(u32::from_be_bytes(bytes))
    }

    fn vec_u16(&mut self, maximum: usize) -> Result<Vec<u8>, PipelineBindingError> {
        let length = usize::from(self.u16()?);
        if length == 0 || length > maximum {
            return Err(PipelineBindingError::CorruptRecord);
        }
        Ok(self.take(length)?.to_vec())
    }

    fn string_u16(&mut self, maximum: usize) -> Result<String, PipelineBindingError> {
        let bytes = self.vec_u16(maximum)?;
        String::from_utf8(bytes).map_err(|_| PipelineBindingError::CorruptRecord)
    }

    fn finish(self) -> Result<(), PipelineBindingError> {
        if self.remaining.is_empty() {
            Ok(())
        } else {
            Err(PipelineBindingError::CorruptRecord)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chief_of_staff_channel_crypto::KeyEpoch;
    use chief_of_staff_channel_endpoints::{OriginatorIdentity, ReceiverIdentity};
    use chief_of_staff_service_registry::{DesiredState, HostEntry};
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    use storage_core::InMemoryStorageBackend;
    use storage_local_folder::LocalFolderStorageBackend;

    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn uuid_v7(tag: u8) -> [u8; 16] {
        let mut bytes = [0u8; 16];
        bytes[0] = tag;
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes
    }

    fn pipeline(tag: u8) -> PipelineId {
        PipelineId::new(uuid_v7(tag)).unwrap()
    }

    fn agent(value: &str) -> AgentId {
        AgentId::new(value.as_bytes().to_vec()).unwrap()
    }

    fn registration(host: &str, hash: u8) -> HostRegistration {
        HostRegistration::new(
            HostName::new(host).unwrap(),
            PackagePath::new(format!("agents/{host}.agent")).unwrap(),
            [hash; 32],
            RestartPolicy::OnFailure,
        )
    }

    fn register(backend: &dyn StorageBackend, registration: HostRegistration) {
        ServiceRegistry::new(backend)
            .register(&HostEntry::registered(registration, DesiredState::Stopped))
            .unwrap();
    }

    fn create_channels(backend: &dyn StorageBackend, agent_id: &AgentId) -> ([u8; 16], [u8; 16]) {
        let read_id = uuid_v7(11);
        let write_id = uuid_v7(12);
        let definitions = ChannelDefinitionStore::new(backend);
        definitions
            .create(
                &ChannelDefinition::new(
                    ChannelId(read_id),
                    OriginatorIdentity {
                        agent_id: agent("request-source"),
                        public_key: [1; 32],
                    },
                    vec![ReceiverIdentity {
                        agent_id: agent_id.clone(),
                        public_key: [2; 32],
                    }],
                    1,
                    KeyEpoch(0),
                )
                .unwrap(),
            )
            .unwrap();
        definitions
            .create(
                &ChannelDefinition::new(
                    ChannelId(write_id),
                    OriginatorIdentity {
                        agent_id: agent_id.clone(),
                        public_key: [3; 32],
                    },
                    vec![ReceiverIdentity {
                        agent_id: agent("report-sink"),
                        public_key: [4; 32],
                    }],
                    2,
                    KeyEpoch(0),
                )
                .unwrap(),
            )
            .unwrap();
        (read_id, write_id)
    }

    fn launch(read_id: [u8; 16], write_id: [u8; 16], model: &str) -> LaunchBindings {
        LaunchBindings::new(
            vec![
                ChannelBinding::new("weather-requests", ChannelBindingAccess::Read, read_id)
                    .unwrap(),
                ChannelBinding::new("weather-reports", ChannelBindingAccess::Write, write_id)
                    .unwrap(),
            ],
            Some(LevelOneModelBinding::new(model, 0.25, 256).unwrap()),
        )
        .unwrap()
    }

    fn binding(
        pipeline_id: PipelineId,
        registration: HostRegistration,
        agent_id: AgentId,
        channels: ([u8; 16], [u8; 16]),
        model: &str,
    ) -> HostPipelineBinding {
        HostPipelineBinding::new(
            pipeline_id,
            registration,
            agent_id,
            launch(channels.0, channels.1, model),
        )
    }

    fn temporary_directory(label: &str) -> std::path::PathBuf {
        let nonce = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "chief-pipeline-bindings-{label}-{}-{now}-{nonce}",
            std::process::id()
        ))
    }

    #[test]
    fn codec_round_trips_and_rejects_every_truncation_and_trailing_byte() {
        let expected = binding(
            pipeline(1),
            registration("weather-host", 7),
            agent("weather-agent"),
            (uuid_v7(11), uuid_v7(12)),
            "test-model",
        );
        let encoded = encode_host_binding(&expected);
        assert_eq!(decode_host_binding(&encoded).unwrap(), expected);
        for length in 0..encoded.len() {
            assert!(
                decode_host_binding(&encoded[..length]).is_err(),
                "accepted {length}"
            );
        }
        let mut trailing = encoded;
        trailing.push(0);
        assert!(decode_host_binding(&trailing).is_err());
        assert!(PipelineId::new([0; 16]).is_err());
    }

    #[test]
    fn wire_is_idempotent_and_launch_revalidates_exact_authority() {
        let backend = InMemoryStorageBackend::new();
        let registration = registration("weather-host", 7);
        let agent_id = agent("weather-agent");
        register(&backend, registration.clone());
        let channels = create_channels(&backend, &agent_id);
        let expected = binding(
            pipeline(1),
            registration.clone(),
            agent_id,
            channels,
            "test-model",
        );
        let store = PipelineBindingStore::new(&backend);
        let first = store.wire(&expected).unwrap();
        let second = store.wire(&expected).unwrap();
        assert_eq!(first, second);
        assert_eq!(
            store.resolve_launch(&registration).unwrap(),
            expected.launch_bindings
        );
        assert!(!first.revision().as_str().is_empty());
    }

    #[test]
    fn registration_direction_lifecycle_and_pipeline_isolation_fail_closed() {
        let backend = InMemoryStorageBackend::new();
        let first_registration = registration("weather-host", 7);
        let second_registration = registration("second-host", 8);
        let agent_id = agent("weather-agent");
        let channels = create_channels(&backend, &agent_id);
        let first = binding(
            pipeline(1),
            first_registration.clone(),
            agent_id.clone(),
            channels,
            "test-model",
        );
        let store = PipelineBindingStore::new(&backend);
        assert!(matches!(
            store.wire(&first),
            Err(PipelineBindingError::HostNotRegistered)
        ));
        register(&backend, first_registration.clone());
        register(&backend, second_registration.clone());

        let wrong_direction = HostPipelineBinding::new(
            pipeline(1),
            second_registration.clone(),
            agent_id.clone(),
            LaunchBindings::new(
                vec![ChannelBinding::new(
                    "weather-requests",
                    ChannelBindingAccess::Write,
                    channels.0,
                )
                .unwrap()],
                None,
            )
            .unwrap(),
        );
        assert!(matches!(
            store.wire(&wrong_direction),
            Err(PipelineBindingError::ChannelUnauthorized)
        ));

        store.wire(&first).unwrap();
        let cross_pipeline = binding(
            pipeline(2),
            second_registration,
            agent_id,
            channels,
            "other-model",
        );
        assert!(matches!(
            store.wire(&cross_pipeline),
            Err(PipelineBindingError::CrossPipelineChannel)
        ));

        ChannelDefinitionStore::new(&backend)
            .destroy(ChannelId(channels.0))
            .unwrap();
        assert!(matches!(
            store.resolve_launch(&first_registration),
            Err(PipelineBindingError::ChannelUnauthorized)
        ));
        assert!(matches!(
            store.resolve_launch(&registration("weather-host", 99)),
            Err(PipelineBindingError::RegistrationMismatch)
        ));
    }

    #[test]
    fn replacement_and_unwiring_are_revision_cas_guarded() {
        let backend = InMemoryStorageBackend::new();
        let registration = registration("weather-host", 7);
        let agent_id = agent("weather-agent");
        register(&backend, registration.clone());
        let channels = create_channels(&backend, &agent_id);
        let store = PipelineBindingStore::new(&backend);
        let first = store
            .wire(&binding(
                pipeline(1),
                registration.clone(),
                agent_id.clone(),
                channels,
                "first-model",
            ))
            .unwrap();
        let replacement = binding(
            pipeline(1),
            registration.clone(),
            agent_id,
            channels,
            "second-model",
        );
        let replaced = store.replace(&first, &replacement).unwrap();
        assert!(matches!(
            store.replace(&first, &replacement),
            Err(PipelineBindingError::ConcurrentUpdate)
        ));
        assert_eq!(
            store.resolve_launch(&registration).unwrap(),
            replacement.launch_bindings
        );
        store.unwire(&replaced).unwrap();
        store.unwire(&replaced).unwrap();
        assert!(store.load(registration.host_name()).unwrap().is_none());
    }

    #[test]
    fn local_folder_restart_recovers_and_revalidates_launch_authority() {
        let root = temporary_directory("restart");
        let registration = registration("weather-host", 7);
        let expected;
        {
            let backend = LocalFolderStorageBackend::new(&root);
            let agent_id = agent("weather-agent");
            register(&backend, registration.clone());
            let channels = create_channels(&backend, &agent_id);
            expected = binding(
                pipeline(1),
                registration.clone(),
                agent_id,
                channels,
                "test-model",
            );
            PipelineBindingStore::new(&backend).wire(&expected).unwrap();
        }
        {
            let backend = LocalFolderStorageBackend::new(&root);
            assert_eq!(
                PipelineBindingStore::new(&backend)
                    .resolve_launch(&registration)
                    .unwrap(),
                expected.launch_bindings
            );
        }
        fs::remove_dir_all(root).unwrap();
    }
}
