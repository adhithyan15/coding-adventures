use tls13_key_share::{
    decode_client_key_shares, decode_server_key_share, encode_client_key_shares,
    validate_client_key_share, KeyShareErrorKind, KeyShareLimits, KeyShareLimitsErrorKind,
    MAX_KEY_SHARE_ENTRIES,
};
use tls13_named_group_policy::{NamedGroup, NamedGroupPolicy, PolicyError};

const X25519_KEY: [u8; 32] = [0x42; 32];
const P256_KEY: [u8; 65] = [0x24; 65];

fn policy() -> NamedGroupPolicy {
    NamedGroupPolicy::new(&[NamedGroup::X25519, NamedGroup::Secp256r1]).unwrap()
}

fn encoded_client_shares() -> [u8; 107] {
    let policy = policy();
    let x25519 = validate_client_key_share(&policy, 0x001d, &X25519_KEY).unwrap();
    let p256 = validate_client_key_share(&policy, 0x0017, &P256_KEY).unwrap();
    let mut encoded = [0xa5; 107];
    encode_client_key_shares(&[x25519, p256], &mut encoded).unwrap();
    encoded
}

#[test]
fn validates_encodes_and_decodes_client_shares_in_order() {
    let policy = policy();
    let x25519 = validate_client_key_share(&policy, 0x001d, &X25519_KEY).unwrap();
    let p256 = validate_client_key_share(&policy, 0x0017, &P256_KEY).unwrap();
    assert_eq!(format!("{x25519:?}"), "ClientKeyShareEntry");

    let mut output = [0xa5; 111];
    {
        let encoded = encode_client_key_shares(&[x25519, p256], &mut output).unwrap();
        assert_eq!(encoded.as_bytes().len(), 107);
        assert_eq!(
            &encoded.as_bytes()[..6],
            &[0x00, 0x69, 0x00, 0x1d, 0x00, 0x20]
        );
        assert_eq!(&encoded.as_bytes()[6..38], &X25519_KEY);
        assert_eq!(&encoded.as_bytes()[38..42], &[0x00, 0x17, 0x00, 0x41]);
        assert_eq!(&encoded.as_bytes()[42..], &P256_KEY);
        assert_eq!(format!("{encoded:?}"), "EncodedClientKeyShares");
    }
    assert_eq!(&output[107..], &[0xa5; 4]);

    let decoded = decode_client_key_shares(
        &output[..107],
        &policy,
        KeyShareLimits::new(MAX_KEY_SHARE_ENTRIES).unwrap(),
    )
    .unwrap();
    assert_eq!(decoded.len(), 2);
    assert!(!decoded.is_empty());
    let entries: Vec<_> = decoded.entries().collect();
    assert_eq!(
        entries[0].selected_group().parameters().group(),
        NamedGroup::X25519
    );
    assert_eq!(entries[0].key_exchange(), X25519_KEY);
    assert_eq!(
        entries[1].selected_group().parameters().group(),
        NamedGroup::Secp256r1
    );
    assert_eq!(entries[1].key_exchange(), P256_KEY);
    assert_eq!(
        format!("{decoded:?}"),
        "DecodedClientKeyShares { entry_count: 2 }"
    );
}

#[test]
fn server_selection_is_bound_to_the_exact_client_offer() {
    let policy = policy();
    let offered_bytes = encoded_client_shares();
    let offered =
        decode_client_key_shares(&offered_bytes, &policy, KeyShareLimits::new(2).unwrap()).unwrap();
    let mut server = [0u8; 36];
    server[..4].copy_from_slice(&[0x00, 0x1d, 0x00, 0x20]);
    server[4..].copy_from_slice(&[0x7e; 32]);

    let selected = decode_server_key_share(&server, &policy, &offered).unwrap();
    assert_eq!(
        selected.selected_group().parameters().group(),
        NamedGroup::X25519
    );
    assert_eq!(selected.key_exchange(), &[0x7e; 32]);
    assert!(core::ptr::eq(selected.offered_client_shares(), &offered));
    assert_eq!(format!("{selected:?}"), "DecodedServerKeyShare");
}

#[test]
fn policy_rejects_unknown_disallowed_and_wrong_length_shares() {
    let x25519_only = NamedGroupPolicy::new(&[NamedGroup::X25519]).unwrap();
    let unknown = validate_client_key_share(&x25519_only, 0x0a0a, &X25519_KEY).unwrap_err();
    assert_eq!(
        unknown.kind(),
        KeyShareErrorKind::NamedGroupPolicy(PolicyError::UnknownNamedGroup)
    );
    let disallowed = validate_client_key_share(&x25519_only, 0x0017, &P256_KEY).unwrap_err();
    assert_eq!(
        disallowed.kind(),
        KeyShareErrorKind::NamedGroupPolicy(PolicyError::DisallowedNamedGroup)
    );
    let short = validate_client_key_share(&x25519_only, 0x001d, &X25519_KEY[..31]).unwrap_err();
    assert_eq!(
        short.kind(),
        KeyShareErrorKind::NamedGroupPolicy(PolicyError::InvalidKeyExchangeLength)
    );
}

#[test]
fn client_encoder_rejects_empty_duplicate_and_small_output_atomically() {
    let policy = policy();
    let mut output = [0xa5; 106];
    assert_eq!(
        encode_client_key_shares(&[], &mut output)
            .unwrap_err()
            .kind(),
        KeyShareErrorKind::EmptyClientShareList
    );
    let first = validate_client_key_share(&policy, 0x001d, &X25519_KEY).unwrap();
    let second = validate_client_key_share(&policy, 0x001d, &X25519_KEY).unwrap();
    assert_eq!(
        encode_client_key_shares(&[first, second], &mut output)
            .unwrap_err()
            .kind(),
        KeyShareErrorKind::DuplicateNamedGroup
    );
    let x25519 = validate_client_key_share(&policy, 0x001d, &X25519_KEY).unwrap();
    let p256 = validate_client_key_share(&policy, 0x0017, &P256_KEY).unwrap();
    assert_eq!(
        encode_client_key_shares(&[x25519, p256], &mut output)
            .unwrap_err()
            .kind(),
        KeyShareErrorKind::OutputTooSmall
    );
    assert_eq!(output, [0xa5; 106]);
}

#[test]
fn client_decoder_rejects_vector_and_entry_framing_failures() {
    let policy = policy();
    let limits = KeyShareLimits::new(2).unwrap();
    let cases: &[(&[u8], KeyShareErrorKind)] = &[
        (&[], KeyShareErrorKind::TruncatedClientVectorLength),
        (&[0x00], KeyShareErrorKind::TruncatedClientVectorLength),
        (&[0x00, 0x01], KeyShareErrorKind::ClientVectorLengthMismatch),
        (&[0x00, 0x00], KeyShareErrorKind::EmptyClientShareList),
        (&[0x00, 0x01, 0x00], KeyShareErrorKind::TruncatedEntryHeader),
        (
            &[0x00, 0x04, 0x00, 0x1d, 0x00, 0x20],
            KeyShareErrorKind::KeyExchangeLengthMismatch,
        ),
    ];
    for (input, expected) in cases {
        assert_eq!(
            decode_client_key_shares(input, &policy, limits)
                .unwrap_err()
                .kind(),
            *expected
        );
    }
}

#[test]
fn client_decoder_enforces_entry_ceiling_and_duplicate_groups() {
    let policy = policy();
    let encoded = encoded_client_shares();
    assert_eq!(
        decode_client_key_shares(&encoded, &policy, KeyShareLimits::new(1).unwrap())
            .unwrap_err()
            .kind(),
        KeyShareErrorKind::TooManyClientShares
    );

    let x25519 = validate_client_key_share(&policy, 0x001d, &X25519_KEY).unwrap();
    let mut one = [0u8; 38];
    encode_client_key_shares(&[x25519], &mut one).unwrap();
    let mut duplicate = [0u8; 74];
    duplicate[..2].copy_from_slice(&72u16.to_be_bytes());
    duplicate[2..38].copy_from_slice(&one[2..]);
    duplicate[38..].copy_from_slice(&one[2..]);
    assert_eq!(
        decode_client_key_shares(&duplicate, &policy, KeyShareLimits::new(2).unwrap())
            .unwrap_err()
            .kind(),
        KeyShareErrorKind::DuplicateNamedGroup
    );
}

#[test]
fn server_decoder_rejects_unoffered_and_malformed_entries() {
    let policy = policy();
    let x25519 = validate_client_key_share(&policy, 0x001d, &X25519_KEY).unwrap();
    let mut client = [0u8; 38];
    encode_client_key_shares(&[x25519], &mut client).unwrap();
    let offered =
        decode_client_key_shares(&client, &policy, KeyShareLimits::new(2).unwrap()).unwrap();

    assert_eq!(
        decode_server_key_share(&[0x00, 0x1d, 0x00], &policy, &offered)
            .unwrap_err()
            .kind(),
        KeyShareErrorKind::TruncatedEntryHeader
    );
    let mut trailing = [0u8; 37];
    trailing[..4].copy_from_slice(&[0x00, 0x1d, 0x00, 0x20]);
    assert_eq!(
        decode_server_key_share(&trailing, &policy, &offered)
            .unwrap_err()
            .kind(),
        KeyShareErrorKind::KeyExchangeLengthMismatch
    );
    let mut p256 = [0u8; 69];
    p256[..4].copy_from_slice(&[0x00, 0x17, 0x00, 0x41]);
    p256[4..].copy_from_slice(&P256_KEY);
    assert_eq!(
        decode_server_key_share(&p256, &policy, &offered)
            .unwrap_err()
            .kind(),
        KeyShareErrorKind::ServerGroupNotOffered
    );
}

#[test]
fn limits_and_errors_are_redacted() {
    let zero = KeyShareLimits::new(0).unwrap_err();
    assert_eq!(zero.kind(), KeyShareLimitsErrorKind::ZeroMaximumEntries);
    let too_large = KeyShareLimits::new(MAX_KEY_SHARE_ENTRIES + 1).unwrap_err();
    assert_eq!(
        too_large.kind(),
        KeyShareLimitsErrorKind::MaximumEntriesTooLarge
    );
    assert_eq!(KeyShareLimits::new(2).unwrap().maximum_entries(), 2);

    let error = validate_client_key_share(
        &NamedGroupPolicy::new(&[NamedGroup::X25519]).unwrap(),
        0xbeef,
        &[0x7d; 32],
    )
    .unwrap_err();
    for rendered in [
        format!("{zero:?}"),
        zero.to_string(),
        format!("{too_large:?}"),
        too_large.to_string(),
        format!("{error:?}"),
        error.to_string(),
    ] {
        assert!(!rendered.contains("beef"));
        assert!(!rendered.contains("0x7d"));
        assert!(!rendered.contains("125"));
    }
}
