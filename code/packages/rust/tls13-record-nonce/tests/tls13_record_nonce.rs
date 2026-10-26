use tls13_record_nonce::{Error, NoncePolicy, RecordNonceSequence, MAX_IV_LENGTH, MIN_IV_LENGTH};

#[test]
fn policy_accepts_closed_iv_bounds_and_nonzero_limits() {
    let minimum = NoncePolicy::new(MIN_IV_LENGTH, 1).unwrap();
    assert_eq!(minimum.iv_length(), MIN_IV_LENGTH);
    assert_eq!(minimum.record_limit(), 1);

    let maximum = NoncePolicy::new(MAX_IV_LENGTH, u64::MAX).unwrap();
    assert_eq!(maximum.iv_length(), MAX_IV_LENGTH);
    assert_eq!(maximum.record_limit(), u64::MAX);
}

#[test]
fn policy_rejects_lengths_outside_the_work_bound() {
    assert_eq!(NoncePolicy::new(7, 1), Err(Error::InvalidIvLength));
    assert_eq!(
        NoncePolicy::new(MAX_IV_LENGTH + 1, 1),
        Err(Error::InvalidIvLength)
    );
    assert_eq!(
        NoncePolicy::new(MIN_IV_LENGTH, 0),
        Err(Error::InvalidRecordLimit)
    );
}

#[test]
fn first_nonce_is_the_static_iv_and_second_xors_one() {
    let policy = NoncePolicy::new(12, 2).unwrap();
    let mut state = RecordNonceSequence::new(policy);
    let static_iv = [0xa5; 12];
    let mut output = [0u8; 12];

    let first = state.reserve_next_nonce(&static_iv, &mut output).unwrap();
    assert_eq!(first.sequence_number(), 0);
    assert_eq!(output, static_iv);
    assert_eq!(state.records_used(), 1);
    assert_eq!(state.remaining_records(), 1);

    let second = state.reserve_next_nonce(&static_iv, &mut output).unwrap();
    assert_eq!(second.sequence_number(), 1);
    let mut expected = [0xa5; 12];
    expected[11] = 0xa4;
    assert_eq!(output, expected);
    assert_eq!(state.records_used(), 2);
    assert_eq!(state.remaining_records(), 0);
}

#[test]
fn eight_byte_iv_uses_the_entire_network_order_sequence() {
    let policy = NoncePolicy::new(8, 2).unwrap();
    let mut state = RecordNonceSequence::new(policy);
    let static_iv = [0xff; 8];
    let mut output = [0u8; 8];

    state.reserve_next_nonce(&static_iv, &mut output).unwrap();
    state.reserve_next_nonce(&static_iv, &mut output).unwrap();

    assert_eq!(output, [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xfe]);
}

#[test]
fn longer_iv_preserves_the_left_padding_region() {
    let policy = NoncePolicy::new(16, 2).unwrap();
    let mut state = RecordNonceSequence::new(policy);
    let static_iv = [0x3c; 16];
    let mut output = [0u8; 16];

    state.reserve_next_nonce(&static_iv, &mut output).unwrap();
    state.reserve_next_nonce(&static_iv, &mut output).unwrap();

    assert_eq!(&output[..15], &[0x3c; 15]);
    assert_eq!(output[15], 0x3d);
}

#[test]
fn exhaustion_leaves_output_and_state_unchanged() {
    let policy = NoncePolicy::new(12, 1).unwrap();
    let mut state = RecordNonceSequence::new(policy);
    let static_iv = [0x11; 12];
    let mut output = [0u8; 12];
    state.reserve_next_nonce(&static_iv, &mut output).unwrap();
    output.fill(0x77);

    assert_eq!(
        state.reserve_next_nonce(&static_iv, &mut output),
        Err(Error::SequenceExhausted)
    );
    assert_eq!(output, [0x77; 12]);
    assert_eq!(state.records_used(), 1);
}

#[test]
fn length_failures_leave_output_and_state_unchanged() {
    let policy = NoncePolicy::new(12, 1).unwrap();
    let mut state = RecordNonceSequence::new(policy);
    let mut output = [0x77; 12];

    assert_eq!(
        state.reserve_next_nonce(&[0x11; 11], &mut output),
        Err(Error::StaticIvLengthMismatch)
    );
    assert_eq!(output, [0x77; 12]);
    assert_eq!(state.records_used(), 0);

    let mut short_output = [0x66; 11];
    assert_eq!(
        state.reserve_next_nonce(&[0x11; 12], &mut short_output),
        Err(Error::OutputLengthMismatch)
    );
    assert_eq!(short_output, [0x66; 11]);
    assert_eq!(state.records_used(), 0);
}

#[test]
fn public_inputs_are_validated_before_the_exhaustion_shortcut() {
    let policy = NoncePolicy::new(12, 1).unwrap();
    let mut state = RecordNonceSequence::new(policy);
    let static_iv = [0x11; 12];
    let mut output = [0u8; 12];
    state.reserve_next_nonce(&static_iv, &mut output).unwrap();

    let mut unchanged = [0x44; 12];
    assert_eq!(
        state.reserve_next_nonce(&[0x11; 11], &mut unchanged),
        Err(Error::StaticIvLengthMismatch)
    );
    assert_eq!(unchanged, [0x44; 12]);
    assert_eq!(state.records_used(), 1);
}

#[test]
fn separate_states_begin_at_zero() {
    let policy = NoncePolicy::new(12, 1).unwrap();
    let mut reading = RecordNonceSequence::new(policy);
    let mut writing = RecordNonceSequence::new(policy);
    let static_iv = [0x22; 12];
    let mut read_nonce = [0u8; 12];
    let mut write_nonce = [0u8; 12];

    let read = reading
        .reserve_next_nonce(&static_iv, &mut read_nonce)
        .unwrap();
    let write = writing
        .reserve_next_nonce(&static_iv, &mut write_nonce)
        .unwrap();

    assert_eq!(read.sequence_number(), 0);
    assert_eq!(write.sequence_number(), 0);
    assert_eq!(read_nonce, write_nonce);
}

#[test]
fn errors_are_stable_and_redacted() {
    let cases = [
        (
            Error::InvalidIvLength,
            "invalid TLS record IV length",
            "InvalidIvLength",
        ),
        (
            Error::InvalidRecordLimit,
            "invalid TLS record limit",
            "InvalidRecordLimit",
        ),
        (
            Error::StaticIvLengthMismatch,
            "TLS static IV length mismatch",
            "StaticIvLengthMismatch",
        ),
        (
            Error::OutputLengthMismatch,
            "TLS nonce output length mismatch",
            "OutputLengthMismatch",
        ),
        (
            Error::SequenceExhausted,
            "TLS record sequence exhausted",
            "SequenceExhausted",
        ),
    ];

    for (error, display, debug) in cases {
        assert_eq!(error.to_string(), display);
        assert_eq!(format!("{error:?}"), debug);
    }
}
