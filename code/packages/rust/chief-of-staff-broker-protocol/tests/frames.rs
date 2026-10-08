//! Every frame round-trips, and everything else is refused.

use chief_of_staff_broker_protocol::*;
use chief_of_staff_channel_crypto::{
    encrypt_message, prepare_message_header, seal_channel_key, ChannelId, ChannelMasterKey,
    KeyEpoch, MessageFields, OriginatorSigningKey, ReceiverKeyPair, Sequence,
};
use chief_of_staff_channel_endpoints::{
    AgentId, ChannelDefinition, OriginatorIdentity, ReceiverIdentity,
};
use chief_of_staff_host_control_protocol::{
    ChannelBinding, ChannelBindingAccess, DataPlaneRequest, DataPlaneResponse, LaunchBindings,
    RequestId,
};
use chief_of_staff_pipeline_bindings::{HostPipelineBinding, PipelineId};
use chief_of_staff_service_registry::{HostName, HostRegistration, PackagePath, RestartPolicy};

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
                ChannelBinding::new("requests", ChannelBindingAccess::Read, uuid_v7(11)).unwrap(),
                ChannelBinding::new("reports", ChannelBindingAccess::Write, uuid_v7(12)).unwrap(),
            ],
            None,
        )
        .unwrap(),
    )
}

fn signing_key() -> OriginatorSigningKey {
    OriginatorSigningKey::from_seed([0x31; 32])
}

fn definition() -> ChannelDefinition {
    ChannelDefinition::new(
        channel(),
        OriginatorIdentity {
            agent_id: AgentId::new(b"weather".to_vec()).unwrap(),
            public_key: signing_key().public_key(),
        },
        vec![ReceiverIdentity {
            agent_id: AgentId::new(b"sink".to_vec()).unwrap(),
            public_key: ReceiverKeyPair::from_private_key([0x42; 32])
                .unwrap()
                .public_key(),
        }],
        1_725_000_000_000_000_000,
        KeyEpoch(0),
    )
    .unwrap()
}

fn fields(sequence: u64) -> MessageFields {
    MessageFields::new(
        uuid_v7(40 + sequence as u8),
        5,
        b"weather".to_vec(),
        channel(),
        Sequence(sequence),
        KeyEpoch(0),
        "text/plain".to_owned(),
    )
}

fn message(sequence: u64, payload: &[u8]) -> chief_of_staff_channel_crypto::EncryptedMessage {
    encrypt_message(
        fields(sequence),
        payload,
        &ChannelMasterKey::from_bytes([0xa5; 32]),
        &signing_key(),
    )
}

fn grant() -> chief_of_staff_channel_crypto::SealedChannelKeyGrant {
    seal_channel_key(
        b"weather",
        b"sink",
        channel(),
        KeyEpoch(0),
        &ChannelMasterKey::from_bytes([0xa5; 32]),
        &ReceiverKeyPair::from_private_key([0x42; 32])
            .unwrap()
            .public_key(),
        &signing_key(),
    )
    .unwrap()
}

fn id() -> RequestId {
    RequestId::new(9).unwrap()
}

fn every_frame_to_broker() -> Vec<ToBroker> {
    let mut frames = vec![
        ToBroker::Bootstrap {
            binding: binding(),
            slots: vec![
                KeySlot {
                    channel_id: ChannelId(uuid_v7(11)),
                    kind: KeyKind::ReceiverPrivateKey,
                },
                KeySlot {
                    channel_id: channel(),
                    kind: KeyKind::OriginatorSigningSeed,
                },
                KeySlot {
                    channel_id: channel(),
                    kind: KeyKind::ChannelMasterKey,
                },
            ],
        },
        ToBroker::Request(DataPlaneRequest::Publish {
            id: id(),
            channel_id: uuid_v7(12),
            content_type: "text/plain".to_owned(),
            payload: b"sunny".to_vec(),
        }),
        ToBroker::Terminate,
    ];
    let replies = vec![
        CallbackReply::Definition(definition()),
        CallbackReply::ReceiverPage {
            first_unread: 3,
            messages: vec![message(3, b"one"), message(4, b"two")],
            grants: vec![grant()],
        },
        CallbackReply::Acknowledged { first_unread: 5 },
        CallbackReply::MissingGrants {
            key_epoch: 0,
            receivers: vec![0, 3],
        },
        CallbackReply::GrantsSaved,
        CallbackReply::Reserved(prepare_message_header(fields(6), b"rain")),
        CallbackReply::Committed { sequence: 6 },
        CallbackReply::Abandoned { abandoned: true },
    ];
    for (index, reply) in replies.into_iter().enumerate() {
        frames.push(ToBroker::CallbackResult {
            callback_id: index as u32 + 1,
            outcome: CallbackOutcome::Ok(reply),
        });
    }
    frames.push(ToBroker::CallbackResult {
        callback_id: 99,
        outcome: CallbackOutcome::Refused {
            op: CallbackOp::ReserveAppend,
            refusal: Refusal::PendingAppend,
        },
    });
    frames
}

fn every_frame_from_broker() -> Vec<FromBroker> {
    let digest = definition_digest(&definition());
    let calls = vec![
        Callback::LoadDefinition {
            channel_id: channel(),
        },
        Callback::ReadReceiverPage {
            channel_id: channel(),
            definition_digest: digest,
            limit: 64,
        },
        Callback::Acknowledge {
            channel_id: channel(),
            definition_digest: digest,
            sequence: 4,
        },
        Callback::LoadMissingGrants {
            channel_id: channel(),
            definition_digest: digest,
        },
        Callback::SaveGrants {
            channel_id: channel(),
            definition_digest: digest,
            grants: vec![grant()],
        },
        Callback::ReserveAppend {
            channel_id: channel(),
            definition_digest: digest,
            content_type: "text/plain".to_owned(),
            plaintext_hash: [9; 32],
        },
        Callback::CommitAppend {
            channel_id: channel(),
            definition_digest: digest,
            message: message(6, b"rain"),
        },
        Callback::AbandonAppend {
            channel_id: channel(),
            sequence: 6,
        },
    ];
    let mut frames = vec![
        FromBroker::Ready {
            public_keys: vec![PublicKey {
                channel_id: channel(),
                kind: KeyKind::OriginatorSigningSeed,
                public_key: signing_key().public_key(),
            }],
        },
        FromBroker::Response(DataPlaneResponse::Published {
            id: id(),
            message_id: uuid_v7(46),
            sequence: 6,
            timestamp_ns: 5,
        }),
    ];
    for (index, call) in calls.into_iter().enumerate() {
        frames.push(FromBroker::Callback {
            callback_id: index as u32 + 1,
            request_id: id(),
            call,
        });
    }
    frames
}

#[test]
fn every_frame_round_trips_in_both_directions() {
    for frame in every_frame_to_broker() {
        let bytes = encode_to_broker(&frame).unwrap();
        assert_eq!(decode_to_broker(&bytes).unwrap(), frame);
    }
    for frame in every_frame_from_broker() {
        let bytes = encode_from_broker(&frame).unwrap();
        assert_eq!(decode_from_broker(&bytes).unwrap(), frame);
    }
}

#[test]
fn every_truncation_and_any_trailing_byte_is_refused() {
    for frame in every_frame_to_broker() {
        let bytes = encode_to_broker(&frame).unwrap();
        for cut in 0..bytes.len() {
            assert!(
                decode_to_broker(&bytes[..cut]).is_err(),
                "{frame:?} cut at {cut}"
            );
        }
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(
            decode_to_broker(&trailing).is_err(),
            "{frame:?} with a trailing byte"
        );
    }
    for frame in every_frame_from_broker() {
        let bytes = encode_from_broker(&frame).unwrap();
        for cut in 0..bytes.len() {
            assert!(
                decode_from_broker(&bytes[..cut]).is_err(),
                "{frame:?} cut at {cut}"
            );
        }
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(
            decode_from_broker(&trailing).is_err(),
            "{frame:?} with a trailing byte"
        );
    }
}

#[test]
fn a_frame_sent_the_wrong_way_or_with_a_wrong_header_is_refused() {
    let to = encode_to_broker(&ToBroker::Terminate).unwrap();
    let from = encode_from_broker(&every_frame_from_broker()[0]).unwrap();
    assert_eq!(decode_from_broker(&to), Err(ProtocolError::UnknownKind));
    assert_eq!(decode_to_broker(&from), Err(ProtocolError::UnknownKind));

    let mut magic = to.clone();
    magic[0] = b'X';
    assert_eq!(decode_to_broker(&magic), Err(ProtocolError::Header));
    let mut version = to.clone();
    version[4] = 2;
    assert_eq!(decode_to_broker(&version), Err(ProtocolError::Header));
    let mut kind = to;
    kind[5] = 0x7f;
    assert_eq!(decode_to_broker(&kind), Err(ProtocolError::UnknownKind));
}

#[test]
fn bounds_are_enforced_on_the_way_out_and_on_the_way_in() {
    let digest = definition_digest(&definition());
    // A page limit of 0 or 65.
    for limit in [0, 65] {
        let frame = FromBroker::Callback {
            callback_id: 1,
            request_id: id(),
            call: Callback::ReadReceiverPage {
                channel_id: channel(),
                definition_digest: digest,
                limit,
            },
        };
        assert!(encode_from_broker(&frame).is_err(), "limit {limit}");
    }
    // An empty or overlong content type.
    for content_type in [String::new(), "a".repeat(1025)] {
        let frame = FromBroker::Callback {
            callback_id: 1,
            request_id: id(),
            call: Callback::ReserveAppend {
                channel_id: channel(),
                definition_digest: digest,
                content_type,
                plaintext_hash: [0; 32],
            },
        };
        assert!(encode_from_broker(&frame).is_err());
    }
    // Too many key slots.
    let slots = vec![
        KeySlot {
            channel_id: channel(),
            kind: KeyKind::ChannelMasterKey,
        };
        MAX_KEY_SLOTS + 1
    ];
    assert!(encode_to_broker(&ToBroker::Bootstrap {
        binding: binding(),
        slots,
    })
    .is_err());
    // A master key has no public half to report.
    assert!(encode_from_broker(&FromBroker::Ready {
        public_keys: vec![PublicKey {
            channel_id: channel(),
            kind: KeyKind::ChannelMasterKey,
            public_key: [0; 32],
        }],
    })
    .is_err());
    // More than a page of messages.
    let too_many = ToBroker::CallbackResult {
        callback_id: 1,
        outcome: CallbackOutcome::Ok(CallbackReply::ReceiverPage {
            first_unread: 0,
            messages: (0..65).map(|sequence| message(sequence, b"x")).collect(),
            grants: Vec::new(),
        }),
    };
    assert!(encode_to_broker(&too_many).is_err());
    // More than a page of bytes: two messages of 512 KiB.
    let big = vec![0u8; 512 * 1024];
    let too_big = ToBroker::CallbackResult {
        callback_id: 1,
        outcome: CallbackOutcome::Ok(CallbackReply::ReceiverPage {
            first_unread: 0,
            messages: vec![message(0, &big), message(1, &big)],
            grants: Vec::new(),
        }),
    };
    assert!(encode_to_broker(&too_big).is_err());
}

#[test]
fn hand_built_hostile_counts_are_refused_before_allocation() {
    // A Bootstrap claiming 65535 slots in a tiny body.
    let mut body = encode_to_broker(&ToBroker::Bootstrap {
        binding: binding(),
        slots: Vec::new(),
    })
    .unwrap();
    let count_at = body.len() - 2;
    body[count_at..].copy_from_slice(&u16::MAX.to_be_bytes());
    assert_eq!(decode_to_broker(&body), Err(ProtocolError::Malformed));

    // A Request claiming a 4 GiB nested record.
    let mut body = b"D18K\x01\x02".to_vec();
    body.extend_from_slice(&u32::MAX.to_be_bytes());
    assert_eq!(decode_to_broker(&body), Err(ProtocolError::Malformed));

    // A key kind that does not exist.
    let mut body = encode_to_broker(&every_frame_to_broker()[0]).unwrap();
    let last = body.len() - 1;
    body[last] = 9;
    assert_eq!(decode_to_broker(&body), Err(ProtocolError::Malformed));
}

#[test]
fn framing_refuses_empty_oversized_and_truncated_frames() {
    let mut stream = Vec::new();
    write_frame(&mut stream, b"hello").unwrap();
    assert_eq!(&stream[..4], &5u32.to_be_bytes());
    assert_eq!(read_frame(&mut stream.as_slice()).unwrap(), b"hello");

    assert_eq!(
        write_frame(&mut Vec::new(), b""),
        Err(ProtocolError::FrameLength)
    );
    assert_eq!(
        write_frame(&mut Vec::new(), &vec![0; MAX_FRAME_BYTES + 1]),
        Err(ProtocolError::FrameLength)
    );
    // A zero or oversized length on the wire is refused before reading on.
    for length in [0u32, MAX_FRAME_BYTES as u32 + 1, u32::MAX] {
        let wire = length.to_be_bytes();
        assert_eq!(
            read_frame(&mut wire.as_slice()),
            Err(ProtocolError::FrameLength)
        );
    }
    // A body shorter than its length, or a cut length.
    assert_eq!(read_frame(&mut &stream[..7]), Err(ProtocolError::Io));
    assert_eq!(read_frame(&mut &stream[..2]), Err(ProtocolError::Io));
}

#[test]
fn debug_output_never_carries_a_payload() {
    let call = Callback::CommitAppend {
        channel_id: channel(),
        definition_digest: [0; 32],
        message: message(0, b"the secret plan"),
    };
    let reply = CallbackReply::ReceiverPage {
        first_unread: 0,
        messages: vec![message(0, b"the secret plan")],
        grants: vec![grant()],
    };
    for text in [format!("{call:?}"), format!("{reply:?}")] {
        assert!(!text.contains("ciphertext"), "{text}");
        assert!(text.len() < 200, "{text}");
    }
}

#[test]
fn the_definition_digest_tracks_the_exact_definition() {
    let first = definition_digest(&definition());
    assert_eq!(first, definition_digest(&definition()));
    let other = ChannelDefinition::new(
        channel(),
        definition().originator().clone(),
        definition().receivers().to_vec(),
        1_725_000_000_000_000_001,
        KeyEpoch(0),
    )
    .unwrap();
    assert_ne!(first, definition_digest(&other));
}
