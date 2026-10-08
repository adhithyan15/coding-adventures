//! The real broker binary, launched the way the supervisor will launch it:
//! verified by digest, exec'd by descriptor, keys on 3..3+n, Ready checked
//! against the definitions, then relayed to (D18S P2.6d-2a).

#![cfg(target_os = "linux")]

use std::fs;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chief_of_staff_broker_launcher::{
    launch, start_relay, BrokerKeyFiles, HostGone, KeyFileDeclaration, LaunchError,
    PinnedBindingResolver, RelayConfig, ResponseSink, VerifiedExecutable,
};
use chief_of_staff_broker_protocol::KeyKind;
use chief_of_staff_channel_crypto::{ChannelId, KeyEpoch, OriginatorSigningKey, ReceiverKeyPair};
use chief_of_staff_channel_endpoints::{
    AgentId, ChannelDefinition, ChannelDefinitionStore, MessageId, MessageMetadata,
    MessageMetadataError, MessageMetadataSource, OriginatorIdentity, ReceiverIdentity,
};
use chief_of_staff_host_control_protocol::{
    ChannelBinding, ChannelBindingAccess, DataPlaneRequest, DataPlaneResponse, LaunchBindings,
    RequestId,
};
use chief_of_staff_pipeline_bindings::{HostPipelineBinding, PipelineId};
use chief_of_staff_service_registry::{HostName, HostRegistration, PackagePath, RestartPolicy};
use storage_core::{InMemoryStorageBackend, StorageBackend};

const BROKER: &str = env!("CARGO_BIN_EXE_chief-of-staff-agent-broker");
const SEED: [u8; 32] = [0x31; 32];
const CMK: [u8; 32] = [0xa5; 32];

fn uuid_v7(tag: u8) -> [u8; 16] {
    let mut bytes = [0u8; 16];
    bytes[0] = tag;
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes
}

fn channel() -> ChannelId {
    ChannelId(uuid_v7(12))
}

fn agent(name: &str) -> AgentId {
    AgentId::new(name.as_bytes().to_vec()).unwrap()
}

fn binding() -> HostPipelineBinding {
    HostPipelineBinding::new(
        PipelineId::new(uuid_v7(1)).unwrap(),
        HostRegistration::new(
            HostName::new("weather").unwrap(),
            PackagePath::new("agents/weather.agent").unwrap(),
            [7; 32],
            RestartPolicy::OnFailure,
        ),
        agent("weather"),
        LaunchBindings::new(
            vec![ChannelBinding::new("reports", ChannelBindingAccess::Write, uuid_v7(12)).unwrap()],
            None,
        )
        .unwrap(),
    )
}

fn broker() -> VerifiedExecutable {
    let digest = coding_adventures_sha256::sha256(&fs::read(BROKER).unwrap());
    VerifiedExecutable::open(Path::new(BROKER), digest).unwrap()
}

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        // Owner-only, as every directory holding keys must be (P2.6d-3),
        // and under Cargo's own scratch directory, not a shared /tmp.
        let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
            "broker-launched-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&path);
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        Self(fs::canonicalize(path).unwrap())
    }

    fn key(&self, name: &str, bytes: &[u8; 32]) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, bytes).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        path
    }

    fn keys(&self, seed: &[u8; 32]) -> BrokerKeyFiles {
        let declare = |kind, path| KeyFileDeclaration {
            pipeline_id: PipelineId::new(uuid_v7(1)).unwrap(),
            agent_id: agent("weather"),
            channel_id: channel(),
            kind,
            path,
        };
        BrokerKeyFiles::new(vec![
            declare(KeyKind::OriginatorSigningSeed, self.key("seed", seed)),
            declare(KeyKind::ChannelMasterKey, self.key("cmk", &CMK)),
        ])
        .unwrap()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn backend() -> Arc<dyn StorageBackend> {
    let backend: Arc<dyn StorageBackend> = Arc::new(InMemoryStorageBackend::new());
    ChannelDefinitionStore::new(&*backend)
        .create(
            &ChannelDefinition::new(
                channel(),
                OriginatorIdentity {
                    agent_id: agent("weather"),
                    public_key: OriginatorSigningKey::from_seed(SEED).public_key(),
                },
                vec![ReceiverIdentity {
                    agent_id: agent("sink"),
                    public_key: ReceiverKeyPair::from_private_key([0x42; 32])
                        .unwrap()
                        .public_key(),
                }],
                1,
                KeyEpoch(0),
            )
            .unwrap(),
        )
        .unwrap();
    backend
}

struct Metadata(Mutex<Vec<MessageMetadata>>);

impl MessageMetadataSource for Metadata {
    fn next_metadata(&self) -> Result<MessageMetadata, MessageMetadataError> {
        self.0
            .lock()
            .unwrap()
            .pop()
            .ok_or_else(|| MessageMetadataError::new("exhausted"))
    }
}

struct Sink(SyncSender<DataPlaneResponse>);

impl ResponseSink for Sink {
    fn deliver(&mut self, response: DataPlaneResponse) -> Result<(), HostGone> {
        self.0.send(response).map_err(|_| HostGone)
    }
}

const READY: Duration = Duration::from_secs(10);

#[test]
fn a_launched_broker_publishes_through_the_relay() {
    let scratch = Scratch::new();
    let backend = backend();
    let launched = launch(
        &broker(),
        &binding(),
        &scratch.keys(&SEED),
        &*backend,
        READY,
    )
    .unwrap();
    let (delivered_tx, delivered) = mpsc::sync_channel(1);
    let metadata = Arc::new(Metadata(Mutex::new(vec![MessageMetadata {
        message_id: MessageId::from_uuid_v7(uuid_v7(101)).unwrap(),
        timestamp_ns: 7,
    }])));
    let (mut child, mut relay) = start_relay(
        launched,
        backend,
        metadata,
        Box::new(PinnedBindingResolver::new(|| Some(binding()), &binding())),
        Box::new(Sink(delivered_tx)),
        RelayConfig::default(),
    )
    .unwrap();

    relay
        .relay(DataPlaneRequest::Publish {
            id: RequestId::new(1).unwrap(),
            channel_id: uuid_v7(12),
            content_type: "text/plain".to_owned(),
            payload: b"sunny".to_vec(),
        })
        .unwrap();
    assert_eq!(
        delivered.recv_timeout(Duration::from_secs(10)).unwrap(),
        DataPlaneResponse::Published {
            id: RequestId::new(1).unwrap(),
            message_id: uuid_v7(101),
            sequence: 0,
            timestamp_ns: 7,
        }
    );
    assert_eq!(relay.ended(), None);

    // The supervisor's teardown: kill the session, reap, then stop.
    chief_of_staff_spawn_isolation::kill_session(&child).unwrap();
    child.wait().unwrap();
    assert!(relay.stop());
}

#[test]
fn a_key_file_that_is_not_the_definition_s_key_is_caught_at_launch() {
    let scratch = Scratch::new();
    let backend = backend();
    let error = launch(
        &broker(),
        &binding(),
        &scratch.keys(&[0x32; 32]),
        &*backend,
        READY,
    )
    .err()
    .expect("launched with the wrong key");
    assert_eq!(error, LaunchError::WrongKeys);
}

#[test]
fn a_key_file_open_to_others_is_refused_before_spawn() {
    let scratch = Scratch::new();
    let keys = scratch.keys(&SEED);
    fs::set_permissions(scratch.0.join("cmk"), fs::Permissions::from_mode(0o640)).unwrap();
    let error = launch(&broker(), &binding(), &keys, &*backend(), READY)
        .err()
        .expect("launched with a group-readable key");
    assert_eq!(error, LaunchError::KeyFile);
}

#[test]
fn a_key_directory_open_to_others_is_refused_before_anything_is_opened() {
    let scratch = Scratch::new();
    let keys = scratch.keys(&SEED);
    fs::set_permissions(&scratch.0, fs::Permissions::from_mode(0o750)).unwrap();
    let error = launch(&broker(), &binding(), &keys, &*backend(), READY)
        .err()
        .expect("launched with a group-searchable key directory");
    assert_eq!(error, LaunchError::SecretDirectory);
}

#[test]
fn another_secret_directory_open_to_others_is_refused_too() {
    let scratch = Scratch::new();
    let vault = scratch.0.join("vault");
    fs::create_dir(&vault).unwrap();
    fs::set_permissions(&vault, fs::Permissions::from_mode(0o755)).unwrap();
    let keys = scratch
        .keys(&SEED)
        .with_secret_directories(vec![vault.clone()]);
    let error = launch(&broker(), &binding(), &keys, &*backend(), READY)
        .err()
        .expect("launched with a world-readable vault directory");
    assert_eq!(error, LaunchError::SecretDirectory);
    fs::set_permissions(&vault, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(launch(&broker(), &binding(), &keys, &*backend(), READY).is_ok());
}

#[test]
fn the_launched_broker_is_confined() {
    let scratch = Scratch::new();
    let backend = backend();
    let launched = launch(
        &broker(),
        &binding(),
        &scratch.keys(&SEED),
        &*backend,
        READY,
    )
    .unwrap();
    let (mut child, mut relay) = start_relay(
        launched,
        backend,
        Arc::new(Metadata(Mutex::new(Vec::new()))),
        Box::new(PinnedBindingResolver::new(|| Some(binding()), &binding())),
        Box::new(Sink(mpsc::sync_channel(1).0)),
        RelayConfig::default(),
    )
    .unwrap();
    // Read from outside: seccomp in filter mode with no_new_privs, and not
    // dumpable, which also keeps /proc/<pid>/mem and ptrace away.
    let status = fs::read_to_string(format!("/proc/{}/status", child.id())).unwrap();
    let field = |name: &str| {
        status
            .lines()
            .find_map(|line| line.strip_prefix(name))
            .map(str::trim)
            .unwrap_or_else(|| panic!("{name} missing"))
            .to_owned()
    };
    assert_eq!(field("NoNewPrivs:"), "1");
    assert_eq!(field("Seccomp:"), "2");
    let mem = fs::metadata(format!("/proc/{}/mem", child.id())).unwrap();
    // Not dumpable: the kernel gives /proc/<pid>/mem to root, not to us
    // (when the tests themselves run as root, there is nothing to show).
    if unsafe { libc::geteuid() } != 0 {
        use std::os::unix::fs::MetadataExt;
        assert_eq!(mem.uid(), 0);
    }
    chief_of_staff_spawn_isolation::kill_session(&child).unwrap();
    child.wait().unwrap();
    assert!(relay.stop());
}
