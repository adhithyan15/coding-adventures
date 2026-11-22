use tls13_cipher_suite_offer::ClientCipherSuiteOffer;
use tls13_cipher_suite_policy::{CipherSuite, CipherSuitePolicy, CipherSuitePolicyEntry};
use tls13_client_hello_message::{decode_client_hello, ClientHelloLimits};
use tls13_extension_block::ExtensionBlockLimits;
use tls13_extension_context_policy::{ExtensionContextPolicy, STANDARD_TLS13_EXTENSION_RULES};
use tls13_key_share::{decode_client_key_shares, decode_server_key_share, KeyShareLimits};
use tls13_named_group_policy::{NamedGroup, NamedGroupPolicy};
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
use tls13_x25519_key_agreement::KeyAgreementErrorKind;
use tls13_x25519_key_share_binding::{
    derive_bound_x25519_shared_secret, derive_message_bound_retry_x25519_shared_secret,
    KeyShareBindingError, KeyShareBindingErrorKind,
};

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
const SESSION_ID: &[u8] = b"retry-x25519";
const CLIENT_RANDOM: [u8; 32] = [0x11; 32];
const FINAL_SERVER_RANDOM: [u8; 32] = [0x22; 32];
const TLS13: u16 = 0x0304;

fn policy() -> NamedGroupPolicy {
    NamedGroupPolicy::new(&[NamedGroup::X25519, NamedGroup::Secp256r1])
        .expect("test policy is valid")
}

fn x25519_client_offer(public_key: &[u8; 32]) -> [u8; 38] {
    let mut bytes = [0u8; 38];
    bytes[..6].copy_from_slice(&[0x00, 0x24, 0x00, 0x1d, 0x00, 0x20]);
    bytes[6..].copy_from_slice(public_key);
    bytes
}

fn p256_then_x25519_client_offer(x25519_public_key: &[u8; 32]) -> [u8; 107] {
    let mut bytes = [0u8; 107];
    bytes[..6].copy_from_slice(&[0x00, 0x69, 0x00, 0x17, 0x00, 0x41]);
    bytes[6] = 0x04;
    bytes[71..75].copy_from_slice(&[0x00, 0x1d, 0x00, 0x20]);
    bytes[75..].copy_from_slice(x25519_public_key);
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
        policy(),
        NamedGroupListLimits::new(2).unwrap(),
        KeyShareLimits::new(2).unwrap(),
    )
    .unwrap();
    let final_messages = bind_final_server_key_share_message(&messages).unwrap();
    callback(&final_messages)
}

fn error_kind<T>(result: Result<T, KeyShareBindingError>) -> KeyShareBindingErrorKind {
    match result {
        Ok(_) => panic!("expected key-share binding rejection"),
        Err(error) => error.kind(),
    }
}

#[test]
fn exact_offer_scalar_and_server_share_derive_rfc_secret_and_wipe_output() {
    let policy = policy();
    let offer_bytes = p256_then_x25519_client_offer(&ALICE_PUBLIC);
    let offer = decode_client_key_shares(&offer_bytes, &policy, KeyShareLimits::new(2).unwrap())
        .expect("valid client offer");
    let server_bytes = x25519_server_share(&BOB_PUBLIC);
    let server =
        decode_server_key_share(&server_bytes, &policy, &offer).expect("offered server selection");
    let mut output = [0xa5; 32];
    {
        let secret = derive_bound_x25519_shared_secret(&server, &ALICE_PRIVATE, &mut output)
            .expect("RFC 7748 values must bind and derive");
        assert_eq!(secret.as_bytes(), &SHARED_SECRET);
        assert_eq!(
            secret.selected_group().parameters().group(),
            NamedGroup::X25519
        );
    }
    assert_eq!(output, [0; 32]);
}

#[test]
fn scalar_that_does_not_match_exact_offer_leaves_output_unchanged() {
    let policy = policy();
    let offer_bytes = x25519_client_offer(&ALICE_PUBLIC);
    let offer = decode_client_key_shares(&offer_bytes, &policy, KeyShareLimits::new(2).unwrap())
        .expect("valid client offer");
    let server_bytes = x25519_server_share(&BOB_PUBLIC);
    let server =
        decode_server_key_share(&server_bytes, &policy, &offer).expect("offered server selection");
    let mut output = [0xa5; 32];

    assert_eq!(
        error_kind(derive_bound_x25519_shared_secret(
            &server,
            &[0x11; 32],
            &mut output,
        )),
        KeyShareBindingErrorKind::PrivateScalarDoesNotMatchOffer
    );
    assert_eq!(output, [0xa5; 32]);
}

#[test]
fn unsupported_selected_group_leaves_output_unchanged() {
    let policy = policy();
    let mut offer_bytes = [0u8; 71];
    offer_bytes[..6].copy_from_slice(&[0x00, 0x45, 0x00, 0x17, 0x00, 0x41]);
    offer_bytes[6] = 0x04;
    let offer = decode_client_key_shares(&offer_bytes, &policy, KeyShareLimits::new(2).unwrap())
        .expect("length-valid P-256 offer");
    let mut server_bytes = [0u8; 69];
    server_bytes[..4].copy_from_slice(&[0x00, 0x17, 0x00, 0x41]);
    server_bytes[4] = 0x04;
    let server =
        decode_server_key_share(&server_bytes, &policy, &offer).expect("offered P-256 selection");
    let mut output = [0xa5; 32];

    assert_eq!(
        error_kind(derive_bound_x25519_shared_secret(
            &server,
            &ALICE_PRIVATE,
            &mut output,
        )),
        KeyShareBindingErrorKind::UnsupportedSelectedGroup
    );
    assert_eq!(output, [0xa5; 32]);
}

#[test]
fn low_order_exact_server_share_preserves_downstream_rejection() {
    let policy = policy();
    let offer_bytes = x25519_client_offer(&ALICE_PUBLIC);
    let offer = decode_client_key_shares(&offer_bytes, &policy, KeyShareLimits::new(2).unwrap())
        .expect("valid client offer");
    let server_bytes = x25519_server_share(&[0; 32]);
    let server = decode_server_key_share(&server_bytes, &policy, &offer)
        .expect("length-valid server selection");
    let mut output = [0xa5; 32];

    assert_eq!(
        error_kind(derive_bound_x25519_shared_secret(
            &server,
            &ALICE_PRIVATE,
            &mut output,
        )),
        KeyShareBindingErrorKind::KeyAgreement(KeyAgreementErrorKind::PeerPublicKeyRejected)
    );
    assert_eq!(output, [0xa5; 32]);
}

#[test]
fn public_error_text_contains_only_stable_categories() {
    let policy = policy();
    let offer_bytes = x25519_client_offer(&ALICE_PUBLIC);
    let offer = decode_client_key_shares(&offer_bytes, &policy, KeyShareLimits::new(2).unwrap())
        .expect("valid client offer");
    let server_bytes = x25519_server_share(&BOB_PUBLIC);
    let server =
        decode_server_key_share(&server_bytes, &policy, &offer).expect("offered server selection");
    let mut output = [0xa5; 32];
    let result = derive_bound_x25519_shared_secret(&server, &[0x11; 32], &mut output);
    let error = match result {
        Ok(_) => panic!("mismatched scalar must fail"),
        Err(error) => error,
    };
    assert_eq!(
        format!("{error:?}"),
        "KeyShareBindingError { kind: PrivateScalarDoesNotMatchOffer }"
    );
    assert_eq!(
        error.to_string(),
        "TLS 1.3 X25519 key-share binding error PrivateScalarDoesNotMatchOffer"
    );
}

#[test]
fn exact_retry_messages_remain_bound_to_the_derived_secret_and_output_is_wiped() {
    let mut output = [0xa5; 32];
    with_retry_x25519_messages(|messages| {
        {
            let secret = derive_message_bound_retry_x25519_shared_secret(
                messages,
                &ALICE_PRIVATE,
                &mut output,
            )
            .expect("exact retry messages and RFC 7748 keys must bind");
            assert!(core::ptr::eq(secret.messages(), messages));
            assert_eq!(secret.shared_secret().as_bytes(), &SHARED_SECRET);
            let debug = format!("{secret:?}");
            assert!(debug.contains("[RETAINED]"));
            assert!(debug.contains("[SECRET]"));
            for disclosed in ["retry-x25519", "8520f009", "4a5d9d5b"] {
                assert!(!debug.contains(disclosed));
            }
        }
        assert_eq!(output, [0; 32]);
    });
}

#[test]
fn message_bound_scalar_mismatch_preserves_output_and_existing_error_category() {
    let mut output = [0xa5; 32];
    with_retry_x25519_messages(|messages| {
        assert_eq!(
            error_kind(derive_message_bound_retry_x25519_shared_secret(
                messages,
                &[0x11; 32],
                &mut output,
            )),
            KeyShareBindingErrorKind::PrivateScalarDoesNotMatchOffer
        );
        assert_eq!(output, [0xa5; 32]);
    });
}
