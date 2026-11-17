use coding_adventures_hkdf::{FixedHkdfHash, HkdfPrk};
use tls13_aes128_gcm_record::{
    open_tls13_aes128_gcm_record, seal_tls13_aes128_gcm_record, RecordProtectionErrorKind,
    Tls13Plaintext,
};
use tls13_cipher_suite_policy::{CipherSuite, CipherSuitePolicy, CipherSuitePolicyEntry};
use tls13_inner_plaintext::Tls13InnerPlaintextPolicy;
use tls13_record::{Tls13CiphertextPolicy, TlsContentType};
use tls13_record_protection_state::RecordProtectionState;
use tls13_traffic_key_material::{derive_traffic_key_material, TrafficKeyMaterialStorage};

fn selected(suite: CipherSuite, limit: u64) -> tls13_cipher_suite_policy::SelectedCipherSuite {
    CipherSuitePolicy::new(&[CipherSuitePolicyEntry::new(suite, limit).unwrap()])
        .unwrap()
        .select(suite.wire_id())
        .unwrap()
}

fn policies() -> (Tls13CiphertextPolicy, Tls13InnerPlaintextPolicy) {
    (
        Tls13CiphertextPolicy::new(1024).unwrap(),
        Tls13InnerPlaintextPolicy::new(
            &[TlsContentType::Handshake, TlsContentType::ApplicationData],
            1008,
        )
        .unwrap(),
    )
}

fn state(
    suite: tls13_cipher_suite_policy::SelectedCipherSuite,
    hash: FixedHkdfHash,
    secret_byte: u8,
) -> RecordProtectionState<'static> {
    let secret_length = hash.hash_len();
    let secret_bytes = vec![secret_byte; secret_length];
    let secret = HkdfPrk::import_exact(hash, &secret_bytes).unwrap();
    let storage = Box::leak(Box::new(TrafficKeyMaterialStorage::new(suite).unwrap()));
    let material = derive_traffic_key_material(&secret, storage).unwrap();
    RecordProtectionState::new(material).unwrap()
}

#[test]
fn round_trip_authenticates_header_type_content_and_padding() {
    let suite = selected(CipherSuite::TlsAes128GcmSha256, 2);
    let mut write_state = state(suite, FixedHkdfHash::Sha256, 0x42);
    let mut read_state = state(suite, FixedHkdfHash::Sha256, 0x42);
    let (ciphertext_policy, inner_policy) = policies();
    let mut scratch = [0xa5; 128];
    let mut wire = [0xcc; 160];

    let used = seal_tls13_aes128_gcm_record(
        &mut write_state,
        &ciphertext_policy,
        &inner_policy,
        Tls13Plaintext::new(TlsContentType::Handshake, b"server flight", 7),
        &mut scratch,
        &mut wire,
    )
    .unwrap();

    assert_eq!(&wire[..3], &[23, 3, 3]);
    assert_eq!(u16::from_be_bytes([wire[3], wire[4]]) as usize, used - 5);
    assert_eq!(scratch[..21], [0; 21]);
    assert!(scratch[21..].iter().all(|byte| *byte == 0xa5));
    assert_eq!(write_state.records_used(), 1);

    let mut opened_storage = [0x5a; 128];
    {
        let opened = open_tls13_aes128_gcm_record(
            &mut read_state,
            &ciphertext_policy,
            &inner_policy,
            &wire[..used],
            &mut opened_storage,
        )
        .unwrap();
        assert_eq!(opened.content_type(), TlsContentType::Handshake);
        assert_eq!(opened.content(), b"server flight");
        assert_eq!(opened.padding_length(), 7);
        assert!(!format!("{opened:?}").contains("server flight"));
    }
    assert_eq!(opened_storage[..21], [0; 21]);
    assert!(opened_storage[21..].iter().all(|byte| *byte == 0x5a));
    assert_eq!(read_state.records_used(), 1);
}

#[test]
fn authentication_failure_consumes_nonce_without_releasing_plaintext() {
    let suite = selected(CipherSuite::TlsAes128GcmSha256, 2);
    let mut write_state = state(suite, FixedHkdfHash::Sha256, 0x24);
    let mut read_state = state(suite, FixedHkdfHash::Sha256, 0x24);
    let (ciphertext_policy, inner_policy) = policies();
    let mut scratch = [0; 64];
    let mut wire = [0; 96];
    let used = seal_tls13_aes128_gcm_record(
        &mut write_state,
        &ciphertext_policy,
        &inner_policy,
        Tls13Plaintext::new(TlsContentType::ApplicationData, b"token-shaped secret", 0),
        &mut scratch,
        &mut wire,
    )
    .unwrap();
    wire[used - 1] ^= 1;
    let mut output = [0xa5; 64];

    let error = open_tls13_aes128_gcm_record(
        &mut read_state,
        &ciphertext_policy,
        &inner_policy,
        &wire[..used],
        &mut output,
    )
    .unwrap_err();
    assert_eq!(
        error.kind(),
        RecordProtectionErrorKind::AuthenticationFailed
    );
    assert_eq!(output, [0xa5; 64]);
    assert_eq!(read_state.records_used(), 1);
    assert!(!format!("{error:?}").contains("token-shaped"));
}

#[test]
fn exact_outer_header_is_authenticated() {
    let suite = selected(CipherSuite::TlsAes128GcmSha256, 2);
    let mut write_state = state(suite, FixedHkdfHash::Sha256, 0x25);
    let mut read_state = state(suite, FixedHkdfHash::Sha256, 0x25);
    let (ciphertext_policy, inner_policy) = policies();
    let mut scratch = [0; 64];
    let mut wire = [0; 96];
    let used = seal_tls13_aes128_gcm_record(
        &mut write_state,
        &ciphertext_policy,
        &inner_policy,
        Tls13Plaintext::new(TlsContentType::ApplicationData, b"header-bound", 0),
        &mut scratch,
        &mut wire,
    )
    .unwrap();

    wire[used] = 0;
    let changed_fragment_length = u16::from_be_bytes([wire[3], wire[4]]) + 1;
    wire[3..5].copy_from_slice(&changed_fragment_length.to_be_bytes());
    let mut output = [0xa5; 64];
    let error = open_tls13_aes128_gcm_record(
        &mut read_state,
        &ciphertext_policy,
        &inner_policy,
        &wire[..used + 1],
        &mut output,
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        RecordProtectionErrorKind::AuthenticationFailed
    );
    assert_eq!(read_state.records_used(), 1);
    assert_eq!(output, [0xa5; 64]);
}

#[test]
fn public_framing_and_capacity_failures_do_not_consume_nonce() {
    let suite = selected(CipherSuite::TlsAes128GcmSha256, 3);
    let mut state = state(suite, FixedHkdfHash::Sha256, 0x11);
    let (ciphertext_policy, inner_policy) = policies();
    let mut output = [0x77; 32];

    let mut malformed = [0_u8; 22];
    malformed[..5].copy_from_slice(&[22, 3, 3, 0, 17]);
    let error = open_tls13_aes128_gcm_record(
        &mut state,
        &ciphertext_policy,
        &inner_policy,
        &malformed,
        &mut output,
    )
    .unwrap_err();
    assert!(matches!(error.kind(), RecordProtectionErrorKind::Record(_)));
    assert_eq!(state.records_used(), 0);
    assert_eq!(output, [0x77; 32]);

    let mut too_small = [0_u8; 22];
    too_small[..5].copy_from_slice(&[23, 3, 3, 0, 17]);
    let error = open_tls13_aes128_gcm_record(
        &mut state,
        &ciphertext_policy,
        &inner_policy,
        &too_small,
        &mut output[..0],
    )
    .unwrap_err();
    assert_eq!(error.kind(), RecordProtectionErrorKind::OutputTooSmall);
    assert_eq!(state.records_used(), 0);
}

#[test]
fn seal_capacity_failure_clears_scratch_before_nonce_use() {
    let suite = selected(CipherSuite::TlsAes128GcmSha256, 1);
    let mut state = state(suite, FixedHkdfHash::Sha256, 0x31);
    let (ciphertext_policy, inner_policy) = policies();
    let mut scratch = [0xa5; 32];
    let mut output = [0x44; 8];

    let error = seal_tls13_aes128_gcm_record(
        &mut state,
        &ciphertext_policy,
        &inner_policy,
        Tls13Plaintext::new(TlsContentType::Handshake, b"hello", 2),
        &mut scratch,
        &mut output,
    )
    .unwrap_err();
    assert_eq!(error.kind(), RecordProtectionErrorKind::OutputTooSmall);
    assert_eq!(state.records_used(), 0);
    assert_eq!(scratch[..8], [0; 8]);
    assert_eq!(output, [0x44; 8]);
}

#[test]
fn non_aes128_routes_fail_before_scratch_or_sequence_changes() {
    let cases = [
        (CipherSuite::TlsAes256GcmSha384, FixedHkdfHash::Sha384),
        (
            CipherSuite::TlsChacha20Poly1305Sha256,
            FixedHkdfHash::Sha256,
        ),
    ];

    for (cipher_suite, hash) in cases {
        let suite = selected(cipher_suite, 1);
        let mut state = state(suite, hash, 0x55);
        let (ciphertext_policy, inner_policy) = policies();
        let mut scratch = [0xa5; 32];
        let mut output = [0x44; 64];
        let error = seal_tls13_aes128_gcm_record(
            &mut state,
            &ciphertext_policy,
            &inner_policy,
            Tls13Plaintext::new(TlsContentType::Handshake, b"hello", 0),
            &mut scratch,
            &mut output,
        )
        .unwrap_err();

        assert_eq!(
            error.kind(),
            RecordProtectionErrorKind::UnsupportedCipherSuite
        );
        assert_eq!(state.records_used(), 0);
        assert_eq!(scratch, [0xa5; 32]);
        assert_eq!(output, [0x44; 64]);
    }
}

#[test]
fn authenticated_but_disallowed_inner_type_is_cleared_and_consumed() {
    let suite = selected(CipherSuite::TlsAes128GcmSha256, 1);
    let mut write_state = state(suite, FixedHkdfHash::Sha256, 0x61);
    let mut read_state = state(suite, FixedHkdfHash::Sha256, 0x61);
    let ciphertext_policy = Tls13CiphertextPolicy::new(1024).unwrap();
    let write_policy =
        Tls13InnerPlaintextPolicy::new(&[TlsContentType::ApplicationData], 1008).unwrap();
    let read_policy = Tls13InnerPlaintextPolicy::new(&[TlsContentType::Handshake], 1008).unwrap();
    let mut scratch = [0; 64];
    let mut wire = [0; 96];
    let used = seal_tls13_aes128_gcm_record(
        &mut write_state,
        &ciphertext_policy,
        &write_policy,
        Tls13Plaintext::new(TlsContentType::ApplicationData, b"secret", 0),
        &mut scratch,
        &mut wire,
    )
    .unwrap();
    let mut output = [0xa5; 64];

    let error = open_tls13_aes128_gcm_record(
        &mut read_state,
        &ciphertext_policy,
        &read_policy,
        &wire[..used],
        &mut output,
    )
    .unwrap_err();
    assert!(matches!(
        error.kind(),
        RecordProtectionErrorKind::InnerPlaintext(_)
    ));
    assert_eq!(output[..7], [0; 7]);
    assert_eq!(read_state.records_used(), 1);
}

#[test]
fn sequence_exhaustion_is_closed() {
    let suite = selected(CipherSuite::TlsAes128GcmSha256, 1);
    let mut state = state(suite, FixedHkdfHash::Sha256, 0x71);
    let (ciphertext_policy, inner_policy) = policies();
    let mut scratch = [0; 32];
    let mut wire = [0; 64];
    seal_tls13_aes128_gcm_record(
        &mut state,
        &ciphertext_policy,
        &inner_policy,
        Tls13Plaintext::new(TlsContentType::ApplicationData, b"one", 0),
        &mut scratch,
        &mut wire,
    )
    .unwrap();

    let error = seal_tls13_aes128_gcm_record(
        &mut state,
        &ciphertext_policy,
        &inner_policy,
        Tls13Plaintext::new(TlsContentType::ApplicationData, b"two", 0),
        &mut scratch,
        &mut wire,
    )
    .unwrap_err();
    assert!(matches!(error.kind(), RecordProtectionErrorKind::State(_)));
    assert_eq!(state.records_used(), 1);
    assert_eq!(scratch[..4], [0; 4]);
}
