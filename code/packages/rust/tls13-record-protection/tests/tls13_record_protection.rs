use coding_adventures_hkdf::{FixedHkdfHash, HkdfPrk};
use tls13_cipher_suite_policy::{CipherSuite, CipherSuitePolicy, CipherSuitePolicyEntry};
use tls13_inner_plaintext::Tls13InnerPlaintextPolicy;
use tls13_record::{Tls13CiphertextPolicy, TlsContentType};
use tls13_record_protection::{
    open_tls13_record, seal_tls13_record, RecordProtectionErrorKind, Tls13Plaintext,
};
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
    let secret_bytes = vec![secret_byte; hash.hash_len()];
    let secret = HkdfPrk::import_exact(hash, &secret_bytes).unwrap();
    let storage = Box::leak(Box::new(TrafficKeyMaterialStorage::new(suite).unwrap()));
    let material = derive_traffic_key_material(&secret, storage).unwrap();
    RecordProtectionState::new(material).unwrap()
}

fn round_trip(suite: CipherSuite, hash: FixedHkdfHash) {
    let selected = selected(suite, 2);
    let mut write_state = state(selected, hash, 0x42);
    let mut read_state = state(selected, hash, 0x42);
    let (ciphertext_policy, inner_policy) = policies();
    let mut scratch = [0xa5; 128];
    let mut wire = [0xcc; 160];

    let used = seal_tls13_record(
        &mut write_state,
        &ciphertext_policy,
        &inner_policy,
        Tls13Plaintext::new(TlsContentType::Handshake, b"server flight", 7),
        &mut scratch,
        &mut wire,
    )
    .unwrap();

    assert_eq!(&wire[..3], &[23, 3, 3]);
    assert_eq!(scratch[..21], [0; 21]);
    assert!(scratch[21..].iter().all(|byte| *byte == 0xa5));
    assert_eq!(write_state.records_used(), 1);

    let mut opened_storage = [0x5a; 128];
    {
        let opened = open_tls13_record(
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
        let debug = format!("{opened:?}");
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("server flight"));
        assert!(!debug.contains("Aes"));
        assert!(!debug.contains("Chacha"));
    }
    assert_eq!(opened_storage[..21], [0; 21]);
    assert!(opened_storage[21..].iter().all(|byte| *byte == 0x5a));
    assert_eq!(read_state.records_used(), 1);
}

#[test]
fn aes128_route_round_trips_and_wipes_opened_storage() {
    round_trip(CipherSuite::TlsAes128GcmSha256, FixedHkdfHash::Sha256);
}

#[test]
fn chacha20_route_round_trips_and_wipes_opened_storage() {
    round_trip(
        CipherSuite::TlsChacha20Poly1305Sha256,
        FixedHkdfHash::Sha256,
    );
}

#[test]
fn aes256_seal_fails_before_scratch_output_or_sequence_changes() {
    let suite = selected(CipherSuite::TlsAes256GcmSha384, 1);
    let mut state = state(suite, FixedHkdfHash::Sha384, 0x55);
    let (ciphertext_policy, inner_policy) = policies();
    let mut scratch = [0xa5; 32];
    let mut output = [0x44; 64];

    let error = seal_tls13_record(
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

#[test]
fn aes256_open_fails_before_parsing_output_or_sequence_changes() {
    let suite = selected(CipherSuite::TlsAes256GcmSha384, 1);
    let mut state = state(suite, FixedHkdfHash::Sha384, 0x56);
    let (ciphertext_policy, inner_policy) = policies();
    let malformed_record = b"not a TLS record";
    let mut output = [0x44; 64];

    let error = open_tls13_record(
        &mut state,
        &ciphertext_policy,
        &inner_policy,
        malformed_record,
        &mut output,
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        RecordProtectionErrorKind::UnsupportedCipherSuite
    );
    assert_eq!(state.records_used(), 0);
    assert_eq!(output, [0x44; 64]);
}

#[test]
fn route_specific_authentication_failure_is_redacted_and_consumes_nonce() {
    let suite = selected(CipherSuite::TlsAes128GcmSha256, 2);
    let mut write_state = state(suite, FixedHkdfHash::Sha256, 0x24);
    let mut read_state = state(suite, FixedHkdfHash::Sha256, 0x24);
    let (ciphertext_policy, inner_policy) = policies();
    let mut scratch = [0; 64];
    let mut wire = [0; 96];
    let used = seal_tls13_record(
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

    let error = open_tls13_record(
        &mut read_state,
        &ciphertext_policy,
        &inner_policy,
        &wire[..used],
        &mut output,
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        RecordProtectionErrorKind::Aes128Gcm(
            tls13_aes128_gcm_record::RecordProtectionErrorKind::AuthenticationFailed,
        )
    );
    assert_eq!(output, [0xa5; 64]);
    assert_eq!(read_state.records_used(), 1);
    assert!(!format!("{error:?}").contains("token-shaped"));
}

#[test]
fn chacha20_route_error_remains_distinguishable_without_nonce_use() {
    let suite = selected(CipherSuite::TlsChacha20Poly1305Sha256, 1);
    let mut state = state(suite, FixedHkdfHash::Sha256, 0x31);
    let (ciphertext_policy, inner_policy) = policies();
    let mut scratch = [0xa5; 32];
    let mut output = [0x44; 8];

    let error = seal_tls13_record(
        &mut state,
        &ciphertext_policy,
        &inner_policy,
        Tls13Plaintext::new(TlsContentType::Handshake, b"hello", 2),
        &mut scratch,
        &mut output,
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        RecordProtectionErrorKind::Chacha20Poly1305(
            tls13_chacha20_poly1305_record::RecordProtectionErrorKind::OutputTooSmall,
        )
    );
    assert_eq!(state.records_used(), 0);
    assert_eq!(scratch[..8], [0; 8]);
    assert_eq!(output, [0x44; 8]);
}

#[test]
fn plaintext_debug_redacts_borrowed_content() {
    let plaintext = Tls13Plaintext::new(
        TlsContentType::ApplicationData,
        b"credential-shaped content",
        0,
    );
    let debug = format!("{plaintext:?}");

    assert!(debug.contains("[REDACTED]"));
    assert!(!debug.contains("credential-shaped"));
}
