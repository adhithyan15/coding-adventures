use coding_adventures_hkdf::{FixedHkdfHash, HkdfPrk, HkdfPrkImportError};
use std::error::Error;
use tls13_cipher_suite_offer::ClientCipherSuiteOffer;
use tls13_cipher_suite_policy::{
    CipherSuite, CipherSuitePolicy, CipherSuitePolicyEntry, HkdfHash, SelectedCipherSuite,
};
use tls13_client_hello_message::{decode_client_hello, ClientHelloLimits};
use tls13_extension_block::ExtensionBlockLimits;
use tls13_extension_context_policy::{ExtensionContextPolicy, STANDARD_TLS13_EXTENSION_RULES};
use tls13_handshake_traffic_secrets::HandshakeTrafficSecretStorage;
use tls13_key_schedule::BaseSecretStorage;
use tls13_key_share::KeyShareLimits;
use tls13_named_group_policy::{NamedGroup, NamedGroupPolicy};
use tls13_retry_handshake_traffic_key_material_binding::{
    bind_retry_handshake_traffic_key_material, HandshakeTrafficSender,
    RetryHandshakeTrafficKeyMaterialBindingError,
};
use tls13_retry_handshake_traffic_secrets_binding::bind_retry_handshake_traffic_secrets;
use tls13_retry_key_schedule_binding::bind_retry_key_schedule;
use tls13_retry_transcript_binding::bind_retry_transcript;
use tls13_server_hello_message::{
    decode_tls13_server_hello, Tls13ServerHelloLimits, TLS13_HELLO_RETRY_REQUEST_RANDOM,
};
use tls13_server_hello_retry_selection::{
    bind_final_server_key_share_message, bind_retry_key_share_messages,
    bind_server_hello_retry_selection, BoundRetryServerKeyShareMessages,
};
use tls13_server_hello_selection::bind_server_hello_selection;
use tls13_server_hello_version_selection::bind_server_hello_version_selection;
use tls13_supported_groups::NamedGroupListLimits;
use tls13_supported_versions::ClientVersionOffer;
use tls13_traffic_key_material::{
    derive_traffic_key_material, TrafficKeyMaterialError, TrafficKeyMaterialStorage,
};
use tls13_transcript::TranscriptPolicy;
use tls13_x25519_key_share_binding::derive_message_bound_retry_x25519_shared_secret;

// RFC 7748 section 6.1 Alice and Bob values.
const ALICE_PRIVATE: [u8; 32] = [
    0x77, 0x07, 0x6d, 0x0a, 0x73, 0x18, 0xa5, 0x7d, 0x3c, 0x16, 0xc1, 0x72, 0x51, 0xb2, 0x66, 0x45,
    0xdf, 0x4c, 0x2f, 0x87, 0xeb, 0xc0, 0x99, 0x2a, 0xb1, 0x77, 0xfb, 0xa5, 0x1d, 0xb9, 0x2c, 0x2a,
];
const ALICE_PUBLIC: [u8; 32] = [
    0x85, 0x20, 0xf0, 0x09, 0x89, 0x30, 0xa7, 0x54, 0x74, 0x8b, 0x7d, 0xdc, 0xb4, 0x3e, 0xf7, 0x5a,
    0x0d, 0xbf, 0x3a, 0x0d, 0x26, 0x38, 0x1a, 0xf4, 0xeb, 0xa4, 0xa9, 0x8e, 0xaa, 0x9b, 0x4e, 0x6a,
];
const BOB_PUBLIC: [u8; 32] = [
    0xde, 0x9e, 0xdb, 0x7d, 0x7b, 0x7d, 0xc1, 0xb4, 0xd3, 0x5b, 0x61, 0xc2, 0xec, 0xe4, 0x35, 0x37,
    0x3f, 0x83, 0x43, 0xc8, 0x5b, 0x78, 0x67, 0x4d, 0xad, 0xfc, 0x7e, 0x14, 0x6f, 0x88, 0x2b, 0x4f,
];
const SESSION_ID: &[u8] = b"retry-key-material";
const CLIENT_RANDOM: [u8; 32] = [0x11; 32];
const FINAL_SERVER_RANDOM: [u8; 32] = [0x22; 32];
const TLS13: u16 = 0x0304;

fn selected_suite(suite: CipherSuite) -> SelectedCipherSuite {
    let entry = CipherSuitePolicyEntry::new(suite, 1_000).unwrap();
    CipherSuitePolicy::new(&[entry])
        .unwrap()
        .select(suite.wire_id())
        .unwrap()
}

fn fixed_hash(suite: SelectedCipherSuite) -> FixedHkdfHash {
    match suite.parameters().hkdf_hash() {
        HkdfHash::Sha256 => FixedHkdfHash::Sha256,
        HkdfHash::Sha384 => FixedHkdfHash::Sha384,
    }
}

fn expected_material(suite: SelectedCipherSuite, secret: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let prk = HkdfPrk::import_exact(fixed_hash(suite), secret).unwrap();
    let mut storage = TrafficKeyMaterialStorage::new(suite).unwrap();
    let material = derive_traffic_key_material(&prk, &mut storage).unwrap();
    (material.key().to_vec(), material.iv().to_vec())
}

fn group_policy() -> NamedGroupPolicy {
    NamedGroupPolicy::new(&[NamedGroup::X25519, NamedGroup::Secp256r1]).unwrap()
}

fn x25519_client_offer(public_key: &[u8; 32]) -> [u8; 38] {
    let mut bytes = [0u8; 38];
    bytes[..6].copy_from_slice(&[0x00, 0x24, 0x00, 0x1d, 0x00, 0x20]);
    bytes[6..].copy_from_slice(public_key);
    bytes
}

fn x25519_server_share(public_key: &[u8; 32]) -> [u8; 36] {
    let mut bytes = [0u8; 36];
    bytes[..4].copy_from_slice(&[0x00, 0x1d, 0x00, 0x20]);
    bytes[4..].copy_from_slice(public_key);
    bytes
}

fn append_extensions(body: &mut Vec<u8>, extensions: &[(u16, &[u8])]) {
    let payload_length: usize = extensions.iter().map(|(_, data)| 4 + data.len()).sum();
    body.extend_from_slice(&(payload_length as u16).to_be_bytes());
    for (extension_type, data) in extensions {
        body.extend_from_slice(&extension_type.to_be_bytes());
        body.extend_from_slice(&(data.len() as u16).to_be_bytes());
        body.extend_from_slice(data);
    }
}

fn client_body(suite: CipherSuite, extensions: &[(u16, &[u8])]) -> Vec<u8> {
    let mut body = vec![0x03, 0x03];
    body.extend_from_slice(&CLIENT_RANDOM);
    body.push(SESSION_ID.len() as u8);
    body.extend_from_slice(SESSION_ID);
    body.extend_from_slice(&[0x00, 0x02]);
    body.extend_from_slice(&suite.wire_id().to_be_bytes());
    body.extend_from_slice(&[0x01, 0x00]);
    let version_data = [0x02, 0x03, 0x04];
    let mut all_extensions = vec![(43, version_data.as_slice())];
    all_extensions.extend_from_slice(extensions);
    append_extensions(&mut body, &all_extensions);
    body
}

fn server_body(suite: CipherSuite, random: [u8; 32], extensions: &[(u16, &[u8])]) -> Vec<u8> {
    let mut body = vec![0x03, 0x03];
    body.extend_from_slice(&random);
    body.push(SESSION_ID.len() as u8);
    body.extend_from_slice(SESSION_ID);
    body.extend_from_slice(&suite.wire_id().to_be_bytes());
    body.push(0x00);
    let version_data = TLS13.to_be_bytes();
    let mut all_extensions = vec![(43, version_data.as_slice())];
    all_extensions.extend_from_slice(extensions);
    append_extensions(&mut body, &all_extensions);
    body
}

fn version_selection<'client, 'server>(
    suite: CipherSuite,
    client_bytes: &'client [u8],
    server_bytes: &'server [u8],
) -> tls13_server_hello_version_selection::BoundServerHelloVersionSelection<
    'client,
    'static,
    'server,
    'static,
    'static,
> {
    static AES128_SUITE: [u16; 1] = [0x1301];
    static AES256_SUITE: [u16; 1] = [0x1302];
    static VERSIONS: [u16; 1] = [TLS13];
    let suites = match suite {
        CipherSuite::TlsAes128GcmSha256 => &AES128_SUITE,
        CipherSuite::TlsAes256GcmSha384 => &AES256_SUITE,
        CipherSuite::TlsChacha20Poly1305Sha256 => unreachable!("test suite is AES-only"),
    };
    let client_limits =
        ClientHelloLimits::new(4, ExtensionBlockLimits::new(256, 4).unwrap()).unwrap();
    let server_limits = Tls13ServerHelloLimits::new(256, 256, 4).unwrap();
    let client = decode_client_hello(client_bytes, client_limits).unwrap();
    let server = decode_tls13_server_hello(server_bytes, server_limits).unwrap();
    let entries = [CipherSuitePolicyEntry::new(suite, 1_000).unwrap()];
    let offer =
        ClientCipherSuiteOffer::new(suites, CipherSuitePolicy::new(&entries).unwrap()).unwrap();
    let selection = bind_server_hello_selection(client, offer, server).unwrap();
    bind_server_hello_version_selection(
        selection,
        ClientVersionOffer::new(&VERSIONS).unwrap(),
        ExtensionContextPolicy::new(&STANDARD_TLS13_EXTENSION_RULES).unwrap(),
    )
    .unwrap()
}

fn with_retry_x25519_messages<T>(
    suite: CipherSuite,
    callback: impl FnOnce(
        &BoundRetryServerKeyShareMessages<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    ) -> T,
) -> T {
    let p256_key = [0x04; 65];
    let supported_groups = [0x00, 0x04, 0x00, 0x17, 0x00, 0x1d];
    let mut initial_share = [0u8; 71];
    initial_share[..6].copy_from_slice(&[0x00, 0x45, 0x00, 0x17, 0x00, 0x41]);
    initial_share[6..].copy_from_slice(&p256_key);
    let retry_share = [0x00, 0x1d];
    let retried_share = x25519_client_offer(&ALICE_PUBLIC);
    let final_share = x25519_server_share(&BOB_PUBLIC);
    let first_client = client_body(suite, &[(10, &supported_groups), (51, &initial_share)]);
    let retry_server = server_body(
        suite,
        TLS13_HELLO_RETRY_REQUEST_RANDOM,
        &[(51, &retry_share)],
    );
    let second_client = client_body(suite, &[(10, &supported_groups), (51, &retried_share)]);
    let final_server = server_body(suite, FINAL_SERVER_RANDOM, &[(51, &final_share)]);
    let selection = bind_server_hello_retry_selection(
        version_selection(suite, &first_client, &retry_server),
        version_selection(suite, &second_client, &final_server),
    )
    .unwrap();
    let messages = bind_retry_key_share_messages(
        selection,
        group_policy(),
        NamedGroupListLimits::new(2).unwrap(),
        KeyShareLimits::new(2).unwrap(),
    )
    .unwrap();
    let final_messages = bind_final_server_key_share_message(&messages).unwrap();
    callback(&final_messages)
}

#[test]
fn derives_both_senders_from_and_retains_exact_retry_evidence() {
    let mut x25519_output = [0xa5; 32];
    with_retry_x25519_messages(CipherSuite::TlsAes128GcmSha256, |messages| {
        let secret = derive_message_bound_retry_x25519_shared_secret(
            messages,
            &ALICE_PRIVATE,
            &mut x25519_output,
        )
        .unwrap();
        let transcript = bind_retry_transcript(
            messages.messages().selection(),
            TranscriptPolicy::new(4_096).unwrap(),
        )
        .unwrap();
        let suite = transcript.transcript().selected_suite();
        let mut base_storage = BaseSecretStorage::new(suite).unwrap();
        let base = bind_retry_key_schedule(&secret, &transcript, &mut base_storage).unwrap();
        let mut traffic_storage = HandshakeTrafficSecretStorage::new(suite).unwrap();
        let traffic = bind_retry_handshake_traffic_secrets(&base, &mut traffic_storage).unwrap();

        let expected_client = expected_material(suite, traffic.traffic_secrets().client_secret());
        let expected_server = expected_material(suite, traffic.traffic_secrets().server_secret());
        let mut client_storage = TrafficKeyMaterialStorage::new(suite).unwrap();
        let mut server_storage = TrafficKeyMaterialStorage::new(suite).unwrap();
        let client = bind_retry_handshake_traffic_key_material(
            &traffic,
            HandshakeTrafficSender::Client,
            &mut client_storage,
        )
        .unwrap();
        let server = bind_retry_handshake_traffic_key_material(
            &traffic,
            HandshakeTrafficSender::Server,
            &mut server_storage,
        )
        .unwrap();

        assert!(core::ptr::eq(client.traffic_secrets(), &traffic));
        assert!(core::ptr::eq(server.traffic_secrets(), &traffic));
        assert_eq!(client.sender(), HandshakeTrafficSender::Client);
        assert_eq!(server.sender(), HandshakeTrafficSender::Server);
        assert_eq!(client.selected_suite(), suite);
        assert_eq!(server.selected_suite(), suite);
        assert_eq!(client.key_material().key(), expected_client.0);
        assert_eq!(client.key_material().iv(), expected_client.1);
        assert_eq!(server.key_material().key(), expected_server.0);
        assert_eq!(server.key_material().iv(), expected_server.1);
        assert_ne!(client.key_material().key(), server.key_material().key());

        let debug = format!("{client:?}");
        assert!(debug.contains("[RETAINED]"));
        assert!(debug.contains("[SECRET]"));
        for disclosed in ["retry-key-material", "8520f009", "de9edb7d"] {
            assert!(!debug.contains(disclosed));
        }
    });
}

#[test]
fn routes_sha384_from_the_retained_suite() {
    let mut x25519_output = [0xa5; 32];
    with_retry_x25519_messages(CipherSuite::TlsAes256GcmSha384, |messages| {
        let secret = derive_message_bound_retry_x25519_shared_secret(
            messages,
            &ALICE_PRIVATE,
            &mut x25519_output,
        )
        .unwrap();
        let transcript = bind_retry_transcript(
            messages.messages().selection(),
            TranscriptPolicy::new(4_096).unwrap(),
        )
        .unwrap();
        let suite = transcript.transcript().selected_suite();
        let mut base_storage = BaseSecretStorage::new(suite).unwrap();
        let base = bind_retry_key_schedule(&secret, &transcript, &mut base_storage).unwrap();
        let mut traffic_storage = HandshakeTrafficSecretStorage::new(suite).unwrap();
        let traffic = bind_retry_handshake_traffic_secrets(&base, &mut traffic_storage).unwrap();
        let expected = expected_material(suite, traffic.traffic_secrets().server_secret());
        let mut output_storage = TrafficKeyMaterialStorage::new(suite).unwrap();

        let bound = bind_retry_handshake_traffic_key_material(
            &traffic,
            HandshakeTrafficSender::Server,
            &mut output_storage,
        )
        .unwrap();

        assert_eq!(bound.key_material().key(), expected.0);
        assert_eq!(bound.key_material().iv(), expected.1);
        assert_eq!(bound.key_material().key().len(), 32);
        assert_eq!(bound.key_material().iv().len(), 12);
    });
}

#[test]
fn rejects_output_storage_for_another_suite_before_derivation() {
    let mut x25519_output = [0xa5; 32];
    with_retry_x25519_messages(CipherSuite::TlsAes128GcmSha256, |messages| {
        let secret = derive_message_bound_retry_x25519_shared_secret(
            messages,
            &ALICE_PRIVATE,
            &mut x25519_output,
        )
        .unwrap();
        let transcript = bind_retry_transcript(
            messages.messages().selection(),
            TranscriptPolicy::new(4_096).unwrap(),
        )
        .unwrap();
        let mut base_storage =
            BaseSecretStorage::new(transcript.transcript().selected_suite()).unwrap();
        let base = bind_retry_key_schedule(&secret, &transcript, &mut base_storage).unwrap();
        let mut traffic_storage =
            HandshakeTrafficSecretStorage::new(transcript.transcript().selected_suite()).unwrap();
        let traffic = bind_retry_handshake_traffic_secrets(&base, &mut traffic_storage).unwrap();
        let mut output_storage =
            TrafficKeyMaterialStorage::new(selected_suite(CipherSuite::TlsAes256GcmSha384))
                .unwrap();

        assert_eq!(
            bind_retry_handshake_traffic_key_material(
                &traffic,
                HandshakeTrafficSender::Client,
                &mut output_storage,
            )
            .unwrap_err(),
            RetryHandshakeTrafficKeyMaterialBindingError::CipherSuiteMismatch
        );
    });
}

#[test]
fn errors_are_stable_and_payload_free() {
    let mismatch = RetryHandshakeTrafficKeyMaterialBindingError::CipherSuiteMismatch;
    assert_eq!(
        mismatch.to_string(),
        "TLS 1.3 retry traffic secret and key-material storage use different cipher suites"
    );
    assert!(mismatch.source().is_none());

    let import = RetryHandshakeTrafficKeyMaterialBindingError::TrafficSecretImport(
        HkdfPrkImportError::InvalidLength,
    );
    assert_eq!(
        import.to_string(),
        "TLS 1.3 retry handshake traffic-secret import failed"
    );
    assert!(import.source().is_some());

    let derivation = RetryHandshakeTrafficKeyMaterialBindingError::KeyMaterial(
        TrafficKeyMaterialError::HashMismatch,
    );
    assert_eq!(
        derivation.to_string(),
        "TLS 1.3 retry handshake traffic-key derivation failed"
    );
    assert!(derivation.source().is_some());
    assert!(!format!("{derivation:?}").contains("retry-key-material"));
}
