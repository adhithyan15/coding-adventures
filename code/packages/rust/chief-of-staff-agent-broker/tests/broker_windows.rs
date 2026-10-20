//! The broker binary on Windows (D18S P2.8b-1), as the supervisor will run
//! it: confined by the applier in its own `chief-of-staff-broker.`
//! container, its key files opened owner-only and passed as handles on the
//! applier's handle list, named in `CHIEF_OF_STAFF_HANDLES`, and frames on
//! stdin and stdout.

#![cfg(windows)]

use std::fs;
use std::io::Read;
use std::os::windows::io::OwnedHandle;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use capability_os_sandbox::{plan_from_json, OsFamily, SandboxPlan};
use chief_of_staff_agent_broker::protocol::{
    decode_from_broker, encode_to_broker, read_frame, write_frame, FromBroker, KeyKind, KeySlot,
    ToBroker, KEY_HANDLES_VARIABLE,
};
use chief_of_staff_channel_crypto::{ChannelId, OriginatorSigningKey};
use chief_of_staff_channel_endpoints::AgentId;
use chief_of_staff_daemon_secret_file::{open_owner_only_secret, write_test_secret};
use chief_of_staff_host_control_protocol::{ChannelBinding, ChannelBindingAccess, LaunchBindings};
use chief_of_staff_pipeline_bindings::{HostPipelineBinding, PipelineId};
use chief_of_staff_service_registry::{HostName, HostRegistration, PackagePath, RestartPolicy};
use chief_of_staff_spawn_isolation::VerifiedExecutable;
use chief_of_staff_windows_sandbox::{ConfinedChild, SpawnRequest, WindowsConfinement};

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

/// A distinct agent per test: each broker gets its own container.
fn agent_name() -> String {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    format!(
        "broker-windows-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}

fn bootstrap(agent: &str) -> ToBroker {
    ToBroker::Bootstrap {
        binding: HostPipelineBinding::new(
            PipelineId::new(uuid_v7(1)).unwrap(),
            HostRegistration::new(
                HostName::new("weather").unwrap(),
                PackagePath::new("agents/weather.agent").unwrap(),
                [7; 32],
                RestartPolicy::OnFailure,
            ),
            AgentId::new(agent.as_bytes().to_vec()).unwrap(),
            LaunchBindings::new(
                vec![
                    ChannelBinding::new("reports", ChannelBindingAccess::Write, uuid_v7(12))
                        .unwrap(),
                ],
                None,
            )
            .unwrap(),
        ),
        slots: vec![
            KeySlot {
                channel_id: ChannelId(uuid_v7(12)),
                kind: KeyKind::OriginatorSigningSeed,
            },
            KeySlot {
                channel_id: ChannelId(uuid_v7(12)),
                kind: KeyKind::ChannelMasterKey,
            },
        ],
    }
}

fn plan() -> SandboxPlan {
    plan_from_json(
        r#"{"version":1,"package":"rust/chief-of-staff-agent-broker","capabilities":[],"justification":"The broker needs no capability: its keys are inherited."}"#,
        OsFamily::Windows,
    )
    .unwrap()
}

/// A directory of this test's own under the user's temp directory: the
/// broker's copy (the build directory may be writable by others, which
/// `VerifiedExecutable` refuses) and the key files.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "agent-broker-windows-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        let canonical = fs::canonicalize(path).unwrap();
        let text = canonical.to_str().unwrap().to_string();
        Self(PathBuf::from(text.strip_prefix(r"\\?\").unwrap_or(&text)))
    }

    fn key(&self, name: &str, bytes: &[u8]) -> OwnedHandle {
        let path = self.0.join(name);
        write_test_secret(&path, bytes);
        open_owner_only_secret(&path).unwrap().into()
    }

    /// The broker, at its canonical path in this directory, verified.
    fn broker(&self) -> VerifiedExecutable {
        let copy = self.0.join("chief-of-staff-agent-broker.exe");
        fs::copy(BROKER, &copy).unwrap();
        let digest = coding_adventures_sha256::sha256(&fs::read(&copy).unwrap());
        VerifiedExecutable::open(&copy, digest).unwrap()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn launch(scratch: &Scratch, agent: &str, keys: Vec<OwnedHandle>) -> ConfinedChild {
    WindowsConfinement::prepare_verified_broker(&plan(), &scratch.broker(), agent.as_bytes())
        .unwrap_or_else(|error| panic!("prepare: {error}"))
        .spawn(SpawnRequest {
            inherited: keys,
            ..SpawnRequest::default()
        })
        .unwrap_or_else(|error| panic!("spawn: {error}"))
}

fn receive(child: &mut ConfinedChild) -> Option<FromBroker> {
    let body = read_frame(child.stdout.as_mut().unwrap()).ok()?;
    Some(decode_from_broker(&body).unwrap())
}

fn exit_code(mut child: ConfinedChild) -> u32 {
    let mut rest = Vec::new();
    let _ = child.stdout.take().unwrap().read_to_end(&mut rest);
    child.wait().unwrap().code().unwrap_or(-1) as u32
}

#[test]
fn the_variable_is_the_applier_s() {
    assert_eq!(
        KEY_HANDLES_VARIABLE,
        chief_of_staff_windows_sandbox::HANDLES_VARIABLE
    );
}

#[test]
fn a_confined_broker_loads_its_keys_from_handles_and_comes_up() {
    let scratch = Scratch::new();
    let agent = agent_name();
    let keys = vec![scratch.key("seed", &SEED), scratch.key("cmk", &CMK)];
    let mut child = launch(&scratch, &agent, keys);
    write_frame(
        child.stdin.as_mut().unwrap(),
        &encode_to_broker(&bootstrap(&agent)).unwrap(),
    )
    .unwrap();
    let Some(FromBroker::Ready { public_keys }) = receive(&mut child) else {
        panic!("no Ready (exit {:#x})", exit_code(child))
    };
    assert_eq!(public_keys.len(), 1);
    assert_eq!(
        public_keys[0].public_key,
        OriginatorSigningKey::from_seed(SEED).public_key()
    );
    write_frame(
        child.stdin.as_mut().unwrap(),
        &encode_to_broker(&ToBroker::Terminate).unwrap(),
    )
    .unwrap();
    assert_eq!(exit_code(child), 0);
}

#[test]
fn a_handle_short_of_the_slots_stops_the_broker_before_any_key_is_read() {
    let scratch = Scratch::new();
    let agent = agent_name();
    let mut child = launch(&scratch, &agent, vec![scratch.key("seed", &SEED)]);
    write_frame(
        child.stdin.as_mut().unwrap(),
        &encode_to_broker(&bootstrap(&agent)).unwrap(),
    )
    .unwrap();
    assert!(receive(&mut child).is_none(), "came up one key short");
    assert_eq!(exit_code(child), 66);
}

#[test]
fn a_key_of_the_wrong_length_is_refused() {
    let scratch = Scratch::new();
    let agent = agent_name();
    let keys = vec![scratch.key("seed", &SEED[..31]), scratch.key("cmk", &CMK)];
    let mut child = launch(&scratch, &agent, keys);
    write_frame(
        child.stdin.as_mut().unwrap(),
        &encode_to_broker(&bootstrap(&agent)).unwrap(),
    )
    .unwrap();
    assert!(receive(&mut child).is_none(), "came up with a short key");
    assert_eq!(exit_code(child), 67);
}
