use tls13_cipher_suite_offer::ClientCipherSuiteOffer;
use tls13_cipher_suite_policy::{CipherSuite, CipherSuitePolicy, CipherSuitePolicyEntry};
use tls13_client_hello_message::{decode_client_hello, ClientHelloLimits};
use tls13_extension_block::ExtensionBlockLimits;
use tls13_extension_context_policy::{ExtensionContextPolicy, STANDARD_TLS13_EXTENSION_RULES};
use tls13_key_schedule::{derive_ephemeral_base_secrets, BaseSecretStorage};
use tls13_key_share::KeyShareLimits;
use tls13_named_group_policy::{NamedGroup, NamedGroupPolicy};
use tls13_retry_key_schedule_binding::{bind_retry_key_schedule, RetryKeyScheduleBindingError};
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
const SHARED_SECRET: [u8; 32] = [
    0x4a, 0x5d, 0x9d, 0x5b, 0xa4, 0xce, 0x2d, 0xe1, 0x72, 0x8e, 0x3b, 0xf4, 0x80, 0x35, 0x0f, 0x25,
    0xe0, 0x7e, 0x21, 0xc9, 0x47, 0xd1, 0x9e, 0x33, 0x76, 0xf0, 0x9b, 0x3c, 0x1e, 0x16, 0x17, 0x42,
];
const SESSION_ID: &[u8] = b"retry-schedule";
const CLIENT_RANDOM: [u8; 32] = [0x11; 32];
const FINAL_SERVER_RANDOM: [u8; 32] = [0x22; 32];
const TLS13: u16 = 0x0304;

fn group_policy() -> NamedGroupPolicy {
    NamedGroupPolicy::new(&[NamedGroup::X25519, NamedGroup::Secp256r1]).unwrap()
}

fn selected_suite(suite: CipherSuite) -> tls13_cipher_suite_policy::SelectedCipherSuite {
    let entry = CipherSuitePolicyEntry::new(suite, 1_000).unwrap();
    CipherSuitePolicy::new(&[entry])
        .unwrap()
        .select(suite.wire_id())
        .unwrap()
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

fn client_body(extensions: &[(u16, &[u8])]) -> Vec<u8> {
    let mut body = vec![0x03, 0x03];
    body.extend_from_slice(&CLIENT_RANDOM);
    body.push(SESSION_ID.len() as u8);
    body.extend_from_slice(SESSION_ID);
    body.extend_from_slice(&[0x00, 0x02, 0x13, 0x01]);
    body.extend_from_slice(&[0x01, 0x00]);
    let version_data = [0x02, 0x03, 0x04];
    let mut all_extensions = vec![(43, version_data.as_slice())];
    all_extensions.extend_from_slice(extensions);
    append_extensions(&mut body, &all_extensions);
    body
}

fn server_body(random: [u8; 32], extensions: &[(u16, &[u8])]) -> Vec<u8> {
    let mut body = vec![0x03, 0x03];
    body.extend_from_slice(&random);
    body.push(SESSION_ID.len() as u8);
    body.extend_from_slice(SESSION_ID);
    body.extend_from_slice(&[0x13, 0x01, 0x00]);
    let version_data = TLS13.to_be_bytes();
    let mut all_extensions = vec![(43, version_data.as_slice())];
    all_extensions.extend_from_slice(extensions);
    append_extensions(&mut body, &all_extensions);
    body
}

fn version_selection<'client, 'server>(
    client_bytes: &'client [u8],
    server_bytes: &'server [u8],
) -> tls13_server_hello_version_selection::BoundServerHelloVersionSelection<
    'client,
    'static,
    'server,
    'static,
    'static,
> {
    static SUITES: [u16; 1] = [0x1301];
    static VERSIONS: [u16; 1] = [TLS13];
    let client_limits =
        ClientHelloLimits::new(4, ExtensionBlockLimits::new(256, 4).unwrap()).unwrap();
    let server_limits = Tls13ServerHelloLimits::new(256, 256, 4).unwrap();
    let client = decode_client_hello(client_bytes, client_limits).unwrap();
    let server = decode_tls13_server_hello(server_bytes, server_limits).unwrap();
    let entries = [CipherSuitePolicyEntry::new(CipherSuite::TlsAes128GcmSha256, 1_000).unwrap()];
    let offer =
        ClientCipherSuiteOffer::new(&SUITES, CipherSuitePolicy::new(&entries).unwrap()).unwrap();
    let selection = bind_server_hello_selection(client, offer, server).unwrap();
    bind_server_hello_version_selection(
        selection,
        ClientVersionOffer::new(&VERSIONS).unwrap(),
        ExtensionContextPolicy::new(&STANDARD_TLS13_EXTENSION_RULES).unwrap(),
    )
    .unwrap()
}

fn with_retry_x25519_messages<T>(
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
    let first_client = client_body(&[(10, &supported_groups), (51, &initial_share)]);
    let retry_server = server_body(TLS13_HELLO_RETRY_REQUEST_RANDOM, &[(51, &retry_share)]);
    let second_client = client_body(&[(10, &supported_groups), (51, &retried_share)]);
    let final_server = server_body(FINAL_SERVER_RANDOM, &[(51, &final_share)]);
    let selection = bind_server_hello_retry_selection(
        version_selection(&first_client, &retry_server),
        version_selection(&second_client, &final_server),
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
fn exact_retry_evidence_derives_the_existing_base_schedule_and_stays_attached() {
    let mut expected_storage =
        BaseSecretStorage::new(selected_suite(CipherSuite::TlsAes128GcmSha256)).unwrap();
    let (expected_early, expected_handshake, expected_master) = {
        let expected =
            derive_ephemeral_base_secrets(&SHARED_SECRET, &mut expected_storage).unwrap();
        (
            expected.early_secret().to_vec(),
            expected.handshake_secret().to_vec(),
            expected.master_secret().to_vec(),
        )
    };

    let mut output = [0xa5; 32];
    with_retry_x25519_messages(|messages| {
        let secret =
            derive_message_bound_retry_x25519_shared_secret(messages, &ALICE_PRIVATE, &mut output)
                .unwrap();
        let transcript = bind_retry_transcript(
            messages.messages().selection(),
            TranscriptPolicy::new(4_096).unwrap(),
        )
        .unwrap();
        let mut storage = BaseSecretStorage::new(transcript.transcript().selected_suite()).unwrap();

        let bound = bind_retry_key_schedule(&secret, &transcript, &mut storage).unwrap();

        assert!(core::ptr::eq(bound.shared_secret(), &secret));
        assert!(core::ptr::eq(bound.transcript(), &transcript));
        assert_eq!(bound.base_secrets().early_secret(), expected_early);
        assert_eq!(bound.base_secrets().handshake_secret(), expected_handshake);
        assert_eq!(bound.base_secrets().master_secret(), expected_master);
        let debug = format!("{bound:?}");
        assert!(debug.contains("[RETAINED]"));
        assert!(debug.matches("[SECRET]").count() >= 2);
        for disclosed in ["retry-schedule", "4a5d9d5b", "8520f009"] {
            assert!(!debug.contains(disclosed));
        }
    });
}

#[test]
fn independently_constructed_retry_proofs_are_rejected() {
    let mut output = [0xa5; 32];
    with_retry_x25519_messages(|secret_messages| {
        let secret = derive_message_bound_retry_x25519_shared_secret(
            secret_messages,
            &ALICE_PRIVATE,
            &mut output,
        )
        .unwrap();
        with_retry_x25519_messages(|transcript_messages| {
            let transcript = bind_retry_transcript(
                transcript_messages.messages().selection(),
                TranscriptPolicy::new(4_096).unwrap(),
            )
            .unwrap();
            let mut storage =
                BaseSecretStorage::new(transcript.transcript().selected_suite()).unwrap();

            assert_eq!(
                bind_retry_key_schedule(&secret, &transcript, &mut storage).unwrap_err(),
                RetryKeyScheduleBindingError::MessageProofMismatch
            );
        });
    });
}

#[test]
fn caller_storage_with_a_different_suite_is_rejected() {
    let mut output = [0xa5; 32];
    with_retry_x25519_messages(|messages| {
        let secret =
            derive_message_bound_retry_x25519_shared_secret(messages, &ALICE_PRIVATE, &mut output)
                .unwrap();
        let transcript = bind_retry_transcript(
            messages.messages().selection(),
            TranscriptPolicy::new(4_096).unwrap(),
        )
        .unwrap();
        let mut storage =
            BaseSecretStorage::new(selected_suite(CipherSuite::TlsAes256GcmSha384)).unwrap();

        assert_eq!(
            bind_retry_key_schedule(&secret, &transcript, &mut storage).unwrap_err(),
            RetryKeyScheduleBindingError::CipherSuiteMismatch
        );
    });
}

#[test]
fn public_errors_are_stable_and_payload_free() {
    assert_eq!(
        RetryKeyScheduleBindingError::MessageProofMismatch.to_string(),
        "TLS 1.3 retry transcript and shared secret use different message proofs"
    );
    assert_eq!(
        RetryKeyScheduleBindingError::CipherSuiteMismatch.to_string(),
        "TLS 1.3 retry transcript and base-secret storage use different cipher suites"
    );
}
