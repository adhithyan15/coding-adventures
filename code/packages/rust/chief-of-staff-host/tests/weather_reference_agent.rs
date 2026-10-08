//! # P1.5: the weather reference agent, end to end (#142)
//!
//! The acceptance test for the Chief of Staff backlog (#13980): an agent
//! written as **one SKILL.md file and no code** answers a real request by
//! reaching the network, and every boundary between the request and the
//! answer is the production one.
//!
//! ```text
//!  "Seattle" ──▶ weather-requests channel (sealed, signed)
//!                    │
//!                    ▼
//!   chief-of-staff-host  ◀── spawned by ProcessHostSupervisor from a
//!   (real child process)      signed SKILL package, verified by digest
//!                    │ CompleteWithTools: catalog offered to the model
//!                    ▼
//!   daemon data plane ── compose_host_data_plane_with_fetcher ──┐
//!                    │ model asks for net.fetch                 │
//!                    ▼                                          │
//!   AgentModelTools: manifest from the verified package, so     │
//!   net.fetch is offered and api.weather.gov is the allowlist   │
//!                    │                                          │
//!                    ▼                                          │
//!   NetFetch: authorize ─ resolve ─ public check ─ request ─ decode
//!                    │          (only DNS and the socket are fakes) ◀┘
//!                    ▼
//!   tool result ──▶ model ──▶ final text ──▶ weather-reports channel
//! ```
//!
//! Two things are scripted, because CI has neither:
//!
//! - **The model.** A local HTTP server speaking Ollama's `/api/chat`. It
//!   asks for `net.fetch` on its first turn and answers on its second, which
//!   is exactly what a real model told the skill's instructions would do.
//! - **The internet.** A `NetFetch` over a resolver that answers with a
//!   public address and a transport that returns a recorded
//!   `api.weather.gov` response. Everything around them is real: the URL is
//!   authorized against the signed manifest, the address passes the public
//!   check, the request is built by the production encoder, and the response
//!   is bounded, filtered and decoded by the production decoder.
//!
//! #142 asks for "weather in Tokyo". The National Weather Service covers only
//! the United States, so the reference agent answers for Seattle.

#![cfg(unix)]

use chief_of_staff_channel_crypto::{
    ChannelId, ChannelMasterKey, KeyEpoch, OriginatorSigningKey, ReceiverKeyPair,
};
use chief_of_staff_channel_endpoints::{
    AgentId, ChannelDefinition, ChannelDefinitionStore, DurableOriginator, DurableReceiver,
    MessageId, MessageMetadata, MessageMetadataError, MessageMetadataSource, Originator,
    OriginatorIdentity, Receiver, ReceiverIdentity,
};
use chief_of_staff_daemon::compose_host_data_plane_with_fetcher;
use chief_of_staff_daemon_config::parse_config;
use chief_of_staff_host_control_protocol::{
    ChannelBinding, ChannelBindingAccess, LaunchBindings, LevelOneModelBinding,
};
use chief_of_staff_host_runtime::{
    verify_agent_package, PackageKeyType, PackageKeyring, TrustedPackageKey,
};
use chief_of_staff_net_fetch::{read_limited, FetchError, Limits, NetFetch, Resolver, Transport};
use chief_of_staff_pipeline_bindings::{HostPipelineBinding, PipelineBindingStore, PipelineId};
use chief_of_staff_process_supervisor::{
    DurableHostLaunchBindings, HostProgram, MonotonicClock, ProcessHostSupervisor,
    ProcessSupervisorConfig, ProcessSupervisorError, SessionIdSource,
};
use chief_of_staff_secure_host_channel::SessionId;
use chief_of_staff_service_reconciler::{HostSupervisor, SupervisorObservation, SupervisorPhase};
use chief_of_staff_service_registry::{
    DesiredState, HostEntry, HostName, HostRegistration, PackagePath, RestartPolicy,
    ServiceRegistry,
};
use chief_of_staff_skill_package::build_signed_skill_package;
use chief_of_staff_skill_runtime::LEVEL_ONE_RESPONSE_CONTENT_TYPE;
use chief_of_staff_tool_api::PrivilegeTier;
use coding_adventures_ed25519::generate_keypair;
use coding_adventures_storage_fs::FsStorageBackend;
use coding_adventures_x3dh::generate_identity_keypair;
use coding_adventures_zeroize::Zeroizing;
use std::fs;
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use storage_core::StorageBackend;

/// The reference agent, exactly as checked in.
const SKILL: &str = include_str!("../reference-agents/weather/SKILL.md");
const KEY_ID: &str = "weather-reference-dev";
const KEY_SEED: [u8; 32] = [91; 32];
const FORECAST_URL: &str = "https://api.weather.gov/gridpoints/SEW/124,67/forecast";
const FINAL_TEXT: &str = "Tonight in Seattle: Rain Likely, 46 F.";

/// A trimmed recording of an `api.weather.gov` gridpoint forecast. The
/// `set-cookie` and `x-request-id` headers are there to show they never
/// reach the model (V-R6).
fn recorded_forecast() -> Vec<u8> {
    let body = r#"{"properties":{"periods":[{"number":1,"name":"Tonight","temperature":46,"temperatureUnit":"F","shortForecast":"Rain Likely"}]}}"#;
    format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/geo+json\r\nset-cookie: tracker=1\r\nx-request-id: abc\r\ncontent-length: {}\r\n\r\n{body}",
        body.len()
    )
    .into_bytes()
}

fn uuid_v7(last: u8) -> [u8; 16] {
    let mut bytes = [0; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes[15] = last;
    bytes
}

// ── The two fakes ────────────────────────────────────────────────────────────

/// Answers every lookup with one public address (TEST-NET would be refused
/// by the public-address check, which is the point of that check).
struct PublicResolver;

impl Resolver for PublicResolver {
    fn resolve(&self, _host: &str, port: u16) -> std::io::Result<Vec<SocketAddr>> {
        Ok(vec![SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(23, 32, 0, 1)),
            port,
        )])
    }
}

/// What the daemon put on the wire, and to whom.
#[derive(Clone, Default)]
struct Wire(Arc<Mutex<Vec<(String, SocketAddr, String)>>>);

impl Transport for Wire {
    fn exchange(
        &self,
        server_name: &str,
        address: SocketAddr,
        request: &[u8],
        limits: &Limits,
    ) -> Result<Zeroizing<Vec<u8>>, FetchError> {
        self.0.lock().unwrap().push((
            server_name.to_string(),
            address,
            String::from_utf8_lossy(request).into_owned(),
        ));
        read_limited(&mut recorded_forecast().as_slice(), limits.max_wire_bytes)
    }
}

/// The model: a two-turn script over Ollama's `/api/chat`.
struct ScriptedModel {
    endpoint: String,
    requests: Arc<Mutex<Vec<String>>>,
    join: Option<thread::JoinHandle<()>>,
}

impl ScriptedModel {
    fn spawn() -> Self {
        let tool_call = serde_json::json!({
            "kind": "tool_call",
            "call_id": "call-forecast",
            "name": "net.fetch",
            "arguments": {
                "url": FORECAST_URL,
                "method": "GET",
                "headers": {"accept": "application/geo+json"},
            },
        });
        let final_text = serde_json::json!({"kind": "final", "text": FINAL_TEXT});
        let responses: Vec<String> = [tool_call, final_text]
            .iter()
            .map(|content| {
                serde_json::json!({
                    "model": "weather-fixture",
                    "message": {"role": "assistant", "content": content.to_string()},
                    "done": true,
                    "done_reason": "stop",
                    "prompt_eval_count": 12,
                    "eval_count": 4,
                })
                .to_string()
            })
            .collect();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&requests);
        let join = thread::spawn(move || {
            for body in responses {
                let (mut stream, _) = listener.accept().unwrap();
                captured.lock().unwrap().push(read_http_body(&mut stream));
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).unwrap();
            }
        });
        Self {
            endpoint,
            requests,
            join: Some(join),
        }
    }

    fn finish(mut self) -> Vec<String> {
        self.join.take().unwrap().join().unwrap();
        std::mem::take(&mut *self.requests.lock().unwrap())
    }
}

fn read_http_body(stream: &mut std::net::TcpStream) -> String {
    let mut request = Vec::new();
    let mut buffer = [0; 4096];
    let (body_start, length) = loop {
        let count = stream.read(&mut buffer).unwrap();
        assert!(count > 0, "model request ended before its headers");
        request.extend_from_slice(&buffer[..count]);
        let Some(end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") else {
            continue;
        };
        let headers = std::str::from_utf8(&request[..end]).unwrap();
        assert!(headers.starts_with("POST /api/chat HTTP/1.1\r\n"));
        let length = headers
            .lines()
            .find_map(|line| line.strip_prefix("Content-Length:"))
            .unwrap()
            .trim()
            .parse::<usize>()
            .unwrap();
        break (end + 4, length);
    };
    while request.len() - body_start < length {
        let count = stream.read(&mut buffer).unwrap();
        assert!(count > 0, "model request ended before its body");
        request.extend_from_slice(&buffer[..count]);
    }
    String::from_utf8(request[body_start..body_start + length].to_vec()).unwrap()
}

// ── Plumbing the supervisor needs ────────────────────────────────────────────

struct TestHome(PathBuf);

impl TestHome {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "chief-weather-reference-{}-{}",
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

impl Drop for TestHome {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct FixedMetadata;

impl MessageMetadataSource for FixedMetadata {
    fn next_metadata(&self) -> Result<MessageMetadata, MessageMetadataError> {
        Ok(MessageMetadata {
            message_id: MessageId::from_uuid_v7(uuid_v7(4))
                .map_err(|_| MessageMetadataError::new("invalid fixture message ID"))?,
            timestamp_ns: 10,
        })
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

/// The reference SKILL.md, packaged and signed with a developer key. A
/// developer key's ceiling is Tier 1, which is exactly what `net.fetch`
/// requires: a fetch-only agent needs nothing stronger.
fn signed_reference_package(home: &TestHome) -> (HostRegistration, PackageKeyring) {
    fs::create_dir_all(home.0.join("agents")).unwrap();
    let path = home.0.join("agents/weather-reporter");
    let (public_key, secret_key) = generate_keypair(&KEY_SEED);
    build_signed_skill_package(&path, SKILL, KEY_ID, &secret_key).unwrap();
    home.write_secret("dev.pub", &public_key);
    let mut keyring = PackageKeyring::new();
    keyring
        .trust(
            TrustedPackageKey::new(
                KEY_ID,
                PackageKeyType::Developer,
                public_key,
                PrivilegeTier::Tier1,
            )
            .unwrap(),
        )
        .unwrap();
    let digest = verify_agent_package(&path, &keyring).unwrap().digest();
    let registration = HostRegistration::new(
        HostName::new("weather-reference").unwrap(),
        PackagePath::new(path.to_str().unwrap()).unwrap(),
        digest,
        RestartPolicy::Always,
    );
    (registration, keyring)
}

#[test]
fn the_weather_reference_agent_answers_from_a_real_net_fetch() {
    let home = TestHome::new();
    let model = ScriptedModel::spawn();
    let wire = Wire::default();
    let (registration, keyring) = signed_reference_package(&home);

    let worker = AgentId::new(b"weather-reporter".to_vec()).unwrap();
    let request_source = AgentId::new(b"weather-request-source".to_vec()).unwrap();
    let report_sink = AgentId::new(b"weather-report-sink".to_vec()).unwrap();
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

    // Durable state: the registration, both channels, and the pipeline.
    let backend: Arc<dyn StorageBackend> = Arc::new(FsStorageBackend::new(home.0.join("state")));
    backend.initialize().unwrap();
    ServiceRegistry::new(backend.as_ref())
        .register(&HostEntry::registered(
            registration.clone(),
            DesiredState::Running,
        ))
        .unwrap();
    let definitions = ChannelDefinitionStore::new(backend.as_ref());
    for (channel, originator, signing, receiver, receiver_key, version) in [
        (
            input_channel,
            &request_source,
            &input_signing,
            &worker,
            worker_receiver.public_key(),
            1,
        ),
        (
            output_channel,
            &worker,
            &worker_signing,
            &report_sink,
            sink_receiver.public_key(),
            2,
        ),
    ] {
        definitions
            .create(
                &ChannelDefinition::new(
                    channel,
                    OriginatorIdentity {
                        agent_id: originator.clone(),
                        public_key: signing.public_key(),
                    },
                    vec![ReceiverIdentity {
                        agent_id: receiver.clone(),
                        public_key: receiver_key,
                    }],
                    version,
                    KeyEpoch(0),
                )
                .unwrap(),
            )
            .unwrap();
    }
    let binding = HostPipelineBinding::new(
        PipelineId::new(uuid_v7(9)).unwrap(),
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

    // The request.
    let input = DurableOriginator::open(
        backend.as_ref(),
        input_channel,
        &request_source,
        &input_signing,
        &input_key,
        &FixedMetadata,
    )
    .unwrap();
    input.grant_receiver(&worker).unwrap();
    input.publish(b"Seattle", "text/plain").unwrap();

    // The production data plane, with only the network edge supplied.
    let config = parse_config(&format!(
        r#"
[orchestrator]
bind = "127.0.0.1"
port = 7463
packages_dir = "~/agents"
state_dir = "~/state"
credential_path = "~/run/operator.credential"

[keyring]
trusted_keys = [{{ id = "{KEY_ID}", path = "~/keys/dev.pub", type = "developer" }}]

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
        model.endpoint
    ))
    .unwrap();
    let clock: Arc<dyn MonotonicClock> = Arc::new(TestClock::default());
    let dispatcher = compose_host_data_plane_with_fetcher(
        &config,
        &home.0,
        Arc::clone(&backend),
        Arc::clone(&clock),
        Arc::new(NetFetch::new(PublicResolver, wire.clone())),
    )
    .unwrap();

    // The supervisor spawns the real host binary from the signed package.
    let program = HostProgram::new(
        env!("CARGO_BIN_EXE_chief-of-staff-host"),
        std::iter::empty::<&str>(),
    )
    .unwrap();
    let mut supervisor = ProcessHostSupervisor::new(
        ProcessSupervisorConfig::new(program, Duration::from_secs(3), Duration::from_secs(3))
            .unwrap(),
        Arc::new(keyring),
        Arc::new(DurableHostLaunchBindings::new(Arc::clone(&backend))),
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
        if let Some(report) = sink.receive(1).unwrap().pop() {
            break report;
        }
        assert!(
            !matches!(
                observation,
                SupervisorObservation::Instance(ref instance)
                    if matches!(instance.phase(), SupervisorPhase::Exited { .. })
            ),
            "the weather host exited before publishing"
        );
        assert!(
            Instant::now() < deadline,
            "timed out waiting for the weather report"
        );
        thread::sleep(Duration::from_millis(10));
    };
    supervisor.stop(registration.host_name()).unwrap();
    drop(supervisor);

    // The answer reached the report channel.
    assert_eq!(report.payload, FINAL_TEXT.as_bytes());
    assert_eq!(report.content_type, LEVEL_ONE_RESPONSE_CONTENT_TYPE);

    // The daemon made exactly one request: to the manifest's host, at the
    // address it checked, built by the production encoder.
    let sent = wire.0.lock().unwrap().clone();
    assert_eq!(sent.len(), 1, "{sent:?}");
    let (server_name, address, request) = &sent[0];
    assert_eq!(server_name, "api.weather.gov");
    assert_eq!(*address, "23.32.0.1:443".parse::<SocketAddr>().unwrap());
    assert!(
        request.starts_with(
            "GET /gridpoints/SEW/124,67/forecast HTTP/1.1\r\nhost: api.weather.gov\r\n"
        ),
        "{request}"
    );
    assert!(request.contains("accept: application/geo+json\r\n"));
    assert!(request.contains("connection: close\r\n"));
    assert!(!request.contains("authorization"));

    // The model was offered net.fetch, because the signed manifest grants
    // it, and was handed the forecast, but not headers off the allowlist.
    let turns = model.finish();
    assert_eq!(turns.len(), 2);
    assert!(turns[0].contains("Weather Reporter"));
    assert!(turns[0].contains("\"content\":\"Seattle\""));
    // The catalog travels as JSON inside the system prompt, so its quotes
    // are escaped. Matched by name, not merely mentioned: net.fetch's own
    // description names vault.request_lease, and a fetch-only manifest is not
    // offered it.
    assert!(
        turns[0].contains(r#"\"name\":\"net.fetch\""#),
        "{}",
        turns[0]
    );
    assert!(!turns[0].contains(r#"\"name\":\"vault.request_lease\""#));
    assert!(turns[1].contains("call-forecast"));
    assert!(turns[1].contains("Rain Likely"), "{}", turns[1]);
    assert!(!turns[1].contains("tracker=1"));
    assert!(!turns[1].contains("x-request-id"));
}
