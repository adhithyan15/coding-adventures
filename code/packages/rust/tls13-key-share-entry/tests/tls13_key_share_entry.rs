use tls13_key_share_entry::{
    KeyShareEntry, KeyShareEntryError, KeyShareLimits, KEY_SHARE_ENTRY_HEADER_LENGTH,
    MAX_ENCODED_KEY_SHARE_ENTRY_LENGTH, MAX_KEY_EXCHANGE_LENGTH,
};

#[test]
fn encodes_and_decodes_one_exact_entry_prefix() {
    let limits = KeyShareLimits::new(32).unwrap();
    let key_exchange = [0x5a; 32];
    let entry = KeyShareEntry::new(0x001d, &key_exchange, limits).unwrap();

    assert_eq!(entry.group(), 0x001d);
    assert_eq!(entry.key_exchange(), &key_exchange);
    assert_eq!(entry.encoded_len(), 36);

    let mut output = [0xa5; 38];
    assert_eq!(entry.encode(&mut output), Ok(36));
    assert_eq!(&output[..4], &[0x00, 0x1d, 0x00, 0x20]);
    assert_eq!(&output[4..36], &key_exchange);
    assert_eq!(&output[36..], &[0xa5, 0xa5]);

    let (decoded, remainder) = KeyShareEntry::decode_prefix(&output, limits).unwrap();
    assert_eq!(decoded, entry);
    assert_eq!(remainder, &[0xa5, 0xa5]);
}

#[test]
fn preserves_unknown_and_reserved_group_identifiers() {
    let limits = KeyShareLimits::new(8).unwrap();
    let key_exchange = [1, 2, 3];
    let entry = KeyShareEntry::new(0x0a0a, &key_exchange, limits).unwrap();
    let mut encoded = [0_u8; 7];

    assert_eq!(entry.encode(&mut encoded), Ok(7));
    assert_eq!(encoded, [0x0a, 0x0a, 0x00, 0x03, 1, 2, 3]);
    let (decoded, remainder) = KeyShareEntry::decode_prefix(&encoded, limits).unwrap();
    assert_eq!(decoded.group(), 0x0a0a);
    assert_eq!(decoded.key_exchange(), &key_exchange);
    assert!(remainder.is_empty());
}

#[test]
fn validates_limits_and_payload_lengths() {
    assert_eq!(
        KeyShareLimits::new(0),
        Err(KeyShareEntryError::InvalidMaximumKeyExchangeLength)
    );
    assert_eq!(
        KeyShareLimits::new(MAX_KEY_EXCHANGE_LENGTH + 1),
        Err(KeyShareEntryError::InvalidMaximumKeyExchangeLength)
    );

    let limits = KeyShareLimits::new(3).unwrap();
    assert_eq!(limits.max_key_exchange_length(), 3);
    assert_eq!(
        KeyShareEntry::new(0x001d, &[], limits),
        Err(KeyShareEntryError::EmptyKeyExchange)
    );
    assert_eq!(
        KeyShareEntry::new(0x001d, &[1, 2, 3, 4], limits),
        Err(KeyShareEntryError::KeyExchangeTooLong)
    );
}

#[test]
fn encoding_capacity_failure_is_atomic() {
    let entry = KeyShareEntry::new(0x001d, &[1, 2, 3], KeyShareLimits::new(3).unwrap()).unwrap();
    let mut output = [0x6c; 6];

    assert_eq!(
        entry.encode(&mut output),
        Err(KeyShareEntryError::OutputTooSmall)
    );
    assert_eq!(output, [0x6c; 6]);
}

#[test]
fn decoding_rejects_malformed_and_over_policy_entries() {
    let limits = KeyShareLimits::new(3).unwrap();
    assert_eq!(
        KeyShareEntry::decode_prefix(&[0x00, 0x1d, 0x00], limits),
        Err(KeyShareEntryError::TruncatedHeader)
    );
    assert_eq!(
        KeyShareEntry::decode_prefix(&[0x00, 0x1d, 0x00, 0x00], limits),
        Err(KeyShareEntryError::EmptyKeyExchange)
    );
    assert_eq!(
        KeyShareEntry::decode_prefix(&[0x00, 0x1d, 0x00, 0x04], limits),
        Err(KeyShareEntryError::KeyExchangeTooLong)
    );
    assert_eq!(
        KeyShareEntry::decode_prefix(&[0x00, 0x1d, 0x00, 0x03, 1, 2], limits),
        Err(KeyShareEntryError::TruncatedKeyExchange)
    );
}

#[test]
fn accepts_the_wire_format_maximum() {
    let limits = KeyShareLimits::new(MAX_KEY_EXCHANGE_LENGTH).unwrap();
    let key_exchange = vec![0x3c; MAX_KEY_EXCHANGE_LENGTH];
    let entry = KeyShareEntry::new(0xffff, &key_exchange, limits).unwrap();
    assert_eq!(
        entry.encoded_len(),
        KEY_SHARE_ENTRY_HEADER_LENGTH + MAX_KEY_EXCHANGE_LENGTH
    );
    assert_eq!(entry.encoded_len(), MAX_ENCODED_KEY_SHARE_ENTRY_LENGTH);

    let mut output = vec![0_u8; MAX_ENCODED_KEY_SHARE_ENTRY_LENGTH];
    assert_eq!(entry.encode(&mut output), Ok(output.len()));
    assert_eq!(&output[..4], &[0xff, 0xff, 0xff, 0xff]);
    assert_eq!(&output[4..], &key_exchange);
}

#[test]
fn diagnostics_omit_group_and_key_bytes() {
    let entry = KeyShareEntry::new(
        0x0a0a,
        &[0xde, 0xad, 0xbe, 0xef],
        KeyShareLimits::new(4).unwrap(),
    )
    .unwrap();
    let debug = format!("{entry:?}");

    assert!(debug.contains("[OPAQUE GROUP ID]"));
    assert!(debug.contains("[OPAQUE PUBLIC KEY DATA]"));
    assert!(!debug.contains("2570"));
    assert!(!debug.contains("222"));
    assert!(!debug.contains("173"));

    let errors = [
        KeyShareEntryError::InvalidMaximumKeyExchangeLength,
        KeyShareEntryError::EmptyKeyExchange,
        KeyShareEntryError::KeyExchangeTooLong,
        KeyShareEntryError::OutputTooSmall,
        KeyShareEntryError::TruncatedHeader,
        KeyShareEntryError::TruncatedKeyExchange,
    ];
    for error in errors {
        assert!(!format!("{error:?}").contains("0x"));
        assert!(!error.to_string().contains("0x"));
    }
}
