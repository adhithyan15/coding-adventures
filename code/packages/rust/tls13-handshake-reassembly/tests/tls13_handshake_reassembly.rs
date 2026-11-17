use tls13_handshake::{
    encode_tls13_handshake, HandshakeErrorKind, Tls13HandshakePolicy, Tls13HandshakeType,
};
use tls13_handshake_reassembly::{
    ReassemblyErrorKind, ReassemblyProgress, Tls13HandshakeReassembler,
};

fn policy(maximum_body_length: usize) -> Tls13HandshakePolicy {
    Tls13HandshakePolicy::new(
        &[
            Tls13HandshakeType::EncryptedExtensions,
            Tls13HandshakeType::Certificate,
            Tls13HandshakeType::Finished,
        ],
        maximum_body_length,
    )
    .expect("test policy should be valid")
}

fn encoded(handshake_type: Tls13HandshakeType, body: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0_u8; body.len() + 4];
    let used = encode_tls13_handshake(&policy(body.len()), handshake_type, body, &mut bytes)
        .expect("test message should encode");
    bytes.truncate(used);
    bytes
}

#[test]
fn every_header_split_retains_an_incomplete_prefix() {
    let wire = encoded(Tls13HandshakeType::Certificate, &[1, 2, 3, 4]);

    for split in 1..4 {
        let mut storage = [0_u8; 16];
        let mut reassembler = Tls13HandshakeReassembler::new(policy(12), &mut storage).unwrap();
        assert_eq!(
            reassembler
                .push_authenticated_record_content(&wire[..split])
                .unwrap(),
            ReassemblyProgress::Incomplete { consumed: split },
        );
        let progress = reassembler
            .push_authenticated_record_content(&wire[split..])
            .unwrap();
        match progress {
            ReassemblyProgress::Complete { consumed, message } => {
                assert_eq!(consumed, wire.len() - split);
                assert_eq!(message.encoded(), wire);
            }
            ReassemblyProgress::Incomplete { .. } => panic!("message should be complete"),
        }
    }
}

#[test]
fn body_can_span_multiple_authenticated_fragments() {
    let wire = encoded(Tls13HandshakeType::Certificate, &[10, 20, 30, 40, 50]);
    let mut storage = [0_u8; 16];
    let mut reassembler = Tls13HandshakeReassembler::new(policy(12), &mut storage).unwrap();

    for fragment in [&wire[..5], &wire[5..7]] {
        assert_eq!(
            reassembler
                .push_authenticated_record_content(fragment)
                .unwrap()
                .consumed(),
            fragment.len()
        );
    }
    let progress = reassembler
        .push_authenticated_record_content(&wire[7..])
        .unwrap();
    match progress {
        ReassemblyProgress::Complete { consumed, message } => {
            assert_eq!(consumed, 2);
            assert_eq!(message.body(), &[10, 20, 30, 40, 50]);
        }
        ReassemblyProgress::Incomplete { .. } => panic!("message should be complete"),
    }
}

#[test]
fn coalesced_tail_remains_with_caller_for_next_message() {
    let first = encoded(Tls13HandshakeType::EncryptedExtensions, &[1, 2]);
    let second = encoded(Tls13HandshakeType::Finished, &[3, 4, 5]);
    let mut coalesced = first.clone();
    coalesced.extend_from_slice(&second);
    let mut storage = [0_u8; 16];
    let mut reassembler = Tls13HandshakeReassembler::new(policy(12), &mut storage).unwrap();

    let consumed = match reassembler
        .push_authenticated_record_content(&coalesced)
        .unwrap()
    {
        ReassemblyProgress::Complete { consumed, message } => {
            assert_eq!(message.encoded(), first);
            consumed
        }
        ReassemblyProgress::Incomplete { .. } => panic!("first message should be complete"),
    };
    assert_eq!(consumed, first.len());

    match reassembler
        .push_authenticated_record_content(&coalesced[consumed..])
        .unwrap()
    {
        ReassemblyProgress::Complete { consumed, message } => {
            assert_eq!(consumed, second.len());
            assert_eq!(message.encoded(), second);
        }
        ReassemblyProgress::Incomplete { .. } => panic!("second message should be complete"),
    }
}

#[test]
fn progress_debug_is_redacted_for_incomplete_and_complete_messages() {
    let wire = encoded(Tls13HandshakeType::Certificate, &[0xa5, 0x5a]);
    let mut storage = [0_u8; 16];
    let mut reassembler = Tls13HandshakeReassembler::new(policy(12), &mut storage).unwrap();

    let incomplete = reassembler
        .push_authenticated_record_content(&wire[..2])
        .unwrap();
    assert_eq!(format!("{incomplete:?}"), "ReassemblyProgress::Incomplete");

    let complete = reassembler
        .push_authenticated_record_content(&wire[2..])
        .unwrap();
    assert_eq!(format!("{complete:?}"), "ReassemblyProgress::Complete");
}

#[test]
fn caller_can_replay_a_partial_coalesced_tail_in_order() {
    let first = encoded(Tls13HandshakeType::EncryptedExtensions, &[]);
    let second = encoded(Tls13HandshakeType::Finished, &[9, 8]);
    let mut record_one = first.clone();
    record_one.extend_from_slice(&second[..2]);
    let mut storage = [0_u8; 16];
    let mut reassembler = Tls13HandshakeReassembler::new(policy(12), &mut storage).unwrap();

    let first_consumed = match reassembler
        .push_authenticated_record_content(&record_one)
        .unwrap()
    {
        ReassemblyProgress::Complete { consumed, .. } => consumed,
        ReassemblyProgress::Incomplete { .. } => panic!("first message should be complete"),
    };
    assert_eq!(first_consumed, first.len());
    assert_eq!(
        reassembler
            .push_authenticated_record_content(&record_one[first_consumed..])
            .unwrap(),
        ReassemblyProgress::Incomplete { consumed: 2 },
    );
    match reassembler
        .push_authenticated_record_content(&second[2..])
        .unwrap()
    {
        ReassemblyProgress::Complete { consumed, message } => {
            assert_eq!(consumed, second.len() - 2);
            assert_eq!(message.encoded(), second);
        }
        ReassemblyProgress::Incomplete { .. } => panic!("second message should be complete"),
    }
}

#[test]
fn exact_storage_capacity_and_empty_body_are_supported() {
    let wire = encoded(Tls13HandshakeType::Finished, &[1, 2, 3, 4]);
    let mut storage = [0_u8; 8];
    let mut reassembler = Tls13HandshakeReassembler::new(policy(4), &mut storage).unwrap();
    assert!(reassembler
        .push_authenticated_record_content(&wire)
        .unwrap()
        .is_complete());

    let empty = encoded(Tls13HandshakeType::EncryptedExtensions, &[]);
    let mut header_storage = [0_u8; 4];
    let mut empty_reassembler =
        Tls13HandshakeReassembler::new(policy(4), &mut header_storage).unwrap();
    assert!(empty_reassembler
        .push_authenticated_record_content(&empty)
        .unwrap()
        .is_complete());
}

#[test]
fn constructor_and_declared_capacity_fail_with_payload_free_categories() {
    let mut short = [0_u8; 3];
    let error = match Tls13HandshakeReassembler::new(policy(8), &mut short) {
        Ok(_) => panic!("short storage must fail"),
        Err(error) => error,
    };
    assert_eq!(error.kind(), ReassemblyErrorKind::StorageTooSmall);

    let wire = encoded(Tls13HandshakeType::Certificate, &[0xa5; 5]);
    let mut storage = [0_u8; 8];
    let mut reassembler = Tls13HandshakeReassembler::new(policy(8), &mut storage).unwrap();
    let error = reassembler
        .push_authenticated_record_content(&wire)
        .unwrap_err();
    assert_eq!(error.kind(), ReassemblyErrorKind::MessageTooLargeForStorage);
    let diagnostic = format!("{error:?} {error}");
    assert!(!diagnostic.contains("165"));
    assert!(!diagnostic.contains("a5"));
}

#[test]
fn header_policy_failures_clear_state_and_retain_only_a_category() {
    let mut storage = [0_u8; 16];
    let restricted = Tls13HandshakePolicy::new(&[Tls13HandshakeType::Finished], 4).unwrap();
    let mut reassembler = Tls13HandshakeReassembler::new(restricted, &mut storage).unwrap();

    let disallowed = encoded(Tls13HandshakeType::Certificate, &[1]);
    let error = reassembler
        .push_authenticated_record_content(&disallowed)
        .unwrap_err();
    assert_eq!(
        error.kind(),
        ReassemblyErrorKind::HandshakeFraming(HandshakeErrorKind::DisallowedHandshakeType)
    );

    let allowed = encoded(Tls13HandshakeType::Finished, &[2]);
    assert!(reassembler
        .push_authenticated_record_content(&allowed)
        .unwrap()
        .is_complete());
}

#[test]
fn over_policy_length_fails_when_header_becomes_complete() {
    let mut storage = [0_u8; 16];
    let mut reassembler = Tls13HandshakeReassembler::new(policy(2), &mut storage).unwrap();
    let header = [Tls13HandshakeType::Certificate.wire_value(), 0, 0, 3];
    let error = reassembler
        .push_authenticated_record_content(&header)
        .unwrap_err();
    assert_eq!(
        error.kind(),
        ReassemblyErrorKind::HandshakeFraming(HandshakeErrorKind::BodyTooLong)
    );
}

#[test]
fn explicit_reset_clears_only_the_initialized_storage_prefix() {
    let mut storage = [0xcc_u8; 12];
    {
        let mut reassembler = Tls13HandshakeReassembler::new(policy(8), &mut storage).unwrap();
        assert_eq!(
            reassembler
                .push_authenticated_record_content(&[11, 0, 0])
                .unwrap(),
            ReassemblyProgress::Incomplete { consumed: 3 },
        );
        reassembler.reset();
    }
    assert_eq!(&storage[..3], &[0, 0, 0]);
    assert_eq!(&storage[3..], &[0xcc; 9]);
}

#[test]
fn drop_clears_a_completed_message_from_caller_storage() {
    let wire = encoded(Tls13HandshakeType::Finished, &[1, 2, 3]);
    let mut storage = [0xcc_u8; 12];
    {
        let mut reassembler = Tls13HandshakeReassembler::new(policy(8), &mut storage).unwrap();
        assert!(reassembler
            .push_authenticated_record_content(&wire)
            .unwrap()
            .is_complete());
    }
    assert!(storage[..wire.len()].iter().all(|&byte| byte == 0));
    assert_eq!(&storage[wire.len()..], &[0xcc; 5]);
}

#[test]
fn empty_push_is_a_bounded_no_op() {
    let mut storage = [0_u8; 8];
    let mut reassembler = Tls13HandshakeReassembler::new(policy(4), &mut storage).unwrap();
    assert_eq!(
        reassembler.push_authenticated_record_content(&[]).unwrap(),
        ReassemblyProgress::Incomplete { consumed: 0 },
    );
}
