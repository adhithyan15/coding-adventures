use coding_adventures_hkdf::HkdfPrk;
use tls13_cipher_suite_policy::{
    CipherSuite, CipherSuitePolicy, CipherSuitePolicyEntry, HkdfHash, SelectedCipherSuite,
};
use tls13_handshake_traffic_secrets::{
    derive_handshake_traffic_secrets, HandshakeTrafficSecretError, HandshakeTrafficSecretStorage,
};
use tls13_hkdf_label::derive_secret_from_transcript_hash_fixed;
use tls13_key_schedule::{derive_ephemeral_base_secrets, BaseSecretStorage};
use tls13_transcript::{PendingTls13Transcript, TranscriptHash, TranscriptPolicy};

fn decode_hex(value: &str) -> Vec<u8> {
    assert_eq!(value.len() % 2, 0);
    (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16).unwrap())
        .collect()
}

fn selected(suite: CipherSuite) -> SelectedCipherSuite {
    let entry = CipherSuitePolicyEntry::new(suite, 4096).unwrap();
    CipherSuitePolicy::new(&[entry])
        .unwrap()
        .select(suite.wire_id())
        .unwrap()
}

fn transcript_hash(
    selected_suite: SelectedCipherSuite,
    before_selection: &[u8],
    after_selection: &[u8],
) -> TranscriptHash {
    let maximum = (before_selection.len() + after_selection.len()) as u64;
    let mut pending = PendingTls13Transcript::new(TranscriptPolicy::new(maximum).unwrap());
    pending.update(before_selection).unwrap();
    let mut transcript = pending.select(selected_suite);
    transcript.update(after_selection).unwrap();
    transcript.snapshot()
}

#[test]
fn derives_rfc8448_sha256_handshake_traffic_secrets() {
    let suite = selected(CipherSuite::TlsAes128GcmSha256);
    let client_hello = decode_hex(concat!(
        "010000c00303cb34ecb1e78163ba1c38c6dacb196a6dffa21a8d9912ec18a2ef",
        "6283024dece7000006130113031302010000910000000b00090000067365727665",
        "72ff01000100000a00140012001d00170018001901000101010201030104002300",
        "00003300260024001d002099381de560e4bd43d23d8e435a7dbafeb3c06e51c1",
        "3cae4d5413691e529aaf2c002b0003020304000d0020001e040305030603020308",
        "040805080604010501060102010402050206020202002d00020101001c00024001",
    ));
    let server_hello = decode_hex(concat!(
        "020000560303a6af06a4121860dc5e6e60249cd34c95930c8ac5cb1434dac1557",
        "72ed3e2692800130100002e00330024001d0020c9828876112095fe66762bdbf7c",
        "672e156d6cc253b833df1dd69b1b04e751f0f002b00020304",
    ));
    assert_eq!(client_hello.len(), 196);
    assert_eq!(server_hello.len(), 90);

    let snapshot = transcript_hash(suite, &client_hello, &server_hello);
    assert_eq!(
        snapshot.as_bytes(),
        decode_hex("860c06edc07858ee8e78f0e7428c58edd6b43f2ca3e6e95f02ed063cf0e1cad8")
    );

    let shared_secret =
        decode_hex("8bd4054fb55b9d63fdfbacf9f04b9f0d35e6d63f537563efd46272900f89492d");
    let mut base_storage = BaseSecretStorage::new(suite).unwrap();
    let base_secrets = derive_ephemeral_base_secrets(&shared_secret, &mut base_storage).unwrap();
    let mut output = HandshakeTrafficSecretStorage::new(suite).unwrap();

    let secrets = derive_handshake_traffic_secrets(&base_secrets, &snapshot, &mut output).unwrap();

    assert_eq!(secrets.selected_suite(), suite);
    assert_eq!(
        secrets.client_secret(),
        decode_hex("b3eddb126e067f35a780b3abf45e2d8f3b1a950738f52e9600746a0e27a55a21")
    );
    assert_eq!(
        secrets.server_secret(),
        decode_hex("b67b7d690cc16c4e75e54213cb2d37b4e9c912bcded9105d42befd59d391ad38")
    );

    let debug = format!("{secrets:?}");
    assert!(debug.contains("TlsAes128GcmSha256"));
    assert!(debug.contains("secret_length: 32"));
    assert!(!debug.contains("b3eddb12"));
}

#[test]
fn routes_sha384_through_the_selected_suite() {
    let suite = selected(CipherSuite::TlsAes256GcmSha384);
    let snapshot = transcript_hash(suite, b"client hello", b"server hello");
    let mut base_storage = BaseSecretStorage::new(suite).unwrap();
    let base_secrets = derive_ephemeral_base_secrets(&[0x42; 48], &mut base_storage).unwrap();
    let mut output = HandshakeTrafficSecretStorage::new(suite).unwrap();

    let secrets = derive_handshake_traffic_secrets(&base_secrets, &snapshot, &mut output).unwrap();

    assert_eq!(secrets.client_secret().len(), 48);
    assert_eq!(secrets.server_secret().len(), 48);

    let prk = HkdfPrk::import_exact(
        coding_adventures_hkdf::FixedHkdfHash::Sha384,
        base_secrets.handshake_secret(),
    )
    .unwrap();
    let mut expected_client = [0u8; 48];
    let mut expected_server = [0u8; 48];
    derive_secret_from_transcript_hash_fixed(
        &prk,
        b"c hs traffic",
        snapshot.as_bytes(),
        &mut expected_client,
    )
    .unwrap();
    derive_secret_from_transcript_hash_fixed(
        &prk,
        b"s hs traffic",
        snapshot.as_bytes(),
        &mut expected_server,
    )
    .unwrap();
    assert_eq!(secrets.client_secret(), expected_client);
    assert_eq!(secrets.server_secret(), expected_server);
}

#[test]
fn rejects_suite_mismatch_before_derivation() {
    let base_suite = selected(CipherSuite::TlsAes128GcmSha256);
    let output_suite = selected(CipherSuite::TlsChacha20Poly1305Sha256);
    let snapshot = transcript_hash(base_suite, b"client", b"server");
    let mut base_storage = BaseSecretStorage::new(base_suite).unwrap();
    let base_secrets = derive_ephemeral_base_secrets(&[0x11; 32], &mut base_storage).unwrap();
    let mut output = HandshakeTrafficSecretStorage::new(output_suite).unwrap();

    assert_eq!(
        derive_handshake_traffic_secrets(&base_secrets, &snapshot, &mut output).unwrap_err(),
        HandshakeTrafficSecretError::CipherSuiteMismatch
    );
}

#[test]
fn rejects_transcript_hash_mismatch_before_derivation() {
    let suite = selected(CipherSuite::TlsAes128GcmSha256);
    let sha384_suite = selected(CipherSuite::TlsAes256GcmSha384);
    let snapshot = transcript_hash(sha384_suite, b"client", b"server");
    assert_eq!(snapshot.algorithm(), HkdfHash::Sha384);
    let mut base_storage = BaseSecretStorage::new(suite).unwrap();
    let base_secrets = derive_ephemeral_base_secrets(&[0x11; 32], &mut base_storage).unwrap();
    let mut output = HandshakeTrafficSecretStorage::new(suite).unwrap();

    assert_eq!(
        derive_handshake_traffic_secrets(&base_secrets, &snapshot, &mut output).unwrap_err(),
        HandshakeTrafficSecretError::TranscriptHashMismatch
    );
}

#[test]
fn errors_and_storage_debug_are_payload_free() {
    assert_eq!(
        HandshakeTrafficSecretError::CipherSuiteMismatch.to_string(),
        "TLS 1.3 cipher suite does not match caller storage"
    );
    assert_eq!(
        HandshakeTrafficSecretError::TranscriptHashMismatch.to_string(),
        "TLS 1.3 transcript hash does not match the selected cipher suite"
    );
    assert_eq!(
        HandshakeTrafficSecretError::UnsupportedParameters.to_string(),
        "TLS 1.3 cipher-suite parameters exceed reviewed bounds"
    );
    assert_eq!(
        HandshakeTrafficSecretError::DerivationFailed.to_string(),
        "TLS 1.3 handshake traffic-secret derivation failed"
    );

    let storage =
        HandshakeTrafficSecretStorage::new(selected(CipherSuite::TlsAes128GcmSha256)).unwrap();
    let debug = format!("{storage:?}");
    assert!(debug.contains("[REDACTED]"));
    assert!(!debug.contains("client_secret"));
    assert!(!debug.contains("server_secret"));
}
