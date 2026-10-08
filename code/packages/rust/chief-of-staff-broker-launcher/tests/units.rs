//! The launcher's decisions that need no process: key files to slots,
//! abandoning reservations, the Ready check, and the pinned resolver.

use std::path::PathBuf;

use chief_of_staff_broker_callbacks::BindingResolver;
use chief_of_staff_broker_launcher::{
    abandon_pending_on_write_channels, check_ready, BrokerKeyFiles, KeyFileDeclaration,
    LaunchError, PinnedBindingResolver,
};
use chief_of_staff_broker_protocol::{KeyKind, KeySlot, PublicKey};
use chief_of_staff_channel_crypto::{
    ChannelId, KeyEpoch, OriginatorSigningKey, ReceiverKeyPair, Sequence,
};
use chief_of_staff_channel_endpoints::{
    AgentId, ChannelDefinition, ChannelDefinitionStore, OriginatorIdentity, ReceiverIdentity,
};
use chief_of_staff_channel_store::{AppendRequest, ChannelStore};
use chief_of_staff_host_control_protocol::{ChannelBinding, ChannelBindingAccess, LaunchBindings};
use chief_of_staff_pipeline_bindings::{HostPipelineBinding, PipelineId};
use chief_of_staff_service_registry::{HostName, HostRegistration, PackagePath, RestartPolicy};
use storage_core::InMemoryStorageBackend;

fn uuid_v7(tag: u8) -> [u8; 16] {
    let mut bytes = [0u8; 16];
    bytes[0] = tag;
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes
}

const READS: u8 = 11;
const WRITES: u8 = 12;

fn channel(tag: u8) -> ChannelId {
    ChannelId(uuid_v7(tag))
}

fn agent(name: &str) -> AgentId {
    AgentId::new(name.as_bytes().to_vec()).unwrap()
}

fn pipeline(tag: u8) -> PipelineId {
    PipelineId::new(uuid_v7(tag)).unwrap()
}

fn binding_for(pipeline_tag: u8, name: &str) -> HostPipelineBinding {
    HostPipelineBinding::new(
        pipeline(pipeline_tag),
        HostRegistration::new(
            HostName::new("weather").unwrap(),
            PackagePath::new("agents/weather.agent").unwrap(),
            [7; 32],
            RestartPolicy::OnFailure,
        ),
        agent(name),
        LaunchBindings::new(
            vec![
                ChannelBinding::new("reports", ChannelBindingAccess::Write, uuid_v7(WRITES))
                    .unwrap(),
                ChannelBinding::new("requests", ChannelBindingAccess::Read, uuid_v7(READS))
                    .unwrap(),
            ],
            None,
        )
        .unwrap(),
    )
}

fn binding() -> HostPipelineBinding {
    binding_for(1, "weather")
}

fn declare(channel_tag: u8, kind: KeyKind, path: &str) -> KeyFileDeclaration {
    KeyFileDeclaration {
        pipeline_id: pipeline(1),
        agent_id: agent("weather"),
        channel_id: channel(channel_tag),
        kind,
        path: PathBuf::from(path),
    }
}

fn all_keys() -> Vec<KeyFileDeclaration> {
    vec![
        declare(WRITES, KeyKind::ChannelMasterKey, "/keys/cmk"),
        declare(READS, KeyKind::ReceiverPrivateKey, "/keys/receiver"),
        declare(WRITES, KeyKind::OriginatorSigningSeed, "/keys/seed"),
    ]
}

// ---- key files and slots ---------------------------------------------------

#[test]
fn slots_follow_channel_order_with_each_direction_s_keys() {
    let keys = BrokerKeyFiles::new(all_keys()).unwrap();
    let slots = keys.slots_for(&binding()).unwrap();
    let summary: Vec<(u8, KeyKind, &str)> = slots
        .iter()
        .map(|(slot, path)| (slot.channel_id.0[0], slot.kind, path.to_str().unwrap()))
        .collect();
    assert_eq!(
        summary,
        [
            (READS, KeyKind::ReceiverPrivateKey, "/keys/receiver"),
            (WRITES, KeyKind::OriginatorSigningSeed, "/keys/seed"),
            (WRITES, KeyKind::ChannelMasterKey, "/keys/cmk"),
        ]
    );
}

#[test]
fn a_missing_key_is_refused_before_anything_is_spawned() {
    let mut keys = all_keys();
    keys.remove(0);
    assert_eq!(
        BrokerKeyFiles::new(keys)
            .unwrap()
            .slots_for(&binding())
            .unwrap_err(),
        LaunchError::MissingKey
    );
    // Keys declared for another agent, or another pipeline, do not count.
    let elsewhere: Vec<_> = all_keys()
        .into_iter()
        .map(|mut key| {
            key.pipeline_id = pipeline(2);
            key
        })
        .collect();
    assert_eq!(
        BrokerKeyFiles::new(elsewhere)
            .unwrap()
            .slots_for(&binding())
            .unwrap_err(),
        LaunchError::MissingKey
    );
}

#[test]
fn keys_for_unbound_channels_are_not_passed() {
    let mut keys = all_keys();
    keys.push(declare(99, KeyKind::ReceiverPrivateKey, "/keys/other"));
    let slots = BrokerKeyFiles::new(keys)
        .unwrap()
        .slots_for(&binding())
        .unwrap();
    assert_eq!(slots.len(), 3);
    assert!(slots.iter().all(|(slot, _)| slot.channel_id != channel(99)));
}

#[test]
fn a_key_declared_twice_or_for_both_directions_is_refused() {
    let mut twice = all_keys();
    twice.push(declare(
        WRITES,
        KeyKind::ChannelMasterKey,
        "/keys/cmk-again",
    ));
    assert_eq!(
        BrokerKeyFiles::new(twice).unwrap_err(),
        LaunchError::DuplicateKey
    );
    let mut both = all_keys();
    both.push(declare(
        WRITES,
        KeyKind::ReceiverPrivateKey,
        "/keys/receiver-too",
    ));
    assert_eq!(
        BrokerKeyFiles::new(both).unwrap_err(),
        LaunchError::DuplicateKey
    );
}

// ---- abandoning -------------------------------------------------------------

#[test]
fn only_write_channels_have_their_reservations_abandoned() {
    let backend = InMemoryStorageBackend::new();
    for tag in [READS, WRITES] {
        let store = ChannelStore::new(&backend, channel(tag));
        store.initialize().unwrap();
        store
            .reserve_append(
                AppendRequest {
                    message_id: uuid_v7(50 + tag),
                    timestamp_ns: 1,
                    originator_id: b"someone".to_vec(),
                    key_epoch: KeyEpoch(0),
                    content_type: "text/plain".to_owned(),
                },
                b"pending",
            )
            .unwrap();
    }
    assert_eq!(
        abandon_pending_on_write_channels(&backend, &binding()).unwrap(),
        vec![channel(WRITES)]
    );
    let write = ChannelStore::new(&backend, channel(WRITES))
        .state()
        .unwrap();
    assert_eq!(write.pending_header, None);
    assert_eq!(write.next_sequence, Sequence(1), "the sequence stays used");
    // The read channel belongs to another originator: untouched.
    assert!(ChannelStore::new(&backend, channel(READS))
        .state()
        .unwrap()
        .pending_header
        .is_some());
    // Nothing pending, or never written: nothing to do.
    assert!(
        abandon_pending_on_write_channels(&InMemoryStorageBackend::new(), &binding())
            .unwrap()
            .is_empty()
    );
}

// ---- the Ready check --------------------------------------------------------

const SEED: [u8; 32] = [0x31; 32];
const RECEIVER: [u8; 32] = [0x42; 32];

fn define(backend: &InMemoryStorageBackend) {
    let definitions = ChannelDefinitionStore::new(backend);
    definitions
        .create(
            &ChannelDefinition::new(
                channel(WRITES),
                OriginatorIdentity {
                    agent_id: agent("weather"),
                    public_key: OriginatorSigningKey::from_seed(SEED).public_key(),
                },
                vec![ReceiverIdentity {
                    agent_id: agent("sink"),
                    public_key: [9; 32],
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
                channel(READS),
                OriginatorIdentity {
                    agent_id: agent("source"),
                    public_key: [8; 32],
                },
                vec![ReceiverIdentity {
                    agent_id: agent("weather"),
                    public_key: ReceiverKeyPair::from_private_key(RECEIVER)
                        .unwrap()
                        .public_key(),
                }],
                1,
                KeyEpoch(0),
            )
            .unwrap(),
        )
        .unwrap();
}

fn slots() -> Vec<KeySlot> {
    BrokerKeyFiles::new(all_keys())
        .unwrap()
        .slots_for(&binding())
        .unwrap()
        .into_iter()
        .map(|(slot, _)| slot)
        .collect()
}

fn honest_ready() -> Vec<PublicKey> {
    vec![
        PublicKey {
            channel_id: channel(READS),
            kind: KeyKind::ReceiverPrivateKey,
            public_key: ReceiverKeyPair::from_private_key(RECEIVER)
                .unwrap()
                .public_key(),
        },
        PublicKey {
            channel_id: channel(WRITES),
            kind: KeyKind::OriginatorSigningSeed,
            public_key: OriginatorSigningKey::from_seed(SEED).public_key(),
        },
    ]
}

#[test]
fn ready_must_report_the_keys_the_definitions_name() {
    let backend = InMemoryStorageBackend::new();
    define(&backend);
    assert_eq!(
        check_ready(&backend, &binding(), &slots(), &honest_ready()),
        Ok(())
    );

    // A wrong key file: its public half is not the definition's.
    let mut wrong = honest_ready();
    wrong[1].public_key = OriginatorSigningKey::from_seed([0x32; 32]).public_key();
    assert_eq!(
        check_ready(&backend, &binding(), &slots(), &wrong),
        Err(LaunchError::WrongKeys)
    );
    // Out of slot order, missing one, or one extra.
    let mut reversed = honest_ready();
    reversed.reverse();
    let missing = honest_ready()[..1].to_vec();
    let mut extra = honest_ready();
    extra.push(honest_ready()[0]);
    for keys in [reversed, missing, extra] {
        assert_eq!(
            check_ready(&backend, &binding(), &slots(), &keys),
            Err(LaunchError::WrongKeys)
        );
    }
    // The definition names someone else as originator.
    let impostor = binding_for(1, "impostor");
    assert_eq!(
        check_ready(&backend, &impostor, &slots(), &honest_ready()),
        Err(LaunchError::WrongKeys)
    );
    // A destroyed channel.
    ChannelDefinitionStore::new(&backend)
        .destroy(channel(WRITES))
        .unwrap();
    assert_eq!(
        check_ready(&backend, &binding(), &slots(), &honest_ready()),
        Err(LaunchError::WrongKeys)
    );
}

// ---- the pinned resolver ----------------------------------------------------

#[test]
fn the_resolver_answers_only_for_the_launched_identity() {
    let launched = binding();
    let same = PinnedBindingResolver::new(|| Some(binding()), &launched);
    assert_eq!(same.current_binding(), Some(binding()));

    let rewired = PinnedBindingResolver::new(|| Some(binding_for(1, "other-agent")), &launched);
    assert_eq!(rewired.current_binding(), None);
    let moved = PinnedBindingResolver::new(|| Some(binding_for(2, "weather")), &launched);
    assert_eq!(moved.current_binding(), None);
    let unwired = PinnedBindingResolver::new(|| None, &launched);
    assert_eq!(unwired.current_binding(), None);
}
