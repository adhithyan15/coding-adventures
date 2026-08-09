use chief_of_staff_host_control_protocol::{
    ChannelBinding, ChannelBindingAccess, CompletionFinishReason, CompletionProvider,
    CompletionResult, CompletionUsage, DataPlaneFailure, DataPlaneMessage, DataPlaneOperation,
    DataPlaneRequest, DataPlaneResponse, LaunchBindings, LevelOneModelBinding, PromptRole,
};
use chief_of_staff_host_data_plane::HostDataPlaneDispatcher;
use chief_of_staff_host_runtime::{
    verify_agent_package, AgentPackageRuntime, PackageKeyType, PackageKeyring, TrustedPackageKey,
};
use chief_of_staff_process_supervisor::{
    HostLaunchBindingProvider, HostProgram, LaunchBindingProviderError, MonotonicClock,
    ProcessHostSupervisor, ProcessSupervisorConfig, ProcessSupervisorError, SessionIdSource,
};
use chief_of_staff_secure_host_channel::SessionId;
use chief_of_staff_service_reconciler::{HostSupervisor, SupervisorObservation, SupervisorPhase};
use chief_of_staff_service_registry::{HostName, HostRegistration, PackagePath, RestartPolicy};
use chief_of_staff_skill_package::build_signed_skill_package;
use chief_of_staff_skill_runtime::LEVEL_ONE_RESPONSE_CONTENT_TYPE;
use chief_of_staff_tool_api::PrivilegeTier;
use coding_adventures_ed25519::generate_keypair;
use coding_adventures_x3dh::generate_identity_keypair;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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
            DataPlaneRequest::Complete { id, call } => {
                assert_eq!(call.model, "test-model");
                assert_eq!(call.temperature, 0.0);
                assert_eq!(call.max_tokens, Some(128));
                assert!(call
                    .system
                    .as_deref()
                    .is_some_and(|system| system.contains("brief forecast")));
                assert_eq!(call.messages.len(), 1);
                assert_eq!(call.messages[0].role, PromptRole::User);
                assert_eq!(call.messages[0].text, "Paris");
                assert_eq!(
                    call.metadata.get("agent").map(String::as_str),
                    Some("weather-reporter")
                );
                operations.push(DataPlaneOperation::Complete);
                DataPlaneResponse::Completed {
                    id: *id,
                    result: Box::new(CompletionResult {
                        text: "Sunny and mild.".to_string(),
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
                    }),
                }
            }
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
        DataPlaneOperation::Complete,
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
