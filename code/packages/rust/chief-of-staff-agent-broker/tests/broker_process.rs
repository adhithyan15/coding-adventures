//! The broker binary, as the supervisor will run it: keys on inherited
//! descriptors 3..3+n, frames on stdin and stdout, and nothing else.
//!
//! The test plays the supervisor. It opens the key files, parks the
//! descriptors high, and in the child places them at 3..3+n and closes
//! everything above. Then it speaks the protocol, answering callbacks with
//! the daemon's real callback server.

#![cfg(unix)]

use std::collections::VecDeque;
use std::fs;
use std::io::{BufReader, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use chief_of_staff_agent_broker::protocol::{
    decode_from_broker, encode_to_broker, read_frame, write_frame, FromBroker, KeyKind, KeySlot,
    ToBroker,
};
use chief_of_staff_broker_callbacks::{BindingResolver, CallbackServer, InFlight};
use chief_of_staff_channel_crypto::{ChannelId, KeyEpoch, OriginatorSigningKey, ReceiverKeyPair};
use chief_of_staff_channel_endpoints::{
    AgentId, ChannelDefinition, ChannelDefinitionStore, MessageId, MessageMetadata,
    MessageMetadataError, MessageMetadataSource, OriginatorIdentity, ReceiverIdentity,
};
use chief_of_staff_daemon_secret_file::open_owner_only_secret;
use chief_of_staff_host_control_protocol::{
    ChannelBinding, ChannelBindingAccess, DataPlaneRequest, DataPlaneResponse, LaunchBindings,
    RequestId,
};
use chief_of_staff_pipeline_bindings::{HostPipelineBinding, PipelineId};
use chief_of_staff_service_registry::{HostName, HostRegistration, PackagePath, RestartPolicy};
use storage_core::InMemoryStorageBackend;

const SEED: [u8; 32] = [0x31; 32];
const CMK: [u8; 32] = [0xa5; 32];
const RECEIVER: [u8; 32] = [0x42; 32];
/// Where the parent parks key descriptors before the child moves them.
const PARK: i32 = 200;

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

fn definition() -> ChannelDefinition {
    ChannelDefinition::new(
        channel(),
        OriginatorIdentity {
            agent_id: agent("weather"),
            public_key: OriginatorSigningKey::from_seed(SEED).public_key(),
        },
        vec![ReceiverIdentity {
            agent_id: agent("sink"),
            public_key: ReceiverKeyPair::from_private_key(RECEIVER)
                .unwrap()
                .public_key(),
        }],
        1_725_000_000_000_000_000,
        KeyEpoch(0),
    )
    .unwrap()
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

fn slots() -> Vec<KeySlot> {
    vec![
        KeySlot {
            channel_id: channel(),
            kind: KeyKind::OriginatorSigningSeed,
        },
        KeySlot {
            channel_id: channel(),
            kind: KeyKind::ChannelMasterKey,
        },
    ]
}

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "agent-broker-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(fs::canonicalize(path).unwrap())
    }

    fn key(&self, name: &str, bytes: &[u8], mode: u32) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, bytes).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Duplicate `fd` onto `PARK + index`, close-on-exec, in the parent.
fn park(fd: i32, index: i32) -> OwnedFd {
    // SAFETY: F_DUPFD_CLOEXEC returns a new descriptor at or above the
    // target, owned by nobody else.
    let parked = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, PARK + index) };
    assert!(parked >= 0);
    // SAFETY: a fresh descriptor this function just created.
    unsafe { OwnedFd::from_raw_fd(parked) }
}

struct Broker {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    _parked: Vec<OwnedFd>,
}

/// Spawn the broker with `keys` on 3..3+n, and `extra` more descriptors
/// left open after them.
fn spawn(keys: &[&Path], extra: usize) -> Broker {
    spawn_with_stray(keys, extra, None)
}

/// As [`spawn`], plus one stray descriptor left open at `stray`.
fn spawn_with_stray(keys: &[&Path], extra: usize, stray: Option<i32>) -> Broker {
    let files: Vec<fs::File> = keys
        .iter()
        .map(|path| open_owner_only_secret(path).unwrap_or_else(|_| fs::File::open(path).unwrap()))
        .collect();
    let mut parked: Vec<OwnedFd> = files
        .iter()
        .enumerate()
        .map(|(index, file)| park(file.as_raw_fd(), index as i32))
        .collect();
    for index in 0..extra {
        let devnull = fs::File::open("/dev/null").unwrap();
        parked.push(park(devnull.as_raw_fd(), (keys.len() + index) as i32));
    }
    // Concurrent tests park too, so a parked descriptor is wherever
    // F_DUPFD_CLOEXEC put it: pass the child the actual numbers.
    let sources: Vec<i32> = parked.iter().map(|fd| fd.as_raw_fd()).collect();
    let count = sources.len() as i32;
    let mut command = Command::new(env!("CARGO_BIN_EXE_chief-of-staff-agent-broker"));
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .env_clear();
    // SAFETY: only async-signal-safe calls (dup2, close) between fork and
    // exec, on descriptors that exist in the child.
    unsafe {
        command.pre_exec(move || {
            // Every source is at PARK or above, and every target below it,
            // so no dup2 overwrites a source still to be moved.
            for (index, source) in sources.iter().enumerate() {
                if libc::dup2(*source, 3 + index as i32) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
            }
            for fd in 3 + count..1024 {
                libc::close(fd);
            }
            if let Some(stray) = stray {
                if libc::dup2(0, stray) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
            }
            Ok(())
        });
    }
    let mut child = command.spawn().unwrap();
    let stdin = child.stdin.take().unwrap();
    let stdout = BufReader::new(child.stdout.take().unwrap());
    Broker {
        child,
        stdin,
        stdout,
        _parked: parked,
    }
}

impl Broker {
    fn send(&mut self, frame: &ToBroker) {
        write_frame(&mut self.stdin, &encode_to_broker(frame).unwrap()).unwrap();
    }

    fn send_raw(&mut self, body: &[u8]) {
        write_frame(&mut self.stdin, body).unwrap();
    }

    fn receive(&mut self) -> Option<FromBroker> {
        let body = read_frame(&mut self.stdout).ok()?;
        Some(decode_from_broker(&body).unwrap())
    }

    /// Wait for the broker to exit, and return its status code.
    fn exit_code(mut self) -> i32 {
        drop(self.stdin);
        let mut rest = Vec::new();
        let _ = self.stdout.read_to_end(&mut rest);
        self.child.wait().unwrap().code().unwrap_or(-1)
    }
}

struct Resolver;

impl BindingResolver for Resolver {
    fn current_binding(&self) -> Option<HostPipelineBinding> {
        Some(binding())
    }
}

struct Metadata(Mutex<VecDeque<MessageMetadata>>);

impl MessageMetadataSource for Metadata {
    fn next_metadata(&self) -> Result<MessageMetadata, MessageMetadataError> {
        self.0
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| MessageMetadataError::new("exhausted"))
    }
}

fn good_keys(scratch: &Scratch) -> Vec<PathBuf> {
    vec![
        scratch.key("seed", &SEED, 0o600),
        scratch.key("cmk", &CMK, 0o600),
    ]
}

fn bootstrap() -> ToBroker {
    ToBroker::Bootstrap {
        binding: binding(),
        slots: slots(),
    }
}

#[test]
fn the_broker_process_publishes_through_framed_callbacks() {
    let scratch = Scratch::new();
    let keys = good_keys(&scratch);
    let mut broker = spawn(&[&keys[0], &keys[1]], 0);
    broker.send(&bootstrap());
    let Some(FromBroker::Ready { public_keys }) = broker.receive() else {
        panic!("no Ready")
    };
    assert_eq!(public_keys.len(), 1);
    assert_eq!(
        public_keys[0].public_key,
        OriginatorSigningKey::from_seed(SEED).public_key()
    );

    let backend = InMemoryStorageBackend::new();
    ChannelDefinitionStore::new(&backend)
        .create(&definition())
        .unwrap();
    let metadata = Metadata(Mutex::new(
        vec![MessageMetadata {
            message_id: MessageId::from_uuid_v7(uuid_v7(101)).unwrap(),
            timestamp_ns: 7,
        }]
        .into(),
    ));
    let server = CallbackServer::new(&backend, &metadata, &Resolver);
    let request = DataPlaneRequest::Publish {
        id: RequestId::new(1).unwrap(),
        channel_id: uuid_v7(12),
        content_type: "text/plain".to_owned(),
        payload: b"sunny".to_vec(),
    };
    let mut in_flight = InFlight::for_request(&request).unwrap();
    broker.send(&ToBroker::Request(request));
    let response = loop {
        match broker.receive().expect("the broker exited") {
            FromBroker::Callback {
                callback_id,
                request_id,
                call,
            } => {
                let outcome = server.serve(&mut in_flight, request_id, call).unwrap();
                broker.send(&ToBroker::CallbackResult {
                    callback_id,
                    outcome,
                });
            }
            FromBroker::Response(response) => break response,
            other => panic!("unexpected {other:?}"),
        }
    };
    assert_eq!(
        response,
        DataPlaneResponse::Published {
            id: RequestId::new(1).unwrap(),
            message_id: uuid_v7(101),
            sequence: 0,
            timestamp_ns: 7,
        }
    );
    broker.send(&ToBroker::Terminate);
    assert_eq!(broker.exit_code(), 0);
}

/// Bootstrap with these key files and extra descriptors; return whether the
/// broker came up, and its exit code once stdin closes.
fn comes_up(keys: &[&Path], extra: usize) -> (bool, i32) {
    let mut broker = spawn(keys, extra);
    broker.send(&bootstrap());
    let ready = matches!(broker.receive(), Some(FromBroker::Ready { .. }));
    (ready, broker.exit_code())
}

#[test]
fn a_descriptor_beyond_the_slots_stops_the_broker_before_any_key_is_read() {
    let scratch = Scratch::new();
    let keys = good_keys(&scratch);
    let (ready, code) = comes_up(&[&keys[0], &keys[1]], 1);
    assert!(!ready);
    assert_ne!(code, 0);
}

#[test]
fn a_stray_descriptor_anywhere_above_the_slots_stops_the_broker() {
    let scratch = Scratch::new();
    let keys = good_keys(&scratch);
    let mut broker = spawn_with_stray(&[&keys[0], &keys[1]], 0, Some(900));
    broker.send(&bootstrap());
    assert!(
        broker.receive().is_none(),
        "came up with a stray descriptor"
    );
    assert_ne!(broker.exit_code(), 0);
}

#[test]
fn a_missing_slot_descriptor_stops_the_broker() {
    let scratch = Scratch::new();
    let keys = good_keys(&scratch);
    let (ready, code) = comes_up(&[&keys[0]], 0);
    assert!(!ready);
    assert_ne!(code, 0);
}

#[test]
fn a_key_file_that_is_not_owner_only_or_not_32_bytes_or_zero_is_refused() {
    let scratch = Scratch::new();
    let cmk = scratch.key("cmk", &CMK, 0o600);
    for (name, bytes, mode) in [
        ("open-to-group", SEED.to_vec(), 0o640),
        ("short", SEED[..31].to_vec(), 0o600),
        ("long", [SEED.as_slice(), &[1]].concat(), 0o600),
        ("zero", vec![0; 32], 0o600),
    ] {
        let seed = scratch.key(name, &bytes, mode);
        let (ready, code) = comes_up(&[&seed, &cmk], 0);
        assert!(!ready, "{name}");
        assert_ne!(code, 0, "{name}");
    }
}

#[test]
fn a_broken_frame_or_a_reply_to_the_wrong_callback_stops_the_broker() {
    let scratch = Scratch::new();
    let keys = good_keys(&scratch);

    // A first frame that is not a Bootstrap.
    let mut broker = spawn(&[&keys[0], &keys[1]], 0);
    broker.send_raw(b"D18K\x01\x7f");
    assert!(broker.receive().is_none());
    assert_ne!(broker.exit_code(), 0);

    // A callback result for a callback it did not make.
    let mut broker = spawn(&[&keys[0], &keys[1]], 0);
    broker.send(&bootstrap());
    assert!(matches!(broker.receive(), Some(FromBroker::Ready { .. })));
    broker.send(&ToBroker::Request(DataPlaneRequest::Publish {
        id: RequestId::new(1).unwrap(),
        channel_id: uuid_v7(12),
        content_type: "text/plain".to_owned(),
        payload: b"x".to_vec(),
    }));
    let Some(FromBroker::Callback { callback_id, .. }) = broker.receive() else {
        panic!("no callback")
    };
    broker.send(&ToBroker::CallbackResult {
        callback_id: callback_id + 1,
        outcome: chief_of_staff_agent_broker::protocol::CallbackOutcome::Refused {
            op: chief_of_staff_agent_broker::protocol::CallbackOp::LoadDefinition,
            refusal: chief_of_staff_agent_broker::protocol::Refusal::Unavailable,
        },
    });
    assert!(broker.receive().is_none());
    assert_ne!(broker.exit_code(), 0);

    // A second Bootstrap.
    let mut broker = spawn(&[&keys[0], &keys[1]], 0);
    broker.send(&bootstrap());
    assert!(matches!(broker.receive(), Some(FromBroker::Ready { .. })));
    broker.send(&bootstrap());
    assert!(broker.receive().is_none());
    assert_ne!(broker.exit_code(), 0);
}

#[test]
fn the_running_broker_cannot_dump_core() {
    // P2.6a: the broker hardens itself first, before reading any key.
    let scratch = Scratch::new();
    let keys = good_keys(&scratch);
    let mut broker = spawn(&[&keys[0], &keys[1]], 0);
    broker.send(&bootstrap());
    assert!(matches!(broker.receive(), Some(FromBroker::Ready { .. })));
    if cfg!(target_os = "linux") {
        let limits = fs::read_to_string(format!("/proc/{}/limits", broker.child.id())).unwrap();
        let core = limits
            .lines()
            .find(|line| line.starts_with("Max core file size"))
            .unwrap();
        let fields: Vec<&str> = core.split_whitespace().collect();
        assert_eq!(&fields[4..6], ["0", "0"], "{core}");
    }
    broker.send(&ToBroker::Terminate);
    let _ = broker.stdin.flush();
    assert_eq!(broker.exit_code(), 0);
}
