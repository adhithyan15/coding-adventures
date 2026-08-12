//! Durable authorization and injected execution for the D18 host data plane.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use chief_of_staff_channel_crypto::{
    ChannelId, ChannelMasterKey, OriginatorSigningKey, ReceiverKeyPair, Sequence,
};
use chief_of_staff_channel_endpoints::{
    AgentId as ChannelAgentId, ChannelDefinitionStore, ChannelEndpointError, DurableOriginator,
    DurableReceiver, MessageId, MessageMetadataSource, Originator, Receiver,
};
use chief_of_staff_channel_store::ChannelStore;
use chief_of_staff_host_control_protocol::{
    validate_data_plane_response, ChannelBindingAccess, CompletionCall, CompletionFinishReason,
    CompletionProvider, CompletionResult, CompletionUsage, DataPlaneFailure, DataPlaneMessage,
    DataPlaneRequest, DataPlaneResponse, ModelToolCall, ModelToolChoice, ModelToolDefinition,
    ModelToolResult, PromptRole, ToolCompletionCall, ToolCompletionOutput, ToolCompletionResult,
    MAX_DATA_PLANE_MESSAGES, MAX_DATA_PLANE_PAYLOAD_BYTES,
};
use chief_of_staff_pipeline_bindings::{
    HostPipelineBinding, PipelineBindingError, PipelineBindingStore, PipelineId,
};
use chief_of_staff_service_registry::HostRegistration;
use coding_adventures_zeroize::Zeroizing;
use llm_gateway::{
    CompletionRequest, FinishReason, LlmClient, Message, MessageContent,
    ModelToolCall as GatewayModelToolCall, ModelToolChoice as GatewayModelToolChoice,
    ModelToolDefinition as GatewayModelToolDefinition, ModelToolResult as GatewayModelToolResult,
    Role, ToolCompletionOutput as GatewayToolCompletionOutput,
    ToolCompletionRequest as GatewayToolCompletionRequest,
};
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};
use storage_core::StorageBackend;

const MAX_PENDING_DELIVERIES: usize = MAX_DATA_PLANE_MESSAGES * 64;

/// Injected execution authority reached only after durable request authorization.
pub trait HostDataPlaneService: Send + Sync {
    /// Execute one already-authorized request for the exact current pipeline binding.
    fn execute(
        &self,
        binding: &HostPipelineBinding,
        request: &DataPlaneRequest,
    ) -> Result<DataPlaneResponse, DataPlaneFailure>;
}

/// Manifest-blind boundary used by process supervision to answer one child request.
pub trait HostDataPlaneDispatcher: Send + Sync {
    /// Reauthorize, execute, and return one exactly correlated response.
    fn dispatch(
        &self,
        registration: &HostRegistration,
        request: &DataPlaneRequest,
    ) -> DataPlaneResponse;
}

/// Dispatcher for compositions that intentionally expose no data-plane service.
#[derive(Default)]
pub struct UnavailableHostDataPlaneDispatcher;

impl HostDataPlaneDispatcher for UnavailableHostDataPlaneDispatcher {
    fn dispatch(
        &self,
        _registration: &HostRegistration,
        request: &DataPlaneRequest,
    ) -> DataPlaneResponse {
        failed(request, DataPlaneFailure::Unavailable)
    }
}

/// Fail-closed service used until channel-key and model-provider authorities are composed.
#[derive(Default)]
pub struct UnavailableHostDataPlaneService;

impl HostDataPlaneService for UnavailableHostDataPlaneService {
    fn execute(
        &self,
        _binding: &HostPipelineBinding,
        _request: &DataPlaneRequest,
    ) -> Result<DataPlaneResponse, DataPlaneFailure> {
        Err(DataPlaneFailure::Unavailable)
    }
}

/// Stable channel-key lookup failure supplied by an isolated custody adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChannelKeyAuthorityError {
    /// Key custody is temporarily sealed or otherwise unavailable.
    Unavailable,
    /// The exact pipeline, agent, channel, or direction has no matching key authority.
    Unauthorized,
}

impl core::fmt::Display for ChannelKeyAuthorityError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(match self {
            Self::Unavailable => "channel key authority unavailable",
            Self::Unauthorized => "channel key authority denied the request",
        })
    }
}

impl std::error::Error for ChannelKeyAuthorityError {}

/// Owned originator secrets released for one exact authorized channel operation.
pub struct OriginatorChannelKeys {
    signing_key: OriginatorSigningKey,
    channel_key: ChannelMasterKey,
}

impl OriginatorChannelKeys {
    /// Bind one signing key and channel master key for a single service operation.
    pub fn new(signing_key: OriginatorSigningKey, channel_key: ChannelMasterKey) -> Self {
        Self {
            signing_key,
            channel_key,
        }
    }
}

/// Isolated authority that releases only keys for an exact durable pipeline binding.
///
/// Production implementations may be backed by a vault actor, hardware custodian,
/// or another process boundary. The dispatcher and orchestration core never receive
/// these values.
pub trait ChannelKeyAuthority: Send + Sync {
    /// Release the receiver private key for one exact read channel.
    fn receiver_key(
        &self,
        binding: &HostPipelineBinding,
        channel_id: ChannelId,
    ) -> Result<ReceiverKeyPair, ChannelKeyAuthorityError>;

    /// Release the signing and current-epoch channel keys for one exact write channel.
    fn originator_keys(
        &self,
        binding: &HostPipelineBinding,
        channel_id: ChannelId,
    ) -> Result<OriginatorChannelKeys, ChannelKeyAuthorityError>;
}

/// Stable failure while provisioning an exact channel-key authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChannelKeyRegistrationError {
    /// A retained secret was an all-zero placeholder rather than provisioned key material.
    InvalidSecret,
    /// The exact pipeline, agent, and channel already has a read or write key binding.
    Duplicate,
}

impl core::fmt::Display for ChannelKeyRegistrationError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidSecret => "channel key registration contains an invalid secret",
            Self::Duplicate => "channel key registration already exists",
        })
    }
}

impl std::error::Error for ChannelKeyRegistrationError {}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ChannelAuthorityKey {
    pipeline_id: [u8; 16],
    agent_id: Vec<u8>,
    channel_id: [u8; 16],
}

impl ChannelAuthorityKey {
    fn new(pipeline_id: PipelineId, agent_id: &ChannelAgentId, channel_id: ChannelId) -> Self {
        Self {
            pipeline_id: *pipeline_id.as_bytes(),
            agent_id: agent_id.as_bytes().to_vec(),
            channel_id: channel_id.0,
        }
    }

    fn from_binding(binding: &HostPipelineBinding, channel_id: ChannelId) -> Self {
        Self::new(binding.pipeline_id(), binding.agent_id(), channel_id)
    }
}

struct StoredOriginatorKeys {
    signing_seed: Zeroizing<[u8; 32]>,
    channel_key: Zeroizing<[u8; 32]>,
}

/// Immutable exact pipeline/agent/channel authority for already-provisioned keys.
///
/// Secret inputs must already be held in zeroizing wrappers. The registry retains
/// them in the same form, exposes no secret getters or formatting implementation,
/// rejects cross-direction duplicates, and reconstructs short-lived cryptographic
/// key owners only for an exact current durable binding.
#[derive(Default)]
pub struct ExactChannelKeyAuthority {
    receivers: BTreeMap<ChannelAuthorityKey, Zeroizing<[u8; 32]>>,
    originators: BTreeMap<ChannelAuthorityKey, StoredOriginatorKeys>,
}

impl ExactChannelKeyAuthority {
    /// Create an empty authority that denies every key release.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register one exact read-channel receiver private key before sharing the authority.
    pub fn register_receiver(
        &mut self,
        pipeline_id: PipelineId,
        agent_id: &ChannelAgentId,
        channel_id: ChannelId,
        private_key: Zeroizing<[u8; 32]>,
    ) -> Result<(), ChannelKeyRegistrationError> {
        if secret_is_zero(&private_key) {
            return Err(ChannelKeyRegistrationError::InvalidSecret);
        }
        let key = ChannelAuthorityKey::new(pipeline_id, agent_id, channel_id);
        if self.receivers.contains_key(&key) || self.originators.contains_key(&key) {
            return Err(ChannelKeyRegistrationError::Duplicate);
        }
        self.receivers.insert(key, private_key);
        Ok(())
    }

    /// Register one exact write-channel signing seed and current-epoch channel key.
    pub fn register_originator(
        &mut self,
        pipeline_id: PipelineId,
        agent_id: &ChannelAgentId,
        channel_id: ChannelId,
        signing_seed: Zeroizing<[u8; 32]>,
        channel_key: Zeroizing<[u8; 32]>,
    ) -> Result<(), ChannelKeyRegistrationError> {
        if secret_is_zero(&signing_seed) || secret_is_zero(&channel_key) {
            return Err(ChannelKeyRegistrationError::InvalidSecret);
        }
        let key = ChannelAuthorityKey::new(pipeline_id, agent_id, channel_id);
        if self.receivers.contains_key(&key) || self.originators.contains_key(&key) {
            return Err(ChannelKeyRegistrationError::Duplicate);
        }
        self.originators.insert(
            key,
            StoredOriginatorKeys {
                signing_seed,
                channel_key,
            },
        );
        Ok(())
    }

    /// Return the number of exact directional key bindings retained by this authority.
    pub fn len(&self) -> usize {
        self.receivers.len() + self.originators.len()
    }

    /// Return whether this authority denies every key release.
    pub fn is_empty(&self) -> bool {
        self.receivers.is_empty() && self.originators.is_empty()
    }
}

impl ChannelKeyAuthority for ExactChannelKeyAuthority {
    fn receiver_key(
        &self,
        binding: &HostPipelineBinding,
        channel_id: ChannelId,
    ) -> Result<ReceiverKeyPair, ChannelKeyAuthorityError> {
        if !binding_allows_channel(binding, channel_id, ChannelBindingAccess::Read) {
            return Err(ChannelKeyAuthorityError::Unauthorized);
        }
        let private_key = self
            .receivers
            .get(&ChannelAuthorityKey::from_binding(binding, channel_id))
            .ok_or(ChannelKeyAuthorityError::Unauthorized)?;
        ReceiverKeyPair::from_private_key(copy_secret(private_key).into_inner())
            .map_err(|_| ChannelKeyAuthorityError::Unavailable)
    }

    fn originator_keys(
        &self,
        binding: &HostPipelineBinding,
        channel_id: ChannelId,
    ) -> Result<OriginatorChannelKeys, ChannelKeyAuthorityError> {
        if !binding_allows_channel(binding, channel_id, ChannelBindingAccess::Write) {
            return Err(ChannelKeyAuthorityError::Unauthorized);
        }
        let keys = self
            .originators
            .get(&ChannelAuthorityKey::from_binding(binding, channel_id))
            .ok_or(ChannelKeyAuthorityError::Unauthorized)?;
        Ok(OriginatorChannelKeys::new(
            OriginatorSigningKey::from_seed(copy_secret(&keys.signing_seed).into_inner()),
            ChannelMasterKey::from_bytes(copy_secret(&keys.channel_key).into_inner()),
        ))
    }
}

fn secret_is_zero(secret: &Zeroizing<[u8; 32]>) -> bool {
    secret.iter().all(|byte| *byte == 0)
}

fn copy_secret(secret: &Zeroizing<[u8; 32]>) -> Zeroizing<[u8; 32]> {
    let mut copy = Zeroizing::new([0; 32]);
    copy.copy_from_slice(secret.as_slice());
    copy
}

fn binding_allows_channel(
    binding: &HostPipelineBinding,
    channel_id: ChannelId,
    access: ChannelBindingAccess,
) -> bool {
    binding
        .launch_bindings()
        .channels()
        .iter()
        .any(|candidate| candidate.channel_id() == channel_id.0 && candidate.access() == access)
}

/// Stable exact-model provider lookup failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelProviderError;

impl core::fmt::Display for ModelProviderError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("exact model provider unavailable")
    }
}

impl std::error::Error for ModelProviderError {}

/// Authority that resolves one exact authorized model selector to an LLM client.
pub trait ModelProviderAuthority: Send + Sync {
    /// Resolve a client for the exact durable pipeline and model selector.
    fn resolve(
        &self,
        binding: &HostPipelineBinding,
        model: &str,
    ) -> Result<Arc<dyn LlmClient>, ModelProviderError>;
}

/// Immutable exact-selector registry for already-constructed provider clients.
#[derive(Default)]
pub struct ExactModelProviderRegistry {
    providers: BTreeMap<String, Arc<dyn LlmClient>>,
}

impl ExactModelProviderRegistry {
    /// Create an empty registry that fails closed for every selector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register one non-empty selector exactly once before sharing the registry.
    pub fn register(
        &mut self,
        model: impl Into<String>,
        provider: Arc<dyn LlmClient>,
    ) -> Result<(), ModelProviderError> {
        let model = model.into();
        if model.is_empty() || self.providers.contains_key(&model) {
            return Err(ModelProviderError);
        }
        self.providers.insert(model, provider);
        Ok(())
    }

    /// Return the number of exact model selectors retained by this registry.
    pub fn len(&self) -> usize {
        self.providers.len()
    }

    /// Return whether this registry denies every model lookup.
    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }
}

impl ModelProviderAuthority for ExactModelProviderRegistry {
    fn resolve(
        &self,
        _binding: &HostPipelineBinding,
        model: &str,
    ) -> Result<Arc<dyn LlmClient>, ModelProviderError> {
        self.providers.get(model).cloned().ok_or(ModelProviderError)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct DeliveryKey {
    pipeline_id: [u8; 16],
    agent_id: Vec<u8>,
    channel_id: [u8; 16],
    message_id: [u8; 16],
}

/// Concrete service that executes authorized operations against encrypted durable
/// channel endpoints and exact provider-neutral LLM clients.
///
/// Receiver delivery receipts are retained until acknowledgement so a child may
/// acknowledge only a message returned by this service instance. Channel keys and
/// provider credentials stay behind separately injected authorities.
pub struct AuthorityBackedHostDataPlaneService {
    backend: Arc<dyn StorageBackend>,
    key_authority: Arc<dyn ChannelKeyAuthority>,
    model_authority: Arc<dyn ModelProviderAuthority>,
    metadata_source: Arc<dyn MessageMetadataSource>,
    deliveries: Mutex<BTreeMap<DeliveryKey, Sequence>>,
}

impl AuthorityBackedHostDataPlaneService {
    /// Compose storage, isolated key custody, exact model authority, and metadata.
    pub fn new(
        backend: Arc<dyn StorageBackend>,
        key_authority: Arc<dyn ChannelKeyAuthority>,
        model_authority: Arc<dyn ModelProviderAuthority>,
        metadata_source: Arc<dyn MessageMetadataSource>,
    ) -> Self {
        Self {
            backend,
            key_authority,
            model_authority,
            metadata_source,
            deliveries: Mutex::new(BTreeMap::new()),
        }
    }

    fn receive(
        &self,
        binding: &HostPipelineBinding,
        id: chief_of_staff_host_control_protocol::RequestId,
        channel_id: [u8; 16],
        limit: u16,
    ) -> Result<DataPlaneResponse, DataPlaneFailure> {
        let channel_id = ChannelId(channel_id);
        let receiver_key = self
            .key_authority
            .receiver_key(binding, channel_id)
            .map_err(key_failure)?;
        let mut receiver = DurableReceiver::open(
            self.backend.as_ref(),
            channel_id,
            binding.agent_id().clone(),
            receiver_key,
        )
        .map_err(endpoint_failure)?;
        let received = receiver
            .receive(usize::from(limit))
            .map_err(endpoint_failure)?;
        let mut prepared = Vec::with_capacity(received.len());
        for message in received {
            let message_id = *message.message_id.as_bytes();
            let key = delivery_key(binding, channel_id, message_id);
            prepared.push((
                key,
                message.sequence,
                DataPlaneMessage {
                    message_id,
                    sequence: message.sequence.0,
                    timestamp_ns: message.timestamp_ns,
                    content_type: message.content_type,
                    payload: message.payload,
                },
            ));
        }
        let mut deliveries = self
            .deliveries
            .lock()
            .map_err(|_| DataPlaneFailure::Internal)?;
        let new_deliveries = prepared
            .iter()
            .filter(|(key, _, _)| !deliveries.contains_key(key))
            .count();
        if deliveries
            .len()
            .checked_add(new_deliveries)
            .is_none_or(|total| total > MAX_PENDING_DELIVERIES)
        {
            return Err(DataPlaneFailure::Unavailable);
        }
        let mut messages = Vec::with_capacity(prepared.len());
        for (key, sequence, message) in prepared {
            if deliveries
                .insert(key, sequence)
                .is_some_and(|previous| previous != sequence)
            {
                return Err(DataPlaneFailure::Internal);
            }
            messages.push(message);
        }
        Ok(DataPlaneResponse::Received { id, messages })
    }

    fn publish(
        &self,
        binding: &HostPipelineBinding,
        id: chief_of_staff_host_control_protocol::RequestId,
        channel_id: [u8; 16],
        content_type: &str,
        payload: &[u8],
    ) -> Result<DataPlaneResponse, DataPlaneFailure> {
        let channel_id = ChannelId(channel_id);
        let keys = self
            .key_authority
            .originator_keys(binding, channel_id)
            .map_err(key_failure)?;
        let originator = DurableOriginator::open(
            self.backend.as_ref(),
            channel_id,
            binding.agent_id(),
            &keys.signing_key,
            &keys.channel_key,
            self.metadata_source.as_ref(),
        )
        .map_err(endpoint_failure)?;
        let definition = ChannelDefinitionStore::new(self.backend.as_ref())
            .load(channel_id)
            .map_err(endpoint_failure)?
            .ok_or(DataPlaneFailure::Unauthorized)?;
        for receiver in definition.receivers() {
            originator
                .grant_receiver(&receiver.agent_id)
                .map_err(endpoint_failure)?;
        }
        let published = originator
            .publish(payload, content_type)
            .map_err(endpoint_failure)?;
        Ok(DataPlaneResponse::Published {
            id,
            message_id: *published.message_id.as_bytes(),
            sequence: published.sequence.0,
            timestamp_ns: published.timestamp_ns,
        })
    }

    fn acknowledge(
        &self,
        binding: &HostPipelineBinding,
        id: chief_of_staff_host_control_protocol::RequestId,
        channel_id: [u8; 16],
        message_id: [u8; 16],
    ) -> Result<DataPlaneResponse, DataPlaneFailure> {
        let channel_id = ChannelId(channel_id);
        let receiver_key = self
            .key_authority
            .receiver_key(binding, channel_id)
            .map_err(key_failure)?;
        DurableReceiver::open(
            self.backend.as_ref(),
            channel_id,
            binding.agent_id().clone(),
            receiver_key,
        )
        .map_err(endpoint_failure)?;
        MessageId::from_uuid_v7(message_id).map_err(endpoint_failure)?;
        let key = delivery_key(binding, channel_id, message_id);
        let mut deliveries = self
            .deliveries
            .lock()
            .map_err(|_| DataPlaneFailure::Internal)?;
        let sequence = deliveries
            .get(&key)
            .copied()
            .ok_or(DataPlaneFailure::Unauthorized)?;
        let acknowledged = ChannelStore::new(self.backend.as_ref(), channel_id)
            .acknowledge(binding.agent_id().as_bytes(), sequence)
            .map_err(|_| DataPlaneFailure::Channel)?;
        deliveries.retain(|candidate, delivered_sequence| {
            candidate.pipeline_id != *binding.pipeline_id().as_bytes()
                || candidate.agent_id != binding.agent_id().as_bytes()
                || candidate.channel_id != channel_id.0
                || *delivered_sequence >= acknowledged
        });
        Ok(DataPlaneResponse::Acknowledged {
            id,
            sequence: acknowledged.0,
        })
    }

    fn complete(
        &self,
        binding: &HostPipelineBinding,
        id: chief_of_staff_host_control_protocol::RequestId,
        call: &CompletionCall,
    ) -> Result<DataPlaneResponse, DataPlaneFailure> {
        let provider = self
            .model_authority
            .resolve(binding, &call.model)
            .map_err(|_| DataPlaneFailure::Unavailable)?;
        let request = gateway_completion_request(call);
        let response = provider
            .complete(request)
            .map_err(|_| DataPlaneFailure::Completion)?;
        if response.text.len() > MAX_DATA_PLANE_PAYLOAD_BYTES {
            return Err(DataPlaneFailure::Completion);
        }
        let input_tokens =
            u64::try_from(response.usage.input_tokens).map_err(|_| DataPlaneFailure::Internal)?;
        let output_tokens =
            u64::try_from(response.usage.output_tokens).map_err(|_| DataPlaneFailure::Internal)?;
        let cached_tokens =
            u64::try_from(response.usage.cached_tokens).map_err(|_| DataPlaneFailure::Internal)?;
        Ok(DataPlaneResponse::Completed {
            id,
            result: Box::new(CompletionResult {
                text: response.text,
                model: response.model,
                provider: CompletionProvider {
                    vendor: response.provider_id.vendor,
                    model_family: response.provider_id.model_family,
                    model_version: response.provider_id.model_version,
                    endpoint: response.provider_id.endpoint,
                },
                usage: CompletionUsage {
                    input_tokens,
                    output_tokens,
                    cached_tokens,
                },
                finish_reason: gateway_finish_reason(response.finish_reason),
                latency_ms: response.latency_ms,
            }),
        })
    }

    fn complete_with_tools(
        &self,
        binding: &HostPipelineBinding,
        id: chief_of_staff_host_control_protocol::RequestId,
        call: &ToolCompletionCall,
    ) -> Result<DataPlaneResponse, DataPlaneFailure> {
        let provider = self
            .model_authority
            .resolve(binding, &call.completion.model)
            .map_err(|_| DataPlaneFailure::Unavailable)?;
        let response = provider
            .complete_with_tools(GatewayToolCompletionRequest {
                completion: gateway_completion_request(&call.completion),
                tools: call.tools.iter().map(gateway_tool_definition).collect(),
                choice: match &call.choice {
                    ModelToolChoice::Auto => GatewayModelToolChoice::Auto,
                    ModelToolChoice::Required => GatewayModelToolChoice::Required,
                    ModelToolChoice::Named(name) => GatewayModelToolChoice::Named(name.clone()),
                },
                results: call.results.iter().map(gateway_tool_result).collect(),
            })
            .map_err(|_| DataPlaneFailure::Completion)?;
        if !gateway_tool_output_allowed(&response.output, &call.tools, &call.choice) {
            return Err(DataPlaneFailure::Completion);
        }
        let input_tokens =
            u64::try_from(response.usage.input_tokens).map_err(|_| DataPlaneFailure::Internal)?;
        let output_tokens =
            u64::try_from(response.usage.output_tokens).map_err(|_| DataPlaneFailure::Internal)?;
        let cached_tokens =
            u64::try_from(response.usage.cached_tokens).map_err(|_| DataPlaneFailure::Internal)?;
        Ok(DataPlaneResponse::ToolCompleted {
            id,
            result: Box::new(ToolCompletionResult {
                output: match response.output {
                    GatewayToolCompletionOutput::FinalText(text) => {
                        ToolCompletionOutput::FinalText(text)
                    }
                    GatewayToolCompletionOutput::ToolCall(call) => {
                        ToolCompletionOutput::ToolCall(protocol_tool_call(call))
                    }
                },
                model: response.model,
                provider: CompletionProvider {
                    vendor: response.provider_id.vendor,
                    model_family: response.provider_id.model_family,
                    model_version: response.provider_id.model_version,
                    endpoint: response.provider_id.endpoint,
                },
                usage: CompletionUsage {
                    input_tokens,
                    output_tokens,
                    cached_tokens,
                },
                finish_reason: gateway_finish_reason(response.finish_reason),
                latency_ms: response.latency_ms,
                polyfill_used: response.polyfill_used,
            }),
        })
    }
}

fn gateway_completion_request(call: &CompletionCall) -> CompletionRequest {
    CompletionRequest {
        model: call.model.clone(),
        system: call.system.clone(),
        messages: call
            .messages
            .iter()
            .map(|message| Message {
                role: match message.role {
                    PromptRole::System => Role::System,
                    PromptRole::User => Role::User,
                    PromptRole::Assistant => Role::Assistant,
                },
                content: MessageContent::Text(message.text.clone()),
            })
            .collect(),
        temperature: call.temperature,
        max_tokens: call.max_tokens.map(|value| value as usize),
        stop_sequences: call.stop_sequences.clone(),
        seed: call.seed,
        metadata: call
            .metadata
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect::<HashMap<_, _>>(),
    }
}

fn gateway_tool_definition(definition: &ModelToolDefinition) -> GatewayModelToolDefinition {
    GatewayModelToolDefinition {
        name: definition.name.clone(),
        description: definition.description.clone(),
        input_schema: definition.input_schema.clone(),
    }
}

fn gateway_tool_result(result: &ModelToolResult) -> GatewayModelToolResult {
    GatewayModelToolResult {
        call: GatewayModelToolCall {
            call_id: result.call.call_id.clone(),
            name: result.call.name.clone(),
            arguments: result.call.arguments.clone(),
        },
        output: result.output.clone(),
        is_error: result.is_error,
    }
}

fn protocol_tool_call(call: GatewayModelToolCall) -> ModelToolCall {
    ModelToolCall {
        call_id: call.call_id,
        name: call.name,
        arguments: call.arguments,
    }
}

fn gateway_tool_output_allowed(
    output: &GatewayToolCompletionOutput,
    tools: &[ModelToolDefinition],
    choice: &ModelToolChoice,
) -> bool {
    match output {
        GatewayToolCompletionOutput::FinalText(text) => {
            !text.is_empty() && matches!(choice, ModelToolChoice::Auto)
        }
        GatewayToolCompletionOutput::ToolCall(call) => {
            tools.iter().any(|tool| tool.name == call.name)
                && match choice {
                    ModelToolChoice::Named(name) => name == &call.name,
                    ModelToolChoice::Auto | ModelToolChoice::Required => true,
                }
        }
    }
}

fn gateway_finish_reason(reason: FinishReason) -> CompletionFinishReason {
    match reason {
        FinishReason::Stop => CompletionFinishReason::Stop,
        FinishReason::MaxTokens => CompletionFinishReason::MaxTokens,
        FinishReason::Refusal => CompletionFinishReason::Refusal,
        FinishReason::Other => CompletionFinishReason::Other,
    }
}

impl HostDataPlaneService for AuthorityBackedHostDataPlaneService {
    fn execute(
        &self,
        binding: &HostPipelineBinding,
        request: &DataPlaneRequest,
    ) -> Result<DataPlaneResponse, DataPlaneFailure> {
        let response = match request {
            DataPlaneRequest::Receive {
                id,
                channel_id,
                limit,
            } => self.receive(binding, *id, *channel_id, *limit),
            DataPlaneRequest::Publish {
                id,
                channel_id,
                content_type,
                payload,
            } => self.publish(binding, *id, *channel_id, content_type, payload),
            DataPlaneRequest::Acknowledge {
                id,
                channel_id,
                message_id,
            } => self.acknowledge(binding, *id, *channel_id, *message_id),
            DataPlaneRequest::Complete { id, call } => self.complete(binding, *id, call),
            DataPlaneRequest::CompleteWithTools { id, call } => {
                self.complete_with_tools(binding, *id, call)
            }
        }?;
        validate_data_plane_response(&response).map_err(|_| match request {
            DataPlaneRequest::Complete { .. } | DataPlaneRequest::CompleteWithTools { .. } => {
                DataPlaneFailure::Completion
            }
            _ => DataPlaneFailure::Channel,
        })?;
        Ok(response)
    }
}

fn delivery_key(
    binding: &HostPipelineBinding,
    channel_id: ChannelId,
    message_id: [u8; 16],
) -> DeliveryKey {
    DeliveryKey {
        pipeline_id: *binding.pipeline_id().as_bytes(),
        agent_id: binding.agent_id().as_bytes().to_vec(),
        channel_id: channel_id.0,
        message_id,
    }
}

fn key_failure(error: ChannelKeyAuthorityError) -> DataPlaneFailure {
    match error {
        ChannelKeyAuthorityError::Unavailable => DataPlaneFailure::Unavailable,
        ChannelKeyAuthorityError::Unauthorized => DataPlaneFailure::Unauthorized,
    }
}

fn endpoint_failure(error: ChannelEndpointError) -> DataPlaneFailure {
    match error {
        ChannelEndpointError::DefinitionNotFound
        | ChannelEndpointError::DefinitionChanged
        | ChannelEndpointError::ChannelDestroyed
        | ChannelEndpointError::UnauthorizedOriginator
        | ChannelEndpointError::UnauthorizedReceiver
        | ChannelEndpointError::PublicKeyMismatch
        | ChannelEndpointError::UnauthorizedMessage => DataPlaneFailure::Unauthorized,
        ChannelEndpointError::Storage(_) | ChannelEndpointError::ConcurrentUpdate => {
            DataPlaneFailure::Unavailable
        }
        ChannelEndpointError::InvalidDefinition(_)
        | ChannelEndpointError::InvalidMessageId
        | ChannelEndpointError::ConflictingDefinition
        | ChannelEndpointError::CorruptDefinition(_)
        | ChannelEndpointError::MissingKeyGrant(_)
        | ChannelEndpointError::UnknownMessageId(_)
        | ChannelEndpointError::Store(_)
        | ChannelEndpointError::Crypto(_)
        | ChannelEndpointError::Metadata(_) => DataPlaneFailure::Channel,
    }
}

/// Storage-backed dispatcher that revalidates pipeline authority for every request.
pub struct DurableHostDataPlaneDispatcher {
    backend: Arc<dyn StorageBackend>,
    service: Arc<dyn HostDataPlaneService>,
}

impl DurableHostDataPlaneDispatcher {
    /// Bind durable authorization to one backend and separately injected service.
    pub fn new(backend: Arc<dyn StorageBackend>, service: Arc<dyn HostDataPlaneService>) -> Self {
        Self { backend, service }
    }
}

impl HostDataPlaneDispatcher for DurableHostDataPlaneDispatcher {
    fn dispatch(
        &self,
        registration: &HostRegistration,
        request: &DataPlaneRequest,
    ) -> DataPlaneResponse {
        let binding = match PipelineBindingStore::new(self.backend.as_ref())
            .resolve_launch_binding(registration)
        {
            Ok(binding) => binding,
            Err(error) => return failed(request, binding_failure(&error)),
        };
        if !request_is_authorized(&binding, request) {
            return failed(request, DataPlaneFailure::Unauthorized);
        }
        match self.service.execute(&binding, request) {
            Ok(response) if response_matches(request, &response) => response,
            Ok(_) => failed(request, DataPlaneFailure::Internal),
            Err(failure) => failed(request, failure),
        }
    }
}

fn request_is_authorized(binding: &HostPipelineBinding, request: &DataPlaneRequest) -> bool {
    match request {
        DataPlaneRequest::Receive { channel_id, .. }
        | DataPlaneRequest::Acknowledge { channel_id, .. } => {
            binding.launch_bindings().channels().iter().any(|binding| {
                binding.channel_id() == *channel_id
                    && binding.access() == ChannelBindingAccess::Read
            })
        }
        DataPlaneRequest::Publish { channel_id, .. } => {
            binding.launch_bindings().channels().iter().any(|binding| {
                binding.channel_id() == *channel_id
                    && binding.access() == ChannelBindingAccess::Write
            })
        }
        DataPlaneRequest::Complete { call, .. } => binding
            .launch_bindings()
            .level_one_model()
            .is_some_and(|model| {
                model.model() == call.model
                    && model.temperature().to_bits() == call.temperature.to_bits()
                    && Some(model.max_tokens()) == call.max_tokens
            }),
        DataPlaneRequest::CompleteWithTools { call, .. } => binding
            .launch_bindings()
            .level_one_model()
            .is_some_and(|model| {
                model.model() == call.completion.model
                    && model.temperature().to_bits() == call.completion.temperature.to_bits()
                    && Some(model.max_tokens()) == call.completion.max_tokens
            }),
    }
}

fn response_matches(request: &DataPlaneRequest, response: &DataPlaneResponse) -> bool {
    response.id() == request.id()
        && response
            .operation()
            .is_none_or(|operation| operation == request.operation())
}

fn failed(request: &DataPlaneRequest, failure: DataPlaneFailure) -> DataPlaneResponse {
    DataPlaneResponse::Failed {
        id: request.id(),
        failure,
    }
}

fn binding_failure(error: &PipelineBindingError) -> DataPlaneFailure {
    match error {
        PipelineBindingError::Storage(_)
        | PipelineBindingError::Registry(_)
        | PipelineBindingError::Channel(_)
        | PipelineBindingError::ConcurrentUpdate => DataPlaneFailure::Unavailable,
        PipelineBindingError::CorruptRecord => DataPlaneFailure::Internal,
        PipelineBindingError::InvalidPipelineId
        | PipelineBindingError::HostNotRegistered
        | PipelineBindingError::RegistrationMismatch
        | PipelineBindingError::ChannelUnavailable
        | PipelineBindingError::ChannelUnauthorized
        | PipelineBindingError::CrossPipelineChannel
        | PipelineBindingError::ConflictingHostBinding => DataPlaneFailure::Unauthorized,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chief_of_staff_channel_crypto::{
        ChannelId, ChannelMasterKey, KeyEpoch, OriginatorSigningKey, ReceiverKeyPair,
    };
    use chief_of_staff_channel_endpoints::{
        AgentId, ChannelDefinition, ChannelDefinitionStore, DurableOriginator, DurableReceiver,
        MessageMetadata, MessageMetadataError, OriginatorIdentity, ReceiverIdentity,
    };
    use chief_of_staff_host_control_protocol::{
        ChannelBinding, CompletionCall, DataPlaneMessage, LevelOneModelBinding, PromptMessage,
        PromptRole, RequestId,
    };
    use chief_of_staff_pipeline_bindings::PipelineId;
    use chief_of_staff_service_registry::{
        DesiredState, HostEntry, HostName, PackagePath, RestartPolicy, ServiceRegistry,
    };
    use llm_gateway::{
        MockLlmClient, MockResponse, ProviderIdentity, RequestFingerprint,
        ToolCompletionRequest as GatewayToolCompletionRequest,
    };
    use std::collections::BTreeMap;
    use storage_core::InMemoryStorageBackend;

    fn uuid_v7(last: u8) -> [u8; 16] {
        let mut bytes = [0; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes[15] = last;
        bytes
    }

    fn agent(name: &str) -> AgentId {
        AgentId::new(name.as_bytes().to_vec()).unwrap()
    }

    fn registration() -> HostRegistration {
        HostRegistration::new(
            HostName::new("weather-host").unwrap(),
            PackagePath::new("/srv/weather.agent").unwrap(),
            [7; 32],
            RestartPolicy::Always,
        )
    }

    fn install_binding(backend: &dyn StorageBackend) -> HostPipelineBinding {
        let registration = registration();
        ServiceRegistry::new(backend)
            .register(&HostEntry::registered(
                registration.clone(),
                DesiredState::Running,
            ))
            .unwrap();
        let worker = agent("weather-agent");
        let read_id = uuid_v7(1);
        let write_id = uuid_v7(2);
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
                        agent_id: worker.clone(),
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
                        agent_id: worker.clone(),
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
        let binding = HostPipelineBinding::new(
            PipelineId::new(uuid_v7(9)).unwrap(),
            registration,
            worker,
            chief_of_staff_host_control_protocol::LaunchBindings::new(
                vec![
                    ChannelBinding::new("weather-requests", ChannelBindingAccess::Read, read_id)
                        .unwrap(),
                    ChannelBinding::new("weather-reports", ChannelBindingAccess::Write, write_id)
                        .unwrap(),
                ],
                Some(LevelOneModelBinding::new("test-model", 0.25, 256).unwrap()),
            )
            .unwrap(),
        );
        PipelineBindingStore::new(backend).wire(&binding).unwrap();
        binding
    }

    struct FixedMetadata {
        message_id: [u8; 16],
        timestamp_ns: u64,
    }

    impl MessageMetadataSource for FixedMetadata {
        fn next_metadata(&self) -> Result<MessageMetadata, MessageMetadataError> {
            Ok(MessageMetadata {
                message_id: MessageId::from_uuid_v7(self.message_id)
                    .map_err(|_| MessageMetadataError::new("invalid fixed message ID"))?,
                timestamp_ns: self.timestamp_ns,
            })
        }
    }

    fn install_real_binding(backend: &dyn StorageBackend) -> HostPipelineBinding {
        let registration = registration();
        ServiceRegistry::new(backend)
            .register(&HostEntry::registered(
                registration.clone(),
                DesiredState::Running,
            ))
            .unwrap();
        let worker = agent("weather-agent");
        let input_originator = OriginatorSigningKey::from_seed([0x11; 32]);
        let worker_receiver = ReceiverKeyPair::from_private_key([0x13; 32]).unwrap();
        let worker_originator = OriginatorSigningKey::from_seed([0x21; 32]);
        let sink_receiver = ReceiverKeyPair::from_private_key([0x23; 32]).unwrap();
        let definitions = ChannelDefinitionStore::new(backend);
        definitions
            .create(
                &ChannelDefinition::new(
                    ChannelId(uuid_v7(1)),
                    OriginatorIdentity {
                        agent_id: agent("request-source"),
                        public_key: input_originator.public_key(),
                    },
                    vec![ReceiverIdentity {
                        agent_id: worker.clone(),
                        public_key: worker_receiver.public_key(),
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
                    ChannelId(uuid_v7(2)),
                    OriginatorIdentity {
                        agent_id: worker.clone(),
                        public_key: worker_originator.public_key(),
                    },
                    vec![ReceiverIdentity {
                        agent_id: agent("report-sink"),
                        public_key: sink_receiver.public_key(),
                    }],
                    2,
                    KeyEpoch(0),
                )
                .unwrap(),
            )
            .unwrap();
        let binding = HostPipelineBinding::new(
            PipelineId::new(uuid_v7(9)).unwrap(),
            registration,
            worker,
            chief_of_staff_host_control_protocol::LaunchBindings::new(
                vec![
                    ChannelBinding::new("weather-requests", ChannelBindingAccess::Read, uuid_v7(1))
                        .unwrap(),
                    ChannelBinding::new("weather-reports", ChannelBindingAccess::Write, uuid_v7(2))
                        .unwrap(),
                ],
                Some(LevelOneModelBinding::new("test-model", 0.25, 256).unwrap()),
            )
            .unwrap(),
        );
        PipelineBindingStore::new(backend).wire(&binding).unwrap();

        let input_metadata = FixedMetadata {
            message_id: uuid_v7(4),
            timestamp_ns: 10,
        };
        let input_key = ChannelMasterKey::from_bytes([0x12; 32]);
        let input = DurableOriginator::open(
            backend,
            ChannelId(uuid_v7(1)),
            &agent("request-source"),
            &input_originator,
            &input_key,
            &input_metadata,
        )
        .unwrap();
        input.grant_receiver(&agent("weather-agent")).unwrap();
        input.publish(b"Seattle", "text/plain").unwrap();
        binding
    }

    fn exact_keys(binding: &HostPipelineBinding) -> ExactChannelKeyAuthority {
        let mut keys = ExactChannelKeyAuthority::new();
        keys.register_receiver(
            binding.pipeline_id(),
            binding.agent_id(),
            ChannelId(uuid_v7(1)),
            Zeroizing::new([0x13; 32]),
        )
        .unwrap();
        keys.register_originator(
            binding.pipeline_id(),
            binding.agent_id(),
            ChannelId(uuid_v7(2)),
            Zeroizing::new([0x21; 32]),
            Zeroizing::new([0x22; 32]),
        )
        .unwrap();
        keys
    }

    fn request_id(value: u64) -> RequestId {
        RequestId::new(value).unwrap()
    }

    fn completion(id: u64, model: &str, temperature: f32, max_tokens: u32) -> DataPlaneRequest {
        DataPlaneRequest::Complete {
            id: request_id(id),
            call: CompletionCall {
                model: model.to_string(),
                system: None,
                messages: vec![PromptMessage {
                    role: PromptRole::User,
                    text: "Seattle".to_string(),
                }],
                temperature,
                max_tokens: Some(max_tokens),
                stop_sequences: Vec::new(),
                seed: None,
                metadata: BTreeMap::new(),
            },
        }
    }

    struct EchoService;

    impl HostDataPlaneService for EchoService {
        fn execute(
            &self,
            binding: &HostPipelineBinding,
            request: &DataPlaneRequest,
        ) -> Result<DataPlaneResponse, DataPlaneFailure> {
            assert_eq!(binding.agent_id().as_bytes(), b"weather-agent");
            Ok(match request {
                DataPlaneRequest::Receive { id, .. } => DataPlaneResponse::Received {
                    id: *id,
                    messages: vec![DataPlaneMessage {
                        message_id: uuid_v7(4),
                        sequence: 1,
                        timestamp_ns: 10,
                        content_type: "text/plain".to_string(),
                        payload: b"Seattle".to_vec(),
                    }],
                },
                DataPlaneRequest::Publish { id, .. } => DataPlaneResponse::Published {
                    id: *id,
                    message_id: uuid_v7(5),
                    sequence: 2,
                    timestamp_ns: 11,
                },
                DataPlaneRequest::Acknowledge { id, .. } => DataPlaneResponse::Acknowledged {
                    id: *id,
                    sequence: 1,
                },
                DataPlaneRequest::Complete { id, .. } => DataPlaneResponse::Failed {
                    id: *id,
                    failure: DataPlaneFailure::Completion,
                },
                DataPlaneRequest::CompleteWithTools { id, .. } => DataPlaneResponse::Failed {
                    id: *id,
                    failure: DataPlaneFailure::Completion,
                },
            })
        }
    }

    fn dispatcher(
        backend: Arc<dyn StorageBackend>,
        service: Arc<dyn HostDataPlaneService>,
    ) -> DurableHostDataPlaneDispatcher {
        DurableHostDataPlaneDispatcher::new(backend, service)
    }

    #[test]
    fn authorized_channel_operations_reach_the_service() {
        let backend: Arc<dyn StorageBackend> = Arc::new(InMemoryStorageBackend::new());
        install_binding(backend.as_ref());
        let dispatcher = dispatcher(Arc::clone(&backend), Arc::new(EchoService));
        let requests = [
            DataPlaneRequest::Receive {
                id: request_id(1),
                channel_id: uuid_v7(1),
                limit: 1,
            },
            DataPlaneRequest::Publish {
                id: request_id(2),
                channel_id: uuid_v7(2),
                content_type: "text/plain".to_string(),
                payload: b"forecast".to_vec(),
            },
            DataPlaneRequest::Acknowledge {
                id: request_id(3),
                channel_id: uuid_v7(1),
                message_id: uuid_v7(4),
            },
        ];
        for request in requests {
            let response = dispatcher.dispatch(&registration(), &request);
            assert_eq!(response.id(), request.id());
            assert_eq!(response.operation(), Some(request.operation()));
        }
        assert!(matches!(
            dispatcher.dispatch(&registration(), &completion(4, "test-model", 0.25, 256)),
            DataPlaneResponse::Failed {
                failure: DataPlaneFailure::Completion,
                ..
            }
        ));
    }

    #[test]
    fn directions_unknown_channels_and_model_drift_are_denied() {
        let backend: Arc<dyn StorageBackend> = Arc::new(InMemoryStorageBackend::new());
        install_binding(backend.as_ref());
        let dispatcher = dispatcher(Arc::clone(&backend), Arc::new(EchoService));
        let requests = [
            DataPlaneRequest::Receive {
                id: request_id(1),
                channel_id: uuid_v7(2),
                limit: 1,
            },
            DataPlaneRequest::Publish {
                id: request_id(2),
                channel_id: uuid_v7(1),
                content_type: "text/plain".to_string(),
                payload: Vec::new(),
            },
            DataPlaneRequest::Receive {
                id: request_id(3),
                channel_id: uuid_v7(8),
                limit: 1,
            },
            completion(4, "wrong-model", 0.25, 256),
            completion(5, "test-model", 0.5, 256),
            completion(6, "test-model", 0.25, 128),
        ];
        for request in requests {
            assert!(matches!(
                dispatcher.dispatch(&registration(), &request),
                DataPlaneResponse::Failed {
                    failure: DataPlaneFailure::Unauthorized,
                    ..
                }
            ));
        }
    }

    #[test]
    fn every_request_revalidates_current_durable_authority() {
        let backend: Arc<dyn StorageBackend> = Arc::new(InMemoryStorageBackend::new());
        install_binding(backend.as_ref());
        let dispatcher = dispatcher(Arc::clone(&backend), Arc::new(EchoService));
        let request = DataPlaneRequest::Receive {
            id: request_id(1),
            channel_id: uuid_v7(1),
            limit: 1,
        };
        assert!(matches!(
            dispatcher.dispatch(&registration(), &request),
            DataPlaneResponse::Received { .. }
        ));
        ChannelDefinitionStore::new(backend.as_ref())
            .destroy(ChannelId(uuid_v7(1)))
            .unwrap();
        assert!(matches!(
            dispatcher.dispatch(&registration(), &request),
            DataPlaneResponse::Failed {
                failure: DataPlaneFailure::Unauthorized,
                ..
            }
        ));
    }

    struct WrongResponseService;

    impl HostDataPlaneService for WrongResponseService {
        fn execute(
            &self,
            _binding: &HostPipelineBinding,
            request: &DataPlaneRequest,
        ) -> Result<DataPlaneResponse, DataPlaneFailure> {
            Ok(DataPlaneResponse::Acknowledged {
                id: request.id(),
                sequence: 1,
            })
        }
    }

    #[test]
    fn unavailable_and_malformed_services_are_redacted() {
        let backend: Arc<dyn StorageBackend> = Arc::new(InMemoryStorageBackend::new());
        install_binding(backend.as_ref());
        let request = DataPlaneRequest::Receive {
            id: request_id(1),
            channel_id: uuid_v7(1),
            limit: 1,
        };
        assert!(matches!(
            dispatcher(
                Arc::clone(&backend),
                Arc::new(UnavailableHostDataPlaneService)
            )
            .dispatch(&registration(), &request),
            DataPlaneResponse::Failed {
                failure: DataPlaneFailure::Unavailable,
                ..
            }
        ));
        assert!(matches!(
            dispatcher(backend, Arc::new(WrongResponseService)).dispatch(&registration(), &request),
            DataPlaneResponse::Failed {
                failure: DataPlaneFailure::Internal,
                ..
            }
        ));
    }

    #[test]
    fn tool_outputs_cannot_escape_the_offered_catalog_or_choice() {
        let tools = vec![ModelToolDefinition {
            name: "smart_home.list_entities".to_string(),
            description: "List normalized entities".to_string(),
            input_schema: serde_json::json!({"type": "object"}),
        }];
        let offered = GatewayToolCompletionOutput::ToolCall(GatewayModelToolCall {
            call_id: "call-1".to_string(),
            name: "smart_home.list_entities".to_string(),
            arguments: serde_json::json!({}),
        });
        let unoffered = GatewayToolCompletionOutput::ToolCall(GatewayModelToolCall {
            call_id: "call-2".to_string(),
            name: "smart_home.command".to_string(),
            arguments: serde_json::json!({}),
        });
        assert!(gateway_tool_output_allowed(
            &offered,
            &tools,
            &ModelToolChoice::Required
        ));
        assert!(!gateway_tool_output_allowed(
            &unoffered,
            &tools,
            &ModelToolChoice::Auto
        ));
        assert!(!gateway_tool_output_allowed(
            &GatewayToolCompletionOutput::FinalText("done".to_string()),
            &tools,
            &ModelToolChoice::Named("smart_home.list_entities".to_string())
        ));
    }

    #[test]
    fn exact_channel_key_authority_is_zeroizing_directional_and_identity_scoped() {
        let backend = InMemoryStorageBackend::new();
        let binding = install_real_binding(&backend);
        let mut keys = ExactChannelKeyAuthority::new();
        assert!(keys.is_empty());
        assert_eq!(
            keys.register_receiver(
                binding.pipeline_id(),
                binding.agent_id(),
                ChannelId(uuid_v7(1)),
                Zeroizing::new([0; 32]),
            ),
            Err(ChannelKeyRegistrationError::InvalidSecret)
        );
        keys.register_receiver(
            binding.pipeline_id(),
            binding.agent_id(),
            ChannelId(uuid_v7(1)),
            Zeroizing::new([0x13; 32]),
        )
        .unwrap();
        assert_eq!(
            keys.register_receiver(
                binding.pipeline_id(),
                binding.agent_id(),
                ChannelId(uuid_v7(1)),
                Zeroizing::new([0x14; 32]),
            ),
            Err(ChannelKeyRegistrationError::Duplicate)
        );
        assert_eq!(
            keys.register_originator(
                binding.pipeline_id(),
                binding.agent_id(),
                ChannelId(uuid_v7(1)),
                Zeroizing::new([0x21; 32]),
                Zeroizing::new([0x22; 32]),
            ),
            Err(ChannelKeyRegistrationError::Duplicate)
        );
        assert_eq!(
            keys.register_originator(
                binding.pipeline_id(),
                binding.agent_id(),
                ChannelId(uuid_v7(2)),
                Zeroizing::new([0x21; 32]),
                Zeroizing::new([0; 32]),
            ),
            Err(ChannelKeyRegistrationError::InvalidSecret)
        );
        keys.register_originator(
            binding.pipeline_id(),
            binding.agent_id(),
            ChannelId(uuid_v7(2)),
            Zeroizing::new([0x21; 32]),
            Zeroizing::new([0x22; 32]),
        )
        .unwrap();
        assert_eq!(keys.len(), 2);

        assert_eq!(
            keys.receiver_key(&binding, ChannelId(uuid_v7(1)))
                .unwrap()
                .public_key(),
            ReceiverKeyPair::from_private_key([0x13; 32])
                .unwrap()
                .public_key()
        );
        let originator = keys
            .originator_keys(&binding, ChannelId(uuid_v7(2)))
            .unwrap();
        assert_eq!(
            originator.signing_key.public_key(),
            OriginatorSigningKey::from_seed([0x21; 32]).public_key()
        );
        assert_eq!(originator.channel_key.as_bytes(), &[0x22; 32]);

        assert!(matches!(
            keys.originator_keys(&binding, ChannelId(uuid_v7(1))),
            Err(ChannelKeyAuthorityError::Unauthorized)
        ));
        assert!(matches!(
            keys.receiver_key(&binding, ChannelId(uuid_v7(2))),
            Err(ChannelKeyAuthorityError::Unauthorized)
        ));
        assert!(matches!(
            keys.receiver_key(&binding, ChannelId(uuid_v7(8))),
            Err(ChannelKeyAuthorityError::Unauthorized)
        ));

        let wrong_pipeline = HostPipelineBinding::new(
            PipelineId::new(uuid_v7(8)).unwrap(),
            binding.registration().clone(),
            binding.agent_id().clone(),
            binding.launch_bindings().clone(),
        );
        assert!(matches!(
            keys.receiver_key(&wrong_pipeline, ChannelId(uuid_v7(1))),
            Err(ChannelKeyAuthorityError::Unauthorized)
        ));
        let wrong_agent = HostPipelineBinding::new(
            binding.pipeline_id(),
            binding.registration().clone(),
            agent("other-agent"),
            binding.launch_bindings().clone(),
        );
        assert!(matches!(
            keys.receiver_key(&wrong_agent, ChannelId(uuid_v7(1))),
            Err(ChannelKeyAuthorityError::Unauthorized)
        ));

        let wrong_directions = HostPipelineBinding::new(
            binding.pipeline_id(),
            binding.registration().clone(),
            binding.agent_id().clone(),
            chief_of_staff_host_control_protocol::LaunchBindings::new(
                vec![
                    ChannelBinding::new(
                        "weather-requests",
                        ChannelBindingAccess::Write,
                        uuid_v7(1),
                    )
                    .unwrap(),
                    ChannelBinding::new("weather-reports", ChannelBindingAccess::Read, uuid_v7(2))
                        .unwrap(),
                ],
                binding.launch_bindings().level_one_model().cloned(),
            )
            .unwrap(),
        );
        assert!(matches!(
            keys.receiver_key(&wrong_directions, ChannelId(uuid_v7(1))),
            Err(ChannelKeyAuthorityError::Unauthorized)
        ));
        assert!(matches!(
            keys.originator_keys(&wrong_directions, ChannelId(uuid_v7(2))),
            Err(ChannelKeyAuthorityError::Unauthorized)
        ));
    }

    #[test]
    fn authority_backed_service_executes_real_encrypted_turn() {
        let backend: Arc<dyn StorageBackend> = Arc::new(InMemoryStorageBackend::new());
        let binding = install_real_binding(backend.as_ref());
        let DataPlaneRequest::Complete {
            call: completion_call,
            ..
        } = completion(9, "test-model", 0.25, 256)
        else {
            unreachable!();
        };
        let tool_call = ToolCompletionCall {
            completion: completion_call,
            tools: vec![ModelToolDefinition {
                name: "smart_home.list_entities".to_string(),
                description: "List normalized entities".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "additionalProperties": false
                }),
            }],
            choice: ModelToolChoice::Required,
            results: vec![ModelToolResult {
                call: ModelToolCall {
                    call_id: "prior-1".to_string(),
                    name: "smart_home.list_entities".to_string(),
                    arguments: serde_json::json!({}),
                },
                output: serde_json::json!({"entities": []}),
                is_error: false,
            }],
        };
        let tool_fingerprint =
            RequestFingerprint::for_tool_completion(&GatewayToolCompletionRequest {
                completion: gateway_completion_request(&tool_call.completion),
                tools: tool_call
                    .tools
                    .iter()
                    .map(gateway_tool_definition)
                    .collect(),
                choice: GatewayModelToolChoice::Required,
                results: tool_call.results.iter().map(gateway_tool_result).collect(),
            });
        let mut models = ExactModelProviderRegistry::new();
        models
            .register(
                "test-model",
                Arc::new(
                    MockLlmClient::new()
                        .with_response(
                            tool_fingerprint,
                            MockResponse::ToolCall(GatewayModelToolCall {
                                call_id: "next-2".to_string(),
                                name: "smart_home.list_entities".to_string(),
                                arguments: serde_json::json!({}),
                            }),
                        )
                        .with_default(MockResponse::Text("Bring an umbrella".to_string()))
                        .with_identity(ProviderIdentity {
                            vendor: "fixture".to_string(),
                            model_family: "weather".to_string(),
                            model_version: "v1".to_string(),
                            endpoint: Some("in-process".to_string()),
                        }),
                ),
            )
            .unwrap();
        let service = AuthorityBackedHostDataPlaneService::new(
            Arc::clone(&backend),
            Arc::new(exact_keys(&binding)),
            Arc::new(models),
            Arc::new(FixedMetadata {
                message_id: uuid_v7(5),
                timestamp_ns: 11,
            }),
        );

        let received = service
            .execute(
                &binding,
                &DataPlaneRequest::Receive {
                    id: request_id(1),
                    channel_id: uuid_v7(1),
                    limit: 1,
                },
            )
            .unwrap();
        let DataPlaneResponse::Received { messages, .. } = received else {
            panic!("expected received response");
        };
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].payload, b"Seattle");

        let completed = service
            .execute(&binding, &completion(2, "test-model", 0.25, 256))
            .unwrap();
        let DataPlaneResponse::Completed {
            result: completion_result,
            ..
        } = completed
        else {
            panic!("expected completed response");
        };
        assert_eq!(completion_result.text, "Bring an umbrella");
        assert_eq!(completion_result.provider.vendor, "fixture");

        let tool_completed = service
            .execute(
                &binding,
                &DataPlaneRequest::CompleteWithTools {
                    id: request_id(6),
                    call: Box::new(tool_call),
                },
            )
            .unwrap();
        let DataPlaneResponse::ToolCompleted {
            result: tool_result,
            ..
        } = tool_completed
        else {
            panic!("expected tool-completed response");
        };
        assert_eq!(tool_result.provider.vendor, "fixture");
        assert!(!tool_result.polyfill_used);
        assert_eq!(
            tool_result.output,
            ToolCompletionOutput::ToolCall(ModelToolCall {
                call_id: "next-2".to_string(),
                name: "smart_home.list_entities".to_string(),
                arguments: serde_json::json!({}),
            })
        );

        let published = service
            .execute(
                &binding,
                &DataPlaneRequest::Publish {
                    id: request_id(3),
                    channel_id: uuid_v7(2),
                    content_type: "text/plain".to_string(),
                    payload: completion_result.text.into_bytes(),
                },
            )
            .unwrap();
        assert!(matches!(
            published,
            DataPlaneResponse::Published {
                message_id,
                sequence: 0,
                timestamp_ns: 11,
                ..
            } if message_id == uuid_v7(5)
        ));

        let acknowledged = service
            .execute(
                &binding,
                &DataPlaneRequest::Acknowledge {
                    id: request_id(4),
                    channel_id: uuid_v7(1),
                    message_id: messages[0].message_id,
                },
            )
            .unwrap();
        assert!(matches!(
            acknowledged,
            DataPlaneResponse::Acknowledged { sequence: 1, .. }
        ));
        assert_eq!(
            ChannelStore::new(backend.as_ref(), ChannelId(uuid_v7(1)))
                .receiver_cursor(b"weather-agent")
                .unwrap(),
            Sequence(1)
        );

        let mut sink = DurableReceiver::open(
            backend.as_ref(),
            ChannelId(uuid_v7(2)),
            agent("report-sink"),
            ReceiverKeyPair::from_private_key([0x23; 32]).unwrap(),
        )
        .unwrap();
        let reports = sink.receive(1).unwrap();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].payload, b"Bring an umbrella");
    }

    #[test]
    fn exact_model_registry_and_delivery_ledger_fail_closed() {
        let backend: Arc<dyn StorageBackend> = Arc::new(InMemoryStorageBackend::new());
        let binding = install_real_binding(backend.as_ref());
        let mut models = ExactModelProviderRegistry::new();
        let provider: Arc<dyn LlmClient> = Arc::new(
            MockLlmClient::new()
                .with_default(MockResponse::Text("forecast".to_string()))
                .with_identity(ProviderIdentity {
                    vendor: "x".repeat(513),
                    model_family: "weather".to_string(),
                    model_version: "v1".to_string(),
                    endpoint: None,
                }),
        );
        assert!(models.register("", Arc::clone(&provider)).is_err());
        models
            .register("test-model", Arc::clone(&provider))
            .unwrap();
        assert!(models.register("test-model", provider).is_err());
        assert!(models.resolve(&binding, "other-model").is_err());
        let service = AuthorityBackedHostDataPlaneService::new(
            backend,
            Arc::new(exact_keys(&binding)),
            Arc::new(models),
            Arc::new(FixedMetadata {
                message_id: uuid_v7(5),
                timestamp_ns: 11,
            }),
        );
        assert_eq!(
            service.execute(
                &binding,
                &DataPlaneRequest::Acknowledge {
                    id: request_id(1),
                    channel_id: uuid_v7(1),
                    message_id: uuid_v7(4),
                },
            ),
            Err(DataPlaneFailure::Unauthorized)
        );
        assert_eq!(
            service.execute(&binding, &completion(2, "test-model", 0.25, 256)),
            Err(DataPlaneFailure::Completion)
        );
    }
}
