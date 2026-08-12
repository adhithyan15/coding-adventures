#[cfg(unix)]
use chief_of_staff_channel_crypto::{
    ChannelId, ChannelMasterKey, KeyEpoch, OriginatorSigningKey, ReceiverKeyPair, Sequence,
};
#[cfg(unix)]
use chief_of_staff_channel_endpoints::{
    AgentId, ChannelDefinition, ChannelDefinitionStore, DurableOriginator, DurableReceiver,
    MessageId, MessageMetadata, MessageMetadataError, MessageMetadataSource, Originator,
    OriginatorIdentity, Receiver, ReceiverIdentity,
};
#[cfg(unix)]
use chief_of_staff_channel_store::ChannelStore;
#[cfg(unix)]
use chief_of_staff_daemon::compose_host_data_plane;
#[cfg(unix)]
use chief_of_staff_daemon_config::parse_config;
use chief_of_staff_host_control_protocol::{
    ChannelBinding, ChannelBindingAccess, CompletionFinishReason, CompletionProvider,
    CompletionUsage, DataPlaneFailure, DataPlaneMessage, DataPlaneOperation, DataPlaneRequest,
    DataPlaneResponse, LaunchBindings, LevelOneModelBinding, ModelToolCall, ModelToolDefinition,
    ModelToolResult, PromptRole, ToolCompletionOutput, ToolCompletionResult,
};
use chief_of_staff_host_data_plane::HostDataPlaneDispatcher;
use chief_of_staff_host_runtime::{
    verify_agent_package, AgentPackageRuntime, PackageKeyType, PackageKeyring, TrustedPackageKey,
};
#[cfg(unix)]
use chief_of_staff_pipeline_bindings::{HostPipelineBinding, PipelineBindingStore, PipelineId};
#[cfg(unix)]
use chief_of_staff_process_supervisor::DurableHostLaunchBindings;
use chief_of_staff_process_supervisor::{
    HostLaunchBindingProvider, HostProgram, LaunchBindingProviderError, MonotonicClock,
    ProcessHostSupervisor, ProcessSupervisorConfig, ProcessSupervisorError, SessionIdSource,
};
use chief_of_staff_secure_host_channel::SessionId;
use chief_of_staff_service_reconciler::{HostSupervisor, SupervisorObservation, SupervisorPhase};
#[cfg(unix)]
use chief_of_staff_service_registry::{DesiredState, HostEntry, ServiceRegistry};
use chief_of_staff_service_registry::{HostName, HostRegistration, PackagePath, RestartPolicy};
use chief_of_staff_skill_package::build_signed_skill_package;
use chief_of_staff_skill_runtime::LEVEL_ONE_RESPONSE_CONTENT_TYPE;
use chief_of_staff_tool_api::PrivilegeTier;
use coding_adventures_ed25519::generate_keypair;
#[cfg(unix)]
use coding_adventures_storage_fs::FsStorageBackend;
use coding_adventures_x3dh::generate_identity_keypair;
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::io::{Read, Write};
#[cfg(unix)]
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
#[cfg(unix)]
use storage_core::StorageBackend;

const TEST_KEY_ID: &str = "level-one-host-test";
const TEST_SEED: [u8; 32] = [73; 32];
const SKILL: &str = "---\nagent: weather-reporter\ndescription: Reports a forecast for a requested city.\nprivilege_tier: 0\nreads: [weather-requests]\nwrites: [weather-reports]\nmessage_schema_versions: [weather-requests=1, weather-reports=1]\n---\n# Weather Reporter\n\nReport a brief forecast for the requested city.\n\n## Capabilities needed\n- none\n";

fn uuid_v7(last: u8) -> [u8; 16] {
    let mut bytes = [0; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes[15] = last;
    bytes
}

struct TestPackage {
    path: PathBuf,
    digest: [u8; 32],
}

impl TestPackage {
    fn new() -> (Self, PackageKeyring) {
        let path = std::env::temp_dir().join(format!(
            "chief-level-one-host-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let (public_key, secret_key) = generate_keypair(&TEST_SEED);
        build_signed_skill_package(&path, SKILL, TEST_KEY_ID, &secret_key).unwrap();
        let mut keyring = PackageKeyring::new();
        keyring
            .trust(
                TrustedPackageKey::new(
                    TEST_KEY_ID,
                    PackageKeyType::Developer,
                    public_key,
                    PrivilegeTier::Tier1,
                )
                .unwrap(),
            )
            .unwrap();
        let digest = verify_agent_package(&path, &keyring).unwrap().digest();
        (Self { path, digest }, keyring)
    }

    fn registration(&self) -> HostRegistration {
        HostRegistration::new(
            HostName::new("weather-level-one").unwrap(),
            PackagePath::new(self.path.to_str().unwrap()).unwrap(),
            self.digest,
            RestartPolicy::Always,
        )
    }
}

impl Drop for TestPackage {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[cfg(unix)]
struct TestHome(PathBuf);

#[cfg(unix)]
impl TestHome {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "chief-level-one-production-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        Self(path.canonicalize().unwrap())
    }

    fn write_secret(&self, name: &str, bytes: &[u8]) {
        use std::os::unix::fs::PermissionsExt;

        let keys = self.0.join("keys");
        fs::create_dir_all(&keys).unwrap();
        let path = keys.join(name);
        fs::write(&path, bytes).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    }
}

#[cfg(unix)]
impl Drop for TestHome {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[cfg(unix)]
struct FixedMetadata {
    message_id: [u8; 16],
    timestamp_ns: u64,
}

#[cfg(unix)]
impl MessageMetadataSource for FixedMetadata {
    fn next_metadata(&self) -> Result<MessageMetadata, MessageMetadataError> {
        Ok(MessageMetadata {
            message_id: MessageId::from_uuid_v7(self.message_id)
                .map_err(|_| MessageMetadataError::new("invalid fixture message ID"))?,
            timestamp_ns: self.timestamp_ns,
        })
    }
}

#[cfg(unix)]
struct ScriptedOllama {
    endpoint: String,
    request_body: Arc<Mutex<Option<String>>>,
    join: Option<thread::JoinHandle<()>>,
}

#[cfg(unix)]
impl ScriptedOllama {
    fn spawn() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let request_body = Arc::new(Mutex::new(None));
        let captured = Arc::clone(&request_body);
        let join = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            let mut buffer = [0; 4096];
            let (body_start, content_length) = loop {
                let count = stream.read(&mut buffer).unwrap();
                assert!(count > 0, "Ollama fixture request ended before headers");
                request.extend_from_slice(&buffer[..count]);
                let Some(header_end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n")
                else {
                    continue;
                };
                let headers = std::str::from_utf8(&request[..header_end]).unwrap();
                assert!(headers.starts_with("POST /api/chat HTTP/1.1\r\n"));
                let length = headers
                    .lines()
                    .find_map(|line| line.strip_prefix("Content-Length:"))
                    .map(str::trim)
                    .unwrap()
                    .parse::<usize>()
                    .unwrap();
                break (header_end + 4, length);
            };
            while request.len() - body_start < content_length {
                let count = stream.read(&mut buffer).unwrap();
                assert!(count > 0, "Ollama fixture request ended before its body");
                request.extend_from_slice(&buffer[..count]);
            }
            *captured.lock().unwrap() = Some(
                String::from_utf8(request[body_start..body_start + content_length].to_vec())
                    .unwrap(),
            );

            let body = r#"{"model":"weather-fixture","message":{"role":"assistant","content":"{\"kind\":\"final\",\"text\":\"Bring an umbrella.\"}"},"done":true,"done_reason":"stop","prompt_eval_count":12,"eval_count":4}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
        });
        Self {
            endpoint,
            request_body,
            join: Some(join),
        }
    }

    fn endpoint(&self) -> &str {
        &self.endpoint
    }

    fn finish(mut self) -> String {
        self.join.take().unwrap().join().unwrap();
        let body = self.request_body.lock().unwrap().take().unwrap();
        body
    }
}

struct TestLaunchBindings;

impl HostLaunchBindingProvider for TestLaunchBindings {
    fn launch_bindings(
        &self,
        _registration: &HostRegistration,
        runtime: AgentPackageRuntime,
    ) -> Result<LaunchBindings, LaunchBindingProviderError> {
        if runtime != AgentPackageRuntime::Skill {
            return Err(LaunchBindingProviderError);
        }
        LaunchBindings::new(
            vec![
                ChannelBinding::new("weather-requests", ChannelBindingAccess::Read, uuid_v7(1))
                    .unwrap(),
                ChannelBinding::new("weather-reports", ChannelBindingAccess::Write, uuid_v7(2))
                    .unwrap(),
            ],
            Some(LevelOneModelBinding::new("test-model", 0.0, 128).unwrap()),
        )
        .map_err(|_| LaunchBindingProviderError)
    }
}

#[derive(Default)]
struct TestClock(AtomicU64);

impl MonotonicClock for TestClock {
    fn now_ns(&self) -> u64 {
        self.0.fetch_add(1, Ordering::SeqCst) + 1
    }
}

struct TestSessions(u8);

impl SessionIdSource for TestSessions {
    fn next_session(&mut self) -> Result<SessionId, ProcessSupervisorError> {
        self.0 = self.0.wrapping_add(1);
        SessionId::new(uuid_v7(self.0)).map_err(|_| ProcessSupervisorError::SessionGeneration)
    }
}

#[derive(Default)]
struct ScriptedDataPlane {
    operations: Mutex<Vec<DataPlaneOperation>>,
}

impl ScriptedDataPlane {
    fn operations(&self) -> Vec<DataPlaneOperation> {
        self.operations.lock().unwrap().clone()
    }
}

fn test_model_tools() -> Vec<ModelToolDefinition> {
    vec![ModelToolDefinition {
        name: "smart_home.list_entities".to_string(),
        description: "List normalized entities".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "additionalProperties": false
        }),
    }]
}

fn tool_turn_result(output: ToolCompletionOutput) -> ToolCompletionResult {
    ToolCompletionResult {
        output,
        model: "test-model".to_string(),
        provider: CompletionProvider {
            vendor: "fixture".to_string(),
            model_family: "weather".to_string(),
            model_version: "1".to_string(),
            endpoint: None,
        },
        usage: CompletionUsage {
            input_tokens: 8,
            output_tokens: 4,
            cached_tokens: 0,
        },
        finish_reason: CompletionFinishReason::Stop,
        latency_ms: 1,
        polyfill_used: false,
    }
}

impl HostDataPlaneDispatcher for ScriptedDataPlane {
    fn dispatch(
        &self,
        _registration: &HostRegistration,
        request: &DataPlaneRequest,
    ) -> DataPlaneResponse {
        let mut operations = self.operations.lock().unwrap();
        match request {
            DataPlaneRequest::Receive {
                id,
                channel_id,
                limit,
            } => {
                assert_eq!(*channel_id, uuid_v7(1));
                assert_eq!(*limit, 1);
                operations.push(DataPlaneOperation::Receive);
                let receive_count = operations
                    .iter()
                    .filter(|operation| **operation == DataPlaneOperation::Receive)
                    .count();
                match receive_count {
                    1 => DataPlaneResponse::Received {
                        id: *id,
                        messages: vec![DataPlaneMessage {
                            message_id: uuid_v7(3),
                            sequence: 7,
                            timestamp_ns: 99,
                            content_type: "text/plain".to_string(),
                            payload: b"Paris".to_vec(),
                        }],
                    },
                    2 => DataPlaneResponse::Failed {
                        id: *id,
                        failure: DataPlaneFailure::Unavailable,
                    },
                    _ => DataPlaneResponse::Received {
                        id: *id,
                        messages: Vec::new(),
                    },
                }
            }
            DataPlaneRequest::ListModelTools { id } => {
                operations.push(DataPlaneOperation::ListModelTools);
                DataPlaneResponse::ModelToolsListed {
                    id: *id,
                    tools: test_model_tools(),
                }
            }
            DataPlaneRequest::CompleteWithTools { id, call } => {
                assert_eq!(call.completion.model, "test-model");
                assert_eq!(call.completion.temperature, 0.0);
                assert_eq!(call.completion.max_tokens, Some(128));
                assert!(call
                    .completion
                    .system
                    .as_deref()
                    .is_some_and(|system| system.contains("brief forecast")));
                assert_eq!(call.completion.messages.len(), 1);
                assert_eq!(call.completion.messages[0].role, PromptRole::User);
                assert_eq!(call.completion.messages[0].text, "Paris");
                assert_eq!(
                    call.completion.metadata.get("agent").map(String::as_str),
                    Some("weather-reporter")
                );
                assert_eq!(call.tools, test_model_tools());
                operations.push(DataPlaneOperation::CompleteWithTools);
                let output = if call.results.is_empty() {
                    ToolCompletionOutput::ToolCall(ModelToolCall {
                        call_id: "call-1".to_string(),
                        name: "smart_home.list_entities".to_string(),
                        arguments: serde_json::json!({}),
                    })
                } else {
                    assert_eq!(call.results.len(), 1);
                    assert_eq!(call.results[0].output, serde_json::json!({"entities": []}));
                    ToolCompletionOutput::FinalText("Sunny and mild.".to_string())
                };
                DataPlaneResponse::ToolCompleted {
                    id: *id,
                    result: Box::new(tool_turn_result(output)),
                }
            }
            DataPlaneRequest::ExecuteTool { id, call } => {
                operations.push(DataPlaneOperation::ExecuteTool);
                DataPlaneResponse::ToolExecuted {
                    id: *id,
                    result: Box::new(ModelToolResult {
                        call: (**call).clone(),
                        output: serde_json::json!({"entities": []}),
                        is_error: false,
                    }),
                }
            }
            DataPlaneRequest::Complete { id, .. } => DataPlaneResponse::Failed {
                id: *id,
                failure: DataPlaneFailure::Unavailable,
            },
            DataPlaneRequest::Publish {
                id,
                channel_id,
                content_type,
                payload,
            } => {
                assert_eq!(*channel_id, uuid_v7(2));
                assert_eq!(content_type, LEVEL_ONE_RESPONSE_CONTENT_TYPE);
                assert_eq!(payload, b"Sunny and mild.");
                operations.push(DataPlaneOperation::Publish);
                DataPlaneResponse::Published {
                    id: *id,
                    message_id: uuid_v7(4),
                    sequence: 8,
                    timestamp_ns: 100,
                }
            }
            DataPlaneRequest::Acknowledge {
                id,
                channel_id,
                message_id,
            } => {
                assert_eq!(*channel_id, uuid_v7(1));
                assert_eq!(*message_id, uuid_v7(3));
                operations.push(DataPlaneOperation::Acknowledge);
                DataPlaneResponse::Acknowledged {
                    id: *id,
                    sequence: 7,
                }
            }
        }
    }
}

fn await_phase(
    supervisor: &mut ProcessHostSupervisor,
    registration: &HostRegistration,
    expected: SupervisorPhase,
) {
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        match supervisor.inspect(registration) {
            Ok(SupervisorObservation::Instance(instance)) if instance.phase() == expected => return,
            Ok(_) => {}
            Err(error) => panic!("unexpected supervisor error: {error}"),
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {expected:?}"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn real_level_one_host_runs_one_authenticated_turn_and_terminates_cleanly() {
    let (package, keyring) = TestPackage::new();
    let registration = package.registration();
    let dispatcher = Arc::new(ScriptedDataPlane::default());
    let program = HostProgram::new(
        env!("CARGO_BIN_EXE_chief-of-staff-host"),
        std::iter::empty::<&str>(),
    )
    .unwrap();
    let config =
        ProcessSupervisorConfig::new(program, Duration::from_secs(3), Duration::from_secs(3))
            .unwrap();
    let mut supervisor = ProcessHostSupervisor::new(
        config,
        Arc::new(keyring),
        Arc::new(TestLaunchBindings),
        Arc::new(generate_identity_keypair()),
        Arc::new(TestClock::default()),
        Box::new(TestSessions(0)),
    )
    .with_data_plane_dispatcher(dispatcher.clone());

    supervisor.start(&registration).unwrap();
    let expected = [
        DataPlaneOperation::Receive,
        DataPlaneOperation::ListModelTools,
        DataPlaneOperation::CompleteWithTools,
        DataPlaneOperation::ExecuteTool,
        DataPlaneOperation::CompleteWithTools,
        DataPlaneOperation::Publish,
        DataPlaneOperation::Acknowledge,
        DataPlaneOperation::Receive,
        DataPlaneOperation::Receive,
    ];
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        supervisor.inspect(&registration).unwrap();
        if dispatcher.operations() == expected {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for Level 1 turn"
        );
        thread::sleep(Duration::from_millis(10));
    }

    supervisor.stop(registration.host_name()).unwrap();
    await_phase(
        &mut supervisor,
        &registration,
        SupervisorPhase::Exited { exit_code: Some(0) },
    );
    assert_eq!(dispatcher.operations(), expected);
}

#[cfg(unix)]
#[test]
fn production_composition_runs_encrypted_weather_turn_through_ollama() {
    let home = TestHome::new();
    let ollama = ScriptedOllama::spawn();
    let (package, keyring) = TestPackage::new();
    let registration = package.registration();
    let worker = AgentId::new(b"weather-reporter".to_vec()).unwrap();
    let request_source = AgentId::new(b"weather-request-source".to_vec()).unwrap();
    let report_sink = AgentId::new(b"weather-report-sink".to_vec()).unwrap();
    let pipeline_id = PipelineId::new(uuid_v7(9)).unwrap();
    let input_channel = ChannelId(uuid_v7(1));
    let output_channel = ChannelId(uuid_v7(2));

    let input_signing = OriginatorSigningKey::from_seed([0x11; 32]);
    let input_key = ChannelMasterKey::from_bytes([0x12; 32]);
    let worker_receiver = ReceiverKeyPair::from_private_key([0x13; 32]).unwrap();
    let worker_signing = OriginatorSigningKey::from_seed([0x21; 32]);
    let sink_receiver = ReceiverKeyPair::from_private_key([0x23; 32]).unwrap();
    home.write_secret("weather-receiver.bin", &[0x13; 32]);
    home.write_secret("weather-signing.bin", &[0x21; 32]);
    home.write_secret("weather-channel.bin", &[0x22; 32]);

    let backend: Arc<dyn StorageBackend> = Arc::new(FsStorageBackend::new(home.0.join("state")));
    backend.initialize().unwrap();
    ServiceRegistry::new(backend.as_ref())
        .register(&HostEntry::registered(
            registration.clone(),
            DesiredState::Running,
        ))
        .unwrap();
    let definitions = ChannelDefinitionStore::new(backend.as_ref());
    definitions
        .create(
            &ChannelDefinition::new(
                input_channel,
                OriginatorIdentity {
                    agent_id: request_source.clone(),
                    public_key: input_signing.public_key(),
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
                output_channel,
                OriginatorIdentity {
                    agent_id: worker.clone(),
                    public_key: worker_signing.public_key(),
                },
                vec![ReceiverIdentity {
                    agent_id: report_sink.clone(),
                    public_key: sink_receiver.public_key(),
                }],
                2,
                KeyEpoch(0),
            )
            .unwrap(),
        )
        .unwrap();
    let binding = HostPipelineBinding::new(
        pipeline_id,
        registration.clone(),
        worker.clone(),
        LaunchBindings::new(
            vec![
                ChannelBinding::new(
                    "weather-requests",
                    ChannelBindingAccess::Read,
                    input_channel.0,
                )
                .unwrap(),
                ChannelBinding::new(
                    "weather-reports",
                    ChannelBindingAccess::Write,
                    output_channel.0,
                )
                .unwrap(),
            ],
            Some(LevelOneModelBinding::new("weather-fixture", 0.0, 128).unwrap()),
        )
        .unwrap(),
    );
    PipelineBindingStore::new(backend.as_ref())
        .wire(&binding)
        .unwrap();

    let input_metadata = FixedMetadata {
        message_id: uuid_v7(4),
        timestamp_ns: 10,
    };
    let input = DurableOriginator::open(
        backend.as_ref(),
        input_channel,
        &request_source,
        &input_signing,
        &input_key,
        &input_metadata,
    )
    .unwrap();
    input.grant_receiver(&worker).unwrap();
    input.publish(b"Seattle", "text/plain").unwrap();

    let config = parse_config(&format!(
        r#"
[orchestrator]
bind = "127.0.0.1"
port = 7463
packages_dir = "~/agents"
state_dir = "~/state"
credential_path = "~/run/operator.credential"

[keyring]
trusted_keys = [{{ id = "dev", path = "~/keys/dev.pub", type = "developer" }}]

[hosts.defaults]
restart_policy = "on-failure"
health_check_interval = 20
executable = "~/bin/chief-of-staff-host"
bootstrap_timeout = 3000
graceful_stop_timeout = 3000

[vault]
storage_path = "~/vault"
default_lease_ttl = 30
container = true

[privilege]
tier_1_auto_approve_timeout = 5
biometric_timeout = 30
hardware_key_timeout = 60

[data_plane]
channel_keys = [
  {{ pipeline_id = "00000000-0000-7000-8000-000000000009", agent_id = "weather-reporter", channel_id = "00000000-0000-7000-8000-000000000001", access = "read", private_key_path = "~/keys/weather-receiver.bin" }},
  {{ pipeline_id = "00000000-0000-7000-8000-000000000009", agent_id = "weather-reporter", channel_id = "00000000-0000-7000-8000-000000000002", access = "write", signing_seed_path = "~/keys/weather-signing.bin", channel_key_path = "~/keys/weather-channel.bin" }},
]
ollama_models = [
  {{ model = "weather-fixture", endpoint = "{}", timeout = 3000 }},
]
"#,
        ollama.endpoint()
    ))
    .unwrap();
    let clock: Arc<dyn MonotonicClock> = Arc::new(TestClock::default());
    let dispatcher =
        compose_host_data_plane(&config, &home.0, Arc::clone(&backend), Arc::clone(&clock))
            .unwrap();
    let launch_bindings = Arc::new(DurableHostLaunchBindings::new(Arc::clone(&backend)));
    let program = HostProgram::new(
        env!("CARGO_BIN_EXE_chief-of-staff-host"),
        std::iter::empty::<&str>(),
    )
    .unwrap();
    let process_config =
        ProcessSupervisorConfig::new(program, Duration::from_secs(3), Duration::from_secs(3))
            .unwrap();
    let mut supervisor = ProcessHostSupervisor::new(
        process_config,
        Arc::new(keyring),
        launch_bindings,
        Arc::new(generate_identity_keypair()),
        clock,
        Box::new(TestSessions(0)),
    )
    .with_data_plane_dispatcher(dispatcher);
    let mut sink =
        DurableReceiver::open(backend.as_ref(), output_channel, report_sink, sink_receiver)
            .unwrap();

    supervisor.start(&registration).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let report = loop {
        let observation = supervisor.inspect(&registration).unwrap();
        let mut reports = sink.receive(1).unwrap();
        if let Some(report) = reports.pop() {
            break report;
        }
        assert!(
            !matches!(
                observation,
                SupervisorObservation::Instance(ref instance)
                    if matches!(instance.phase(), SupervisorPhase::Exited { .. })
            ),
            "production Level 1 host exited before publishing"
        );
        assert!(
            Instant::now() < deadline,
            "timed out waiting for production Level 1 weather report"
        );
        thread::sleep(Duration::from_millis(10));
    };
    assert_eq!(report.payload, b"Bring an umbrella.");
    assert_eq!(report.content_type, LEVEL_ONE_RESPONSE_CONTENT_TYPE);

    while ChannelStore::new(backend.as_ref(), input_channel)
        .receiver_cursor(worker.as_bytes())
        .unwrap()
        != Sequence(1)
    {
        supervisor.inspect(&registration).unwrap();
        assert!(
            Instant::now() < deadline,
            "timed out waiting for production input acknowledgement"
        );
        thread::sleep(Duration::from_millis(10));
    }

    supervisor.stop(registration.host_name()).unwrap();
    await_phase(
        &mut supervisor,
        &registration,
        SupervisorPhase::Exited { exit_code: Some(0) },
    );
    let request = ollama.finish();
    assert!(request.contains("\"model\":\"weather-fixture\""));
    assert!(request.contains("Weather Reporter"));
    assert!(request.contains("\"content\":\"Seattle\""));
    assert!(
        request.contains("smart_home.list_devices"),
        "production catalog was absent from Ollama request: {request}"
    );
}
