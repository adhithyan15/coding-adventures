//! The broker engine in process, against the daemon's real callback server.
//! Two brokers, one per agent, talk only through storage: a publisher and a
//! receiver, each holding only its own keys.

use std::collections::VecDeque;
use std::sync::Mutex;

use chief_of_staff_agent_broker::protocol::{
    Callback, CallbackOp, CallbackOutcome, CallbackReply, KeyKind, KeySlot,
};
use chief_of_staff_agent_broker::{
    BrokerError, ChannelBroker, ChannelCallbacks, KeyError, LoadedKeys,
};
use chief_of_staff_broker_callbacks::{BindingResolver, CallbackServer, InFlight};
use chief_of_staff_channel_crypto::{
    ChannelId, ChannelMasterKey, KeyEpoch, OriginatorSigningKey, ReceiverKeyPair,
};
use chief_of_staff_channel_endpoints::{
    AgentId, ChannelDefinition, ChannelDefinitionStore, MessageId, MessageMetadata,
    MessageMetadataError, MessageMetadataSource, OriginatorIdentity, ReceiverIdentity,
};
use chief_of_staff_channel_store::{AppendRequest, ChannelStore};
use chief_of_staff_host_control_protocol::{
    ChannelBinding, ChannelBindingAccess, DataPlaneFailure, DataPlaneRequest, DataPlaneResponse,
    LaunchBindings, RequestId,
};
use chief_of_staff_pipeline_bindings::{HostPipelineBinding, PipelineId};
use chief_of_staff_service_registry::{HostName, HostRegistration, PackagePath, RestartPolicy};
use coding_adventures_zeroize::Zeroizing;
use storage_core::InMemoryStorageBackend;

// ---- fixtures -------------------------------------------------------------

fn uuid_v7(tag: u8) -> [u8; 16] {
    let mut bytes = [0u8; 16];
    bytes[0] = tag;
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes
}

const REPORTS: u8 = 12;
const SEED: [u8; 32] = [0x31; 32];
const CMK: [u8; 32] = [0xa5; 32];
const RECEIVER: [u8; 32] = [0x42; 32];

fn channel() -> ChannelId {
    ChannelId(uuid_v7(REPORTS))
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

fn binding(name: &str, access: ChannelBindingAccess) -> HostPipelineBinding {
    HostPipelineBinding::new(
        PipelineId::new(uuid_v7(1)).unwrap(),
        HostRegistration::new(
            HostName::new(name).unwrap(),
            PackagePath::new(format!("agents/{name}.agent")).unwrap(),
            [7; 32],
            RestartPolicy::OnFailure,
        ),
        agent(name),
        LaunchBindings::new(
            vec![ChannelBinding::new("reports", access, uuid_v7(REPORTS)).unwrap()],
            None,
        )
        .unwrap(),
    )
}

fn slot(kind: KeyKind) -> KeySlot {
    KeySlot {
        channel_id: channel(),
        kind,
    }
}

fn load(
    binding: &HostPipelineBinding,
    slots: &[KeySlot],
    keys: &[[u8; 32]],
) -> Result<LoadedKeys, KeyError> {
    LoadedKeys::load(binding, slots, |index| {
        Ok(Zeroizing::new(keys[index].to_vec()))
    })
}

fn publisher() -> ChannelBroker {
    let binding = binding("weather", ChannelBindingAccess::Write);
    let keys = load(
        &binding,
        &[
            slot(KeyKind::OriginatorSigningSeed),
            slot(KeyKind::ChannelMasterKey),
        ],
        &[SEED, CMK],
    )
    .unwrap();
    ChannelBroker::new(binding, keys)
}

fn receiver_with(key: [u8; 32]) -> ChannelBroker {
    let binding = binding("sink", ChannelBindingAccess::Read);
    let keys = load(&binding, &[slot(KeyKind::ReceiverPrivateKey)], &[key]).unwrap();
    ChannelBroker::new(binding, keys)
}

struct Resolver(HostPipelineBinding);

impl BindingResolver for Resolver {
    fn current_binding(&self) -> Option<HostPipelineBinding> {
        Some(self.0.clone())
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

fn metadata() -> Metadata {
    Metadata(Mutex::new(
        (1..=30)
            .map(|n| MessageMetadata {
                message_id: MessageId::from_uuid_v7(uuid_v7(100 + n)).unwrap(),
                timestamp_ns: 1_000 + u64::from(n),
            })
            .collect(),
    ))
}

/// The daemon side, for one broker: its server and the request in flight.
/// A violation would end the broker; in these tests it is a failure.
struct Daemon<'a> {
    server: CallbackServer<'a>,
    in_flight: Option<InFlight>,
    calls: Vec<CallbackOp>,
}

impl<'a> Daemon<'a> {
    fn new(
        backend: &'a InMemoryStorageBackend,
        metadata: &'a Metadata,
        resolver: &'a Resolver,
    ) -> Self {
        Self {
            server: CallbackServer::new(backend, metadata, resolver),
            in_flight: None,
            calls: Vec::new(),
        }
    }

    /// Relay one request, as the supervisor will.
    fn relay(
        &mut self,
        broker: &mut ChannelBroker,
        request: DataPlaneRequest,
    ) -> DataPlaneResponse {
        self.in_flight = InFlight::for_request(&request);
        self.calls.clear();
        let response = broker.serve(&request, self).unwrap();
        assert_eq!(response.id(), request.id());
        response
    }
}

impl ChannelCallbacks for Daemon<'_> {
    fn call(
        &mut self,
        request_id: RequestId,
        call: Callback,
    ) -> Result<CallbackOutcome, BrokerError> {
        self.calls.push(call.op());
        let in_flight = self
            .in_flight
            .as_mut()
            .expect("a callback with no request in flight");
        Ok(self
            .server
            .serve(in_flight, request_id, call)
            .expect("an honest broker made a violating callback"))
    }
}

struct World {
    backend: InMemoryStorageBackend,
    metadata: Metadata,
    weather: Resolver,
    sink: Resolver,
}

impl World {
    fn new() -> Self {
        let backend = InMemoryStorageBackend::new();
        ChannelDefinitionStore::new(&backend)
            .create(&definition())
            .unwrap();
        Self {
            backend,
            metadata: metadata(),
            weather: Resolver(binding("weather", ChannelBindingAccess::Write)),
            sink: Resolver(binding("sink", ChannelBindingAccess::Read)),
        }
    }
}

fn id(n: u64) -> RequestId {
    RequestId::new(n).unwrap()
}

fn publish(n: u64, payload: &[u8]) -> DataPlaneRequest {
    DataPlaneRequest::Publish {
        id: id(n),
        channel_id: uuid_v7(REPORTS),
        content_type: "text/plain".to_owned(),
        payload: payload.to_vec(),
    }
}

fn receive(n: u64) -> DataPlaneRequest {
    DataPlaneRequest::Receive {
        id: id(n),
        channel_id: uuid_v7(REPORTS),
        limit: 10,
    }
}

fn acknowledge(n: u64, message_id: [u8; 16]) -> DataPlaneRequest {
    DataPlaneRequest::Acknowledge {
        id: id(n),
        channel_id: uuid_v7(REPORTS),
        message_id,
    }
}

fn failure(response: DataPlaneResponse) -> DataPlaneFailure {
    match response {
        DataPlaneResponse::Failed { failure, .. } => failure,
        other => panic!("expected a failure, got {other:?}"),
    }
}

// ---- the round trip -------------------------------------------------------

#[test]
fn two_brokers_each_with_only_their_own_keys_carry_a_message_end_to_end() {
    let world = World::new();
    let mut weather = publisher();
    let mut sink = receiver_with(RECEIVER);
    let mut weather_daemon = Daemon::new(&world.backend, &world.metadata, &world.weather);
    let mut sink_daemon = Daemon::new(&world.backend, &world.metadata, &world.sink);

    let DataPlaneResponse::Published {
        message_id,
        sequence,
        timestamp_ns,
        ..
    } = weather_daemon.relay(&mut weather, publish(1, b"sunny"))
    else {
        panic!("not published")
    };
    assert_eq!((sequence, timestamp_ns), (0, 1_001));
    // The first publish granted the receiver; later ones skip that.
    assert_eq!(
        weather_daemon.calls,
        [
            CallbackOp::LoadDefinition,
            CallbackOp::LoadMissingGrants,
            CallbackOp::SaveGrants,
            CallbackOp::ReserveAppend,
            CallbackOp::CommitAppend
        ]
    );
    weather_daemon.relay(&mut weather, publish(2, b"rain later"));
    assert_eq!(
        weather_daemon.calls,
        [
            CallbackOp::LoadDefinition,
            CallbackOp::ReserveAppend,
            CallbackOp::CommitAppend
        ]
    );

    let DataPlaneResponse::Received { messages, .. } = sink_daemon.relay(&mut sink, receive(3))
    else {
        panic!("not received")
    };
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].payload, b"sunny");
    assert_eq!(messages[0].message_id, message_id);
    assert_eq!(messages[1].payload, b"rain later");

    // Acknowledge the first: the cursor moves past it, and only the second
    // is delivered again.
    assert_eq!(
        sink_daemon.relay(&mut sink, acknowledge(4, message_id)),
        DataPlaneResponse::Acknowledged {
            id: id(4),
            sequence: 1
        }
    );
    let DataPlaneResponse::Received { messages, .. } = sink_daemon.relay(&mut sink, receive(5))
    else {
        panic!("not received")
    };
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].payload, b"rain later");

    // As the old dispatcher did: an acknowledged message's receipt is gone,
    // so acknowledging it again is refused, and so is a message never
    // delivered.
    assert_eq!(
        failure(sink_daemon.relay(&mut sink, acknowledge(6, message_id))),
        DataPlaneFailure::Unauthorized
    );
    assert_eq!(
        failure(sink_daemon.relay(&mut sink, acknowledge(7, uuid_v7(77)))),
        DataPlaneFailure::Unauthorized
    );
}

// ---- refusals that need no callback ---------------------------------------

#[test]
fn a_channel_without_a_key_here_is_refused_before_any_callback() {
    let world = World::new();
    let mut weather = publisher();
    let mut sink = receiver_with(RECEIVER);
    let mut daemon = Daemon::new(&world.backend, &world.metadata, &world.weather);
    // The publisher holds no receiver key, the receiver no signing key.
    assert_eq!(
        failure(daemon.relay(&mut weather, receive(1))),
        DataPlaneFailure::Unauthorized
    );
    assert!(daemon.calls.is_empty());
    let mut daemon = Daemon::new(&world.backend, &world.metadata, &world.sink);
    assert_eq!(
        failure(daemon.relay(&mut sink, publish(2, b"x"))),
        DataPlaneFailure::Unauthorized
    );
    assert!(daemon.calls.is_empty());
    // And a channel it has no key for at all.
    let other = DataPlaneRequest::Receive {
        id: id(3),
        channel_id: uuid_v7(99),
        limit: 1,
    };
    assert_eq!(
        failure(sink.serve(&other, &mut daemon).unwrap()),
        DataPlaneFailure::Unauthorized
    );
    assert!(daemon.calls.is_empty());
}

#[test]
fn a_non_channel_request_is_unavailable_here() {
    let world = World::new();
    let mut sink = receiver_with(RECEIVER);
    let mut daemon = Daemon::new(&world.backend, &world.metadata, &world.sink);
    assert_eq!(
        failure(daemon.relay(&mut sink, DataPlaneRequest::ListModelTools { id: id(1) })),
        DataPlaneFailure::Unavailable
    );
    assert!(daemon.calls.is_empty());
}

#[test]
fn a_key_that_is_not_the_one_the_definition_names_is_refused() {
    let world = World::new();
    let mut impostor = receiver_with([0x43; 32]);
    let mut daemon = Daemon::new(&world.backend, &world.metadata, &world.sink);
    assert_eq!(
        failure(daemon.relay(&mut impostor, receive(1))),
        DataPlaneFailure::Unauthorized
    );

    let binding = binding("weather", ChannelBindingAccess::Write);
    let keys = load(
        &binding,
        &[
            slot(KeyKind::OriginatorSigningSeed),
            slot(KeyKind::ChannelMasterKey),
        ],
        &[[0x32; 32], CMK],
    )
    .unwrap();
    let mut wrong_seed = ChannelBroker::new(binding, keys);
    let mut daemon = Daemon::new(&world.backend, &world.metadata, &world.weather);
    assert_eq!(
        failure(daemon.relay(&mut wrong_seed, publish(2, b"x"))),
        DataPlaneFailure::Unauthorized
    );
    assert_eq!(daemon.calls, [CallbackOp::LoadDefinition]);
}

#[test]
fn a_destroyed_channel_is_refused() {
    let world = World::new();
    let mut weather = publisher();
    let mut daemon = Daemon::new(&world.backend, &world.metadata, &world.weather);
    daemon.relay(&mut weather, publish(1, b"before"));
    ChannelDefinitionStore::new(&world.backend)
        .destroy(channel())
        .unwrap();
    assert_eq!(
        failure(daemon.relay(&mut weather, publish(2, b"after"))),
        DataPlaneFailure::Unauthorized
    );
}

#[test]
fn a_message_with_no_grant_for_the_receiver_fails_closed() {
    let world = World::new();
    // Appended without granting the receiver first.
    ChannelStore::new(&world.backend, channel())
        .append(
            AppendRequest {
                message_id: uuid_v7(150),
                timestamp_ns: 1,
                originator_id: b"weather".to_vec(),
                key_epoch: KeyEpoch(0),
                content_type: "text/plain".to_owned(),
            },
            b"ungranted",
            &ChannelMasterKey::from_bytes(CMK),
            &OriginatorSigningKey::from_seed(SEED),
        )
        .unwrap();
    let mut sink = receiver_with(RECEIVER);
    let mut daemon = Daemon::new(&world.backend, &world.metadata, &world.sink);
    assert_eq!(
        failure(daemon.relay(&mut sink, receive(1))),
        DataPlaneFailure::Channel
    );
}

// ---- a daemon that lies ---------------------------------------------------

/// Wraps the honest daemon, rewriting what one callback returns.
struct Lying<'a, F: FnMut(CallbackReply) -> CallbackReply> {
    honest: Daemon<'a>,
    rewrite_reserve: F,
}

impl<F: FnMut(CallbackReply) -> CallbackReply> ChannelCallbacks for Lying<'_, F> {
    fn call(
        &mut self,
        request_id: RequestId,
        call: Callback,
    ) -> Result<CallbackOutcome, BrokerError> {
        let op = call.op();
        let outcome = self.honest.call(request_id, call)?;
        Ok(match outcome {
            CallbackOutcome::Ok(reply) if op == CallbackOp::ReserveAppend => {
                CallbackOutcome::Ok((self.rewrite_reserve)(reply))
            }
            other => other,
        })
    }
}

#[test]
fn the_broker_refuses_to_encrypt_under_a_header_it_did_not_ask_for() {
    use chief_of_staff_channel_crypto::{
        plaintext_hash, prepare_message_header_with_hash, MessageFields, Sequence,
    };

    // Each lie keeps the reservation's sequence, so the broker's abandon
    // can give it back; what changes is a field the broker checks.
    type Lie = fn(&MessageFields, [u8; 32]) -> (MessageFields, [u8; 32]);
    let lies: [(&str, Lie); 4] = [
        ("content type", |f, h| {
            (
                MessageFields::new(
                    f.message_id(),
                    f.timestamp_ns(),
                    f.originator_id().to_vec(),
                    f.channel_id(),
                    f.sequence(),
                    f.key_epoch(),
                    "text/html".to_owned(),
                ),
                h,
            )
        }),
        ("plaintext hash", |f, _| {
            (f.clone(), plaintext_hash(b"something else"))
        }),
        ("originator", |f, h| {
            (
                MessageFields::new(
                    f.message_id(),
                    f.timestamp_ns(),
                    b"sink".to_vec(),
                    f.channel_id(),
                    f.sequence(),
                    f.key_epoch(),
                    f.content_type().to_owned(),
                ),
                h,
            )
        }),
        ("epoch", |f, h| {
            (
                MessageFields::new(
                    f.message_id(),
                    f.timestamp_ns(),
                    f.originator_id().to_vec(),
                    f.channel_id(),
                    f.sequence(),
                    KeyEpoch(1),
                    f.content_type().to_owned(),
                ),
                h,
            )
        }),
    ];
    for (name, lie) in lies {
        let world = World::new();
        let mut weather = publisher();
        let request = publish(1, b"sunny");
        let mut lying = Lying {
            honest: Daemon::new(&world.backend, &world.metadata, &world.weather),
            rewrite_reserve: |reply| match reply {
                CallbackReply::Reserved(header) => {
                    let (fields, hash) = lie(header.fields(), header.plaintext_hash());
                    CallbackReply::Reserved(prepare_message_header_with_hash(fields, hash))
                }
                other => other,
            },
        };
        lying.honest.in_flight = InFlight::for_request(&request);
        assert_eq!(
            failure(weather.serve(&request, &mut lying).unwrap()),
            DataPlaneFailure::Internal,
            "{name}"
        );
        // Nothing was committed, and the reservation was given back.
        assert_eq!(
            lying.honest.calls.last(),
            Some(&CallbackOp::AbandonAppend),
            "{name}"
        );
        let state = ChannelStore::new(&world.backend, channel())
            .state()
            .unwrap();
        assert_eq!(state.pending_header, None, "{name}");
        assert_eq!(state.next_sequence, Sequence(1), "{name}");
    }
}

#[test]
fn the_broker_never_encrypts_twice_under_one_sequence() {
    use chief_of_staff_channel_crypto::Sequence;
    let world = World::new();
    let mut weather = publisher();
    let mut daemon = Daemon::new(&world.backend, &world.metadata, &world.weather);
    daemon.relay(&mut weather, publish(1, b"first"));
    // Rewind the store's sequence, as a daemon replaying old state would.
    let store = ChannelStore::new(&world.backend, channel());
    let rewound = InMemoryStorageBackend::new();
    ChannelDefinitionStore::new(&rewound)
        .create(&definition())
        .unwrap();
    ChannelStore::new(&rewound, channel()).initialize().unwrap();
    // Copy the grant across so only the sequence differs.
    let grant = store.key_grant(KeyEpoch(0), b"sink").unwrap().unwrap();
    ChannelStore::new(&rewound, channel())
        .save_key_grant(&grant)
        .unwrap();
    let mut replaying = Daemon::new(&rewound, &world.metadata, &world.weather);
    // The broker still remembers sequence 0.
    assert_eq!(
        failure(replaying.relay(&mut weather, publish(2, b"second"))),
        DataPlaneFailure::Internal
    );
    assert_eq!(replaying.calls.last(), Some(&CallbackOp::AbandonAppend));
    let state = ChannelStore::new(&rewound, channel()).state().unwrap();
    assert_eq!(state.pending_header, None);
    assert_eq!(state.next_sequence, Sequence(1));
}

#[test]
fn a_reply_to_another_callback_is_fatal() {
    struct Confused;
    impl ChannelCallbacks for Confused {
        fn call(&mut self, _: RequestId, _: Callback) -> Result<CallbackOutcome, BrokerError> {
            Ok(CallbackOutcome::Ok(CallbackReply::GrantsSaved))
        }
    }
    let mut sink = receiver_with(RECEIVER);
    assert_eq!(
        sink.serve(&receive(1), &mut Confused).unwrap_err(),
        BrokerError::WrongReply
    );
}

// ---- key loading ----------------------------------------------------------

#[test]
fn keys_must_match_the_binding_exactly() {
    let write = binding("weather", ChannelBindingAccess::Write);
    let read = binding("sink", ChannelBindingAccess::Read);
    let both = [
        slot(KeyKind::OriginatorSigningSeed),
        slot(KeyKind::ChannelMasterKey),
    ];
    let error = |binding: &HostPipelineBinding, slots: &[KeySlot], keys: &[[u8; 32]]| {
        load(binding, slots, keys).err().expect("loaded")
    };

    assert_eq!(
        error(&read, &[slot(KeyKind::OriginatorSigningSeed)], &[SEED]),
        KeyError::WrongDirection
    );
    assert_eq!(
        error(&write, &[slot(KeyKind::ReceiverPrivateKey)], &[RECEIVER]),
        KeyError::WrongDirection
    );
    assert_eq!(
        error(
            &write,
            &[
                KeySlot {
                    channel_id: ChannelId(uuid_v7(99)),
                    kind: KeyKind::ChannelMasterKey
                },
                both[0],
                both[1]
            ],
            &[CMK, SEED, CMK]
        ),
        KeyError::UnboundChannel
    );
    assert_eq!(
        error(&write, &[both[0], both[0], both[1]], &[SEED, SEED, CMK]),
        KeyError::Duplicate
    );
    assert_eq!(error(&write, &both[..1], &[SEED]), KeyError::Missing);
    assert_eq!(error(&read, &[], &[]), KeyError::Missing);
    assert_eq!(error(&write, &both, &[SEED, [0; 32]]), KeyError::InvalidKey);
    assert_eq!(
        LoadedKeys::load(&write, &both, |_| Ok(Zeroizing::new(vec![1; 31])))
            .err()
            .unwrap(),
        KeyError::Unreadable
    );
    assert_eq!(
        LoadedKeys::load(&write, &both, |_| Err(KeyError::Unreadable))
            .err()
            .unwrap(),
        KeyError::Unreadable
    );

    // The slot table is checked whole before any key is read.
    let mut reads = 0;
    let _ = LoadedKeys::load(&read, &[slot(KeyKind::OriginatorSigningSeed)], |_| {
        reads += 1;
        Ok(Zeroizing::new(SEED.to_vec()))
    });
    assert_eq!(reads, 0);
}

#[test]
fn ready_reports_public_halves_in_slot_order_and_never_a_channel_key() {
    let write = binding("weather", ChannelBindingAccess::Write);
    let keys = load(
        &write,
        &[
            slot(KeyKind::ChannelMasterKey),
            slot(KeyKind::OriginatorSigningSeed),
        ],
        &[CMK, SEED],
    )
    .unwrap();
    let public = keys.public_keys();
    assert_eq!(public.len(), 1);
    assert_eq!(public[0].kind, KeyKind::OriginatorSigningSeed);
    assert_eq!(
        public[0].public_key,
        OriginatorSigningKey::from_seed(SEED).public_key()
    );
}
