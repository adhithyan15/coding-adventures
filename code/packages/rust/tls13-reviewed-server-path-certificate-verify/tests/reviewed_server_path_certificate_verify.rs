use core::ptr;

use coding_adventures_hkdf::{FixedHkdfHash, HkdfPrk};
use der_asn1::{Asn1Decoder, Asn1Limits};
use http1_request_host_authority::{
    validate_http1_request_host_authority, Http1RequestHostAuthorityLimits,
};
use https_destination_name_binding::bind_https_dns_destination;
use https_reviewed_server_path_binding::{
    bind_https_dns_destination_to_reviewed_server_path,
    bind_https_dns_destination_to_reviewed_server_path_anchor_self_signature,
};
use rsa_pss_sha256_signature::RsaPssSha256SignatureLimits;
use rsa_public_operation::RsaPublicOperationLimits;
use tls13_certificate_message::Tls13CertificateMessageLimits;
use tls13_certificate_verify_input::{CertificateVerifyRole, MAXIMUM_INPUT_LENGTH};
use tls13_certificate_verify_message::CertificateVerifyMessage;
use tls13_certificate_verify_transcript_binding::{
    bind_certificate_verify_transcript, hash_transcript_bound_certificate_verify_input,
    CertificateVerifyFinishedContinuityErrorKind,
};
use tls13_cipher_suite_offer::ClientCipherSuiteOffer;
use tls13_cipher_suite_policy::{
    CipherSuite, CipherSuitePolicy, CipherSuitePolicyEntry, HkdfHash, SelectedCipherSuite,
};
use tls13_client_hello_message::{decode_client_hello, ClientHelloLimits};
use tls13_extension_block::ExtensionBlockLimits;
use tls13_extension_context_policy::{ExtensionContextPolicy, STANDARD_TLS13_EXTENSION_RULES};
use tls13_finished::{derive_finished_key, FinishedKeyStorage, FinishedRole};
use tls13_handshake::{encode_tls13_handshake, Tls13HandshakePolicy, Tls13HandshakeType};
use tls13_handshake_traffic_secrets::HandshakeTrafficSecretStorage;
use tls13_inner_plaintext::Tls13InnerPlaintextPolicy;
use tls13_key_schedule::BaseSecretStorage;
use tls13_key_share::KeyShareLimits;
use tls13_named_group_policy::{NamedGroup, NamedGroupPolicy};
use tls13_no_psk_server_flight::ServerFlightState;
use tls13_record::{Tls13CiphertextPolicy, TlsContentType};
use tls13_record_protection::{seal_tls13_record, Tls13Plaintext};
use tls13_record_protection_state::RecordProtectionState;
use tls13_retry_handshake_traffic_key_material_binding::{
    bind_retry_ordered_server_flight_transcript, bind_retry_server_certificate_leaf,
    BoundRetryServerCertificateLeaf, HandshakeTrafficSender,
    RetryHandshakeRecordProtectionStateIssuer, RetryHandshakeRecordReassembler,
    RetryOrderedServerFlight,
};
use tls13_retry_handshake_traffic_secrets_binding::{
    bind_retry_handshake_traffic_secrets, bind_retry_server_finished_key,
    bind_retry_server_transcript_continuation,
};
use tls13_retry_key_schedule_binding::bind_retry_key_schedule;
use tls13_retry_transcript_binding::bind_retry_transcript;
use tls13_reviewed_server_path_certificate_verify::{
    bind_retry_server_certificate_leaf_to_reviewed_server_authentication,
    verify_https_dns_destination_reviewed_server_path_anchor_self_signature_certificate_verify,
    verify_https_dns_destination_reviewed_server_path_certificate_verify,
    verify_reviewed_server_path_certificate_verify,
    verify_reviewed_server_path_certificate_verify_finished_key_transcript_continuity,
    RetryReviewedServerAuthenticationBindingError, ReviewedServerPathCertificateVerifyErrorKind,
    ReviewedServerPathFinishedKeyTranscriptContinuityErrorKind,
};
use tls13_rsa_pss_signature_routing::Tls13RsaPssSignatureLimits;
use tls13_server_hello_message::{
    decode_tls13_server_hello, Tls13ServerHelloLimits, TLS13_HELLO_RETRY_REQUEST_RANDOM,
};
use tls13_server_hello_retry_selection::{
    bind_final_server_key_share_message, bind_retry_key_share_messages,
    bind_server_hello_retry_selection, BoundRetryServerKeyShareMessages,
};
use tls13_server_hello_selection::bind_server_hello_selection;
use tls13_server_hello_version_selection::bind_server_hello_version_selection;
use tls13_server_name::TlsServerName;
use tls13_signature_scheme_policy::{
    AuthorizedSignatureScheme, SignatureScheme, SignatureSchemePolicy,
};
use tls13_supported_groups::NamedGroupListLimits;
use tls13_supported_versions::ClientVersionOffer;
use tls13_traffic_key_material::{derive_traffic_key_material, TrafficKeyMaterialStorage};
use tls13_transcript::{PendingTls13Transcript, SelectedTls13Transcript, TranscriptPolicy};
use tls13_x25519_key_share_binding::derive_message_bound_retry_x25519_shared_secret;
use tls_server_identity::ReferenceIdentity;
use x509_certificate::{decode_x509_certificate, X509Certificate};
use x509_certificate_path_construction::construct_unique_x509_certificate_path_with_name_policy;
use x509_certificate_path_reviewed_anchor_binding::{
    bind_name_policy_constructed_x509_certificate_path_to_reviewed_anchor,
    bind_name_policy_reviewed_constructed_x509_certificate_path_to_anchor_role_policy,
    initialize_name_policy_reviewed_constructed_x509_certificate_path_length,
};
use x509_certificate_policy::{
    classify_certificate_self_issued, validate_certificate_algorithm_policy,
    validate_certificate_role_policy, ValidatedCertificateAlgorithmPolicy,
    X509AlgorithmParametersPolicy, X509CertificateAlgorithmProfile, X509CertificatePurposePolicy,
    X509CertificateRole,
};
use x509_intermediate_certificate_policy::validate_x509_intermediate_certificate_policy;
use x509_name_value_policy::{
    X509NameValueComparisonRule, X509NameValuePolicy, X509NameValuePolicyEntry,
};
use x509_rsa_algorithm_policy::{
    validate_x509_rsa_algorithm_policy, ValidatedX509RsaAlgorithmPolicy,
};
use x509_rsa_constructed_path_signature_routing::{
    accumulate_name_policy_reviewed_x509_rsa_constructed_path_intermediate_path_lengths,
    bind_accumulated_name_policy_reviewed_x509_rsa_constructed_path_target_signature,
    bind_x509_rsa_constructed_path_terminal_to_reviewed_anchor,
    validate_accumulated_name_policy_reviewed_x509_rsa_constructed_path_subordinate_anchor_name_constraints_policy,
    validate_accumulated_name_policy_reviewed_x509_rsa_constructed_path_subordinate_intermediate_name_constraints_policy,
    validate_accumulated_name_policy_reviewed_x509_rsa_constructed_path_target_anchor_policy,
    validate_accumulated_name_policy_reviewed_x509_rsa_constructed_path_target_intermediate_name_constraints_policy,
    verify_x509_rsa_constructed_path_policy_signature,
};
use x509_rsa_public_key::{validate_rsa_public_key, RsaPublicKeyPolicy};
use x509_rsa_reviewed_anchor_self_signature_binding::bind_x509_rsa_reviewed_anchor_self_signature;
use x509_rsa_reviewed_server_path_anchor_self_signature_binding::bind_x509_rsa_reviewed_server_path_anchor_self_signature;
use x509_rsa_signature_routing::{
    verify_x509_rsa_policy_signature, X509RsaSignatureRoute, X509RsaSignatureVerifier,
};
use x509_server_certificate_policy::validate_x509_server_certificate_policy;
use x509_time::{decode_x509_time, X509Time};
use x509_webpki_root_anchor_selection::select_reviewed_webpki_root_anchor;

const LEAF_HEX: &str = include_str!("fixtures/apple_www_2026_11_11_leaf.der.hex");
const INTERMEDIATE_HEX: &str = include_str!(
    "../../x509-ocsp-reviewed-server-revocation-binding/tests/fixtures/apple-ev-intermediate.der.hex"
);
const ROOT_HEX: &str = include_str!(
    "../../x509-ocsp-reviewed-server-revocation-binding/tests/fixtures/digicert-global-root-g2.der.hex"
);
const TRANSCRIPT_HEX: &str =
    include_str!("fixtures/apple_www_2026_11_18_transcript_through_certificate.hex");
const SIGNATURE_HEX: &str =
    include_str!("fixtures/apple_www_2026_11_18_certificate_verify_signature.hex");

const SHA256_WITH_RSA_ENCRYPTION_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 11];
const RSA_ENCRYPTION_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 1];
const SERVER_AUTH_OID: &[u64] = &[1, 3, 6, 1, 5, 5, 7, 3, 1];
const COUNTRY_NAME_OID: &[u64] = &[2, 5, 4, 6];
const ORGANIZATION_NAME_OID: &[u64] = &[2, 5, 4, 10];
const ORGANIZATIONAL_UNIT_NAME_OID: &[u64] = &[2, 5, 4, 11];
const COMMON_NAME_OID: &[u64] = &[2, 5, 4, 3];
const NULL_DER: &[u8] = &[0x05, 0x00];
const ALLOWED_EXPONENTS: &[u64] = &[65_537];
const RETRY_ALICE_PRIVATE: [u8; 32] = [
    0x77, 0x07, 0x6d, 0x0a, 0x73, 0x18, 0xa5, 0x7d, 0x3c, 0x16, 0xc1, 0x72, 0x51, 0xb2, 0x66, 0x45,
    0xdf, 0x4c, 0x2f, 0x87, 0xeb, 0xc0, 0x99, 0x2a, 0xb1, 0x77, 0xfb, 0xa5, 0x1d, 0xb9, 0x2c, 0x2a,
];
const RETRY_ALICE_PUBLIC: [u8; 32] = [
    0x85, 0x20, 0xf0, 0x09, 0x89, 0x30, 0xa7, 0x54, 0x74, 0x8b, 0x7d, 0xdc, 0xb4, 0x3e, 0xf7, 0x5a,
    0x0d, 0xbf, 0x3a, 0x0d, 0x26, 0x38, 0x1a, 0xf4, 0xeb, 0xa4, 0xa9, 0x8e, 0xaa, 0x9b, 0x4e, 0x6a,
];
const RETRY_BOB_PUBLIC: [u8; 32] = [
    0xde, 0x9e, 0xdb, 0x7d, 0x7b, 0x7d, 0xc1, 0xb4, 0xd3, 0x5b, 0x61, 0xc2, 0xec, 0xe4, 0x35, 0x37,
    0x3f, 0x83, 0x43, 0xc8, 0x5b, 0x78, 0x67, 0x4d, 0xad, 0xfc, 0x7e, 0x14, 0x6f, 0x88, 0x2b, 0x4f,
];
const RETRY_SESSION_ID: &[u8] = b"reviewed-server-auth";
const RETRY_CLIENT_RANDOM: [u8; 32] = [0x11; 32];
const RETRY_FINAL_SERVER_RANDOM: [u8; 32] = [0x22; 32];
const TLS13_VERSION: u16 = 0x0304;

fn fixture(hexadecimal: &str) -> Vec<u8> {
    let digits = hexadecimal
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect::<Vec<_>>();
    assert!(digits.len().is_multiple_of(2));
    digits
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn decode(input: &[u8]) -> X509Certificate<'_> {
    decode_x509_certificate(input, Asn1Limits::default()).unwrap()
}

fn retry_u24(value: usize) -> [u8; 3] {
    assert!(value <= 0x00ff_ffff);
    [
        ((value >> 16) & 0xff) as u8,
        ((value >> 8) & 0xff) as u8,
        (value & 0xff) as u8,
    ]
}

fn retry_fixed_hash(suite: SelectedCipherSuite) -> FixedHkdfHash {
    match suite.parameters().hkdf_hash() {
        HkdfHash::Sha256 => FixedHkdfHash::Sha256,
        HkdfHash::Sha384 => FixedHkdfHash::Sha384,
    }
}

fn retry_group_policy() -> NamedGroupPolicy {
    NamedGroupPolicy::new(&[NamedGroup::X25519, NamedGroup::Secp256r1]).unwrap()
}

fn retry_x25519_client_offer(public_key: &[u8; 32]) -> [u8; 38] {
    let mut bytes = [0_u8; 38];
    bytes[..6].copy_from_slice(&[0x00, 0x24, 0x00, 0x1d, 0x00, 0x20]);
    bytes[6..].copy_from_slice(public_key);
    bytes
}

fn retry_x25519_server_share(public_key: &[u8; 32]) -> [u8; 36] {
    let mut bytes = [0_u8; 36];
    bytes[..4].copy_from_slice(&[0x00, 0x1d, 0x00, 0x20]);
    bytes[4..].copy_from_slice(public_key);
    bytes
}

fn retry_append_extensions(body: &mut Vec<u8>, extensions: &[(u16, &[u8])]) {
    let payload_length = extensions
        .iter()
        .map(|(_, data)| 4 + data.len())
        .sum::<usize>();
    body.extend_from_slice(&(payload_length as u16).to_be_bytes());
    for (extension_type, data) in extensions {
        body.extend_from_slice(&extension_type.to_be_bytes());
        body.extend_from_slice(&(data.len() as u16).to_be_bytes());
        body.extend_from_slice(data);
    }
}

fn retry_client_body(extensions: &[(u16, &[u8])]) -> Vec<u8> {
    let mut body = vec![0x03, 0x03];
    body.extend_from_slice(&RETRY_CLIENT_RANDOM);
    body.push(RETRY_SESSION_ID.len() as u8);
    body.extend_from_slice(RETRY_SESSION_ID);
    body.extend_from_slice(&[0x00, 0x02]);
    body.extend_from_slice(&CipherSuite::TlsAes128GcmSha256.wire_id().to_be_bytes());
    body.extend_from_slice(&[0x01, 0x00]);
    let version_data = [0x02, 0x03, 0x04];
    let mut all_extensions = vec![(43, version_data.as_slice())];
    all_extensions.extend_from_slice(extensions);
    retry_append_extensions(&mut body, &all_extensions);
    body
}

fn retry_server_body(random: [u8; 32], extensions: &[(u16, &[u8])]) -> Vec<u8> {
    let mut body = vec![0x03, 0x03];
    body.extend_from_slice(&random);
    body.push(RETRY_SESSION_ID.len() as u8);
    body.extend_from_slice(RETRY_SESSION_ID);
    body.extend_from_slice(&CipherSuite::TlsAes128GcmSha256.wire_id().to_be_bytes());
    body.push(0x00);
    let version_data = TLS13_VERSION.to_be_bytes();
    let mut all_extensions = vec![(43, version_data.as_slice())];
    all_extensions.extend_from_slice(extensions);
    retry_append_extensions(&mut body, &all_extensions);
    body
}

fn retry_version_selection<'client, 'server>(
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
    static VERSIONS: [u16; 1] = [TLS13_VERSION];
    let client = decode_client_hello(
        client_bytes,
        ClientHelloLimits::new(4, ExtensionBlockLimits::new(256, 5).unwrap()).unwrap(),
    )
    .unwrap();
    let server = decode_tls13_server_hello(
        server_bytes,
        Tls13ServerHelloLimits::new(256, 256, 4).unwrap(),
    )
    .unwrap();
    let entries = [CipherSuitePolicyEntry::new(CipherSuite::TlsAes128GcmSha256, 1_000).unwrap()];
    let offer =
        ClientCipherSuiteOffer::new(&AES128_SUITE, CipherSuitePolicy::new(&entries).unwrap())
            .unwrap();
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
    let mut initial_share = [0_u8; 71];
    initial_share[..6].copy_from_slice(&[0x00, 0x45, 0x00, 0x17, 0x00, 0x41]);
    initial_share[6..].copy_from_slice(&p256_key);
    let retry_share = [0x00, 0x1d];
    let retried_share = retry_x25519_client_offer(&RETRY_ALICE_PUBLIC);
    let final_share = retry_x25519_server_share(&RETRY_BOB_PUBLIC);
    let signature_algorithms = [0, 2, 8, 4];
    let first_client = retry_client_body(&[
        (5, &[]),
        (10, &supported_groups),
        (13, &signature_algorithms),
        (51, &initial_share),
    ]);
    let retry_server = retry_server_body(TLS13_HELLO_RETRY_REQUEST_RANDOM, &[(51, &retry_share)]);
    let second_client = retry_client_body(&[
        (5, &[]),
        (10, &supported_groups),
        (13, &signature_algorithms),
        (51, &retried_share),
    ]);
    let final_server = retry_server_body(RETRY_FINAL_SERVER_RANDOM, &[(51, &final_share)]);
    let selection = bind_server_hello_retry_selection(
        retry_version_selection(&first_client, &retry_server),
        retry_version_selection(&second_client, &final_server),
    )
    .unwrap();
    let messages = bind_retry_key_share_messages(
        selection,
        retry_group_policy(),
        NamedGroupListLimits::new(2).unwrap(),
        KeyShareLimits::new(2).unwrap(),
    )
    .unwrap();
    let final_messages = bind_final_server_key_share_message(&messages).unwrap();
    callback(&final_messages)
}

fn with_authenticated_retry_server_certificate_leaf<'certificate, 'encoded, T>(
    leaf_der: &'encoded [u8],
    leaf: &'certificate X509Certificate<'encoded>,
    callback: impl for<'binding, 'pending, 'message, 'retry_evidence, 'issuer> FnOnce(
        &BoundRetryServerCertificateLeaf<
            'binding,
            'pending,
            'message,
            'retry_evidence,
            'issuer,
            'certificate,
            'encoded,
        >,
    ) -> T,
) -> T {
    let mut x25519_output = [0xa5; 32];
    with_retry_x25519_messages(|messages| {
        let secret = derive_message_bound_retry_x25519_shared_secret(
            messages,
            &RETRY_ALICE_PRIVATE,
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
        let issuer = RetryHandshakeRecordProtectionStateIssuer::new(traffic);
        let mut finished_storage = FinishedKeyStorage::new(suite, FinishedRole::Server);
        let server_finished =
            bind_retry_server_finished_key(issuer.traffic_secrets(), &mut finished_storage)
                .unwrap();
        let continuation = bind_retry_server_transcript_continuation(&server_finished).unwrap();
        let mut server_storage = TrafficKeyMaterialStorage::new(suite).unwrap();
        let server_state = issuer
            .issue(HandshakeTrafficSender::Server, &mut server_storage)
            .unwrap();
        let handshake_policy = Tls13HandshakePolicy::new(
            &[
                Tls13HandshakeType::EncryptedExtensions,
                Tls13HandshakeType::Certificate,
            ],
            4_096,
        )
        .unwrap();
        let mut reassembly_storage = [0xa5; 4_096];
        let reassembler = RetryHandshakeRecordReassembler::new(
            server_state,
            handshake_policy,
            &mut reassembly_storage,
        )
        .unwrap();
        let ordered = RetryOrderedServerFlight::new(reassembler).unwrap();
        let mut bound = bind_retry_ordered_server_flight_transcript(ordered, continuation).unwrap();

        let server_prk = HkdfPrk::import_exact(
            retry_fixed_hash(suite),
            issuer.traffic_secrets().traffic_secrets().server_secret(),
        )
        .unwrap();
        let mut peer_storage = TrafficKeyMaterialStorage::new(suite).unwrap();
        let peer_material = derive_traffic_key_material(&server_prk, &mut peer_storage).unwrap();
        let mut peer_state = RecordProtectionState::new(peer_material).unwrap();

        let encoded_extensions = [0, 0];
        let certificate_list_length = 3 + leaf_der.len() + encoded_extensions.len();
        let mut certificate_body = Vec::with_capacity(4 + certificate_list_length);
        certificate_body.push(0);
        certificate_body.extend_from_slice(&retry_u24(certificate_list_length));
        certificate_body.extend_from_slice(&retry_u24(leaf_der.len()));
        certificate_body.extend_from_slice(leaf_der);
        certificate_body.extend_from_slice(&encoded_extensions);
        let mut plaintext = Vec::with_capacity(10 + certificate_body.len());
        plaintext.extend_from_slice(&[8, 0, 0, 2, 0, 0]);
        plaintext.push(11);
        plaintext.extend_from_slice(&retry_u24(certificate_body.len()));
        plaintext.extend_from_slice(&certificate_body);

        let ciphertext_policy = Tls13CiphertextPolicy::new(8_192).unwrap();
        let inner_policy =
            Tls13InnerPlaintextPolicy::new(&[TlsContentType::Handshake], 8_176).unwrap();
        let mut scratch = [0xa5; 4_096];
        let mut wire = [0xcc; 4_096];
        let used = seal_tls13_record(
            &mut peer_state,
            &ciphertext_policy,
            &inner_policy,
            Tls13Plaintext::new(TlsContentType::Handshake, &plaintext, 0),
            &mut scratch,
            &mut wire,
        )
        .unwrap();
        let mut opened = [0x5a; 4_096];
        let mut callback = Some(callback);
        let mut output = None;
        let result = bound
            .open_accept_and_advance(
                &ciphertext_policy,
                &inner_policy,
                &wire[..used],
                &mut opened,
                |pending| match pending.message().handshake_type() {
                    Tls13HandshakeType::EncryptedExtensions => {
                        pending
                            .validate_encrypted_extensions(
                                ExtensionBlockLimits::new(64, 4).unwrap(),
                            )
                            .unwrap();
                        true
                    }
                    Tls13HandshakeType::Certificate => {
                        let proof = pending
                            .validate_server_certificate(
                                Tls13CertificateMessageLimits::new(
                                    certificate_body.len(),
                                    0,
                                    certificate_list_length,
                                    1,
                                    leaf_der.len(),
                                    16,
                                ),
                                ExtensionBlockLimits::new(16, 4).unwrap(),
                            )
                            .unwrap();
                        let leaf_proof = bind_retry_server_certificate_leaf(&proof, leaf).unwrap();
                        output = Some(callback.take().unwrap()(&leaf_proof));
                        true
                    }
                    other => panic!("unexpected handshake type {other:?}"),
                },
            )
            .unwrap();
        assert_eq!(result.completed_messages(), 2);
        assert_eq!(result.state(), ServerFlightState::ExpectCertificateVerify);
        output.expect("authenticated Certificate callback must run")
    })
}

fn observed_time() -> X509Time {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    decode_x509_time(decoder.decode_exact(b"\x17\x0d261118000000Z").unwrap()).unwrap()
}

fn algorithm_profiles() -> [X509CertificateAlgorithmProfile<'static>; 1] {
    [X509CertificateAlgorithmProfile::new(
        SHA256_WITH_RSA_ENCRYPTION_OID,
        X509AlgorithmParametersPolicy::ExactDer(NULL_DER),
        RSA_ENCRYPTION_OID,
        X509AlgorithmParametersPolicy::ExactDer(NULL_DER),
        2_048,
        8_192,
        2_048,
        4_096,
    )
    .unwrap()]
}

fn algorithm_policy<'certificate, 'encoded>(
    certificate: &'certificate X509Certificate<'encoded>,
    profiles: &[X509CertificateAlgorithmProfile<'static>],
) -> ValidatedCertificateAlgorithmPolicy<'certificate, 'encoded> {
    validate_certificate_algorithm_policy(certificate, profiles).unwrap()
}

fn rsa_policy<'certificate, 'encoded>(
    certificate: &'certificate X509Certificate<'encoded>,
    profiles: &[X509CertificateAlgorithmProfile<'static>],
) -> ValidatedX509RsaAlgorithmPolicy<'certificate, 'encoded> {
    validate_x509_rsa_algorithm_policy(
        algorithm_policy(certificate, profiles),
        RsaPublicKeyPolicy::new(2_048, 4_096, ALLOWED_EXPONENTS).unwrap(),
        Asn1Limits::default(),
    )
    .unwrap()
}

fn issuing_ca_role_policy<'certificate, 'encoded>(
    certificate: &'certificate X509Certificate<'encoded>,
) -> x509_certificate_policy::ValidatedCertificateRolePolicy<'certificate, 'encoded> {
    validate_certificate_role_policy(
        certificate,
        X509CertificateRole::IssuingCertificateAuthority,
        Asn1Limits::default(),
        |_| true,
    )
    .unwrap()
}

fn route() -> X509RsaSignatureRoute {
    X509RsaSignatureRoute::new(0, X509RsaSignatureVerifier::RsaPkcs1V15Sha256)
}

fn operation_limits() -> RsaPublicOperationLimits {
    RsaPublicOperationLimits::new(512).unwrap()
}

fn server_policy<'certificate, 'encoded>(
    certificate: &'certificate X509Certificate<'encoded>,
) -> x509_server_certificate_policy::ValidatedX509ServerCertificatePolicy<'certificate, 'encoded> {
    let role_policy = validate_certificate_role_policy(
        certificate,
        X509CertificateRole::EndEntity,
        Asn1Limits::default(),
        |oid| oid.equals(&[2, 5, 29, 15]) || oid.equals(&[2, 5, 29, 19]),
    )
    .unwrap();
    let profiles = algorithm_profiles();
    let algorithm_policy = validate_certificate_algorithm_policy(certificate, &profiles).unwrap();
    let reference = ReferenceIdentity::parse("www.apple.com").unwrap();
    let purpose = X509CertificatePurposePolicy::new(SERVER_AUTH_OID).unwrap();
    validate_x509_server_certificate_policy(
        role_policy,
        algorithm_policy,
        &reference,
        &observed_time(),
        purpose,
        Asn1Limits::default(),
    )
    .unwrap()
}

fn intermediate_policy<'certificate, 'encoded>(
    certificate: &'certificate X509Certificate<'encoded>,
) -> x509_intermediate_certificate_policy::ValidatedX509IntermediateCertificatePolicy<
    'certificate,
    'encoded,
> {
    let role_policy = validate_certificate_role_policy(
        certificate,
        X509CertificateRole::IssuingCertificateAuthority,
        Asn1Limits::default(),
        |oid| oid.equals(&[2, 5, 29, 15]) || oid.equals(&[2, 5, 29, 19]),
    )
    .unwrap();
    let profiles = algorithm_profiles();
    let algorithm_policy = validate_certificate_algorithm_policy(certificate, &profiles).unwrap();
    let self_issued = classify_certificate_self_issued(certificate, |issuer, subject| {
        issuer.encoded() == subject.encoded()
    });
    validate_x509_intermediate_certificate_policy(
        role_policy,
        algorithm_policy,
        self_issued,
        &observed_time(),
    )
    .unwrap()
}

fn selected_suite() -> SelectedCipherSuite {
    let entry = CipherSuitePolicyEntry::new(CipherSuite::TlsAes256GcmSha384, 8_192).unwrap();
    CipherSuitePolicy::new(&[entry])
        .unwrap()
        .select(CipherSuite::TlsAes256GcmSha384.wire_id())
        .unwrap()
}

fn selected_transcript(encoded: &[u8]) -> SelectedTls13Transcript {
    let mut pending = PendingTls13Transcript::new(TranscriptPolicy::new(8_192).unwrap());
    pending.update(encoded).unwrap();
    pending.select(selected_suite())
}

fn authorized_scheme() -> AuthorizedSignatureScheme {
    SignatureSchemePolicy::new(&[SignatureScheme::RsaPssRsaeSha256])
        .unwrap()
        .select(SignatureScheme::RsaPssRsaeSha256.wire_id())
        .unwrap()
}

macro_rules! reviewed_server_path {
    ($validated:ident, $target:expr, $candidates:expr) => {
        let name_policy_entries = [
            X509NameValuePolicyEntry::new(
                COUNTRY_NAME_OID,
                X509NameValueComparisonRule::ExactDer,
            ),
            X509NameValuePolicyEntry::new(
                ORGANIZATION_NAME_OID,
                X509NameValueComparisonRule::ExactDer,
            ),
            X509NameValuePolicyEntry::new(
                ORGANIZATIONAL_UNIT_NAME_OID,
                X509NameValueComparisonRule::ExactDer,
            ),
            X509NameValuePolicyEntry::new(
                COMMON_NAME_OID,
                X509NameValueComparisonRule::ExactDer,
            ),
        ];
        let name_policy = X509NameValuePolicy::new(&name_policy_entries).unwrap();
        let constructed = construct_unique_x509_certificate_path_with_name_policy(
            $target,
            $candidates,
            2,
            3,
            |certificate| ptr::eq(certificate, &$candidates[1]),
            name_policy,
        )
        .unwrap();
        let reviewed = bind_name_policy_constructed_x509_certificate_path_to_reviewed_anchor(
            constructed,
            select_reviewed_webpki_root_anchor(&$candidates[1]).unwrap(),
        )
        .unwrap();
        let reviewed_role =
            bind_name_policy_reviewed_constructed_x509_certificate_path_to_anchor_role_policy(
                reviewed,
                issuing_ca_role_policy(&$candidates[1]),
            )
            .unwrap();
        let initialized =
            initialize_name_policy_reviewed_constructed_x509_certificate_path_length(
                reviewed_role,
            );
        let retained_path = initialized
            .reviewed_path()
            .reviewed_path()
            .constructed_path_with_name_policy()
            .constructed_path();
        let profiles = algorithm_profiles();
        let terminal_signature = verify_x509_rsa_constructed_path_policy_signature(
            retained_path.adjacent_link(1).unwrap(),
            algorithm_policy(&$candidates[0], &profiles),
            issuing_ca_role_policy(&$candidates[1]),
            rsa_policy(&$candidates[1], &profiles),
            &[route()],
            operation_limits(),
        )
        .unwrap();
        let reviewed_terminal = bind_x509_rsa_constructed_path_terminal_to_reviewed_anchor(
            terminal_signature,
            select_reviewed_webpki_root_anchor(&$candidates[1]).unwrap(),
        )
        .unwrap();
        let intermediate_policies = [intermediate_policy(&$candidates[0])];
        let accumulated =
            accumulate_name_policy_reviewed_x509_rsa_constructed_path_intermediate_path_lengths(
                &initialized,
                &reviewed_terminal,
                &[],
                &intermediate_policies,
                1,
            )
            .unwrap();
        let target_signature = verify_x509_rsa_constructed_path_policy_signature(
            retained_path.adjacent_link(0).unwrap(),
            algorithm_policy($target, &profiles),
            issuing_ca_role_policy(&$candidates[0]),
            rsa_policy(&$candidates[0], &profiles),
            &[route()],
            operation_limits(),
        )
        .unwrap();
        let target_closure =
            bind_accumulated_name_policy_reviewed_x509_rsa_constructed_path_target_signature(
                accumulated,
                target_signature,
                server_policy($target),
            )
            .unwrap();
        let anchor_policy =
            validate_accumulated_name_policy_reviewed_x509_rsa_constructed_path_target_anchor_policy(
                target_closure,
                Asn1Limits::default(),
                Asn1Limits::default(),
            )
            .unwrap();
        let target_name_policy =
            validate_accumulated_name_policy_reviewed_x509_rsa_constructed_path_target_intermediate_name_constraints_policy(
                anchor_policy,
                Asn1Limits::default(),
                Asn1Limits::default(),
            )
            .unwrap();
        let subordinate_policy =
            validate_accumulated_name_policy_reviewed_x509_rsa_constructed_path_subordinate_intermediate_name_constraints_policy(
                target_name_policy,
                Asn1Limits::default(),
                Asn1Limits::default(),
            )
            .unwrap();
        let $validated =
            validate_accumulated_name_policy_reviewed_x509_rsa_constructed_path_subordinate_anchor_name_constraints_policy(
                subordinate_policy,
                Asn1Limits::default(),
                Asn1Limits::default(),
            )
            .unwrap();
    };
}

fn exercise_public_composition(role: CertificateVerifyRole, expect_success: bool) {
    let leaf_encoded = fixture(LEAF_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let leaf = decode(&leaf_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    reviewed_server_path!(reviewed_path, &leaf, &candidates);

    let transcript_encoded = fixture(TRANSCRIPT_HEX);
    let transcript = selected_transcript(&transcript_encoded);
    let mut output = [0xa5; MAXIMUM_INPUT_LENGTH + 7];
    let signature = fixture(SIGNATURE_HEX);
    let mut scratch = [0xa5; 223];
    let before_scratch = scratch;
    let expected_input;
    {
        let bound = bind_certificate_verify_transcript(&transcript, role, &mut output).unwrap();
        let hashed = hash_transcript_bound_certificate_verify_input(authorized_scheme(), &bound);
        let message =
            CertificateVerifyMessage::new(SignatureScheme::RsaPssRsaeSha256.wire_id(), &signature)
                .unwrap();
        expected_input = bound.input().as_bytes().to_vec();
        let result = verify_reviewed_server_path_certificate_verify(
            &reviewed_path,
            &hashed,
            &message,
            RsaPublicKeyPolicy::new(2_048, 2_048, ALLOWED_EXPONENTS).unwrap(),
            Asn1Limits::default(),
            &mut scratch,
            Tls13RsaPssSignatureLimits::Sha256(RsaPssSha256SignatureLimits::new(300, 300).unwrap()),
        );

        if expect_success {
            let verified =
                result.expect("captured server signature must verify with reviewed leaf");
            let expected_key = validate_rsa_public_key(
                leaf.tbs_certificate().subject_public_key_info(),
                RsaPublicKeyPolicy::new(2_048, 2_048, ALLOWED_EXPONENTS).unwrap(),
                Asn1Limits::default(),
            )
            .unwrap();
            assert!(ptr::eq(verified.reviewed_server_path(), &reviewed_path));
            assert!(ptr::eq(verified.verification().hashed_input(), &hashed));
            assert!(ptr::eq(
                verified.verification().verification().signature().message(),
                &message,
            ));
            assert_eq!(
                verified
                    .verification()
                    .verification()
                    .signature()
                    .public_key()
                    .modulus(),
                expected_key.modulus(),
            );
        } else {
            assert_eq!(
                result.unwrap_err().kind(),
                ReviewedServerPathCertificateVerifyErrorKind::WrongEndpointRole,
            );
        }
    }
    assert_eq!(&output[..expected_input.len()], expected_input);
    assert!(output[expected_input.len()..]
        .iter()
        .all(|byte| *byte == 0xa5));
    if !expect_success {
        assert_eq!(scratch, before_scratch);
    }
}

#[test]
fn public_composition_verifies_the_captured_signature_with_the_exact_reviewed_leaf() {
    exercise_public_composition(CertificateVerifyRole::Server, true);
}

#[test]
fn public_composition_rejects_client_role_before_output_or_scratch_mutation() {
    exercise_public_composition(CertificateVerifyRole::Client, false);
}

fn certificate_verify_body(signature: &[u8]) -> Vec<u8> {
    let message =
        CertificateVerifyMessage::new(SignatureScheme::RsaPssRsaeSha256.wire_id(), signature)
            .unwrap();
    let mut body = vec![0; message.encoded_length()];
    message.encode(&mut body).unwrap();
    body
}

fn server_finished_verify_data(
    transcript_encoded: &[u8],
    certificate_verify_body: &[u8],
    key: &tls13_finished::FinishedKey<'_>,
) -> Vec<u8> {
    let mut transcript = selected_transcript(transcript_encoded);
    let policy = Tls13HandshakePolicy::new(
        &[Tls13HandshakeType::CertificateVerify],
        certificate_verify_body.len(),
    )
    .unwrap();
    let mut encoding = vec![0; certificate_verify_body.len() + 4];
    let encoded_length = encode_tls13_handshake(
        &policy,
        Tls13HandshakeType::CertificateVerify,
        certificate_verify_body,
        &mut encoding,
    )
    .unwrap();
    transcript.update(&encoding[..encoded_length]).unwrap();
    let snapshot = transcript.snapshot();
    let mut verify_data = vec![0; snapshot.as_bytes().len()];
    key.write_verify_data(snapshot.as_bytes(), &mut verify_data)
        .unwrap();
    verify_data
}

#[test]
fn reviewed_path_finished_key_continuity_retains_path_key_and_exact_snapshot() {
    let leaf_encoded = fixture(LEAF_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let leaf = decode(&leaf_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    reviewed_server_path!(reviewed_path, &leaf, &candidates);

    let transcript_encoded = fixture(TRANSCRIPT_HEX);
    let signature = fixture(SIGNATURE_HEX);
    let certificate_verify_body = certificate_verify_body(&signature);
    let prk = HkdfPrk::import_exact(FixedHkdfHash::Sha384, &[0x11; 48]).unwrap();
    let mut finished_storage = FinishedKeyStorage::new(selected_suite(), FinishedRole::Server);
    let finished_key = derive_finished_key(&prk, &mut finished_storage).unwrap();
    let verify_data =
        server_finished_verify_data(&transcript_encoded, &certificate_verify_body, &finished_key);
    let mut signed_input = [0xa5; MAXIMUM_INPUT_LENGTH + 7];
    let mut handshake_output = vec![0xa5; certificate_verify_body.len() + 8];
    let mut scratch = [0xa5; 223];

    let verified =
        verify_reviewed_server_path_certificate_verify_finished_key_transcript_continuity(
            &reviewed_path,
            selected_transcript(&transcript_encoded),
            authorized_scheme(),
            &certificate_verify_body,
            RsaPublicKeyPolicy::new(2_048, 2_048, ALLOWED_EXPONENTS).unwrap(),
            Asn1Limits::default(),
            &mut signed_input,
            &mut handshake_output,
            &mut scratch,
            Tls13RsaPssSignatureLimits::Sha256(RsaPssSha256SignatureLimits::new(300, 300).unwrap()),
            &finished_key,
            &verify_data,
        )
        .expect("reviewed leaf signature and exact following Finished must verify");

    let expected_key = validate_rsa_public_key(
        leaf.tbs_certificate().subject_public_key_info(),
        RsaPublicKeyPolicy::new(2_048, 2_048, ALLOWED_EXPONENTS).unwrap(),
        Asn1Limits::default(),
    )
    .unwrap();
    assert!(ptr::eq(verified.reviewed_server_path(), &reviewed_path));
    assert_eq!(
        verified.continuity().certificate_verify_body(),
        certificate_verify_body,
    );
    assert_eq!(verified.continuity().role(), CertificateVerifyRole::Server);
    assert_eq!(
        verified.continuity().public_key().modulus(),
        expected_key.modulus(),
    );
    assert_eq!(
        verified.continuity().after_certificate_verify_snapshot(),
        verified.continuity().finished().snapshot(),
    );
    assert_eq!(
        verified.continuity().finished().transcript().snapshot(),
        verified.continuity().finished().snapshot(),
    );
    assert_eq!(
        format!("{verified:?}"),
        "VerifiedReviewedServerPathCertificateVerifyFinishedKeyTranscriptContinuity",
    );
}

#[test]
fn reviewed_path_finished_key_continuity_rejects_invalid_verify_data_without_state() {
    let leaf_encoded = fixture(LEAF_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let leaf = decode(&leaf_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    reviewed_server_path!(reviewed_path, &leaf, &candidates);

    let transcript_encoded = fixture(TRANSCRIPT_HEX);
    let signature = fixture(SIGNATURE_HEX);
    let certificate_verify_body = certificate_verify_body(&signature);
    let prk = HkdfPrk::import_exact(FixedHkdfHash::Sha384, &[0x11; 48]).unwrap();
    let mut finished_storage = FinishedKeyStorage::new(selected_suite(), FinishedRole::Server);
    let finished_key = derive_finished_key(&prk, &mut finished_storage).unwrap();
    let mut verify_data =
        server_finished_verify_data(&transcript_encoded, &certificate_verify_body, &finished_key);
    verify_data[0] ^= 0x80;
    let mut signed_input = [0xa5; MAXIMUM_INPUT_LENGTH + 7];
    let mut handshake_output = vec![0xa5; certificate_verify_body.len() + 8];
    let mut scratch = [0xa5; 223];

    let error = verify_reviewed_server_path_certificate_verify_finished_key_transcript_continuity(
        &reviewed_path,
        selected_transcript(&transcript_encoded),
        authorized_scheme(),
        &certificate_verify_body,
        RsaPublicKeyPolicy::new(2_048, 2_048, ALLOWED_EXPONENTS).unwrap(),
        Asn1Limits::default(),
        &mut signed_input,
        &mut handshake_output,
        &mut scratch,
        Tls13RsaPssSignatureLimits::Sha256(RsaPssSha256SignatureLimits::new(300, 300).unwrap()),
        &finished_key,
        &verify_data,
    )
    .expect_err("invalid Finished must consume and withhold transcript state");

    assert!(matches!(
        error.kind(),
        ReviewedServerPathFinishedKeyTranscriptContinuityErrorKind::Finished(
            CertificateVerifyFinishedContinuityErrorKind::Finished(_)
        )
    ));
    let display = error.to_string();
    let debug = format!("{error:?}");
    for secret in ["www.apple.com", "signature", "modulus"] {
        assert!(!display.contains(secret));
        assert!(!debug.contains(secret));
    }
}

fn exercise_https_destination_composition(role: CertificateVerifyRole, expect_success: bool) {
    let leaf_encoded = fixture(LEAF_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let leaf = decode(&leaf_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    reviewed_server_path!(reviewed_path, &leaf, &candidates);

    let server_name = TlsServerName::new("www.apple.com").unwrap();
    let host = validate_http1_request_host_authority(
        b"WWW.APPLE.COM",
        Some(443),
        Http1RequestHostAuthorityLimits::new(253, 259).unwrap(),
    )
    .unwrap();
    let destination =
        bind_https_dns_destination(&leaf, server_name, host, Asn1Limits::default()).unwrap();
    let destination_path =
        bind_https_dns_destination_to_reviewed_server_path(destination, reviewed_path).unwrap();

    let transcript_encoded = fixture(TRANSCRIPT_HEX);
    let transcript = selected_transcript(&transcript_encoded);
    let mut output = [0xa5; MAXIMUM_INPUT_LENGTH + 7];
    let signature = fixture(SIGNATURE_HEX);
    let mut scratch = [0xa5; 223];
    let before_scratch = scratch;
    let expected_input;
    {
        let bound = bind_certificate_verify_transcript(&transcript, role, &mut output).unwrap();
        let hashed = hash_transcript_bound_certificate_verify_input(authorized_scheme(), &bound);
        let message =
            CertificateVerifyMessage::new(SignatureScheme::RsaPssRsaeSha256.wire_id(), &signature)
                .unwrap();
        expected_input = bound.input().as_bytes().to_vec();
        let result = verify_https_dns_destination_reviewed_server_path_certificate_verify(
            &destination_path,
            &hashed,
            &message,
            RsaPublicKeyPolicy::new(2_048, 2_048, ALLOWED_EXPONENTS).unwrap(),
            Asn1Limits::default(),
            &mut scratch,
            Tls13RsaPssSignatureLimits::Sha256(RsaPssSha256SignatureLimits::new(300, 300).unwrap()),
        );

        if expect_success {
            let verified = result
                .expect("captured server signature must verify through exact HTTPS destination");
            assert!(ptr::eq(
                verified.https_destination_reviewed_server_path(),
                &destination_path,
            ));
            assert!(ptr::eq(
                verified.verification().reviewed_server_path(),
                destination_path.reviewed_server_path(),
            ));
            assert!(ptr::eq(
                verified.verification().verification().hashed_input(),
                &hashed,
            ));
            assert!(ptr::eq(
                verified
                    .verification()
                    .verification()
                    .verification()
                    .signature()
                    .message(),
                &message,
            ));
            assert!(ptr::eq(destination_path.destination().certificate(), &leaf,));
            assert_eq!(
                destination_path.destination().server_name().as_str(),
                "www.apple.com",
            );
            assert_eq!(
                destination_path.destination().host_authority().host(),
                b"WWW.APPLE.COM",
            );
            assert_eq!(
                destination_path.destination().host_authority().port(),
                Some(443),
            );
            assert_eq!(
                format!("{verified:?}"),
                "VerifiedHttpsDnsDestinationReviewedServerPathCertificateVerify",
            );
            with_authenticated_retry_server_certificate_leaf(&leaf_encoded, &leaf, |retry_leaf| {
                let joined = bind_retry_server_certificate_leaf_to_reviewed_server_authentication(
                    retry_leaf, &verified,
                )
                .expect("the authenticated retry leaf is the reviewed HTTPS leaf");
                assert!(ptr::eq(joined.retry_leaf(), retry_leaf));
                assert!(ptr::eq(joined.authentication(), &verified));
                let debug = format!("{joined:?}");
                assert!(debug.contains("[REDACTED]"));
                assert!(!debug.contains("www.apple.com"));
                assert!(!debug.contains("signature"));
            });

            let detached_leaf = decode(&leaf_encoded);
            with_authenticated_retry_server_certificate_leaf(
                &leaf_encoded,
                &detached_leaf,
                |retry_leaf| {
                    let error =
                        bind_retry_server_certificate_leaf_to_reviewed_server_authentication(
                            retry_leaf, &verified,
                        )
                        .expect_err("byte-equal independently decoded leaves must not join");
                    assert_eq!(
                        error,
                        RetryReviewedServerAuthenticationBindingError::LeafCertificateMismatch,
                    );
                    assert_eq!(
                        error.to_string(),
                        "authenticated retry leaf and reviewed server authentication leaf differ",
                    );
                    assert_eq!(format!("{error:?}"), "LeafCertificateMismatch",);
                },
            );
        } else {
            assert_eq!(
                result.unwrap_err().kind(),
                ReviewedServerPathCertificateVerifyErrorKind::WrongEndpointRole,
            );
        }
    }
    assert_eq!(&output[..expected_input.len()], expected_input);
    assert!(output[expected_input.len()..]
        .iter()
        .all(|byte| *byte == 0xa5));
    if !expect_success {
        assert_eq!(scratch, before_scratch);
    }
}

#[test]
fn https_destination_composition_retains_the_exact_destination_path_and_signature() {
    exercise_https_destination_composition(CertificateVerifyRole::Server, true);
}

#[test]
fn https_destination_composition_rejects_client_role_before_buffer_mutation() {
    exercise_https_destination_composition(CertificateVerifyRole::Client, false);
}

fn exercise_https_destination_anchor_composition(
    role: CertificateVerifyRole,
    expect_success: bool,
) {
    let leaf_encoded = fixture(LEAF_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let leaf = decode(&leaf_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    reviewed_server_path!(reviewed_path, &leaf, &candidates);

    let profiles = algorithm_profiles();
    let verified_self_signature = verify_x509_rsa_policy_signature(
        algorithm_policy(&candidates[1], &profiles),
        rsa_policy(&candidates[1], &profiles),
        &[route()],
        operation_limits(),
    )
    .unwrap();
    let anchor_self_signature = bind_x509_rsa_reviewed_anchor_self_signature(
        verified_self_signature,
        select_reviewed_webpki_root_anchor(&candidates[1]).unwrap(),
    )
    .unwrap();
    let reviewed_path_anchor = bind_x509_rsa_reviewed_server_path_anchor_self_signature(
        reviewed_path,
        anchor_self_signature,
    )
    .unwrap();

    let server_name = TlsServerName::new("www.apple.com").unwrap();
    let host = validate_http1_request_host_authority(
        b"WWW.APPLE.COM",
        Some(443),
        Http1RequestHostAuthorityLimits::new(253, 259).unwrap(),
    )
    .unwrap();
    let destination =
        bind_https_dns_destination(&leaf, server_name, host, Asn1Limits::default()).unwrap();
    let destination_path_anchor =
        bind_https_dns_destination_to_reviewed_server_path_anchor_self_signature(
            destination,
            reviewed_path_anchor,
        )
        .unwrap();

    let transcript_encoded = fixture(TRANSCRIPT_HEX);
    let transcript = selected_transcript(&transcript_encoded);
    let mut output = [0xa5; MAXIMUM_INPUT_LENGTH + 7];
    let signature = fixture(SIGNATURE_HEX);
    let mut scratch = [0xa5; 223];
    let before_scratch = scratch;
    let expected_input;
    {
        let bound = bind_certificate_verify_transcript(&transcript, role, &mut output).unwrap();
        let hashed = hash_transcript_bound_certificate_verify_input(authorized_scheme(), &bound);
        let message =
            CertificateVerifyMessage::new(SignatureScheme::RsaPssRsaeSha256.wire_id(), &signature)
                .unwrap();
        expected_input = bound.input().as_bytes().to_vec();
        let result =
            verify_https_dns_destination_reviewed_server_path_anchor_self_signature_certificate_verify(
                &destination_path_anchor,
                &hashed,
                &message,
                RsaPublicKeyPolicy::new(2_048, 2_048, ALLOWED_EXPONENTS).unwrap(),
                Asn1Limits::default(),
                &mut scratch,
                Tls13RsaPssSignatureLimits::Sha256(
                    RsaPssSha256SignatureLimits::new(300, 300).unwrap(),
                ),
            );

        if expect_success {
            let verified = result.expect(
                "captured server signature must verify through destination and anchor proof",
            );
            assert!(ptr::eq(
                verified.https_destination_reviewed_server_path_anchor_self_signature(),
                &destination_path_anchor,
            ));
            assert!(ptr::eq(
                verified.verification().reviewed_server_path(),
                destination_path_anchor
                    .reviewed_server_path_anchor_self_signature()
                    .reviewed_server_path(),
            ));
            assert!(ptr::eq(
                verified.verification().verification().hashed_input(),
                &hashed,
            ));
            assert!(ptr::eq(
                verified
                    .verification()
                    .verification()
                    .verification()
                    .signature()
                    .message(),
                &message,
            ));
            assert!(ptr::eq(
                destination_path_anchor.destination().certificate(),
                &leaf,
            ));
            assert!(ptr::eq(
                destination_path_anchor
                    .reviewed_server_path_anchor_self_signature()
                    .anchor_self_signature()
                    .reviewed_anchor()
                    .candidate_certificate(),
                &candidates[1],
            ));
            assert_eq!(
                destination_path_anchor.destination().server_name().as_str(),
                "www.apple.com",
            );
            assert_eq!(
                destination_path_anchor
                    .destination()
                    .host_authority()
                    .host(),
                b"WWW.APPLE.COM",
            );
            assert_eq!(
                destination_path_anchor
                    .destination()
                    .host_authority()
                    .port(),
                Some(443),
            );
            assert_eq!(
                format!("{verified:?}"),
                "VerifiedHttpsDnsDestinationReviewedServerPathAnchorSelfSignatureCertificateVerify",
            );
        } else {
            assert_eq!(
                result.unwrap_err().kind(),
                ReviewedServerPathCertificateVerifyErrorKind::WrongEndpointRole,
            );
        }
    }
    assert_eq!(&output[..expected_input.len()], expected_input);
    assert!(output[expected_input.len()..]
        .iter()
        .all(|byte| *byte == 0xa5));
    if !expect_success {
        assert_eq!(scratch, before_scratch);
    }
}

#[test]
fn anchor_composition_retains_destination_path_anchor_and_signature() {
    exercise_https_destination_anchor_composition(CertificateVerifyRole::Server, true);
}

#[test]
fn anchor_composition_rejects_client_role_before_buffer_mutation() {
    exercise_https_destination_anchor_composition(CertificateVerifyRole::Client, false);
}
