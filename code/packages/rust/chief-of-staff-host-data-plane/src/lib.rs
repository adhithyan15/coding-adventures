//! Durable authorization and injected execution for the D18 host data plane.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use chief_of_staff_host_control_protocol::{
    validate_data_plane_response, ChannelBindingAccess, CompletionCall, CompletionFinishReason,
    CompletionProvider, CompletionResult, CompletionUsage, DataPlaneFailure, DataPlaneRequest,
    DataPlaneResponse, ModelToolCall, ModelToolChoice, ModelToolDefinition, ModelToolResult,
    PromptRole, ToolCompletionCall, ToolCompletionOutput, ToolCompletionResult,
    MAX_DATA_PLANE_PAYLOAD_BYTES,
};
use chief_of_staff_pipeline_bindings::{
    HostPipelineBinding, PipelineBindingError, PipelineBindingStore,
};
use chief_of_staff_service_registry::HostRegistration;
use llm_gateway::{
    CompletionRequest, FinishReason, LlmClient, Message, MessageContent,
    ModelToolCall as GatewayModelToolCall, ModelToolChoice as GatewayModelToolChoice,
    ModelToolDefinition as GatewayModelToolDefinition, ModelToolResult as GatewayModelToolResult,
    Role, ToolCompletionOutput as GatewayToolCompletionOutput,
    ToolCompletionRequest as GatewayToolCompletionRequest,
};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use storage_core::StorageBackend;

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

/// Fail-closed service used when no model provider or tool grant is composed.
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

/// D18D catalog and execution authority injected into the authenticated model boundary.
pub trait ModelToolDispatcher: Send + Sync {
    /// Return the definitions this exact durable host binding may offer to a model.
    fn definitions(
        &self,
        binding: &HostPipelineBinding,
    ) -> Result<Vec<ModelToolDefinition>, DataPlaneFailure>;

    /// Execute one model-returned call through the canonical D18D runtime.
    fn execute(
        &self,
        binding: &HostPipelineBinding,
        call: &ModelToolCall,
    ) -> Result<ModelToolResult, DataPlaneFailure>;
}

/// Several dispatchers behind one, routed by tool name.
///
/// The daemon composed exactly one `ModelToolDispatcher`, and it was the
/// hardcoded smart-home one. That is what made smart home *the* tool surface
/// rather than one agent among several: there was no way to add a second
/// source without replacing the first.
///
/// Routing is by tool name against each source's own `definitions`, so a
/// source owns exactly the tools it declares and nothing else.
pub struct CompositeModelToolDispatcher {
    sources: Vec<Arc<dyn ModelToolDispatcher>>,
}

impl CompositeModelToolDispatcher {
    /// Build a composite over `sources`, in order.
    pub fn new(sources: Vec<Arc<dyn ModelToolDispatcher>>) -> Self {
        Self { sources }
    }

    /// The offered set and its owner index, built in ONE pass.
    ///
    /// Both public methods go through this, and that is the point. A first
    /// version had `definitions` dedup with its own `seen` set while `owner`
    /// scanned each source independently -- so the two disagreed. With sources
    /// `[A:{x,y}, B:{x}]`, `definitions` refused the whole surface while
    /// `execute("y")` still succeeded from A: a composition the offer path had
    /// declared invalid went on executing tools. Within-source duplicates were
    /// invisible to `owner` entirely, because `.any()` matches once per source.
    ///
    /// Ambiguity is refused rather than resolved by order. Preferring the
    /// earlier source would mean the tool a model called and the tool that ran
    /// could differ by nothing but construction order -- a bug that reproduces
    /// only under a particular build.
    fn index(
        &self,
        binding: &HostPipelineBinding,
    ) -> Result<(Vec<ModelToolDefinition>, BTreeMap<String, usize>), DataPlaneFailure> {
        let mut definitions = Vec::new();
        let mut owners: BTreeMap<String, usize> = BTreeMap::new();
        for (position, source) in self.sources.iter().enumerate() {
            for definition in source.definitions(binding)? {
                // `Internal`, not `InvalidRequest`: a duplicate name is a host
                // COMPOSITION fault, and reporting it as a client fault both
                // mislabels it and hands an authorized child an oracle for
                // probing cross-source name collisions.
                if owners.insert(definition.name.clone(), position).is_some() {
                    return Err(DataPlaneFailure::Internal);
                }
                definitions.push(definition);
            }
        }
        Ok((definitions, owners))
    }
}

impl ModelToolDispatcher for CompositeModelToolDispatcher {
    fn definitions(
        &self,
        binding: &HostPipelineBinding,
    ) -> Result<Vec<ModelToolDefinition>, DataPlaneFailure> {
        Ok(self.index(binding)?.0)
    }

    fn execute(
        &self,
        binding: &HostPipelineBinding,
        call: &ModelToolCall,
    ) -> Result<ModelToolResult, DataPlaneFailure> {
        let (_, owners) = self.index(binding)?;
        let position = owners
            .get(&call.name)
            .copied()
            .ok_or(DataPlaneFailure::Unauthorized)?;
        self.sources[position].execute(binding, call)
    }
}

/// Fail-closed tool authority used when the daemon installed no D18D catalog.
#[derive(Default)]
pub struct UnavailableModelToolDispatcher;

impl ModelToolDispatcher for UnavailableModelToolDispatcher {
    fn definitions(
        &self,
        _binding: &HostPipelineBinding,
    ) -> Result<Vec<ModelToolDefinition>, DataPlaneFailure> {
        Err(DataPlaneFailure::Unavailable)
    }

    fn execute(
        &self,
        _binding: &HostPipelineBinding,
        _call: &ModelToolCall,
    ) -> Result<ModelToolResult, DataPlaneFailure> {
        Err(DataPlaneFailure::Unavailable)
    }
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

/// Concrete service that executes authorized model operations against exact
/// provider-neutral LLM clients and an injected D18D tool catalog.
///
/// It serves no channel operation. Since D18S P2.6d-2c each agent's channel
/// keys live only in that agent's broker, which answers its channel requests
/// itself; the daemon holds none. A channel request that reaches this
/// service is answered `Unauthorized`, and in production none can: the
/// supervisor refuses to launch a host bound to a channel without a broker.
pub struct AuthorityBackedHostDataPlaneService {
    model_authority: Arc<dyn ModelProviderAuthority>,
    model_tools: Arc<dyn ModelToolDispatcher>,
}

impl AuthorityBackedHostDataPlaneService {
    /// Compose exact model authority, with no model tools.
    pub fn new(model_authority: Arc<dyn ModelProviderAuthority>) -> Self {
        Self {
            model_authority,
            model_tools: Arc::new(UnavailableModelToolDispatcher),
        }
    }

    /// Compose the data plane with an explicit D18D catalog and execution authority.
    pub fn with_model_tools(
        model_authority: Arc<dyn ModelProviderAuthority>,
        model_tools: Arc<dyn ModelToolDispatcher>,
    ) -> Self {
        Self {
            model_authority,
            model_tools,
        }
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
        let installed = self.model_tools.definitions(binding)?;
        if call.tools != installed {
            return Err(DataPlaneFailure::Unauthorized);
        }
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

    fn list_model_tools(
        &self,
        binding: &HostPipelineBinding,
        id: chief_of_staff_host_control_protocol::RequestId,
    ) -> Result<DataPlaneResponse, DataPlaneFailure> {
        Ok(DataPlaneResponse::ModelToolsListed {
            id,
            tools: self.model_tools.definitions(binding)?,
        })
    }

    fn execute_tool(
        &self,
        binding: &HostPipelineBinding,
        id: chief_of_staff_host_control_protocol::RequestId,
        call: &ModelToolCall,
    ) -> Result<DataPlaneResponse, DataPlaneFailure> {
        let result = self.model_tools.execute(binding, call)?;
        if result.call != *call {
            return Err(DataPlaneFailure::Internal);
        }
        Ok(DataPlaneResponse::ToolExecuted {
            id,
            result: Box::new(result),
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
            // Channel operations are the agent's broker's (P2.6d-2c).
            DataPlaneRequest::Receive { .. }
            | DataPlaneRequest::Publish { .. }
            | DataPlaneRequest::Acknowledge { .. } => Err(DataPlaneFailure::Unauthorized),
            DataPlaneRequest::Complete { id, call } => self.complete(binding, *id, call),
            DataPlaneRequest::CompleteWithTools { id, call } => {
                self.complete_with_tools(binding, *id, call)
            }
            DataPlaneRequest::ExecuteTool { id, call } => self.execute_tool(binding, *id, call),
            DataPlaneRequest::ListModelTools { id } => self.list_model_tools(binding, *id),
        }?;
        validate_data_plane_response(&response).map_err(|_| match request {
            DataPlaneRequest::Complete { .. } | DataPlaneRequest::CompleteWithTools { .. } => {
                DataPlaneFailure::Completion
            }
            DataPlaneRequest::ExecuteTool { .. } | DataPlaneRequest::ListModelTools { .. } => {
                DataPlaneFailure::Internal
            }
            _ => DataPlaneFailure::Channel,
        })?;
        Ok(response)
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
        DataPlaneRequest::ExecuteTool { .. } | DataPlaneRequest::ListModelTools { .. } => {
            binding.launch_bindings().level_one_model().is_some()
        }
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

    struct FixedTools {
        names: Vec<&'static str>,
        answer: &'static str,
    }

    impl ModelToolDispatcher for FixedTools {
        fn definitions(
            &self,
            _binding: &HostPipelineBinding,
        ) -> Result<Vec<ModelToolDefinition>, DataPlaneFailure> {
            Ok(self
                .names
                .iter()
                .map(|name| ModelToolDefinition {
                    name: (*name).to_string(),
                    description: "fixture".to_string(),
                    input_schema: serde_json::json!({"type": "object"}),
                })
                .collect())
        }

        fn execute(
            &self,
            _binding: &HostPipelineBinding,
            call: &ModelToolCall,
        ) -> Result<ModelToolResult, DataPlaneFailure> {
            Ok(ModelToolResult {
                call: call.clone(),
                output: serde_json::json!({"source": self.answer}),
                is_error: false,
            })
        }
    }

    fn composite_binding() -> HostPipelineBinding {
        // The composite never reads the binding -- it routes on tool name --
        // but the trait takes one, so build a real fixture rather than
        // widening the trait for a test.
        let backend = InMemoryStorageBackend::new();
        install_binding(&backend)
    }

    #[test]
    fn a_composite_offers_every_sources_tools() {
        // The daemon could hold exactly one dispatcher, which is what made
        // smart home THE tool surface rather than one agent among several.
        let composite = CompositeModelToolDispatcher::new(vec![
            Arc::new(FixedTools {
                names: vec!["smart_home.get_state"],
                answer: "home",
            }),
            Arc::new(FixedTools {
                names: vec!["notes.append", "notes.read"],
                answer: "notes",
            }),
        ]);
        let names = composite
            .definitions(&composite_binding())
            .expect("definitions compose")
            .into_iter()
            .map(|definition| definition.name)
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec!["smart_home.get_state", "notes.append", "notes.read"]
        );
    }

    #[test]
    fn a_call_routes_to_the_source_that_declared_it() {
        let composite = CompositeModelToolDispatcher::new(vec![
            Arc::new(FixedTools {
                names: vec!["smart_home.get_state"],
                answer: "home",
            }),
            Arc::new(FixedTools {
                names: vec!["notes.append"],
                answer: "notes",
            }),
        ]);
        let binding = composite_binding();
        for (tool, expected) in [("smart_home.get_state", "home"), ("notes.append", "notes")] {
            let result = composite
                .execute(
                    &binding,
                    &ModelToolCall {
                        call_id: "call-1".to_string(),
                        name: tool.to_string(),
                        arguments: serde_json::json!({}),
                    },
                )
                .expect("a declared tool routes");
            assert_eq!(
                result.output,
                serde_json::json!({"source": expected}),
                "{tool} routed to the wrong source"
            );
        }
    }

    #[test]
    fn a_tool_no_source_declared_is_unauthorized() {
        let composite = CompositeModelToolDispatcher::new(vec![Arc::new(FixedTools {
            names: vec!["notes.append"],
            answer: "notes",
        })]);
        assert!(matches!(
            composite.execute(
                &composite_binding(),
                &ModelToolCall {
                    call_id: "call-1".to_string(),
                    name: "smart_home.get_state".to_string(),
                    arguments: serde_json::json!({}),
                },
            ),
            Err(DataPlaneFailure::Unauthorized)
        ));
    }

    #[test]
    fn two_sources_claiming_one_tool_is_an_error_not_a_race() {
        // First-match-wins would mean the tool a model called and the tool
        // that ran could differ by nothing but construction order. Refused on
        // both the offer and the execute path.
        let composite = CompositeModelToolDispatcher::new(vec![
            Arc::new(FixedTools {
                names: vec!["shared.tool"],
                answer: "first",
            }),
            Arc::new(FixedTools {
                names: vec!["shared.tool"],
                answer: "second",
            }),
        ]);
        let binding = composite_binding();
        assert!(matches!(
            composite.definitions(&binding),
            Err(DataPlaneFailure::Internal)
        ));
        assert!(matches!(
            composite.execute(
                &binding,
                &ModelToolCall {
                    call_id: "call-1".to_string(),
                    name: "shared.tool".to_string(),
                    arguments: serde_json::json!({}),
                },
            ),
            Err(DataPlaneFailure::Internal)
        ));
    }

    #[test]
    fn an_ambiguous_composition_cannot_execute_its_unambiguous_tools_either() {
        // The case the symmetric duplicate test cannot reach, and the one that
        // exposed the two paths disagreeing: with `[A:{x,y}, B:{x}]` an
        // earlier version refused the whole surface from `definitions` while
        // `execute("y")` still succeeded from A -- a composition the offer
        // path had already declared invalid went on executing tools.
        let composite = CompositeModelToolDispatcher::new(vec![
            Arc::new(FixedTools {
                names: vec!["shared.tool", "unique.tool"],
                answer: "first",
            }),
            Arc::new(FixedTools {
                names: vec!["shared.tool"],
                answer: "second",
            }),
        ]);
        assert!(matches!(
            composite.execute(
                &composite_binding(),
                &ModelToolCall {
                    call_id: "call-1".to_string(),
                    name: "unique.tool".to_string(),
                    arguments: serde_json::json!({}),
                },
            ),
            Err(DataPlaneFailure::Internal)
        ));
    }

    #[test]
    fn a_duplicate_within_one_source_is_caught_too() {
        // `.any()` matched once per source, so a source declaring the same
        // name twice was invisible to the execute path entirely.
        let composite = CompositeModelToolDispatcher::new(vec![Arc::new(FixedTools {
            names: vec!["dup.tool", "dup.tool"],
            answer: "only",
        })]);
        let binding = composite_binding();
        assert!(matches!(
            composite.definitions(&binding),
            Err(DataPlaneFailure::Internal)
        ));
        assert!(matches!(
            composite.execute(
                &binding,
                &ModelToolCall {
                    call_id: "call-1".to_string(),
                    name: "dup.tool".to_string(),
                    arguments: serde_json::json!({}),
                },
            ),
            Err(DataPlaneFailure::Internal)
        ));
    }

    #[test]
    fn an_empty_composite_offers_nothing_and_routes_nothing() {
        // Named for what it checks. It asserts the COMPOSITE refuses every
        // call, not that the service refuses the request: an empty offered
        // set makes `call.tools != installed` compare `[] != []`, which
        // passes, and the completion proceeds without tools. The dispatcher
        // this replaces returned Unavailable and refused outright.
        let composite = CompositeModelToolDispatcher::new(Vec::new());
        assert!(composite
            .definitions(&composite_binding())
            .expect("an empty composite still answers")
            .is_empty());
        assert!(matches!(
            composite.execute(
                &composite_binding(),
                &ModelToolCall {
                    call_id: "call-1".to_string(),
                    name: "anything".to_string(),
                    arguments: serde_json::json!({}),
                },
            ),
            Err(DataPlaneFailure::Unauthorized)
        ));
    }
    use super::*;
    use chief_of_staff_channel_crypto::Sequence;
    use chief_of_staff_channel_crypto::{
        ChannelId, ChannelMasterKey, KeyEpoch, OriginatorSigningKey, ReceiverKeyPair,
    };
    use chief_of_staff_channel_endpoints::{
        AgentId, ChannelDefinition, ChannelDefinitionStore, DurableOriginator, MessageId,
        MessageMetadata, MessageMetadataError, MessageMetadataSource, Originator,
        OriginatorIdentity, ReceiverIdentity,
    };
    use chief_of_staff_channel_store::ChannelStore;
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
                DataPlaneRequest::ExecuteTool { id, .. } => DataPlaneResponse::Failed {
                    id: *id,
                    failure: DataPlaneFailure::Internal,
                },
                DataPlaneRequest::ListModelTools { id } => DataPlaneResponse::Failed {
                    id: *id,
                    failure: DataPlaneFailure::Internal,
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

    struct FixtureModelTools {
        definitions: Vec<ModelToolDefinition>,
    }

    impl ModelToolDispatcher for FixtureModelTools {
        fn definitions(
            &self,
            _binding: &HostPipelineBinding,
        ) -> Result<Vec<ModelToolDefinition>, DataPlaneFailure> {
            Ok(self.definitions.clone())
        }

        fn execute(
            &self,
            _binding: &HostPipelineBinding,
            call: &ModelToolCall,
        ) -> Result<ModelToolResult, DataPlaneFailure> {
            Ok(ModelToolResult {
                call: call.clone(),
                output: serde_json::json!({"healthy": true}),
                is_error: false,
            })
        }
    }

    #[test]
    fn authority_backed_service_executes_model_turns_and_no_channel_operation() {
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
        let installed_tools = tool_call.tools.clone();
        let service = AuthorityBackedHostDataPlaneService::with_model_tools(
            Arc::new(models),
            Arc::new(FixtureModelTools {
                definitions: installed_tools,
            }),
        );

        // The daemon serves no channel operation (P2.6d-2c), even one the
        // binding authorizes: those are the agent's broker's.
        for request in [
            DataPlaneRequest::Receive {
                id: request_id(1),
                channel_id: uuid_v7(1),
                limit: 1,
            },
            DataPlaneRequest::Publish {
                id: request_id(3),
                channel_id: uuid_v7(2),
                content_type: "text/plain".to_string(),
                payload: b"forecast".to_vec(),
            },
            DataPlaneRequest::Acknowledge {
                id: request_id(4),
                channel_id: uuid_v7(1),
                message_id: uuid_v7(4),
            },
        ] {
            assert_eq!(
                service.execute(&binding, &request),
                Err(DataPlaneFailure::Unauthorized)
            );
        }
        // And nothing was consumed from the input channel.
        assert_eq!(
            ChannelStore::new(backend.as_ref(), ChannelId(uuid_v7(1)))
                .receiver_cursor(b"weather-agent")
                .unwrap(),
            Sequence(0)
        );

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

        let listed = service
            .execute(
                &binding,
                &DataPlaneRequest::ListModelTools { id: request_id(5) },
            )
            .unwrap();
        let DataPlaneResponse::ModelToolsListed { tools, .. } = listed else {
            panic!("expected model-tools-listed response");
        };
        assert_eq!(tools, tool_call.tools);

        let tool_completed = service
            .execute(
                &binding,
                &DataPlaneRequest::CompleteWithTools {
                    id: request_id(6),
                    call: Box::new(tool_call.clone()),
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

        let ToolCompletionOutput::ToolCall(model_call) = tool_result.output.clone() else {
            unreachable!();
        };
        let executed = service
            .execute(
                &binding,
                &DataPlaneRequest::ExecuteTool {
                    id: request_id(7),
                    call: Box::new(model_call.clone()),
                },
            )
            .unwrap();
        assert!(matches!(
            executed,
            DataPlaneResponse::ToolExecuted { result, .. }
                if result.call == model_call
                    && result.output == serde_json::json!({"healthy": true})
                    && !result.is_error
        ));

        let mut altered_catalog = tool_call;
        altered_catalog.tools[0].description = "Escalated definition".to_string();
        assert_eq!(
            service.execute(
                &binding,
                &DataPlaneRequest::CompleteWithTools {
                    id: request_id(8),
                    call: Box::new(altered_catalog),
                },
            ),
            Err(DataPlaneFailure::Unauthorized)
        );
    }

    #[test]
    fn exact_model_registry_fails_closed() {
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
        let service = AuthorityBackedHostDataPlaneService::new(Arc::new(models));
        assert_eq!(
            service.execute(&binding, &completion(2, "test-model", 0.25, 256)),
            Err(DataPlaneFailure::Completion)
        );
    }
}
