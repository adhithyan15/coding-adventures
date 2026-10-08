//! The callback server against real storage. The test plays the broker: it
//! holds the keys and makes the callbacks an honest broker would, and then
//! every callback a dishonest one might.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::Mutex;

use chief_of_staff_broker_callbacks::{BindingResolver, CallbackServer, InFlight, Violation};
use chief_of_staff_broker_protocol::{
    definition_digest, Callback, CallbackOp, CallbackOutcome, CallbackReply, Refusal,
    MAX_CALLBACKS_PER_REQUEST,
};
use chief_of_staff_channel_crypto::{
    encrypt_message_with_header, plaintext_hash, seal_channel_key, ChannelId, ChannelMasterKey,
    KeyEpoch, OriginatorSigningKey, ReceiverEpochKeys, ReceiverKeyPair, Sequence,
};
use chief_of_staff_channel_endpoints::{
    open_delivered_message, AgentId, ChannelDefinition, ChannelDefinitionStore, MessageId,
    MessageMetadata, MessageMetadataError, MessageMetadataSource, OriginatorIdentity,
    ReceiverIdentity,
};
use chief_of_staff_channel_store::{AppendRequest, ChannelStore};
use chief_of_staff_host_control_protocol::{
    ChannelBinding, ChannelBindingAccess, DataPlaneRequest, LaunchBindings, RequestId,
};
use chief_of_staff_pipeline_bindings::{HostPipelineBinding, PipelineId};
use chief_of_staff_service_registry::{HostName, HostRegistration, PackagePath, RestartPolicy};
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
const OTHER: u8 = 13;

fn channel(tag: u8) -> ChannelId {
    ChannelId(uuid_v7(tag))
}

fn agent(name: &str) -> AgentId {
    AgentId::new(name.as_bytes().to_vec()).unwrap()
}

fn originator_key() -> OriginatorSigningKey {
    OriginatorSigningKey::from_seed([0x31; 32])
}

fn receiver_key() -> ReceiverKeyPair {
    ReceiverKeyPair::from_private_key([0x42; 32]).unwrap()
}

fn cmk() -> ChannelMasterKey {
    ChannelMasterKey::from_bytes([0xa5; 32])
}

/// "weather" writes the reports channel; "sink" reads it.
fn reports_definition(tag: u8) -> ChannelDefinition {
    ChannelDefinition::new(
        channel(tag),
        OriginatorIdentity {
            agent_id: agent("weather"),
            public_key: originator_key().public_key(),
        },
        vec![ReceiverIdentity {
            agent_id: agent("sink"),
            public_key: receiver_key().public_key(),
        }],
        1_725_000_000_000_000_000,
        KeyEpoch(0),
    )
    .unwrap()
}

fn binding(name: &str, channels: Vec<(u8, ChannelBindingAccess)>) -> HostPipelineBinding {
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
            channels
                .into_iter()
                .enumerate()
                .map(|(index, (tag, access))| {
                    ChannelBinding::new(format!("channel-{index}"), access, uuid_v7(tag)).unwrap()
                })
                .collect(),
            None,
        )
        .unwrap(),
    )
}

/// A resolver the test can unwire mid-flight.
struct Resolver(RefCell<Option<HostPipelineBinding>>);

impl BindingResolver for Resolver {
    fn current_binding(&self) -> Option<HostPipelineBinding> {
        self.0.borrow().clone()
    }
}

fn resolver(binding: HostPipelineBinding) -> Resolver {
    Resolver(RefCell::new(Some(binding)))
}

struct Metadata(Mutex<VecDeque<MessageMetadata>>);

impl Metadata {
    fn new() -> Self {
        Self(Mutex::new(
            (1..=20)
                .map(|n| MessageMetadata {
                    message_id: MessageId::from_uuid_v7(uuid_v7(100 + n)).unwrap(),
                    timestamp_ns: 1_000 + u64::from(n),
                })
                .collect(),
        ))
    }
}

impl MessageMetadataSource for Metadata {
    fn next_metadata(&self) -> Result<MessageMetadata, MessageMetadataError> {
        self.0
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| MessageMetadataError::new("exhausted"))
    }
}

fn publish_request(tag: u8) -> DataPlaneRequest {
    DataPlaneRequest::Publish {
        id: RequestId::new(5).unwrap(),
        channel_id: uuid_v7(tag),
        content_type: "text/plain".to_owned(),
        payload: b"sunny".to_vec(),
    }
}

fn receive_request(tag: u8, limit: u16) -> DataPlaneRequest {
    DataPlaneRequest::Receive {
        id: RequestId::new(6).unwrap(),
        channel_id: uuid_v7(tag),
        limit,
    }
}

fn acknowledge_request(tag: u8) -> DataPlaneRequest {
    DataPlaneRequest::Acknowledge {
        id: RequestId::new(7).unwrap(),
        channel_id: uuid_v7(tag),
        message_id: uuid_v7(101),
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
        let definitions = ChannelDefinitionStore::new(&backend);
        definitions.create(&reports_definition(REPORTS)).unwrap();
        definitions.create(&reports_definition(OTHER)).unwrap();
        Self {
            backend,
            metadata: Metadata::new(),
            weather: resolver(binding(
                "weather",
                vec![(REPORTS, ChannelBindingAccess::Write)],
            )),
            sink: resolver(binding("sink", vec![(REPORTS, ChannelBindingAccess::Read)])),
        }
    }

    fn server<'a>(&'a self, resolver: &'a Resolver) -> CallbackServer<'a> {
        CallbackServer::new(&self.backend, &self.metadata, resolver)
    }
}

fn digest() -> [u8; 32] {
    definition_digest(&reports_definition(REPORTS))
}

fn serve(
    server: &CallbackServer<'_>,
    in_flight: &mut InFlight,
    call: Callback,
) -> Result<CallbackOutcome, Violation> {
    let id = in_flight.request_id();
    server.serve(in_flight, id, call)
}

fn ok(outcome: Result<CallbackOutcome, Violation>) -> CallbackReply {
    match outcome {
        Ok(CallbackOutcome::Ok(reply)) => reply,
        other => panic!("expected an answer, got {other:?}"),
    }
}

fn refused(outcome: Result<CallbackOutcome, Violation>) -> Refusal {
    match outcome {
        Ok(CallbackOutcome::Refused { refusal, .. }) => refusal,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// Publish `payload` exactly as an honest broker would.
fn publish(world: &World, payload: &[u8]) -> u64 {
    let server = world.server(&world.weather);
    let mut in_flight = InFlight::for_request(&publish_request(REPORTS)).unwrap();
    let missing = ok(serve(
        &server,
        &mut in_flight,
        Callback::LoadMissingGrants {
            channel_id: channel(REPORTS),
            definition_digest: digest(),
        },
    ));
    if let CallbackReply::MissingGrants { receivers, .. } = missing {
        if !receivers.is_empty() {
            assert_eq!(receivers, vec![0]);
            let grant = seal_channel_key(
                b"weather",
                b"sink",
                channel(REPORTS),
                KeyEpoch(0),
                &cmk(),
                &receiver_key().public_key(),
                &originator_key(),
            )
            .unwrap();
            assert_eq!(
                ok(serve(
                    &server,
                    &mut in_flight,
                    Callback::SaveGrants {
                        channel_id: channel(REPORTS),
                        definition_digest: digest(),
                        grants: vec![grant],
                    }
                )),
                CallbackReply::GrantsSaved
            );
        }
    }
    let header = match ok(serve(
        &server,
        &mut in_flight,
        Callback::ReserveAppend {
            channel_id: channel(REPORTS),
            definition_digest: digest(),
            content_type: "text/plain".to_owned(),
            plaintext_hash: plaintext_hash(payload),
        },
    )) {
        CallbackReply::Reserved(header) => header,
        other => panic!("{other:?}"),
    };
    // The daemon filled in who and when; the broker checks it.
    assert_eq!(header.fields().originator_id(), b"weather");
    assert_eq!(header.fields().key_epoch(), KeyEpoch(0));
    assert_eq!(header.plaintext_hash(), plaintext_hash(payload));
    let message = encrypt_message_with_header(header, payload, &cmk(), &originator_key()).unwrap();
    match ok(serve(
        &server,
        &mut in_flight,
        Callback::CommitAppend {
            channel_id: channel(REPORTS),
            definition_digest: digest(),
            message,
        },
    )) {
        CallbackReply::Committed { sequence } => sequence,
        other => panic!("{other:?}"),
    }
}

// ---- the honest path ------------------------------------------------------

#[test]
fn a_message_published_through_callbacks_is_received_through_callbacks() {
    let world = World::new();
    assert_eq!(publish(&world, b"sunny"), 0);
    assert_eq!(publish(&world, b"rain later"), 1);

    let server = world.server(&world.sink);
    let mut in_flight = InFlight::for_request(&receive_request(REPORTS, 10)).unwrap();
    let definition = match ok(serve(
        &server,
        &mut in_flight,
        Callback::LoadDefinition {
            channel_id: channel(REPORTS),
        },
    )) {
        CallbackReply::Definition(definition) => definition,
        other => panic!("{other:?}"),
    };
    let CallbackReply::ReceiverPage {
        first_unread,
        messages,
        grants,
    } = ok(serve(
        &server,
        &mut in_flight,
        Callback::ReadReceiverPage {
            channel_id: channel(REPORTS),
            definition_digest: definition_digest(&definition),
            limit: 10,
        },
    ))
    else {
        panic!("not a page")
    };
    assert_eq!(first_unread, 0);
    assert_eq!(messages.len(), 2);
    assert_eq!(grants.len(), 1, "one grant for the one epoch");

    // The broker side: open them with the receiver key and those grants.
    let mut keys = ReceiverEpochKeys::new(
        b"weather".to_vec(),
        b"sink".to_vec(),
        channel(REPORTS),
        receiver_key(),
        originator_key().public_key(),
    );
    let opened: Vec<_> = messages
        .iter()
        .map(|message| {
            open_delivered_message(&definition, &mut keys, message, |_| {
                Ok(grants.first().cloned())
            })
            .unwrap()
        })
        .collect();
    assert_eq!(opened[0].payload, b"sunny");
    assert_eq!(opened[1].payload, b"rain later");
    assert_eq!(opened[0].timestamp_ns, 1_001, "the daemon's metadata");

    // Acknowledge the first.
    let server = world.server(&world.sink);
    let mut in_flight = InFlight::for_request(&acknowledge_request(REPORTS)).unwrap();
    assert_eq!(
        ok(serve(
            &server,
            &mut in_flight,
            Callback::Acknowledge {
                channel_id: channel(REPORTS),
                definition_digest: definition_digest(&definition),
                sequence: 0,
            }
        )),
        CallbackReply::Acknowledged { first_unread: 1 }
    );
}

#[test]
fn a_channel_nobody_wrote_to_reads_as_empty() {
    let world = World::new();
    let server = world.server(&world.sink);
    let mut in_flight = InFlight::for_request(&receive_request(REPORTS, 10)).unwrap();
    assert_eq!(
        ok(serve(
            &server,
            &mut in_flight,
            Callback::ReadReceiverPage {
                channel_id: channel(REPORTS),
                definition_digest: digest(),
                limit: 10,
            }
        )),
        CallbackReply::ReceiverPage {
            first_unread: 0,
            messages: Vec::new(),
            grants: Vec::new(),
        }
    );
}

#[test]
fn storage_holds_no_plaintext() {
    let world = World::new();
    publish(&world, b"the porch light code is 4421");
    let store = ChannelStore::new(&world.backend, channel(REPORTS));
    let page = store.read_messages(Sequence(0), 10).unwrap();
    for message in page.messages {
        let bytes = chief_of_staff_channel_crypto::wire::encode_message(&message).unwrap();
        assert!(!bytes.windows(4).any(|window| window == b"4421"));
    }
}

// ---- violations: an honest broker never does these ------------------------

#[test]
fn a_callback_outside_its_request_ends_the_broker() {
    let world = World::new();
    let server = world.server(&world.weather);

    // Another request's id.
    let mut in_flight = InFlight::for_request(&publish_request(REPORTS)).unwrap();
    assert_eq!(
        server
            .serve(
                &mut in_flight,
                RequestId::new(999).unwrap(),
                Callback::LoadDefinition {
                    channel_id: channel(REPORTS)
                }
            )
            .unwrap_err(),
        Violation::NotInFlight
    );
    // Another channel.
    assert_eq!(
        serve(
            &server,
            &mut in_flight,
            Callback::LoadDefinition {
                channel_id: channel(OTHER)
            }
        )
        .unwrap_err(),
        Violation::WrongChannel
    );
    // Receiver callbacks during a Publish, and append callbacks during a
    // Receive or an Acknowledge.
    assert_eq!(
        serve(
            &server,
            &mut in_flight,
            Callback::ReadReceiverPage {
                channel_id: channel(REPORTS),
                definition_digest: digest(),
                limit: 1
            }
        )
        .unwrap_err(),
        Violation::OperationNotAllowed
    );
    for request in [receive_request(REPORTS, 5), acknowledge_request(REPORTS)] {
        let mut in_flight = InFlight::for_request(&request).unwrap();
        assert_eq!(
            serve(
                &server,
                &mut in_flight,
                Callback::ReserveAppend {
                    channel_id: channel(REPORTS),
                    definition_digest: digest(),
                    content_type: "text/plain".to_owned(),
                    plaintext_hash: [0; 32],
                }
            )
            .unwrap_err(),
            Violation::OperationNotAllowed
        );
    }
    // A non-channel request has no callback scope at all.
    assert!(InFlight::for_request(&DataPlaneRequest::ListModelTools {
        id: RequestId::new(1).unwrap()
    })
    .is_none());
}

#[test]
fn a_page_larger_than_the_request_asked_for_ends_the_broker() {
    let world = World::new();
    let server = world.server(&world.sink);
    let mut in_flight = InFlight::for_request(&receive_request(REPORTS, 5)).unwrap();
    assert_eq!(
        serve(
            &server,
            &mut in_flight,
            Callback::ReadReceiverPage {
                channel_id: channel(REPORTS),
                definition_digest: digest(),
                limit: 6
            }
        )
        .unwrap_err(),
        Violation::PageTooLarge
    );
}

#[test]
fn the_callback_budget_is_per_request() {
    let world = World::new();
    let server = world.server(&world.weather);
    let mut in_flight = InFlight::for_request(&publish_request(REPORTS)).unwrap();
    for _ in 0..MAX_CALLBACKS_PER_REQUEST {
        ok(serve(
            &server,
            &mut in_flight,
            Callback::LoadDefinition {
                channel_id: channel(REPORTS),
            },
        ));
    }
    assert_eq!(
        serve(
            &server,
            &mut in_flight,
            Callback::LoadDefinition {
                channel_id: channel(REPORTS)
            }
        )
        .unwrap_err(),
        Violation::BudgetExhausted
    );
    // A new request starts a new budget.
    let mut next = InFlight::for_request(&publish_request(REPORTS)).unwrap();
    ok(serve(
        &server,
        &mut next,
        Callback::LoadDefinition {
            channel_id: channel(REPORTS),
        },
    ));
}

#[test]
fn a_publish_gets_one_reservation_then_one_commit_or_abandon() {
    let world = World::new();
    let server = world.server(&world.weather);
    let reserve = || Callback::ReserveAppend {
        channel_id: channel(REPORTS),
        definition_digest: digest(),
        content_type: "text/plain".to_owned(),
        plaintext_hash: plaintext_hash(b"x"),
    };

    // A commit or abandon with nothing reserved.
    let mut in_flight = InFlight::for_request(&publish_request(REPORTS)).unwrap();
    assert_eq!(
        serve(
            &server,
            &mut in_flight,
            Callback::AbandonAppend {
                channel_id: channel(REPORTS),
                sequence: 0
            }
        )
        .unwrap_err(),
        Violation::AppendOutOfOrder
    );

    let CallbackReply::Reserved(header) = ok(serve(&server, &mut in_flight, reserve())) else {
        panic!()
    };
    assert_eq!(in_flight.open_reservation(), Some(Sequence(0)));
    // A second reservation.
    assert_eq!(
        serve(&server, &mut in_flight, reserve()).unwrap_err(),
        Violation::AppendOutOfOrder
    );
    // Abandoning some other sequence.
    assert_eq!(
        serve(
            &server,
            &mut in_flight,
            Callback::AbandonAppend {
                channel_id: channel(REPORTS),
                sequence: 9
            }
        )
        .unwrap_err(),
        Violation::AppendOutOfOrder
    );
    // Grant work after the reservation.
    assert_eq!(
        serve(
            &server,
            &mut in_flight,
            Callback::LoadMissingGrants {
                channel_id: channel(REPORTS),
                definition_digest: digest()
            }
        )
        .unwrap_err(),
        Violation::AppendOutOfOrder
    );

    let message = encrypt_message_with_header(header, b"x", &cmk(), &originator_key()).unwrap();
    ok(serve(
        &server,
        &mut in_flight,
        Callback::CommitAppend {
            channel_id: channel(REPORTS),
            definition_digest: digest(),
            message: message.clone(),
        },
    ));
    assert_eq!(in_flight.open_reservation(), None);
    // Nothing after the commit: not a second commit, not an abandon.
    assert_eq!(
        serve(
            &server,
            &mut in_flight,
            Callback::CommitAppend {
                channel_id: channel(REPORTS),
                definition_digest: digest(),
                message
            }
        )
        .unwrap_err(),
        Violation::AppendOutOfOrder
    );
    assert_eq!(
        serve(
            &server,
            &mut in_flight,
            Callback::AbandonAppend {
                channel_id: channel(REPORTS),
                sequence: 0
            }
        )
        .unwrap_err(),
        Violation::AppendOutOfOrder
    );
}

// ---- refusals: ordinary answers -------------------------------------------

#[test]
fn identity_and_direction_come_from_the_binding_resolved_each_time() {
    let world = World::new();

    // The sink reads; it may not take originator actions on the channel.
    let server = world.server(&world.sink);
    let mut in_flight = InFlight::for_request(&publish_request(REPORTS)).unwrap();
    assert_eq!(
        refused(serve(
            &server,
            &mut in_flight,
            Callback::LoadMissingGrants {
                channel_id: channel(REPORTS),
                definition_digest: digest()
            }
        )),
        Refusal::Unauthorized
    );

    // The weather agent writes; it may not read as a receiver.
    let server = world.server(&world.weather);
    let mut in_flight = InFlight::for_request(&receive_request(REPORTS, 5)).unwrap();
    assert_eq!(
        refused(serve(
            &server,
            &mut in_flight,
            Callback::ReadReceiverPage {
                channel_id: channel(REPORTS),
                definition_digest: digest(),
                limit: 5
            }
        )),
        Refusal::Unauthorized
    );

    // A channel the binding does not name at all.
    let mut in_flight = InFlight::for_request(&publish_request(OTHER)).unwrap();
    assert_eq!(
        refused(serve(
            &server,
            &mut in_flight,
            Callback::LoadDefinition {
                channel_id: channel(OTHER)
            }
        )),
        Refusal::Unauthorized
    );

    // A binding that names the channel for an agent the definition does not.
    let impostor = resolver(binding(
        "impostor",
        vec![(REPORTS, ChannelBindingAccess::Write)],
    ));
    let server = world.server(&impostor);
    let mut in_flight = InFlight::for_request(&publish_request(REPORTS)).unwrap();
    assert_eq!(
        refused(serve(
            &server,
            &mut in_flight,
            Callback::LoadDefinition {
                channel_id: channel(REPORTS)
            }
        )),
        Refusal::Unauthorized
    );

    // Unwired mid-flight: the very next callback is refused.
    let server = world.server(&world.weather);
    let mut in_flight = InFlight::for_request(&publish_request(REPORTS)).unwrap();
    ok(serve(
        &server,
        &mut in_flight,
        Callback::LoadDefinition {
            channel_id: channel(REPORTS),
        },
    ));
    *world.weather.0.borrow_mut() = None;
    assert_eq!(
        refused(serve(
            &server,
            &mut in_flight,
            Callback::LoadDefinition {
                channel_id: channel(REPORTS)
            }
        )),
        Refusal::Unauthorized
    );
}

#[test]
fn a_stale_or_destroyed_definition_is_refused() {
    let world = World::new();
    let server = world.server(&world.weather);
    let mut in_flight = InFlight::for_request(&publish_request(REPORTS)).unwrap();
    assert_eq!(
        refused(serve(
            &server,
            &mut in_flight,
            Callback::LoadMissingGrants {
                channel_id: channel(REPORTS),
                definition_digest: [0; 32]
            }
        )),
        Refusal::DefinitionChanged
    );
    ChannelDefinitionStore::new(&world.backend)
        .destroy(channel(REPORTS))
        .unwrap();
    assert_eq!(
        refused(serve(
            &server,
            &mut in_flight,
            Callback::LoadDefinition {
                channel_id: channel(REPORTS)
            }
        )),
        Refusal::ChannelDestroyed
    );
}

#[test]
fn only_a_grant_this_originator_signed_for_a_member_at_the_current_epoch_is_stored() {
    let world = World::new();
    let server = world.server(&world.weather);
    let seal = |receiver: &[u8], epoch: u64, signer: &OriginatorSigningKey| {
        seal_channel_key(
            b"weather",
            receiver,
            channel(REPORTS),
            KeyEpoch(epoch),
            &cmk(),
            &receiver_key().public_key(),
            signer,
        )
        .unwrap()
    };
    let good = seal(b"sink", 0, &originator_key());
    let mut flipped = good.clone();
    flipped.originator_signature[0] ^= 1;
    let mut foreign_originator = good.clone();
    foreign_originator.originator_id = b"impostor".to_vec();
    let bad = [
        seal(b"sink", 1, &originator_key()),     // a future epoch
        seal(b"stranger", 0, &originator_key()), // not a receiver
        seal(b"sink", 0, &OriginatorSigningKey::from_seed([0x99; 32])), // another signer
        flipped,
        foreign_originator,
    ];
    for grant in bad {
        let mut in_flight = InFlight::for_request(&publish_request(REPORTS)).unwrap();
        // A bad grant beside a good one: neither is stored.
        assert_eq!(
            refused(serve(
                &server,
                &mut in_flight,
                Callback::SaveGrants {
                    channel_id: channel(REPORTS),
                    definition_digest: digest(),
                    grants: vec![good.clone(), grant],
                }
            )),
            Refusal::InvalidRequest
        );
    }
    let store = ChannelStore::new(&world.backend, channel(REPORTS));
    assert!(store.key_grant(KeyEpoch(0), b"sink").unwrap().is_none());

    // The good one alone is stored, and then never replaced.
    let mut in_flight = InFlight::for_request(&publish_request(REPORTS)).unwrap();
    ok(serve(
        &server,
        &mut in_flight,
        Callback::SaveGrants {
            channel_id: channel(REPORTS),
            definition_digest: digest(),
            grants: vec![good.clone()],
        },
    ));
    let other_key = seal_channel_key(
        b"weather",
        b"sink",
        channel(REPORTS),
        KeyEpoch(0),
        &ChannelMasterKey::from_bytes([0x5b; 32]),
        &receiver_key().public_key(),
        &originator_key(),
    )
    .unwrap();
    let mut in_flight = InFlight::for_request(&publish_request(REPORTS)).unwrap();
    assert_eq!(
        refused(serve(
            &server,
            &mut in_flight,
            Callback::SaveGrants {
                channel_id: channel(REPORTS),
                definition_digest: digest(),
                grants: vec![other_key],
            }
        )),
        Refusal::Conflict
    );
    assert!(store.key_grant(KeyEpoch(0), b"sink").unwrap() == Some(good));
}

#[test]
fn a_commit_signed_by_anyone_else_is_refused_and_the_reservation_can_be_abandoned() {
    let world = World::new();
    let server = world.server(&world.weather);
    let mut in_flight = InFlight::for_request(&publish_request(REPORTS)).unwrap();
    let CallbackReply::Reserved(header) = ok(serve(
        &server,
        &mut in_flight,
        Callback::ReserveAppend {
            channel_id: channel(REPORTS),
            definition_digest: digest(),
            content_type: "text/plain".to_owned(),
            plaintext_hash: plaintext_hash(b"x"),
        },
    )) else {
        panic!()
    };
    let forged = encrypt_message_with_header(
        header,
        b"x",
        &cmk(),
        &OriginatorSigningKey::from_seed([0x99; 32]),
    )
    .unwrap();
    assert_eq!(
        refused(serve(
            &server,
            &mut in_flight,
            Callback::CommitAppend {
                channel_id: channel(REPORTS),
                definition_digest: digest(),
                message: forged
            }
        )),
        Refusal::InvalidRequest
    );
    let store = ChannelStore::new(&world.backend, channel(REPORTS));
    assert!(store.state().unwrap().pending_header.is_some());

    // The broker's next request abandons it, and the channel moves on.
    let mut next = InFlight::for_request(&publish_request(REPORTS)).unwrap();
    assert_eq!(
        refused(serve(
            &server,
            &mut next,
            Callback::ReserveAppend {
                channel_id: channel(REPORTS),
                definition_digest: digest(),
                content_type: "text/plain".to_owned(),
                plaintext_hash: plaintext_hash(b"y"),
            }
        )),
        Refusal::PendingAppend
    );
    assert!(store.abandon_pending_at(Sequence(0)).unwrap());
    assert_eq!(publish(&world, b"after"), 1, "sequence 0 stays consumed");
}

#[test]
fn a_page_holds_whole_messages_within_its_byte_budget() {
    let world = World::new();
    publish(&world, b"grant first");
    let store = ChannelStore::new(&world.backend, channel(REPORTS));
    let append = |payload: &[u8], id: u8| {
        store
            .append(
                AppendRequest {
                    message_id: uuid_v7(id),
                    timestamp_ns: 1,
                    originator_id: b"weather".to_vec(),
                    key_epoch: KeyEpoch(0),
                    content_type: "application/octet-stream".to_owned(),
                },
                payload,
                &cmk(),
                &originator_key(),
            )
            .unwrap()
    };
    let half = vec![0u8; 512 * 1024];
    append(&half, 150);
    append(&half, 151);

    let server = world.server(&world.sink);
    let mut in_flight = InFlight::for_request(&receive_request(REPORTS, 10)).unwrap();
    let CallbackReply::ReceiverPage { messages, .. } = ok(serve(
        &server,
        &mut in_flight,
        Callback::ReadReceiverPage {
            channel_id: channel(REPORTS),
            definition_digest: digest(),
            limit: 10,
        },
    )) else {
        panic!()
    };
    // The small first message and one half-megabyte message fit; the second
    // waits for the next page.
    assert_eq!(messages.len(), 2);

    // A message larger than any page is refused, not skipped.
    let world = World::new();
    publish(&world, b"grant first");
    let store = ChannelStore::new(&world.backend, channel(REPORTS));
    let sink = world.server(&world.sink);
    let mut in_flight = InFlight::for_request(&acknowledge_request(REPORTS)).unwrap();
    ok(serve(
        &sink,
        &mut in_flight,
        Callback::Acknowledge {
            channel_id: channel(REPORTS),
            definition_digest: digest(),
            sequence: 0,
        },
    ));
    store
        .append(
            AppendRequest {
                message_id: uuid_v7(152),
                timestamp_ns: 1,
                originator_id: b"weather".to_vec(),
                key_epoch: KeyEpoch(0),
                content_type: "application/octet-stream".to_owned(),
            },
            &vec![0u8; 1024 * 1024],
            &cmk(),
            &originator_key(),
        )
        .unwrap();
    let mut in_flight = InFlight::for_request(&receive_request(REPORTS, 10)).unwrap();
    assert_eq!(
        refused(serve(
            &sink,
            &mut in_flight,
            Callback::ReadReceiverPage {
                channel_id: channel(REPORTS),
                definition_digest: digest(),
                limit: 10
            }
        )),
        Refusal::TooLarge
    );
}

#[test]
fn a_refusal_names_the_callback_it_answers() {
    let world = World::new();
    let server = world.server(&world.weather);
    let mut in_flight = InFlight::for_request(&publish_request(REPORTS)).unwrap();
    let outcome = serve(
        &server,
        &mut in_flight,
        Callback::LoadMissingGrants {
            channel_id: channel(REPORTS),
            definition_digest: [0; 32],
        },
    )
    .unwrap();
    assert_eq!(outcome.op(), CallbackOp::LoadMissingGrants);
}
