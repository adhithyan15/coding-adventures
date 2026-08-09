//! Durable authorization and injected execution for the D18 host data plane.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use chief_of_staff_host_control_protocol::{
    ChannelBindingAccess, DataPlaneFailure, DataPlaneRequest, DataPlaneResponse,
};
use chief_of_staff_pipeline_bindings::{
    HostPipelineBinding, PipelineBindingError, PipelineBindingStore,
};
use chief_of_staff_service_registry::HostRegistration;
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
    use chief_of_staff_channel_crypto::{ChannelId, KeyEpoch};
    use chief_of_staff_channel_endpoints::{
        AgentId, ChannelDefinition, ChannelDefinitionStore, OriginatorIdentity, ReceiverIdentity,
    };
    use chief_of_staff_host_control_protocol::{
        ChannelBinding, CompletionCall, DataPlaneMessage, LevelOneModelBinding, PromptMessage,
        PromptRole, RequestId,
    };
    use chief_of_staff_pipeline_bindings::PipelineId;
    use chief_of_staff_service_registry::{
        DesiredState, HostEntry, HostName, PackagePath, RestartPolicy, ServiceRegistry,
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
}
