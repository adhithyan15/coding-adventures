//! The relay against a broker the test plays in process, honest and not.
//! The daemon side is real: a `CallbackServer` over in-memory storage.

use std::collections::VecDeque;
use std::io::PipeReader;
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chief_of_staff_broker_callbacks::Violation;
use chief_of_staff_broker_launcher::{
    start_relay_over, BrokerRelay, HostGone, PinnedBindingResolver, RelayConfig, RelayEnd,
    RelayRefused, ResponseSink,
};
use chief_of_staff_broker_protocol::{
    decode_to_broker, definition_digest, read_frame, Callback, CallbackOutcome, CallbackReply,
    FromBroker, ProtocolError, ToBroker,
};
use chief_of_staff_channel_crypto::{plaintext_hash, ChannelId, KeyEpoch};
use chief_of_staff_channel_endpoints::{
    AgentId, ChannelDefinition, ChannelDefinitionStore, MessageId, MessageMetadata,
    MessageMetadataError, MessageMetadataSource, OriginatorIdentity, ReceiverIdentity,
};
use chief_of_staff_host_control_protocol::{
    ChannelBinding, ChannelBindingAccess, DataPlaneFailure, DataPlaneRequest, DataPlaneResponse,
    LaunchBindings, RequestId,
};
use chief_of_staff_pipeline_bindings::{HostPipelineBinding, PipelineId};
use chief_of_staff_service_registry::{HostName, HostRegistration, PackagePath, RestartPolicy};
use storage_core::{InMemoryStorageBackend, StorageBackend};

fn uuid_v7(tag: u8) -> [u8; 16] {
    let mut bytes = [0u8; 16];
    bytes[0] = tag;
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes
}

const REPORTS: u8 = 12;

fn channel() -> ChannelId {
    ChannelId(uuid_v7(REPORTS))
}

fn definition() -> ChannelDefinition {
    ChannelDefinition::new(
        channel(),
        OriginatorIdentity {
            agent_id: AgentId::new(b"weather".to_vec()).unwrap(),
            public_key: [3; 32],
        },
        vec![ReceiverIdentity {
            agent_id: AgentId::new(b"sink".to_vec()).unwrap(),
            public_key: [4; 32],
        }],
        1,
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
        AgentId::new(b"weather".to_vec()).unwrap(),
        LaunchBindings::new(
            vec![
                ChannelBinding::new("reports", ChannelBindingAccess::Write, uuid_v7(REPORTS))
                    .unwrap(),
            ],
            None,
        )
        .unwrap(),
    )
}

/// Mints metadata, optionally slowly: the daemon's own time, which the
/// broker's deadline must not be charged for.
struct Metadata {
    values: Mutex<VecDeque<MessageMetadata>>,
    delay: Duration,
}

impl MessageMetadataSource for Metadata {
    fn next_metadata(&self) -> Result<MessageMetadata, MessageMetadataError> {
        std::thread::sleep(self.delay);
        self.values
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| MessageMetadataError::new("exhausted"))
    }
}

/// Responses delivered to the host.
struct Sink(SyncSender<DataPlaneResponse>, bool);

impl ResponseSink for Sink {
    fn deliver(&mut self, response: DataPlaneResponse) -> Result<(), HostGone> {
        if self.1 {
            return Err(HostGone);
        }
        self.0.send(response).map_err(|_| HostGone)
    }
}

/// The broker, played by the test.
struct FakeBroker {
    from_relay: PipeReader,
    to_relay: SyncSender<Result<FromBroker, ProtocolError>>,
}

impl FakeBroker {
    fn next(&mut self) -> ToBroker {
        decode_to_broker(&read_frame(&mut self.from_relay).unwrap()).unwrap()
    }

    fn send(&self, frame: FromBroker) {
        self.to_relay.send(Ok(frame)).unwrap();
    }

    /// Make a callback and return its outcome.
    fn call(&mut self, callback_id: u32, request_id: RequestId, call: Callback) -> CallbackOutcome {
        self.send(FromBroker::Callback {
            callback_id,
            request_id,
            call,
        });
        match self.next() {
            ToBroker::CallbackResult {
                callback_id: answered,
                outcome,
            } => {
                assert_eq!(answered, callback_id);
                outcome
            }
            other => panic!("expected a callback result, got {other:?}"),
        }
    }
}

struct Rig {
    relay: BrokerRelay,
    broker: FakeBroker,
    delivered: Receiver<DataPlaneResponse>,
}

fn new_rig(deadline: Duration, metadata_delay: Duration, host_gone: bool) -> Rig {
    let backend: Arc<dyn StorageBackend> = Arc::new(InMemoryStorageBackend::new());
    ChannelDefinitionStore::new(&*backend)
        .create(&definition())
        .unwrap();
    let metadata = Arc::new(Metadata {
        values: Mutex::new(
            (1..=5)
                .map(|n| MessageMetadata {
                    message_id: MessageId::from_uuid_v7(uuid_v7(100 + n)).unwrap(),
                    timestamp_ns: u64::from(n),
                })
                .collect(),
        ),
        delay: metadata_delay,
    });
    let (to_relay, frames) = mpsc::sync_channel(4);
    let (from_relay, to_broker) = std::io::pipe().unwrap();
    let (delivered_tx, delivered) = mpsc::sync_channel(4);
    let relay = start_relay_over(
        Box::new(to_broker),
        frames,
        backend,
        metadata,
        Box::new(PinnedBindingResolver::new(|| Some(binding()), &binding())),
        Box::new(Sink(delivered_tx, host_gone)),
        RelayConfig { deadline },
    )
    .unwrap();
    Rig {
        relay,
        broker: FakeBroker {
            from_relay,
            to_relay,
        },
        delivered,
    }
}

fn publish(n: u64) -> DataPlaneRequest {
    DataPlaneRequest::Publish {
        id: RequestId::new(n).unwrap(),
        channel_id: uuid_v7(REPORTS),
        content_type: "text/plain".to_owned(),
        payload: b"sunny".to_vec(),
    }
}

fn published(n: u64) -> DataPlaneResponse {
    DataPlaneResponse::Published {
        id: RequestId::new(n).unwrap(),
        message_id: uuid_v7(101),
        sequence: 0,
        timestamp_ns: 1,
    }
}

/// Wait for the relay to end, and say why.
fn ended(relay: &BrokerRelay) -> RelayEnd {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(end) = relay.ended() {
            return end;
        }
        assert!(Instant::now() < deadline, "the relay did not end");
        std::thread::sleep(Duration::from_millis(5));
    }
}

const LONG: Duration = Duration::from_secs(10);

#[test]
fn an_honest_broker_s_response_reaches_the_host_and_the_relay_goes_on() {
    let mut rig = new_rig(LONG, Duration::ZERO, false);
    for n in 1..=2 {
        rig.relay.relay(publish(n)).unwrap();
        let ToBroker::Request(request) = rig.broker.next() else {
            panic!("no request")
        };
        assert_eq!(request, publish(n));
        let outcome = rig.broker.call(
            1,
            request.id(),
            Callback::LoadDefinition {
                channel_id: channel(),
            },
        );
        assert!(matches!(
            outcome,
            CallbackOutcome::Ok(CallbackReply::Definition(_))
        ));
        rig.broker.send(FromBroker::Response(published(n)));
        assert_eq!(rig.delivered.recv_timeout(LONG).unwrap(), published(n));
    }
    // A failure is a response like any other.
    rig.relay.relay(publish(3)).unwrap();
    rig.broker.next();
    let failed = DataPlaneResponse::Failed {
        id: RequestId::new(3).unwrap(),
        failure: DataPlaneFailure::Unauthorized,
    };
    rig.broker.send(FromBroker::Response(failed.clone()));
    assert_eq!(rig.delivered.recv_timeout(LONG).unwrap(), failed);
    assert_eq!(rig.relay.ended(), None);
    drop(rig.broker);
    assert!(rig.relay.stop());
}

#[test]
fn a_violating_callback_ends_the_relay_and_nothing_reaches_the_host() {
    let mut rig = new_rig(LONG, Duration::ZERO, false);
    rig.relay.relay(publish(1)).unwrap();
    rig.broker.next();
    rig.broker.send(FromBroker::Callback {
        callback_id: 1,
        request_id: RequestId::new(1).unwrap(),
        call: Callback::LoadDefinition {
            channel_id: ChannelId(uuid_v7(99)),
        },
    });
    assert_eq!(
        ended(&rig.relay),
        RelayEnd::Violation(Violation::WrongChannel)
    );
    assert!(rig.delivered.try_recv().is_err());
    // Ended: a further request is refused.
    assert_eq!(rig.relay.relay(publish(2)), Err(RelayRefused::Gone));
}

#[test]
fn a_broker_that_never_answers_misses_its_deadline() {
    let mut rig = new_rig(Duration::from_millis(200), Duration::ZERO, false);
    rig.relay.relay(publish(1)).unwrap();
    rig.broker.next();
    assert_eq!(ended(&rig.relay), RelayEnd::Deadline);
}

#[test]
fn the_deadline_does_not_charge_the_broker_for_the_daemon_s_time() {
    // The daemon takes 400 ms to mint metadata; the broker's deadline is
    // 300 ms. The broker itself is quick, so it must not be ended.
    let mut rig = new_rig(
        Duration::from_millis(300),
        Duration::from_millis(400),
        false,
    );
    rig.relay.relay(publish(1)).unwrap();
    rig.broker.next();
    let outcome = rig.broker.call(
        1,
        RequestId::new(1).unwrap(),
        Callback::ReserveAppend {
            channel_id: channel(),
            definition_digest: definition_digest(&definition()),
            content_type: "text/plain".to_owned(),
            plaintext_hash: plaintext_hash(b"sunny"),
        },
    );
    assert!(
        matches!(outcome, CallbackOutcome::Ok(CallbackReply::Reserved(_))),
        "{outcome:?}"
    );
    rig.broker.send(FromBroker::Response(published(1)));
    assert_eq!(rig.delivered.recv_timeout(LONG).unwrap(), published(1));
    assert_eq!(rig.relay.ended(), None);

    // The broker's own slowness is charged.
    rig.relay.relay(publish(2)).unwrap();
    rig.broker.next();
    std::thread::sleep(Duration::from_millis(400));
    // Too late: the relay has already ended, so this may find no one.
    let _ = rig
        .broker
        .to_relay
        .send(Ok(FromBroker::Response(published(2))));
    assert_eq!(ended(&rig.relay), RelayEnd::Deadline);
    assert!(rig.delivered.try_recv().is_err());
}

#[test]
fn a_response_that_does_not_answer_the_request_ends_the_relay() {
    for wrong in [
        // Another request's id.
        published(9),
        // The right id, the wrong operation.
        DataPlaneResponse::Acknowledged {
            id: RequestId::new(1).unwrap(),
            sequence: 0,
        },
    ] {
        let mut rig = new_rig(LONG, Duration::ZERO, false);
        rig.relay.relay(publish(1)).unwrap();
        rig.broker.next();
        rig.broker.send(FromBroker::Response(wrong));
        assert_eq!(ended(&rig.relay), RelayEnd::BadResponse);
        assert!(rig.delivered.try_recv().is_err());
    }
}

#[test]
fn a_broken_frame_an_unexpected_ready_or_an_exit_ends_the_relay() {
    let mut rig = new_rig(LONG, Duration::ZERO, false);
    rig.relay.relay(publish(1)).unwrap();
    rig.broker.next();
    rig.broker
        .to_relay
        .send(Err(ProtocolError::Malformed))
        .unwrap();
    assert_eq!(ended(&rig.relay), RelayEnd::Protocol);

    let mut rig = new_rig(LONG, Duration::ZERO, false);
    rig.relay.relay(publish(1)).unwrap();
    rig.broker.next();
    rig.broker.send(FromBroker::Ready {
        public_keys: Vec::new(),
    });
    assert_eq!(ended(&rig.relay), RelayEnd::Protocol);

    let mut rig = new_rig(LONG, Duration::ZERO, false);
    rig.relay.relay(publish(1)).unwrap();
    rig.broker.next();
    drop(rig.broker.to_relay);
    assert_eq!(ended(&rig.relay), RelayEnd::Exited);
}

#[test]
fn a_host_that_is_gone_ends_the_relay() {
    let mut rig = new_rig(LONG, Duration::ZERO, true);
    rig.relay.relay(publish(1)).unwrap();
    rig.broker.next();
    rig.broker.send(FromBroker::Response(published(1)));
    assert_eq!(ended(&rig.relay), RelayEnd::HostGone);
}

#[test]
fn a_non_channel_request_is_never_relayed() {
    let rig = new_rig(LONG, Duration::ZERO, false);
    rig.relay
        .relay(DataPlaneRequest::ListModelTools {
            id: RequestId::new(1).unwrap(),
        })
        .unwrap();
    assert_eq!(ended(&rig.relay), RelayEnd::Protocol);
}

#[test]
fn only_one_request_waits_behind_the_one_in_flight() {
    let mut rig = new_rig(LONG, Duration::ZERO, false);
    rig.relay.relay(publish(1)).unwrap();
    rig.broker.next();
    // One may queue; an honest host never sends even that.
    rig.relay.relay(publish(2)).unwrap();
    assert_eq!(rig.relay.relay(publish(3)), Err(RelayRefused::Busy));
}

#[test]
fn a_broker_that_stops_reading_cannot_stall_the_relay() {
    // A request larger than the pipe's buffer, to a broker that never reads
    // it. The write cannot complete, but the relay only queued it, so the
    // deadline still runs and ends it.
    let rig = new_rig(Duration::from_millis(300), Duration::ZERO, false);
    rig.relay
        .relay(DataPlaneRequest::Publish {
            id: RequestId::new(1).unwrap(),
            channel_id: uuid_v7(REPORTS),
            content_type: "application/octet-stream".to_owned(),
            payload: vec![0; 256 * 1024],
        })
        .unwrap();
    assert_eq!(ended(&rig.relay), RelayEnd::Deadline);
    // The broker side is still holding its end, unread: the writer thread
    // is stuck in its write, and the relay ended anyway.
    drop(rig.broker);
}
