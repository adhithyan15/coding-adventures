use coding_adventures_hkdf::{FixedHkdfHash, HkdfPrk, HkdfPrkImportError};
use der_asn1::{Asn1Decoder, Asn1Limits};
use rsa_pss_sha256_signature::RsaPssSha256SignatureLimits;
use rsa_pss_sha384_signature::RsaPssSha384SignatureLimits;
use std::error::Error;
use tls13_certificate_message::Tls13CertificateMessageLimits;
use tls13_certificate_request_message::CertificateRequestLimits;
use tls13_certificate_verify_input::{CertificateVerifyRole, MAXIMUM_INPUT_LENGTH};
use tls13_certificate_verify_message::{
    CertificateVerifyMessage, MessageError as CertificateVerifyMessageError,
};
use tls13_certificate_verify_transcript_binding::{
    bind_certificate_verify_transcript, hash_transcript_bound_certificate_verify_input,
    verify_transcript_bound_certificate_verify,
};
use tls13_cipher_suite_offer::ClientCipherSuiteOffer;
use tls13_cipher_suite_policy::{
    CipherSuite, CipherSuitePolicy, CipherSuitePolicyEntry, HkdfHash, SelectedCipherSuite,
};
use tls13_client_hello_message::{decode_client_hello, ClientHelloLimits};
use tls13_extension_block::ExtensionBlockLimits;
use tls13_extension_context_policy::{ExtensionContextPolicy, STANDARD_TLS13_EXTENSION_RULES};
use tls13_finished::{FinishedKeyStorage, FinishedRole};
use tls13_handshake::{Tls13HandshakePolicy, Tls13HandshakeType};
use tls13_handshake_traffic_secrets::HandshakeTrafficSecretStorage;
use tls13_inner_plaintext::Tls13InnerPlaintextPolicy;
use tls13_key_schedule::BaseSecretStorage;
use tls13_key_share::KeyShareLimits;
use tls13_named_group_policy::{NamedGroup, NamedGroupPolicy};
use tls13_no_psk_server_flight::{ServerFlightErrorKind, ServerFlightState};
use tls13_record::{Tls13CiphertextPolicy, TlsContentType};
use tls13_record_protection::{open_tls13_record, seal_tls13_record, Tls13Plaintext};
use tls13_record_protection_state::{RecordProtectionState, RecordProtectionStateError};
use tls13_retry_handshake_traffic_key_material_binding::{
    bind_retry_handshake_traffic_key_material, bind_retry_ordered_server_flight_transcript,
    bind_retry_server_certificate_leaf, bind_retry_server_certificate_verify_signature,
    open_retry_handshake_record, seal_retry_handshake_record, verify_ordered_retry_server_finished,
    HandshakeTrafficSender, OrderedRetryServerFinishedCommitError,
    OrderedRetryServerFinishedVerificationError, RetryCertificateRequestValidationError,
    RetryHandshakePlaintext, RetryHandshakeRecordBindingError,
    RetryHandshakeRecordProtectionStateBindingError, RetryHandshakeRecordProtectionStateIssuer,
    RetryHandshakeRecordReassembler, RetryHandshakeRecordReassemblyError,
    RetryHandshakeTrafficKeyMaterialBindingError, RetryOrderedServerFlight,
    RetryOrderedServerFlightError, RetryOrderedServerFlightTranscriptBindingError,
    RetryOrderedServerTranscriptAdvanceError, RetryServerCertificateLeafBindingError,
    RetryServerCertificateValidationError, RetryServerCertificateVerifySignatureBindingError,
    RetryServerCertificateVerifyValidationError,
};
use tls13_retry_handshake_traffic_secrets_binding::{
    bind_retry_handshake_traffic_secrets, bind_retry_server_finished_key,
    bind_retry_server_transcript_continuation, ExactRetryBaseSecrets,
};
use tls13_retry_key_schedule_binding::bind_retry_key_schedule;
use tls13_retry_transcript_binding::bind_retry_transcript;
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
use tls13_signature_scheme_list::SignatureSchemeListLimits;
use tls13_signature_scheme_policy::{PolicyError, SignatureScheme, SignatureSchemePolicy};
use tls13_supported_groups::NamedGroupListLimits;
use tls13_supported_versions::ClientVersionOffer;
use tls13_traffic_key_material::{
    derive_traffic_key_material, TrafficKeyMaterialError, TrafficKeyMaterialStorage,
};
use tls13_transcript::{TranscriptErrorKind, TranscriptPolicy};
use tls13_x25519_key_share_binding::derive_message_bound_retry_x25519_shared_secret;
use x509_certificate::decode_x509_certificate;
use x509_rsa_public_key::{validate_rsa_public_key, RsaPublicKeyPolicy, ValidatedRsaPublicKey};
use x509_subject_public_key_info::decode_x509_subject_public_key_info;

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

// Generated with OpenSSL 3.x from one disposable 2048-bit RSA key over the
// exact server CertificateVerify input used by the ordered retry fixture.
// Test execution uses only the repository-owned RSA-PSS verifier.
const PUBLIC_KEY_SPKI_HEX: &str = concat!(
    "30820122300d06092a864886f70d01010105000382010f003082010a0282010100",
    "83d3ec05517aaeab255522497d6e6a1f1ff9a58d71aadc1393a06f5e196eccf1",
    "875a66fb4ded8d6f0bb1d3e944f3a886d81fad2ddd3be809a635827ef2603e0c",
    "f5a320cf31e6a0ba3ad8c5d3a5b91d390e321400c59f8f6654a204d0c306a98",
    "9fad47061f5f0a782657732d7f9845d94eafb10b9fc9181b7ca1c5733ea90747",
    "6dc7303baa2b9bdd9f3ff25e9730943b711f8ee158a3921af3fc24461c5aa22c6",
    "434a6e5feaf026c556282009cab46595bf49c588bf792027fa5a46bc132fb2762",
    "d40bad19bddd79a28a9561c041e8e574219828a9ec0d901f4c7708295ae21efec",
    "2c4dce0398465adee92c6aed48da998e5c08e8fcbb8f01af21b7ca1d7cb3910",
    "203010001",
);
const SERVER_SHA256_SIGNATURE_HEX: &str = concat!(
    "7d4fa22e0d100a2b4d852b8be255e67a7bb47f74b163de6eecd6af419e6db404",
    "fd3044288fe8b7ad4f5c0c02cbe5d9d5ed99b674d15a5d9add4458303a998941",
    "13845b059835f9f11bcccb503b7563017ee18763473722d3d06fe8d0fced59f5",
    "ae4bf985653a9d9ecfb441ea8cfe2f5dcaf18709c0e5f86486f0ac0a9cd502fc",
    "6a205efbc2c70365310c20ff17669fbc4e93af88a6c8e0caeb6aeebd75588be9",
    "bbd525c2157ae71d12e8b962b9a182e066c04d76d5d23d7785d3a92d7c2f5611",
    "9683450d4fdf4f0fbece2119b4cf2aa431d64010c46d5cf5831e3091c277334a",
    "29e7b414ef26e020a73246460e47b1fc9ae8a845f58d0faaf5d613c8d3ec444f",
);
const CLIENT_SHA256_SIGNATURE_HEX: &str = concat!(
    "1ed510f2c43b9485d1fed00cfad4c0ff30ea1c1385c97e05a474f73020e2db5d",
    "654991023ca285040bb114eb5937c00a4c6cbb4b84d0b274c2699a8cecea59e0",
    "ca472cde3869b9135efc91629481847114050b7c26672c9900d2315388ceb737",
    "5eb2ede599b337a970adea080130f4c6969cf9868a7664d551d7184289cf79a7",
    "151a60d2b85ccd964822530e73f058afd59f12e29fc3fb408ea0f986c5ff891",
    "4c00d378a2a745f8d24eecf741a23f396c4bcb6c9f6e06ceef17697c817b13f",
    "a434179fc299c4d692c40dbec7d9d427f24e851a69256ee74c74632a65e3afb",
    "44e3c2da35fdc2837418cf4fda5cd25a0d57b71f36edf23f8408cff89c68d2b4a4b",
);
const PREFLIGHT_SERVER_SHA256_SIGNATURE_HEX: &str = concat!(
    "6542f4a740e79654efb3ae85c3032622269cec70a4795c20f89be275b0dc45aa",
    "571709388bbc1b1a1cf0bdc5eb9a8514a568592729992dabbabe5d09ab011919",
    "6114e554d13dc323d5b7a602eb8e13d1f2ffaf42fcc60a3dc53a8c6d7df89ee",
    "006b2dd944e0a013da76e93d039a5ecfd29e1113eddcd8ad9e8e581ea3b8b69c9",
    "e65fa66b14359da422cdbc4b991609475bde66b9bb1b8f37df85c2c085f011fb",
    "47514b015cedc81d2acfea0f9f7b402d55b62bbfc597884dbedf6e69d46c8517",
    "e28fcdf72b7c70dbfc7d6676d0ac8923ad72583b32f54aeed3f268b978d0194b",
    "8df436ad3df7c39ce69eedbba82b2cb0f58ef5765ed4a5af540af3e0731d5a81",
);
const ALTERNATE_SERVER_SHA256_SIGNATURE_HEX: &str = concat!(
    "106653910efd1106e7e9bd08cb9752f14e3ff8d60784a19aac429b75b6dc3823",
    "60879ac5b4e2945694486c5ac01104b090b5a8e60d00c3a2bf04570366fea3fb",
    "4129e21118918a6ed46f1889904c540fd6d70754277fdbce1bb322a2708041d2",
    "0c4d4be9f1cc032830b88f75b93773d6b95c4d52001ad60b148af8d61e907567",
    "ff64a6e952d9ff7ee46bfabd3849947578cfa25cc1863ca8bfd00341c09a249f",
    "41830a1ae70749b39e840002b4e43205f30b0b8d6e08e97bb09b81473670629e",
    "032258477d9d99da077253145de0cf4f42dc1966b6f3a67282b8f043f9a022ba",
    "fde78f61a2a923359909ee47d066bd4f90b0f75262c743924ddc52d9470b07c1",
);
const SERVER_SHA384_SIGNATURE_HEX: &str = concat!(
    "7a162638f1df60c3cbf8a77c989e5cd65d347225b4949655779ffc68386b02a06",
    "170a3f002cd94fdfd84406c47ac292ad60f78c703cf98128fc2495b9e9dc774",
    "030277a0cc7e8bfe885e8543605f7a64ed8c13b25bd9925a7167dedf054db7e",
    "5a73fc5f464f76495f50d5c89de07f1642f881dd533de72f78026cf219bdd8b64",
    "6ab8340d1a97fae5c2efd7298b470a73c053589b47ccfd6cd8812de4d078250c",
    "d21fe6d8470ab018e95261de43aa979f9052e2f91d756e5b9fe7bb357d913685",
    "0aa136e781dfcce0e35faa316082d42f6864a92b8bbf5a7d8703184a5f92d8f",
    "680ec404e91abf451399a4ce7bea18fd12d20f23ca3ea9686e1849a196868198b",
);

fn decode_hex(hexadecimal: &str) -> Vec<u8> {
    assert!(hexadecimal.len().is_multiple_of(2));
    hexadecimal
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn decode_fixture_hex(hexadecimal: &str) -> Vec<u8> {
    let compact: String = hexadecimal
        .chars()
        .filter(|character| !character.is_ascii_whitespace())
        .collect();
    decode_hex(&compact)
}

fn encode_u24(value: usize) -> [u8; 3] {
    assert!(value <= 0x00ff_ffff);
    [
        ((value >> 16) & 0xff) as u8,
        ((value >> 8) & 0xff) as u8,
        (value & 0xff) as u8,
    ]
}

fn validated_rsa_key(encoded: &[u8]) -> ValidatedRsaPublicKey<'_> {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let root = decoder.decode_exact(encoded).unwrap();
    let subject_public_key_info = decode_x509_subject_public_key_info(&mut decoder, root).unwrap();
    let policy = RsaPublicKeyPolicy::new(2048, 2048, &[65_537]).unwrap();
    validate_rsa_public_key(subject_public_key_info, policy, Asn1Limits::default()).unwrap()
}

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
        ClientHelloLimits::new(4, ExtensionBlockLimits::new(256, 5).unwrap()).unwrap();
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
    with_retry_x25519_messages_and_signature_offer(suite, Some(&[0, 2, 8, 4]), callback)
}

fn with_retry_x25519_messages_and_signature_offer<T>(
    suite: CipherSuite,
    signature_algorithms: Option<&[u8]>,
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
    let mut first_extensions = vec![(5, &[][..]), (10, &supported_groups[..])];
    if let Some(signature_algorithms) = signature_algorithms {
        first_extensions.push((13, signature_algorithms));
    }
    first_extensions.push((51, &initial_share));
    let first_client = client_body(suite, &first_extensions);
    let retry_server = server_body(
        suite,
        TLS13_HELLO_RETRY_REQUEST_RANDOM,
        &[(51, &retry_share)],
    );
    let mut second_extensions = vec![(5, &[][..]), (10, &supported_groups[..])];
    if let Some(signature_algorithms) = signature_algorithms {
        second_extensions.push((13, signature_algorithms));
    }
    second_extensions.push((51, &retried_share));
    let second_client = client_body(suite, &second_extensions);
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

fn assert_retry_certificate_verify_rejected(
    signature_offer: Option<&[u8]>,
    certificate_verify_body: &[u8],
    expected: RetryServerCertificateVerifyValidationError,
) {
    let mut x25519_output = [0xa5; 32];
    with_retry_x25519_messages_and_signature_offer(
        CipherSuite::TlsAes128GcmSha256,
        signature_offer,
        |messages| {
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
            let traffic =
                bind_retry_handshake_traffic_secrets(&base, &mut traffic_storage).unwrap();
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
                    Tls13HandshakeType::CertificateVerify,
                ],
                32,
            )
            .unwrap();
            let mut reassembly_storage = [0xa5; 96];
            let reassembler = RetryHandshakeRecordReassembler::new(
                server_state,
                handshake_policy,
                &mut reassembly_storage,
            )
            .unwrap();
            let ordered = RetryOrderedServerFlight::new(reassembler).unwrap();
            let mut bound =
                bind_retry_ordered_server_flight_transcript(ordered, continuation).unwrap();
            let server_prk = HkdfPrk::import_exact(
                fixed_hash(suite),
                issuer.traffic_secrets().traffic_secrets().server_secret(),
            )
            .unwrap();
            let mut peer_storage = TrafficKeyMaterialStorage::new(suite).unwrap();
            let peer_material =
                derive_traffic_key_material(&server_prk, &mut peer_storage).unwrap();
            let mut peer_state = RecordProtectionState::new(peer_material).unwrap();
            let certificate_body = [0, 0, 0, 8, 0, 0, 3, 0x30, 1, 0, 0, 0];
            let mut plaintext = vec![8, 0, 0, 2, 0, 0, 11, 0, 0, 12];
            plaintext.extend_from_slice(&certificate_body);
            plaintext.push(15);
            plaintext.extend_from_slice(&[
                ((certificate_verify_body.len() >> 16) & 0xff) as u8,
                ((certificate_verify_body.len() >> 8) & 0xff) as u8,
                (certificate_verify_body.len() & 0xff) as u8,
            ]);
            plaintext.extend_from_slice(certificate_verify_body);
            let ciphertext_policy = Tls13CiphertextPolicy::new(1_024).unwrap();
            let inner_policy =
                Tls13InnerPlaintextPolicy::new(&[TlsContentType::Handshake], 1_008).unwrap();
            let mut scratch = [0xa5; 96];
            let mut wire = [0xcc; 128];
            let used = seal_tls13_record(
                &mut peer_state,
                &ciphertext_policy,
                &inner_policy,
                Tls13Plaintext::new(TlsContentType::Handshake, &plaintext, 0),
                &mut scratch,
                &mut wire,
            )
            .unwrap();
            let mut opened = [0x5a; 96];
            let allowed = [SignatureScheme::RsaPssRsaeSha256];
            let policy = SignatureSchemePolicy::new(&allowed).unwrap();
            let mut rejected_snapshot = None;
            let error = bound
                .open_accept_and_advance(
                    &ciphertext_policy,
                    &inner_policy,
                    &wire[..used],
                    &mut opened,
                    |pending| match pending.message().handshake_type() {
                        Tls13HandshakeType::EncryptedExtensions => pending
                            .validate_encrypted_extensions(
                                ExtensionBlockLimits::new(64, 4).unwrap(),
                            )
                            .is_ok(),
                        Tls13HandshakeType::Certificate => pending
                            .validate_server_certificate(
                                Tls13CertificateMessageLimits::new(64, 0, 32, 2, 16, 16),
                                ExtensionBlockLimits::new(16, 4).unwrap(),
                            )
                            .is_ok(),
                        Tls13HandshakeType::CertificateVerify => {
                            rejected_snapshot = Some(pending.transcript_snapshot());
                            let actual = pending
                                .validate_server_certificate_verify(
                                    SignatureSchemeListLimits::new(8).unwrap(),
                                    policy,
                                )
                                .unwrap_err();
                            assert_eq!(actual, expected,);
                            let debug = format!("{actual:?}");
                            assert!(!debug.contains("aabbcc"));
                            assert!(!debug.contains("retry-key-material"));
                            false
                        }
                        other => panic!("unexpected handshake type {other:?}"),
                    },
                )
                .unwrap_err();
            assert_eq!(
                error,
                RetryOrderedServerTranscriptAdvanceError::MessageRejected
            );
            assert_eq!(
                bound.transcript().snapshot(),
                rejected_snapshot.expect("CertificateVerify callback ran"),
            );
            assert!(bound.ordered_flight().is_failed());
            assert_eq!(
                bound.ordered_flight().state(),
                ServerFlightState::ExpectFinished
            );
        },
    );
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
        {
            let mut client_preview = TrafficKeyMaterialStorage::new(suite).unwrap();
            let mut server_preview = TrafficKeyMaterialStorage::new(suite).unwrap();
            let client = bind_retry_handshake_traffic_key_material(
                &traffic,
                HandshakeTrafficSender::Client,
                &mut client_preview,
            )
            .unwrap();
            let server = bind_retry_handshake_traffic_key_material(
                &traffic,
                HandshakeTrafficSender::Server,
                &mut server_preview,
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
        }

        let issuer = RetryHandshakeRecordProtectionStateIssuer::new(traffic);
        assert!(!issuer.has_issued(HandshakeTrafficSender::Client));
        assert!(!issuer.has_issued(HandshakeTrafficSender::Server));
        let mut client_storage = TrafficKeyMaterialStorage::new(suite).unwrap();
        let mut server_storage = TrafficKeyMaterialStorage::new(suite).unwrap();
        let mut client_state =
            match issuer.issue(HandshakeTrafficSender::Client, &mut client_storage) {
                Ok(state) => state,
                Err(_) => panic!("valid exact retry client state issuance must succeed"),
            };
        let server_state = match issuer.issue(HandshakeTrafficSender::Server, &mut server_storage) {
            Ok(state) => state,
            Err(_) => panic!("valid exact retry server state issuance must succeed"),
        };

        assert!(issuer.has_issued(HandshakeTrafficSender::Client));
        assert!(issuer.has_issued(HandshakeTrafficSender::Server));
        assert!(core::ptr::eq(
            client_state.traffic_secrets(),
            issuer.traffic_secrets()
        ));
        assert!(core::ptr::eq(
            server_state.traffic_secrets(),
            issuer.traffic_secrets()
        ));
        assert_eq!(client_state.sender(), HandshakeTrafficSender::Client);
        assert_eq!(server_state.sender(), HandshakeTrafficSender::Server);
        assert_eq!(client_state.selected_suite(), suite);
        assert_eq!(server_state.selected_suite(), suite);
        assert_eq!(client_state.record_state().records_used(), 0);
        assert_eq!(server_state.record_state().records_used(), 0);
        assert_eq!(
            client_state.record_state().remaining_records(),
            suite.record_limit()
        );

        let mut duplicate_storage = TrafficKeyMaterialStorage::new(suite).unwrap();
        let duplicate = match issuer.issue(HandshakeTrafficSender::Client, &mut duplicate_storage) {
            Err(error) => error,
            Ok(_) => panic!("one issuer must reject a second client state"),
        };
        assert_eq!(
            duplicate,
            RetryHandshakeRecordProtectionStateBindingError::SenderAlreadyIssued
        );
        let mut duplicate_server_storage = TrafficKeyMaterialStorage::new(suite).unwrap();
        let duplicate_server = match issuer.issue(
            HandshakeTrafficSender::Server,
            &mut duplicate_server_storage,
        ) {
            Err(error) => error,
            Ok(_) => panic!("one issuer must reject a second server state"),
        };
        assert_eq!(
            duplicate_server,
            RetryHandshakeRecordProtectionStateBindingError::SenderAlreadyIssued
        );

        let ciphertext_policy = Tls13CiphertextPolicy::new(1024).unwrap();
        let inner_policy = Tls13InnerPlaintextPolicy::new(
            &[TlsContentType::Handshake, TlsContentType::Alert],
            1008,
        )
        .unwrap();

        let client_prk = HkdfPrk::import_exact(
            fixed_hash(suite),
            issuer.traffic_secrets().traffic_secrets().client_secret(),
        )
        .unwrap();
        let mut peer_client_storage = TrafficKeyMaterialStorage::new(suite).unwrap();
        let peer_client_material =
            derive_traffic_key_material(&client_prk, &mut peer_client_storage).unwrap();
        let mut peer_client_state = RecordProtectionState::new(peer_client_material).unwrap();

        let mut client_scratch = [0xa5; 128];
        let mut client_wire = [0xcc; 160];
        let client_used;
        {
            let sealed = seal_retry_handshake_record(
                &mut client_state,
                &ciphertext_policy,
                &inner_policy,
                RetryHandshakePlaintext::new(b"client retry flight", 3),
                &mut client_scratch,
                &mut client_wire,
            )
            .unwrap();
            assert!(core::ptr::eq(
                sealed.traffic_secrets(),
                issuer.traffic_secrets()
            ));
            assert_eq!(sealed.sender(), HandshakeTrafficSender::Client);
            assert_eq!(sealed.selected_suite(), suite);
            assert_eq!(sealed.content(), b"client retry flight");
            assert_eq!(sealed.padding_length(), 3);
            assert_eq!(sealed.records_used(), 1);
            assert_eq!(&sealed.output()[..3], &[23, 3, 3]);
            client_used = sealed.output().len();
            let debug = format!("{sealed:?}");
            assert!(debug.contains("[RETAINED]"));
            assert!(debug.contains("[REDACTED]"));
            assert!(!debug.contains("client retry flight"));
        }
        assert_eq!(client_scratch[..23], [0; 23]);

        let mut peer_client_opened = [0x5a; 128];
        {
            let opened = open_tls13_record(
                &mut peer_client_state,
                &ciphertext_policy,
                &inner_policy,
                &client_wire[..client_used],
                &mut peer_client_opened,
            )
            .unwrap();
            assert_eq!(opened.content_type(), TlsContentType::Handshake);
            assert_eq!(opened.content(), b"client retry flight");
            assert_eq!(opened.padding_length(), 3);
        }
        assert_eq!(peer_client_opened[..23], [0; 23]);

        let client_open_used = seal_tls13_record(
            &mut peer_client_state,
            &ciphertext_policy,
            &inner_policy,
            Tls13Plaintext::new(TlsContentType::Handshake, b"client opened flight", 1),
            &mut client_scratch,
            &mut client_wire,
        )
        .unwrap();
        let mut client_opened = [0x5a; 128];
        {
            let opened = open_retry_handshake_record(
                &mut client_state,
                &ciphertext_policy,
                &inner_policy,
                &client_wire[..client_open_used],
                &mut client_opened,
            )
            .unwrap();
            assert_eq!(opened.content(), b"client opened flight");
            assert_eq!(opened.padding_length(), 1);
            assert_eq!(opened.records_used(), 2);
        }
        assert_eq!(client_opened[..22], [0; 22]);

        let server_prk = HkdfPrk::import_exact(
            fixed_hash(suite),
            issuer.traffic_secrets().traffic_secrets().server_secret(),
        )
        .unwrap();
        let mut peer_server_storage = TrafficKeyMaterialStorage::new(suite).unwrap();
        let peer_server_material =
            derive_traffic_key_material(&server_prk, &mut peer_server_storage).unwrap();
        let mut peer_server_state = RecordProtectionState::new(peer_server_material).unwrap();
        let mut server_scratch = [0xa5; 128];
        let mut server_wire = [0xcc; 160];
        let handshake_policy = Tls13HandshakePolicy::new(
            &[
                Tls13HandshakeType::EncryptedExtensions,
                Tls13HandshakeType::Certificate,
            ],
            16,
        )
        .unwrap();
        let mut reassembly_storage = [0xa5; 64];
        let mut reassembler = RetryHandshakeRecordReassembler::new(
            server_state,
            handshake_policy,
            &mut reassembly_storage,
        )
        .unwrap();
        assert!(core::ptr::eq(
            reassembler.traffic_secrets(),
            issuer.traffic_secrets()
        ));
        assert_eq!(reassembler.sender(), HandshakeTrafficSender::Server);
        assert_eq!(reassembler.selected_suite(), suite);
        assert!(!reassembler.message_in_progress());
        assert_eq!(reassembler.records_used(), 0);

        let first_fragment = [8, 0];
        let first_used = seal_tls13_record(
            &mut peer_server_state,
            &ciphertext_policy,
            &inner_policy,
            Tls13Plaintext::new(TlsContentType::Handshake, &first_fragment, 1),
            &mut server_scratch,
            &mut server_wire,
        )
        .unwrap();
        let mut server_opened = [0x5a; 128];
        {
            let reassembled = reassembler
                .open_and_reassemble(
                    &ciphertext_policy,
                    &inner_policy,
                    &server_wire[..first_used],
                    &mut server_opened,
                    |_| panic!("a two-byte handshake header cannot complete a message"),
                )
                .unwrap();
            assert!(core::ptr::eq(
                reassembled.traffic_secrets(),
                issuer.traffic_secrets()
            ));
            assert_eq!(reassembled.sender(), HandshakeTrafficSender::Server);
            assert_eq!(reassembled.selected_suite(), suite);
            assert_eq!(reassembled.input(), &server_wire[..first_used]);
            assert_eq!(reassembled.authenticated_content_length(), 2);
            assert_eq!(reassembled.completed_messages(), 0);
            assert!(reassembled.message_in_progress());
            assert_eq!(reassembled.records_used(), 1);
        }
        assert_eq!(server_opened[..4], [0; 4]);
        assert!(reassembler.message_in_progress());

        let second_fragment = [0, 2, 0xaa, 0xbb, 11, 0, 0, 1, 0xcc];
        let second_used = seal_tls13_record(
            &mut peer_server_state,
            &ciphertext_policy,
            &inner_policy,
            Tls13Plaintext::new(TlsContentType::Handshake, &second_fragment, 1),
            &mut server_scratch,
            &mut server_wire,
        )
        .unwrap();
        server_opened.fill(0x5a);
        let mut received = Vec::new();
        {
            let reassembled = reassembler
                .open_and_reassemble(
                    &ciphertext_policy,
                    &inner_policy,
                    &server_wire[..second_used],
                    &mut server_opened,
                    |message| {
                        received.push((message.handshake_type(), message.body().to_vec()));
                    },
                )
                .unwrap();
            assert_eq!(reassembled.input(), &server_wire[..second_used]);
            assert_eq!(reassembled.authenticated_content_length(), 9);
            assert_eq!(reassembled.completed_messages(), 2);
            assert!(!reassembled.message_in_progress());
            assert_eq!(reassembled.records_used(), 2);
            let debug = format!("{reassembled:?}");
            assert!(debug.contains("[RETAINED]"));
            assert!(debug.contains("[REDACTED]"));
            assert!(!debug.contains("aa"));
        }
        assert_eq!(
            received,
            vec![
                (Tls13HandshakeType::EncryptedExtensions, vec![0xaa, 0xbb]),
                (Tls13HandshakeType::Certificate, vec![0xcc]),
            ]
        );
        assert_eq!(server_opened[..11], [0; 11]);
        assert!(!reassembler.message_in_progress());

        let third_fragment = [8, 0];
        let third_used = seal_tls13_record(
            &mut peer_server_state,
            &ciphertext_policy,
            &inner_policy,
            Tls13Plaintext::new(TlsContentType::Handshake, &third_fragment, 0),
            &mut server_scratch,
            &mut server_wire,
        )
        .unwrap();
        server_opened.fill(0x5a);
        reassembler
            .open_and_reassemble(
                &ciphertext_policy,
                &inner_policy,
                &server_wire[..third_used],
                &mut server_opened,
                |_| panic!("a two-byte handshake header cannot complete a message"),
            )
            .unwrap();
        assert!(reassembler.message_in_progress());
        assert!(!reassembler.is_failed());
        assert_eq!(reassembler.records_used(), 3);

        let alert_used = seal_tls13_record(
            &mut peer_server_state,
            &ciphertext_policy,
            &inner_policy,
            Tls13Plaintext::new(TlsContentType::Alert, &[2, 40], 2),
            &mut server_scratch,
            &mut server_wire,
        )
        .unwrap();
        server_opened.fill(0x5a);
        assert_eq!(
            reassembler
                .open_and_reassemble(
                    &ciphertext_policy,
                    &inner_policy,
                    &server_wire[..alert_used],
                    &mut server_opened,
                    |_| panic!("alert content must not reach handshake reassembly"),
                )
                .unwrap_err(),
            RetryHandshakeRecordReassemblyError::Record(
                RetryHandshakeRecordBindingError::UnexpectedContentType
            )
        );
        assert_eq!(server_opened[..5], [0; 5]);
        assert!(server_opened[5..].iter().all(|byte| *byte == 0x5a));
        assert_eq!(reassembler.records_used(), 4);
        assert!(!reassembler.message_in_progress());
        assert!(reassembler.is_failed());

        server_opened.fill(0x5a);
        assert_eq!(
            reassembler
                .open_and_reassemble(
                    &ciphertext_policy,
                    &inner_policy,
                    &server_wire[..alert_used],
                    &mut server_opened,
                    |_| panic!("a failed owner must not invoke the handler"),
                )
                .unwrap_err(),
            RetryHandshakeRecordReassemblyError::OwnerFailed
        );
        assert!(server_opened.iter().all(|byte| *byte == 0x5a));
        assert_eq!(reassembler.records_used(), 4);

        let reassembler_debug = format!("{reassembler:?}");
        assert!(reassembler_debug.contains("[RETAINED]"));
        assert!(reassembler_debug.contains("[REDACTED]"));
        for disclosed in ["retry-key-material", "8520f009", "de9edb7d"] {
            assert!(!reassembler_debug.contains(disclosed));
        }

        drop(reassembler);
        assert_eq!(reassembly_storage[..2], [0; 2]);

        let client_handshake_policy =
            Tls13HandshakePolicy::new(&[Tls13HandshakeType::EncryptedExtensions], 16).unwrap();
        let mut client_reassembly_storage = [0xa5; 64];
        let mut client_reassembler = RetryHandshakeRecordReassembler::new(
            client_state,
            client_handshake_policy,
            &mut client_reassembly_storage,
        )
        .unwrap();
        let client_fragment = [8, 0];
        let client_fragment_used = seal_tls13_record(
            &mut peer_client_state,
            &ciphertext_policy,
            &inner_policy,
            Tls13Plaintext::new(TlsContentType::Handshake, &client_fragment, 0),
            &mut client_scratch,
            &mut client_wire,
        )
        .unwrap();
        client_opened.fill(0x5a);
        client_reassembler
            .open_and_reassemble(
                &ciphertext_policy,
                &inner_policy,
                &client_wire[..client_fragment_used],
                &mut client_opened,
                |_| panic!("a two-byte handshake header cannot complete a message"),
            )
            .unwrap();
        assert!(client_reassembler.message_in_progress());
        assert_eq!(client_reassembler.records_used(), 3);

        let client_continuation = [0, 2, 0xaa, 0xbb];
        let client_continuation_used = seal_tls13_record(
            &mut peer_client_state,
            &ciphertext_policy,
            &inner_policy,
            Tls13Plaintext::new(TlsContentType::Handshake, &client_continuation, 0),
            &mut client_scratch,
            &mut client_wire,
        )
        .unwrap();
        client_wire[client_continuation_used - 1] ^= 1;
        client_opened.fill(0x5a);
        assert!(matches!(
            client_reassembler.open_and_reassemble(
                &ciphertext_policy,
                &inner_policy,
                &client_wire[..client_continuation_used],
                &mut client_opened,
                |_| panic!("unauthenticated bytes must not reach the handler"),
            ),
            Err(RetryHandshakeRecordReassemblyError::Record(
                RetryHandshakeRecordBindingError::RecordProtection(_)
            ))
        ));
        assert!(!client_reassembler.message_in_progress());
        assert!(client_reassembler.is_failed());
        assert_eq!(client_reassembler.records_used(), 4);

        client_opened.fill(0x5a);
        assert_eq!(
            client_reassembler
                .open_and_reassemble(
                    &ciphertext_policy,
                    &inner_policy,
                    &client_wire[..client_continuation_used],
                    &mut client_opened,
                    |_| panic!("a failed owner must not invoke the handler"),
                )
                .unwrap_err(),
            RetryHandshakeRecordReassemblyError::OwnerFailed
        );
        assert!(client_opened.iter().all(|byte| *byte == 0x5a));
        assert_eq!(client_reassembler.records_used(), 4);

        let client_reassembler_debug = format!("{client_reassembler:?}");
        assert!(client_reassembler_debug.contains("[RETAINED]"));
        assert!(client_reassembler_debug.contains("[REDACTED]"));
        assert!(client_reassembler_debug.contains("failed: true"));
        drop(client_reassembler);
        assert_eq!(client_reassembly_storage[..2], [0; 2]);

        let issuer_debug = format!("{issuer:?}");
        assert!(issuer_debug.contains("[RETAINED]"));
        assert!(!issuer_debug.contains("8520f009"));
    });
}

#[test]
fn handler_unwind_clears_and_permanently_closes_reassembly() {
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
        let issuer = RetryHandshakeRecordProtectionStateIssuer::new(traffic);
        let mut server_storage = TrafficKeyMaterialStorage::new(suite).unwrap();
        let server_state = issuer
            .issue(HandshakeTrafficSender::Server, &mut server_storage)
            .unwrap();

        let server_prk = HkdfPrk::import_exact(
            fixed_hash(suite),
            issuer.traffic_secrets().traffic_secrets().server_secret(),
        )
        .unwrap();
        let mut peer_storage = TrafficKeyMaterialStorage::new(suite).unwrap();
        let peer_material = derive_traffic_key_material(&server_prk, &mut peer_storage).unwrap();
        let mut peer_state = RecordProtectionState::new(peer_material).unwrap();
        let ciphertext_policy = Tls13CiphertextPolicy::new(1024).unwrap();
        let inner_policy =
            Tls13InnerPlaintextPolicy::new(&[TlsContentType::Handshake], 1008).unwrap();
        let handshake_policy =
            Tls13HandshakePolicy::new(&[Tls13HandshakeType::EncryptedExtensions], 16).unwrap();
        let mut reassembly_storage = [0xa5; 64];
        let mut reassembler = RetryHandshakeRecordReassembler::new(
            server_state,
            handshake_policy,
            &mut reassembly_storage,
        )
        .unwrap();

        let message = [8, 0, 0, 0];
        let mut scratch = [0xa5; 32];
        let mut wire = [0xcc; 64];
        let used = seal_tls13_record(
            &mut peer_state,
            &ciphertext_policy,
            &inner_policy,
            Tls13Plaintext::new(TlsContentType::Handshake, &message, 0),
            &mut scratch,
            &mut wire,
        )
        .unwrap();
        let mut opened = [0x5a; 32];
        let unwind = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = reassembler.open_and_reassemble(
                &ciphertext_policy,
                &inner_policy,
                &wire[..used],
                &mut opened,
                |_| panic!("test handler unwind"),
            );
        }));
        assert!(unwind.is_err());
        assert!(reassembler.is_failed());
        assert!(!reassembler.message_in_progress());
        assert_eq!(reassembler.records_used(), 1);
        assert_eq!(opened[..5], [0; 5]);

        opened.fill(0x5a);
        assert_eq!(
            reassembler
                .open_and_reassemble(
                    &ciphertext_policy,
                    &inner_policy,
                    &wire[..used],
                    &mut opened,
                    |_| panic!("a failed owner must not invoke the handler"),
                )
                .unwrap_err(),
            RetryHandshakeRecordReassemblyError::OwnerFailed
        );
        assert!(opened.iter().all(|byte| *byte == 0x5a));
        assert_eq!(
            RetryOrderedServerFlight::new(reassembler).unwrap_err(),
            RetryOrderedServerFlightError::NonFreshReassembler
        );
        assert_eq!(reassembly_storage[..message.len()], [0; 4]);
    });
}

#[test]
fn joins_fresh_ordered_flight_to_exact_retry_transcript_and_finished_key() {
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
        let expected_snapshot = transcript.transcript().snapshot();
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
                Tls13HandshakeType::CertificateVerify,
                Tls13HandshakeType::Finished,
            ],
            16,
        )
        .unwrap();
        let mut reassembly_storage = [0xa5; 128];
        let reassembler = RetryHandshakeRecordReassembler::new(
            server_state,
            handshake_policy,
            &mut reassembly_storage,
        )
        .unwrap();
        let ordered = RetryOrderedServerFlight::new(reassembler).unwrap();

        let bound = bind_retry_ordered_server_flight_transcript(ordered, continuation).unwrap();

        assert!(core::ptr::eq(
            bound.ordered_flight().traffic_secrets(),
            issuer.traffic_secrets(),
        ));
        assert!(core::ptr::eq(bound.server_finished_key(), &server_finished,));
        assert_eq!(bound.selected_suite(), suite);
        assert_eq!(bound.transcript().snapshot(), expected_snapshot);
        assert_eq!(
            bound.ordered_flight().state(),
            ServerFlightState::ExpectEncryptedExtensions,
        );
        assert_eq!(bound.ordered_flight().records_used(), 0);
        assert!(!bound.ordered_flight().message_in_progress());
        assert!(!bound.ordered_flight().is_failed());
        let debug = format!("{bound:?}");
        assert!(debug.contains("[RETAINED]"));
        assert!(debug.contains("[SECRET]"));
        for disclosed in ["retry-key-material", "8520f009", "de9edb7d"] {
            assert!(!debug.contains(disclosed));
        }
    });
}

#[test]
fn authenticates_orders_authorizes_and_advances_exact_messages() {
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
                Tls13HandshakeType::CertificateRequest,
                Tls13HandshakeType::Certificate,
                Tls13HandshakeType::CertificateVerify,
                Tls13HandshakeType::Finished,
            ],
            16,
        )
        .unwrap();
        let mut reassembly_storage = [0xa5; 128];
        let reassembler = RetryHandshakeRecordReassembler::new(
            server_state,
            handshake_policy,
            &mut reassembly_storage,
        )
        .unwrap();
        let ordered = RetryOrderedServerFlight::new(reassembler).unwrap();
        let mut bound = bind_retry_ordered_server_flight_transcript(ordered, continuation).unwrap();

        let server_prk = HkdfPrk::import_exact(
            fixed_hash(suite),
            issuer.traffic_secrets().traffic_secrets().server_secret(),
        )
        .unwrap();
        let mut peer_storage = TrafficKeyMaterialStorage::new(suite).unwrap();
        let peer_material = derive_traffic_key_material(&server_prk, &mut peer_storage).unwrap();
        let mut peer_state = RecordProtectionState::new(peer_material).unwrap();
        let plaintext = [
            8, 0, 0, 1, 0xa1, 13, 0, 0, 1, 0xa2, 11, 0, 0, 1, 0xa3, 15, 0, 0, 1, 0xa4, 20, 0, 0, 1,
            0xa5,
        ];
        let mut expected = issuer
            .traffic_secrets()
            .evidence()
            .reconstruct_transcript()
            .unwrap();
        let mut expected_pre_message = Vec::new();
        for encoded in plaintext.as_chunks::<5>().0 {
            expected_pre_message.push(expected.snapshot());
            expected.update(encoded).unwrap();
        }
        let ciphertext_policy = Tls13CiphertextPolicy::new(1_024).unwrap();
        let inner_policy =
            Tls13InnerPlaintextPolicy::new(&[TlsContentType::Handshake], 1_008).unwrap();
        let mut scratch = [0xa5; 64];
        let mut wire = [0xcc; 96];
        let used = seal_tls13_record(
            &mut peer_state,
            &ciphertext_policy,
            &inner_policy,
            Tls13Plaintext::new(TlsContentType::Handshake, &plaintext, 2),
            &mut scratch,
            &mut wire,
        )
        .unwrap();
        let mut opened = [0x5a; 64];
        let mut received = Vec::new();
        {
            let result = bound
                .open_accept_and_advance(
                    &ciphertext_policy,
                    &inner_policy,
                    &wire[..used],
                    &mut opened,
                    |pending| {
                        received.push((
                            pending.message().handshake_type(),
                            pending.ordered_server_flight_identity(),
                            pending.transcript_snapshot(),
                            pending.state(),
                        ));
                        true
                    },
                )
                .unwrap();
            assert_eq!(result.input(), &wire[..used]);
            assert_eq!(result.authenticated_content_length(), plaintext.len());
            assert_eq!(result.completed_messages(), 5);
            assert!(!result.message_in_progress());
            assert_eq!(result.records_used(), 1);
            assert_eq!(result.state(), ServerFlightState::Complete);
            assert!(result.client_authentication_requested());
            assert!(result.is_complete());
            assert_eq!(result.transcript_snapshot(), expected.snapshot());
            assert!(core::ptr::eq(
                result.server_finished_key(),
                &server_finished,
            ));
            let debug = format!("{result:?}");
            assert!(debug.contains("[REDACTED]"));
            assert!(debug.contains("[SECRET]"));
            assert!(!debug.contains("retry-key-material"));
        }
        assert_eq!(
            received
                .iter()
                .map(|(handshake_type, _, _, _)| *handshake_type)
                .collect::<Vec<_>>(),
            [
                Tls13HandshakeType::EncryptedExtensions,
                Tls13HandshakeType::CertificateRequest,
                Tls13HandshakeType::Certificate,
                Tls13HandshakeType::CertificateVerify,
                Tls13HandshakeType::Finished,
            ],
        );
        let ordered_server_flight_identity = received[0].1;
        assert!(received
            .iter()
            .all(|(_, identity, _, _)| *identity == ordered_server_flight_identity));
        assert_eq!(
            format!("{ordered_server_flight_identity:?}"),
            "OrderedRetryServerFlightIdentity([REDACTED])",
        );
        assert_eq!(
            received
                .iter()
                .map(|(_, _, snapshot, _)| *snapshot)
                .collect::<Vec<_>>(),
            expected_pre_message,
        );
        assert_eq!(bound.transcript().snapshot(), expected.snapshot());
        assert!(bound.ordered_flight().is_complete());
        assert!(!bound.ordered_flight().is_failed());
    });
}

#[test]
fn validates_exact_encrypted_extensions_against_retained_final_offer() {
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
        let handshake_policy =
            Tls13HandshakePolicy::new(&[Tls13HandshakeType::EncryptedExtensions], 16).unwrap();
        let mut reassembly_storage = [0xa5; 32];
        let reassembler = RetryHandshakeRecordReassembler::new(
            server_state,
            handshake_policy,
            &mut reassembly_storage,
        )
        .unwrap();
        let ordered = RetryOrderedServerFlight::new(reassembler).unwrap();
        let mut bound = bind_retry_ordered_server_flight_transcript(ordered, continuation).unwrap();
        let server_prk = HkdfPrk::import_exact(
            fixed_hash(suite),
            issuer.traffic_secrets().traffic_secrets().server_secret(),
        )
        .unwrap();
        let mut peer_storage = TrafficKeyMaterialStorage::new(suite).unwrap();
        let peer_material = derive_traffic_key_material(&server_prk, &mut peer_storage).unwrap();
        let mut peer_state = RecordProtectionState::new(peer_material).unwrap();
        let encrypted_extensions_body = [0, 4, 0, 10, 0, 0];
        let mut plaintext = [0_u8; 10];
        plaintext[..4].copy_from_slice(&[8, 0, 0, 6]);
        plaintext[4..].copy_from_slice(&encrypted_extensions_body);
        let ciphertext_policy = Tls13CiphertextPolicy::new(1_024).unwrap();
        let inner_policy =
            Tls13InnerPlaintextPolicy::new(&[TlsContentType::Handshake], 1_008).unwrap();
        let mut scratch = [0xa5; 32];
        let mut wire = [0xcc; 64];
        let used = seal_tls13_record(
            &mut peer_state,
            &ciphertext_policy,
            &inner_policy,
            Tls13Plaintext::new(TlsContentType::Handshake, &plaintext, 0),
            &mut scratch,
            &mut wire,
        )
        .unwrap();
        let mut opened = [0x5a; 32];
        let mut validated = false;
        let result = bound
            .open_accept_and_advance(
                &ciphertext_policy,
                &inner_policy,
                &wire[..used],
                &mut opened,
                |pending| {
                    let proof = pending
                        .validate_encrypted_extensions(ExtensionBlockLimits::new(64, 4).unwrap())
                        .unwrap();
                    assert_eq!(
                        proof.pending().transcript_snapshot(),
                        pending.transcript_snapshot(),
                    );
                    assert_eq!(
                        proof.encrypted_extensions().body(),
                        encrypted_extensions_body,
                    );
                    assert_eq!(proof.encrypted_extensions().extensions().len(), 1);
                    assert_eq!(
                        proof.encrypted_extensions().extensions().entries()[0]
                            .extension_type()
                            .wire_value(),
                        10,
                    );
                    let debug = format!("{proof:?}");
                    assert!(debug.contains("[RETAINED]"));
                    assert!(debug.contains("[REDACTED]"));
                    validated = true;
                    true
                },
            )
            .unwrap();
        assert!(validated);
        assert_eq!(result.completed_messages(), 1);
        assert_eq!(
            result.state(),
            ServerFlightState::ExpectCertificateRequestOrCertificate
        );
    });
}

#[test]
fn rejects_unsolicited_encrypted_extension_before_transcript_advancement() {
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
        let retained_snapshot = transcript.transcript().snapshot();
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
        let handshake_policy =
            Tls13HandshakePolicy::new(&[Tls13HandshakeType::EncryptedExtensions], 16).unwrap();
        let mut reassembly_storage = [0xa5; 32];
        let reassembler = RetryHandshakeRecordReassembler::new(
            server_state,
            handshake_policy,
            &mut reassembly_storage,
        )
        .unwrap();
        let ordered = RetryOrderedServerFlight::new(reassembler).unwrap();
        let mut bound = bind_retry_ordered_server_flight_transcript(ordered, continuation).unwrap();
        let server_prk = HkdfPrk::import_exact(
            fixed_hash(suite),
            issuer.traffic_secrets().traffic_secrets().server_secret(),
        )
        .unwrap();
        let mut peer_storage = TrafficKeyMaterialStorage::new(suite).unwrap();
        let peer_material = derive_traffic_key_material(&server_prk, &mut peer_storage).unwrap();
        let mut peer_state = RecordProtectionState::new(peer_material).unwrap();
        // ALPN (type 16) is allowed in EncryptedExtensions but the retained
        // final ClientHello fixture did not offer it.
        let encrypted_extensions_body = [0, 4, 0, 16, 0, 0];
        let mut plaintext = [0_u8; 10];
        plaintext[..4].copy_from_slice(&[8, 0, 0, 6]);
        plaintext[4..].copy_from_slice(&encrypted_extensions_body);
        let ciphertext_policy = Tls13CiphertextPolicy::new(1_024).unwrap();
        let inner_policy =
            Tls13InnerPlaintextPolicy::new(&[TlsContentType::Handshake], 1_008).unwrap();
        let mut scratch = [0xa5; 32];
        let mut wire = [0xcc; 64];
        let used = seal_tls13_record(
            &mut peer_state,
            &ciphertext_policy,
            &inner_policy,
            Tls13Plaintext::new(TlsContentType::Handshake, &plaintext, 0),
            &mut scratch,
            &mut wire,
        )
        .unwrap();
        let mut opened = [0x5a; 32];
        let mut saw_unsolicited = false;
        let error = bound
            .open_accept_and_advance(
                &ciphertext_policy,
                &inner_policy,
                &wire[..used],
                &mut opened,
                |pending| {
                    saw_unsolicited = matches!(
                        pending.validate_encrypted_extensions(
                            ExtensionBlockLimits::new(64, 4).unwrap()
                        ),
                        Err(
                            tls13_retry_handshake_traffic_key_material_binding::RetryEncryptedExtensionsValidationError::Context(
                                tls13_extension_context_policy::ExtensionContextPolicyError::UnsolicitedExtension
                            )
                        )
                    );
                    false
                },
            )
            .unwrap_err();
        assert!(saw_unsolicited);
        assert_eq!(
            error,
            RetryOrderedServerTranscriptAdvanceError::MessageRejected
        );
        assert_eq!(bound.transcript().snapshot(), retained_snapshot);
        assert!(bound.ordered_flight().is_failed());
    });
}

#[test]
fn validates_exact_main_handshake_certificate_request_and_signature_list() {
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
                Tls13HandshakeType::CertificateRequest,
            ],
            32,
        )
        .unwrap();
        let mut reassembly_storage = [0xa5; 64];
        let reassembler = RetryHandshakeRecordReassembler::new(
            server_state,
            handshake_policy,
            &mut reassembly_storage,
        )
        .unwrap();
        let ordered = RetryOrderedServerFlight::new(reassembler).unwrap();
        let mut bound = bind_retry_ordered_server_flight_transcript(ordered, continuation).unwrap();
        let server_prk = HkdfPrk::import_exact(
            fixed_hash(suite),
            issuer.traffic_secrets().traffic_secrets().server_secret(),
        )
        .unwrap();
        let mut peer_storage = TrafficKeyMaterialStorage::new(suite).unwrap();
        let peer_material = derive_traffic_key_material(&server_prk, &mut peer_storage).unwrap();
        let mut peer_state = RecordProtectionState::new(peer_material).unwrap();
        let certificate_request_body = [0, 0, 16, 0, 13, 0, 4, 0, 2, 8, 4, 0, 50, 0, 4, 0, 2, 8, 5];
        let mut plaintext = [0_u8; 29];
        plaintext[..6].copy_from_slice(&[8, 0, 0, 2, 0, 0]);
        plaintext[6..10].copy_from_slice(&[13, 0, 0, 19]);
        plaintext[10..].copy_from_slice(&certificate_request_body);
        let ciphertext_policy = Tls13CiphertextPolicy::new(1_024).unwrap();
        let inner_policy =
            Tls13InnerPlaintextPolicy::new(&[TlsContentType::Handshake], 1_008).unwrap();
        let mut scratch = [0xa5; 64];
        let mut wire = [0xcc; 96];
        let used = seal_tls13_record(
            &mut peer_state,
            &ciphertext_policy,
            &inner_policy,
            Tls13Plaintext::new(TlsContentType::Handshake, &plaintext, 0),
            &mut scratch,
            &mut wire,
        )
        .unwrap();
        let mut opened = [0x5a; 64];
        let mut validated_request = false;
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
                    Tls13HandshakeType::CertificateRequest => {
                        let proof = pending
                            .validate_certificate_request(
                                CertificateRequestLimits::new(
                                    0,
                                    ExtensionBlockLimits::new(64, 4).unwrap(),
                                )
                                .unwrap(),
                                SignatureSchemeListLimits::new(8).unwrap(),
                            )
                            .unwrap();
                        assert_eq!(
                            proof.pending().transcript_snapshot(),
                            pending.transcript_snapshot(),
                        );
                        assert_eq!(
                            proof.certificate_request().encoded_body(),
                            certificate_request_body,
                        );
                        assert!(proof.certificate_request().context().is_empty());
                        assert_eq!(proof.signature_algorithms().len(), 1);
                        assert_eq!(proof.signature_algorithms().get(0), Some(0x0804));
                        let signature_algorithms_cert = proof.signature_algorithms_cert().unwrap();
                        assert_eq!(signature_algorithms_cert.len(), 1);
                        assert_eq!(signature_algorithms_cert.get(0), Some(0x0805));
                        let debug = format!("{proof:?}");
                        assert!(debug.contains("[RETAINED]"));
                        assert!(debug.contains("[REDACTED]"));
                        assert!(!debug.contains("2052"));
                        validated_request = true;
                        true
                    }
                    other => panic!("unexpected handshake type {other:?}"),
                },
            )
            .unwrap();
        assert!(validated_request);
        assert_eq!(result.completed_messages(), 2);
        assert_eq!(result.state(), ServerFlightState::ExpectCertificate);
        assert!(result.client_authentication_requested());
    });
}

#[test]
fn validates_exact_nonempty_server_certificate_and_entry_extensions() {
    let mut x25519_output = [0xa5; 32];
    with_retry_x25519_messages(CipherSuite::TlsAes128GcmSha256, |messages| {
        let leaf_der = decode_fixture_hex(include_str!(
            "../../tls13-reviewed-server-path-certificate-verify/tests/fixtures/apple_www_2026_11_11_leaf.der.hex"
        ));
        let leaf = decode_x509_certificate(&leaf_der, Asn1Limits::default()).unwrap();
        let other_der = decode_fixture_hex(include_str!(
            "../../x509-ocsp-reviewed-server-revocation-binding/tests/fixtures/apple-ev-intermediate.der.hex"
        ));
        let other = decode_x509_certificate(&other_der, Asn1Limits::default()).unwrap();
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
            fixed_hash(suite),
            issuer.traffic_secrets().traffic_secrets().server_secret(),
        )
        .unwrap();
        let mut peer_storage = TrafficKeyMaterialStorage::new(suite).unwrap();
        let peer_material = derive_traffic_key_material(&server_prk, &mut peer_storage).unwrap();
        let mut peer_state = RecordProtectionState::new(peer_material).unwrap();
        let encoded_extensions = [0, 4, 0, 5, 0, 0];
        let other_encoded_extensions = [0, 0];
        let certificate_list_length = 3
            + leaf_der.len()
            + encoded_extensions.len()
            + 3
            + other_der.len()
            + other_encoded_extensions.len();
        let mut certificate_body = Vec::with_capacity(4 + certificate_list_length);
        certificate_body.push(0);
        certificate_body.extend_from_slice(&encode_u24(certificate_list_length));
        certificate_body.extend_from_slice(&encode_u24(leaf_der.len()));
        certificate_body.extend_from_slice(&leaf_der);
        certificate_body.extend_from_slice(&encoded_extensions);
        certificate_body.extend_from_slice(&encode_u24(other_der.len()));
        certificate_body.extend_from_slice(&other_der);
        certificate_body.extend_from_slice(&other_encoded_extensions);
        let mut plaintext = Vec::with_capacity(10 + certificate_body.len());
        plaintext.extend_from_slice(&[8, 0, 0, 2, 0, 0]);
        plaintext.push(11);
        plaintext.extend_from_slice(&encode_u24(certificate_body.len()));
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
        let mut validated_certificate = false;
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
                        let redacted_structure_error = pending
                            .validate_server_certificate(
                                Tls13CertificateMessageLimits::new(
                                    certificate_body.len() - 1,
                                    0,
                                    certificate_list_length,
                                    3,
                                    leaf_der.len(),
                                    16,
                                ),
                                ExtensionBlockLimits::new(16, 4).unwrap(),
                            )
                            .unwrap_err();
                        assert!(matches!(
                            redacted_structure_error,
                            RetryServerCertificateValidationError::Structure(
                                tls13_certificate_message::Tls13CertificateMessageErrorKind::MessageTooLarge
                            )
                        ));
                        assert_eq!(
                            redacted_structure_error.to_string(),
                            "TLS 1.3 retry server Certificate structure is invalid"
                        );
                        assert!(redacted_structure_error.source().is_none());
                        let error_debug = format!("{redacted_structure_error:?}");
                        assert!(!error_debug.contains("offset"));
                        assert!(!error_debug.contains("at byte"));
                        assert!(!error_debug.contains("48"));
                        let proof = pending
                            .validate_server_certificate(
                                Tls13CertificateMessageLimits::new(
                                    certificate_body.len(),
                                    0,
                                    certificate_list_length,
                                    3,
                                    leaf_der.len(),
                                    16,
                                ),
                                ExtensionBlockLimits::new(16, 4).unwrap(),
                            )
                            .unwrap();
                        assert_eq!(
                            proof.pending().transcript_snapshot(),
                            pending.transcript_snapshot(),
                        );
                        let certificate = proof.certificate();
                        assert_eq!(certificate.encoded(), certificate_body.as_slice());
                        assert!(certificate.request_context().is_empty());
                        assert_eq!(certificate.certificate_count(), 2);
                        let mut entries = certificate.entries();
                        let first_entry = entries.next().unwrap();
                        assert_eq!(first_entry.certificate_data(), leaf_der.as_slice());
                        assert_eq!(first_entry.encoded_extensions(), encoded_extensions);
                        let second_entry = entries.next().unwrap();
                        assert_eq!(second_entry.certificate_data(), other_der.as_slice());
                        assert_eq!(
                            second_entry.encoded_extensions(),
                            other_encoded_extensions
                        );
                        assert!(entries.next().is_none());
                        let leaf_mismatch = bind_retry_server_certificate_leaf(&proof, &other)
                            .unwrap_err();
                        assert_eq!(
                            leaf_mismatch,
                            RetryServerCertificateLeafBindingError::LeafCertificateMismatch
                        );
                        let leaf_proof =
                            bind_retry_server_certificate_leaf(&proof, &leaf).unwrap();
                        assert!(std::ptr::eq(
                            leaf_proof.server_certificate(),
                            &proof,
                        ));
                        assert!(std::ptr::eq(leaf_proof.leaf_certificate(), &leaf));
                        assert_eq!(leaf_proof.leaf_certificate().encoded(), leaf_der);
                        let leaf_debug = format!("{leaf_proof:?}");
                        assert!(leaf_debug.contains("[RETAINED]"));
                        assert!(leaf_debug.contains("[REDACTED]"));
                        assert!(!leaf_debug.contains("apple"));
                        let debug = format!("{proof:?}");
                        assert!(debug.contains("[RETAINED]"));
                        assert!(debug.contains("[REDACTED]"));
                        assert!(!debug.contains("48"));
                        validated_certificate = true;
                        true
                    }
                    other => panic!("unexpected handshake type {other:?}"),
                },
            )
            .unwrap();
        assert!(validated_certificate);
        assert_eq!(result.completed_messages(), 2);
        assert_eq!(result.state(), ServerFlightState::ExpectCertificateVerify);
        assert!(!result.client_authentication_requested());
    });
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OrderedFinishedCase {
    Success,
    DifferentKey,
    DifferentOwner,
    InvalidLength,
    InvalidVerifyData,
    TrailingFragment,
}

#[test]
fn validates_exact_server_certificate_verify_and_finished() {
    run_exact_server_certificate_verify_and_finished(OrderedFinishedCase::Success);
}

#[test]
fn rejects_post_finished_proof_when_the_record_has_a_trailing_fragment() {
    run_exact_server_certificate_verify_and_finished(OrderedFinishedCase::TrailingFragment);
}

#[test]
fn rejects_post_finished_proof_from_a_different_key_or_owner() {
    run_exact_server_certificate_verify_and_finished(OrderedFinishedCase::DifferentKey);
    run_exact_server_certificate_verify_and_finished(OrderedFinishedCase::DifferentOwner);
}

#[test]
fn rejects_invalid_finished_without_transcript_advancement() {
    run_exact_server_certificate_verify_and_finished(OrderedFinishedCase::InvalidLength);
    run_exact_server_certificate_verify_and_finished(OrderedFinishedCase::InvalidVerifyData);
}

fn run_exact_server_certificate_verify_and_finished(finished_case: OrderedFinishedCase) {
    let mut x25519_output = [0xa5; 32];
    with_retry_x25519_messages(CipherSuite::TlsAes128GcmSha256, |messages| {
        let signature = decode_hex(SERVER_SHA256_SIGNATURE_HEX);
        let certificate_verify_message =
            CertificateVerifyMessage::new(SignatureScheme::RsaPssRsaeSha256.wire_id(), &signature)
                .unwrap();
        let mut certificate_verify_body = Vec::with_capacity(4 + signature.len());
        certificate_verify_body
            .extend_from_slice(&SignatureScheme::RsaPssRsaeSha256.wire_id().to_be_bytes());
        certificate_verify_body.extend_from_slice(&(signature.len() as u16).to_be_bytes());
        certificate_verify_body.extend_from_slice(&signature);
        let certificate_body = [0, 0, 0, 8, 0, 0, 3, 0x30, 1, 0, 0, 0];
        let mut plaintext = Vec::with_capacity(26 + certificate_verify_body.len());
        plaintext.extend_from_slice(&[8, 0, 0, 2, 0, 0]);
        plaintext.extend_from_slice(&[11, 0, 0, 12]);
        plaintext.extend_from_slice(&certificate_body);
        plaintext.extend_from_slice(&[
            15,
            ((certificate_verify_body.len() >> 16) & 0xff) as u8,
            ((certificate_verify_body.len() >> 8) & 0xff) as u8,
            (certificate_verify_body.len() & 0xff) as u8,
        ]);
        plaintext.extend_from_slice(&certificate_verify_body);

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
        let allowed = [SignatureScheme::RsaPssRsaeSha256];
        let policy = SignatureSchemePolicy::new(&allowed).unwrap();
        let preflight_transcript = transcript.reconstruct_transcript().unwrap();
        let mut verifier_transcript = transcript.reconstruct_transcript().unwrap();
        verifier_transcript.update(&plaintext[..22]).unwrap();
        let mut signed_input_output = [0_u8; MAXIMUM_INPUT_LENGTH];
        let verifier_input = bind_certificate_verify_transcript(
            &verifier_transcript,
            CertificateVerifyRole::Server,
            &mut signed_input_output,
        )
        .unwrap();
        let verifier_hash = hash_transcript_bound_certificate_verify_input(
            policy
                .select(SignatureScheme::RsaPssRsaeSha256.wire_id())
                .unwrap(),
            &verifier_input,
        );
        let public_key_bytes = decode_hex(PUBLIC_KEY_SPKI_HEX);
        let public_key = validated_rsa_key(&public_key_bytes);
        let mut verification_scratch = [0xa5; 223];
        let verification = verify_transcript_bound_certificate_verify(
            &verifier_hash,
            &certificate_verify_message,
            public_key,
            &mut verification_scratch,
            Tls13RsaPssSignatureLimits::Sha256(RsaPssSha256SignatureLimits::new(300, 300).unwrap()),
        )
        .unwrap();

        let client_signature = decode_hex(CLIENT_SHA256_SIGNATURE_HEX);
        let client_message = CertificateVerifyMessage::new(
            SignatureScheme::RsaPssRsaeSha256.wire_id(),
            &client_signature,
        )
        .unwrap();
        let mut client_output = [0_u8; MAXIMUM_INPUT_LENGTH];
        let client_input = bind_certificate_verify_transcript(
            &verifier_transcript,
            CertificateVerifyRole::Client,
            &mut client_output,
        )
        .unwrap();
        let client_hash = hash_transcript_bound_certificate_verify_input(
            policy
                .select(SignatureScheme::RsaPssRsaeSha256.wire_id())
                .unwrap(),
            &client_input,
        );
        let mut client_scratch = [0xa5; 223];
        let client_verification = verify_transcript_bound_certificate_verify(
            &client_hash,
            &client_message,
            public_key,
            &mut client_scratch,
            Tls13RsaPssSignatureLimits::Sha256(RsaPssSha256SignatureLimits::new(300, 300).unwrap()),
        )
        .unwrap();

        let preflight_signature = decode_hex(PREFLIGHT_SERVER_SHA256_SIGNATURE_HEX);
        let preflight_message = CertificateVerifyMessage::new(
            SignatureScheme::RsaPssRsaeSha256.wire_id(),
            &preflight_signature,
        )
        .unwrap();
        let mut preflight_output = [0_u8; MAXIMUM_INPUT_LENGTH];
        let preflight_input = bind_certificate_verify_transcript(
            &preflight_transcript,
            CertificateVerifyRole::Server,
            &mut preflight_output,
        )
        .unwrap();
        let preflight_hash = hash_transcript_bound_certificate_verify_input(
            policy
                .select(SignatureScheme::RsaPssRsaeSha256.wire_id())
                .unwrap(),
            &preflight_input,
        );
        let mut preflight_scratch = [0xa5; 223];
        let preflight_verification = verify_transcript_bound_certificate_verify(
            &preflight_hash,
            &preflight_message,
            public_key,
            &mut preflight_scratch,
            Tls13RsaPssSignatureLimits::Sha256(RsaPssSha256SignatureLimits::new(300, 300).unwrap()),
        )
        .unwrap();

        let alternate_signature = decode_hex(ALTERNATE_SERVER_SHA256_SIGNATURE_HEX);
        let alternate_message = CertificateVerifyMessage::new(
            SignatureScheme::RsaPssRsaeSha256.wire_id(),
            &alternate_signature,
        )
        .unwrap();
        let mut alternate_scratch = [0xa5; 223];
        let alternate_verification = verify_transcript_bound_certificate_verify(
            &verifier_hash,
            &alternate_message,
            public_key,
            &mut alternate_scratch,
            Tls13RsaPssSignatureLimits::Sha256(RsaPssSha256SignatureLimits::new(300, 300).unwrap()),
        )
        .unwrap();

        let sha384_signature = decode_hex(SERVER_SHA384_SIGNATURE_HEX);
        let sha384_message = CertificateVerifyMessage::new(
            SignatureScheme::RsaPssRsaeSha384.wire_id(),
            &sha384_signature,
        )
        .unwrap();
        let sha384_allowed = [SignatureScheme::RsaPssRsaeSha384];
        let sha384_policy = SignatureSchemePolicy::new(&sha384_allowed).unwrap();
        let sha384_hash = hash_transcript_bound_certificate_verify_input(
            sha384_policy
                .select(SignatureScheme::RsaPssRsaeSha384.wire_id())
                .unwrap(),
            &verifier_input,
        );
        let mut sha384_scratch = [0xa5; 207];
        let sha384_verification = verify_transcript_bound_certificate_verify(
            &sha384_hash,
            &sha384_message,
            public_key,
            &mut sha384_scratch,
            Tls13RsaPssSignatureLimits::Sha384(RsaPssSha384SignatureLimits::new(300, 300).unwrap()),
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
        let mut alternate_finished_storage = FinishedKeyStorage::new(suite, FinishedRole::Server);
        let alternate_finished = bind_retry_server_finished_key(
            issuer.traffic_secrets(),
            &mut alternate_finished_storage,
        )
        .unwrap();
        let mut detached_base_storage = BaseSecretStorage::new(suite).unwrap();
        let detached_base =
            bind_retry_key_schedule(&secret, &transcript, &mut detached_base_storage).unwrap();
        let mut detached_traffic_storage = HandshakeTrafficSecretStorage::new(suite).unwrap();
        let detached_traffic =
            bind_retry_handshake_traffic_secrets(&detached_base, &mut detached_traffic_storage)
                .unwrap();
        let detached_issuer = RetryHandshakeRecordProtectionStateIssuer::new(detached_traffic);
        let mut detached_finished_storage = FinishedKeyStorage::new(suite, FinishedRole::Server);
        let detached_finished = bind_retry_server_finished_key(
            detached_issuer.traffic_secrets(),
            &mut detached_finished_storage,
        )
        .unwrap();
        let mut finished_transcript = transcript.reconstruct_transcript().unwrap();
        finished_transcript.update(&plaintext).unwrap();
        let finished_snapshot = finished_transcript.snapshot();
        let mut finished_verify_data = [0_u8; 48];
        let finished_verify_data = &mut finished_verify_data[..finished_snapshot.as_bytes().len()];
        server_finished
            .finished_key()
            .write_verify_data(finished_snapshot.as_bytes(), finished_verify_data)
            .unwrap();
        if finished_case == OrderedFinishedCase::InvalidVerifyData {
            finished_verify_data[0] ^= 1;
        }
        let finished_length = if finished_case == OrderedFinishedCase::InvalidLength {
            finished_verify_data.len() - 1
        } else {
            finished_verify_data.len()
        };
        let emitted_finished_verify_data = &finished_verify_data[..finished_length];
        plaintext.extend_from_slice(&[
            Tls13HandshakeType::Finished.wire_value(),
            0,
            0,
            emitted_finished_verify_data.len() as u8,
        ]);
        plaintext.extend_from_slice(emitted_finished_verify_data);
        finished_transcript
            .update(&plaintext[plaintext.len() - finished_length - 4..])
            .unwrap();
        let expected_post_finished = finished_transcript.snapshot();
        if finished_case == OrderedFinishedCase::TrailingFragment {
            plaintext.push(Tls13HandshakeType::Finished.wire_value());
        }
        let continuation = bind_retry_server_transcript_continuation(&server_finished).unwrap();
        let mut server_storage = TrafficKeyMaterialStorage::new(suite).unwrap();
        let server_state = issuer
            .issue(HandshakeTrafficSender::Server, &mut server_storage)
            .unwrap();
        let handshake_policy = Tls13HandshakePolicy::new(
            &[
                Tls13HandshakeType::EncryptedExtensions,
                Tls13HandshakeType::Certificate,
                Tls13HandshakeType::CertificateVerify,
                Tls13HandshakeType::Finished,
            ],
            512,
        )
        .unwrap();
        let mut reassembly_storage = [0xa5; 384];
        let reassembler = RetryHandshakeRecordReassembler::new(
            server_state,
            handshake_policy,
            &mut reassembly_storage,
        )
        .unwrap();
        let ordered = RetryOrderedServerFlight::new(reassembler).unwrap();
        let mut bound = bind_retry_ordered_server_flight_transcript(ordered, continuation).unwrap();
        let server_prk = HkdfPrk::import_exact(
            fixed_hash(suite),
            issuer.traffic_secrets().traffic_secrets().server_secret(),
        )
        .unwrap();
        let mut peer_storage = TrafficKeyMaterialStorage::new(suite).unwrap();
        let peer_material = derive_traffic_key_material(&server_prk, &mut peer_storage).unwrap();
        let mut peer_state = RecordProtectionState::new(peer_material).unwrap();
        let ciphertext_policy = Tls13CiphertextPolicy::new(1_024).unwrap();
        let inner_policy =
            Tls13InnerPlaintextPolicy::new(&[TlsContentType::Handshake], 1_008).unwrap();
        let mut scratch = [0xa5; 384];
        let mut wire = [0xcc; 384];
        let used = seal_tls13_record(
            &mut peer_state,
            &ciphertext_policy,
            &inner_policy,
            Tls13Plaintext::new(TlsContentType::Handshake, &plaintext, 0),
            &mut scratch,
            &mut wire,
        )
        .unwrap();
        let mut opened = [0x5a; 384];
        let disallowed = [SignatureScheme::RsaPssRsaeSha384];
        let disallowed_policy = SignatureSchemePolicy::new(&disallowed).unwrap();
        let mut validated_certificate_verify = false;
        let mut verified_finished = None;
        let mut alternate_key_verdict = None;
        let mut rejected_finished_snapshot = None;
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
                        pending
                            .validate_server_certificate(
                                Tls13CertificateMessageLimits::new(64, 0, 32, 2, 16, 16),
                                ExtensionBlockLimits::new(16, 4).unwrap(),
                            )
                            .unwrap();
                        assert_eq!(
                            pending
                                .validate_server_certificate_verify(
                                    SignatureSchemeListLimits::new(8).unwrap(),
                                    policy,
                                )
                                .unwrap_err(),
                            RetryServerCertificateVerifyValidationError::WrongMessageType,
                        );
                        true
                    }
                    Tls13HandshakeType::CertificateVerify => {
                        assert_eq!(pending.state(), ServerFlightState::ExpectFinished);
                        assert_eq!(
                            pending
                                .validate_server_certificate_verify(
                                    SignatureSchemeListLimits::new(8).unwrap(),
                                    disallowed_policy,
                                )
                                .unwrap_err(),
                            RetryServerCertificateVerifyValidationError::Policy(
                                PolicyError::DisallowedSignatureScheme,
                            ),
                        );
                        let proof = pending
                            .validate_server_certificate_verify(
                                SignatureSchemeListLimits::new(8).unwrap(),
                                policy,
                            )
                            .unwrap();
                        assert_eq!(
                            proof.pending().transcript_snapshot(),
                            pending.transcript_snapshot(),
                        );
                        assert_eq!(proof.message().algorithm_wire_id(), 0x0804);
                        assert_eq!(proof.message().signature(), signature);
                        assert_eq!(proof.offered_signature_algorithms().len(), 1);
                        assert_eq!(proof.offered_signature_algorithms().get(0), Some(0x0804));
                        assert_eq!(
                            proof.authorized_scheme().scheme(),
                            SignatureScheme::RsaPssRsaeSha256,
                        );
                        let debug = format!("{proof:?}");
                        assert!(debug.contains("[RETAINED]"));
                        assert!(debug.contains("[REDACTED]"));
                        assert!(!debug.contains("aabbcc"));
                        assert_eq!(
                            bind_retry_server_certificate_verify_signature(
                                &proof,
                                &client_verification,
                            )
                            .unwrap_err(),
                            RetryServerCertificateVerifySignatureBindingError::WrongEndpointRole,
                        );
                        assert_eq!(
                            bind_retry_server_certificate_verify_signature(
                                &proof,
                                &preflight_verification,
                            )
                            .unwrap_err(),
                            RetryServerCertificateVerifySignatureBindingError::TranscriptSnapshotMismatch,
                        );
                        assert_eq!(
                            bind_retry_server_certificate_verify_signature(
                                &proof,
                                &sha384_verification,
                            )
                            .unwrap_err(),
                            RetryServerCertificateVerifySignatureBindingError::SignatureSchemeMismatch,
                        );
                        assert_eq!(
                            bind_retry_server_certificate_verify_signature(
                                &proof,
                                &alternate_verification,
                            )
                            .unwrap_err(),
                            RetryServerCertificateVerifySignatureBindingError::MessageMismatch,
                        );
                        let signed = bind_retry_server_certificate_verify_signature(
                            &proof,
                            &verification,
                        )
                        .unwrap();
                        assert!(core::ptr::eq(signed.binding(), &proof));
                        assert!(core::ptr::eq(signed.verification(), &verification));
                        assert_eq!(
                            format!("{signed:?}"),
                            "VerifiedBoundRetryServerCertificateVerifySignature",
                        );
                        assert_eq!(
                            verify_ordered_retry_server_finished(&pending, &server_finished)
                                .unwrap_err(),
                            OrderedRetryServerFinishedVerificationError::WrongMessageType,
                        );
                        validated_certificate_verify = true;
                        true
                    }
                    Tls13HandshakeType::Finished => {
                        assert_eq!(pending.state(), ServerFlightState::Complete);
                        assert_eq!(pending.transcript_snapshot(), finished_snapshot);
                        assert_eq!(
                            verify_ordered_retry_server_finished(&pending, &detached_finished)
                                .unwrap_err(),
                            OrderedRetryServerFinishedVerificationError::RetryEvidenceMismatch,
                        );
                        let proof = match verify_ordered_retry_server_finished(
                            &pending,
                            &server_finished,
                        ) {
                            Ok(proof) => proof,
                            Err(error) => {
                                let expected = match finished_case {
                                    OrderedFinishedCase::InvalidLength => {
                                        OrderedRetryServerFinishedVerificationError::PeerVerifyDataLengthMismatch
                                    }
                                    OrderedFinishedCase::InvalidVerifyData => {
                                        OrderedRetryServerFinishedVerificationError::VerifyDataMismatch
                                    }
                                    _ => {
                                        panic!("valid Finished unexpectedly failed: {error}")
                                    }
                                };
                                assert_eq!(error, expected);
                                assert!(!error.to_string().contains("retry-key-material"));
                                rejected_finished_snapshot =
                                    Some(pending.transcript_snapshot());
                                return false;
                            }
                        };
                        assert_eq!(proof.transcript_snapshot(), finished_snapshot);
                        assert!(!proof.client_authentication_requested());
                        assert!(core::ptr::eq(
                            proof.server_finished_key(),
                            &server_finished,
                        ));
                        assert_eq!(
                            proof.verification().peer_verify_data(),
                            pending.message().body(),
                        );
                        assert_eq!(
                            proof.verification().peer_verify_data(),
                            emitted_finished_verify_data,
                        );
                        let debug = format!("{proof:?}");
                        assert!(debug.contains("[SECRET]"));
                        assert!(debug.contains("[REDACTED]"));
                        assert!(!debug.contains(&format!("{finished_verify_data:?}")));
                        alternate_key_verdict = Some(
                            verify_ordered_retry_server_finished(&pending, &alternate_finished)
                                .unwrap(),
                        );
                        verified_finished = Some(proof);
                        true
                    }
                    other => panic!("unexpected handshake type {other:?}"),
                },
            );
        assert!(validated_certificate_verify);
        match finished_case {
            OrderedFinishedCase::Success => {
                let result = result.unwrap();
                assert_eq!(result.completed_messages(), 4);
                assert_eq!(result.state(), ServerFlightState::Complete);
                assert!(result.is_complete());
                let committed = result
                    .bind_committed_finished(verified_finished.unwrap())
                    .unwrap();
                assert_eq!(committed.transcript_snapshot(), expected_post_finished);
                assert_ne!(committed.transcript_snapshot(), finished_snapshot);
                assert_eq!(
                    committed.finished().transcript_snapshot(),
                    finished_snapshot
                );
                assert_eq!(
                    committed.finished().verification().peer_verify_data(),
                    emitted_finished_verify_data,
                );
                assert_eq!(
                    format!("{committed:?}"),
                    "CommittedOrderedRetryServerFinished([REDACTED])",
                );
            }
            OrderedFinishedCase::DifferentKey => {
                assert_eq!(
                    result
                        .unwrap()
                        .bind_committed_finished(alternate_key_verdict.unwrap())
                        .unwrap_err(),
                    OrderedRetryServerFinishedCommitError::FinishedKeyMismatch,
                );
            }
            OrderedFinishedCase::DifferentOwner => {
                let result = result.unwrap();
                // Replay the fixture in a separate, byte-equal connection in
                // this local test. Its independently minted verdict is valid
                // only for that connection's ordered record owner.
                let mut detached_storage = TrafficKeyMaterialStorage::new(suite).unwrap();
                let detached_state = detached_issuer
                    .issue(HandshakeTrafficSender::Server, &mut detached_storage)
                    .unwrap();
                let mut detached_reassembly = [0_u8; 384];
                let detached_reassembler = RetryHandshakeRecordReassembler::new(
                    detached_state,
                    handshake_policy,
                    &mut detached_reassembly,
                )
                .unwrap();
                let mut detached_bound = bind_retry_ordered_server_flight_transcript(
                    RetryOrderedServerFlight::new(detached_reassembler).unwrap(),
                    bind_retry_server_transcript_continuation(&detached_finished).unwrap(),
                )
                .unwrap();
                let mut detached_verdict = None;
                detached_bound
                    .open_accept_and_advance(
                        &ciphertext_policy,
                        &inner_policy,
                        &wire[..used],
                        &mut opened,
                        |pending| {
                            if pending.message().handshake_type() == Tls13HandshakeType::Finished {
                                detached_verdict = Some(
                                    verify_ordered_retry_server_finished(
                                        &pending,
                                        &detached_finished,
                                    )
                                    .unwrap(),
                                );
                            }
                            true
                        },
                    )
                    .unwrap();
                assert_eq!(
                    result
                        .bind_committed_finished(detached_verdict.unwrap())
                        .unwrap_err(),
                    OrderedRetryServerFinishedCommitError::RecordOwnerMismatch,
                );
            }
            OrderedFinishedCase::TrailingFragment => {
                let result = result.unwrap();
                assert!(result.is_complete());
                assert!(result.message_in_progress());
                assert_eq!(
                    result
                        .bind_committed_finished(verified_finished.unwrap())
                        .unwrap_err(),
                    OrderedRetryServerFinishedCommitError::TrailingHandshakeFragment,
                );
            }
            OrderedFinishedCase::InvalidLength | OrderedFinishedCase::InvalidVerifyData => {
                assert_eq!(
                    result.unwrap_err(),
                    RetryOrderedServerTranscriptAdvanceError::MessageRejected,
                );
                let rejected_snapshot =
                    rejected_finished_snapshot.expect("Finished callback rejected");
                assert_eq!(rejected_snapshot, finished_snapshot);
                assert_eq!(bound.transcript().snapshot(), rejected_snapshot);
                assert!(bound.ordered_flight().is_failed());
                assert_eq!(bound.ordered_flight().state(), ServerFlightState::Complete);
                assert!(verified_finished.is_none());
            }
        }
    });
}

#[test]
fn rejects_invalid_server_certificate_verify_without_transcript_advancement() {
    assert_retry_certificate_verify_rejected(
        Some(&[0, 2, 8, 4]),
        &[8, 5, 0, 1, 0xaa],
        RetryServerCertificateVerifyValidationError::UnofferedSignatureScheme,
    );
    assert_retry_certificate_verify_rejected(
        Some(&[0, 2, 8, 4]),
        &[8, 4, 0, 2, 0xaa],
        RetryServerCertificateVerifyValidationError::Structure(
            CertificateVerifyMessageError::SignatureLengthMismatch,
        ),
    );
    assert_retry_certificate_verify_rejected(
        Some(&[0, 3, 8, 4, 8]),
        &[8, 4, 0, 1, 0xaa],
        RetryServerCertificateVerifyValidationError::SignatureAlgorithms(
            tls13_signature_scheme_list::SignatureSchemeListError::OddListLength,
        ),
    );
    assert_retry_certificate_verify_rejected(
        None,
        &[8, 4, 0, 1, 0xaa],
        RetryServerCertificateVerifyValidationError::MissingSignatureAlgorithms,
    );
}

#[test]
fn rejects_an_ordered_owner_from_another_traffic_secret_proof() {
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
        let mut first_storage = HandshakeTrafficSecretStorage::new(suite).unwrap();
        let first = bind_retry_handshake_traffic_secrets(&base, &mut first_storage).unwrap();
        let issuer = RetryHandshakeRecordProtectionStateIssuer::new(first);
        let mut server_storage = TrafficKeyMaterialStorage::new(suite).unwrap();
        let server_state = issuer
            .issue(HandshakeTrafficSender::Server, &mut server_storage)
            .unwrap();
        let handshake_policy =
            Tls13HandshakePolicy::new(&[Tls13HandshakeType::EncryptedExtensions], 16).unwrap();
        let mut reassembly_storage = [0xa5; 64];
        let reassembler = RetryHandshakeRecordReassembler::new(
            server_state,
            handshake_policy,
            &mut reassembly_storage,
        )
        .unwrap();
        let ordered = RetryOrderedServerFlight::new(reassembler).unwrap();

        let mut second_storage = HandshakeTrafficSecretStorage::new(suite).unwrap();
        let second = bind_retry_handshake_traffic_secrets(&base, &mut second_storage).unwrap();
        let mut finished_storage = FinishedKeyStorage::new(suite, FinishedRole::Server);
        let server_finished =
            bind_retry_server_finished_key(&second, &mut finished_storage).unwrap();
        let continuation = bind_retry_server_transcript_continuation(&server_finished).unwrap();

        assert_eq!(
            bind_retry_ordered_server_flight_transcript(ordered, continuation).unwrap_err(),
            RetryOrderedServerFlightTranscriptBindingError::TrafficSecretProofMismatch,
        );
    });
}

#[test]
fn rejection_leaves_the_message_unhashed_and_permanently_closes_the_owner() {
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
            16,
        )
        .unwrap();
        let mut reassembly_storage = [0xa5; 64];
        let reassembler = RetryHandshakeRecordReassembler::new(
            server_state,
            handshake_policy,
            &mut reassembly_storage,
        )
        .unwrap();
        let ordered = RetryOrderedServerFlight::new(reassembler).unwrap();
        let mut bound = bind_retry_ordered_server_flight_transcript(ordered, continuation).unwrap();
        let server_prk = HkdfPrk::import_exact(
            fixed_hash(suite),
            issuer.traffic_secrets().traffic_secrets().server_secret(),
        )
        .unwrap();
        let mut peer_storage = TrafficKeyMaterialStorage::new(suite).unwrap();
        let peer_material = derive_traffic_key_material(&server_prk, &mut peer_storage).unwrap();
        let mut peer_state = RecordProtectionState::new(peer_material).unwrap();
        let plaintext = [8, 0, 0, 1, 0xa1, 11, 0, 0, 1, 0xa2];
        let mut expected = issuer
            .traffic_secrets()
            .evidence()
            .reconstruct_transcript()
            .unwrap();
        expected.update(&plaintext[..5]).unwrap();
        let ciphertext_policy = Tls13CiphertextPolicy::new(1_024).unwrap();
        let inner_policy =
            Tls13InnerPlaintextPolicy::new(&[TlsContentType::Handshake], 1_008).unwrap();
        let mut scratch = [0xa5; 32];
        let mut wire = [0xcc; 64];
        let used = seal_tls13_record(
            &mut peer_state,
            &ciphertext_policy,
            &inner_policy,
            Tls13Plaintext::new(TlsContentType::Handshake, &plaintext, 0),
            &mut scratch,
            &mut wire,
        )
        .unwrap();
        let mut opened = [0x5a; 32];
        let mut seen = Vec::new();
        let error = bound
            .open_accept_and_advance(
                &ciphertext_policy,
                &inner_policy,
                &wire[..used],
                &mut opened,
                |pending| {
                    let handshake_type = pending.message().handshake_type();
                    seen.push(handshake_type);
                    handshake_type != Tls13HandshakeType::Certificate
                },
            )
            .unwrap_err();
        assert_eq!(
            error,
            RetryOrderedServerTranscriptAdvanceError::MessageRejected
        );
        assert_eq!(
            seen,
            [
                Tls13HandshakeType::EncryptedExtensions,
                Tls13HandshakeType::Certificate,
            ]
        );
        assert_eq!(bound.transcript().snapshot(), expected.snapshot());
        assert!(bound.ordered_flight().is_failed());
        assert_eq!(bound.ordered_flight().records_used(), 1);
        opened.fill(0x5a);
        assert_eq!(
            bound
                .open_accept_and_advance(
                    &ciphertext_policy,
                    &inner_policy,
                    &wire[..used],
                    &mut opened,
                    |_| panic!("a failed owner must not invoke the handler"),
                )
                .unwrap_err(),
            RetryOrderedServerTranscriptAdvanceError::Reassembly(
                RetryHandshakeRecordReassemblyError::OwnerFailed,
            )
        );
        assert!(opened.iter().all(|byte| *byte == 0x5a));
        let display = error.to_string();
        let debug = format!("{error:?}");
        assert!(!display.contains("retry-key-material"));
        assert!(!debug.contains("retry-key-material"));
        assert!(error.source().is_none());
    });
}

#[test]
fn transcript_limit_failure_is_terminal_and_retains_the_pre_message_snapshot() {
    let mut x25519_output = [0xa5; 32];
    with_retry_x25519_messages(CipherSuite::TlsAes128GcmSha256, |messages| {
        let observed_before_server_flight = bind_retry_transcript(
            messages.messages().selection(),
            TranscriptPolicy::new(4_096).unwrap(),
        )
        .unwrap()
        .transcript()
        .observed_bytes();
        let secret = derive_message_bound_retry_x25519_shared_secret(
            messages,
            &ALICE_PRIVATE,
            &mut x25519_output,
        )
        .unwrap();
        let transcript = bind_retry_transcript(
            messages.messages().selection(),
            TranscriptPolicy::new(observed_before_server_flight).unwrap(),
        )
        .unwrap();
        let initial_snapshot = transcript.transcript().snapshot();
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
        let handshake_policy =
            Tls13HandshakePolicy::new(&[Tls13HandshakeType::EncryptedExtensions], 16).unwrap();
        let mut reassembly_storage = [0xa5; 32];
        let reassembler = RetryHandshakeRecordReassembler::new(
            server_state,
            handshake_policy,
            &mut reassembly_storage,
        )
        .unwrap();
        let ordered = RetryOrderedServerFlight::new(reassembler).unwrap();
        let mut bound = bind_retry_ordered_server_flight_transcript(ordered, continuation).unwrap();
        let server_prk = HkdfPrk::import_exact(
            fixed_hash(suite),
            issuer.traffic_secrets().traffic_secrets().server_secret(),
        )
        .unwrap();
        let mut peer_storage = TrafficKeyMaterialStorage::new(suite).unwrap();
        let peer_material = derive_traffic_key_material(&server_prk, &mut peer_storage).unwrap();
        let mut peer_state = RecordProtectionState::new(peer_material).unwrap();
        let plaintext = [8, 0, 0, 1, 0xa1];
        let ciphertext_policy = Tls13CiphertextPolicy::new(1_024).unwrap();
        let inner_policy =
            Tls13InnerPlaintextPolicy::new(&[TlsContentType::Handshake], 1_008).unwrap();
        let mut scratch = [0xa5; 32];
        let mut wire = [0xcc; 64];
        let used = seal_tls13_record(
            &mut peer_state,
            &ciphertext_policy,
            &inner_policy,
            Tls13Plaintext::new(TlsContentType::Handshake, &plaintext, 0),
            &mut scratch,
            &mut wire,
        )
        .unwrap();
        let mut opened = [0x5a; 32];
        let mut handler_calls = 0;
        let error = bound
            .open_accept_and_advance(
                &ciphertext_policy,
                &inner_policy,
                &wire[..used],
                &mut opened,
                |pending| {
                    handler_calls += 1;
                    assert_eq!(pending.transcript_snapshot(), initial_snapshot);
                    true
                },
            )
            .unwrap_err();
        match error {
            RetryOrderedServerTranscriptAdvanceError::Transcript(error) => {
                assert_eq!(error.kind(), TranscriptErrorKind::TranscriptLimitExceeded);
            }
            other => panic!("unexpected error: {other:?}"),
        }
        assert_eq!(handler_calls, 1);
        assert_eq!(bound.transcript().snapshot(), initial_snapshot);
        assert!(bound.ordered_flight().is_failed());
        assert_eq!(bound.ordered_flight().records_used(), 1);
    });
}

#[test]
fn authenticates_and_enforces_the_complete_server_flight_before_callbacks() {
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
        let issuer = RetryHandshakeRecordProtectionStateIssuer::new(traffic);

        let mut client_storage = TrafficKeyMaterialStorage::new(suite).unwrap();
        let client_state = issuer
            .issue(HandshakeTrafficSender::Client, &mut client_storage)
            .unwrap();
        let client_policy =
            Tls13HandshakePolicy::new(&[Tls13HandshakeType::EncryptedExtensions], 16).unwrap();
        let mut client_reassembly_storage = [0xa5; 32];
        let client_reassembler = RetryHandshakeRecordReassembler::new(
            client_state,
            client_policy,
            &mut client_reassembly_storage,
        )
        .unwrap();
        assert_eq!(
            RetryOrderedServerFlight::new(client_reassembler).unwrap_err(),
            RetryOrderedServerFlightError::ClientSender
        );

        let mut server_storage = TrafficKeyMaterialStorage::new(suite).unwrap();
        let server_state = issuer
            .issue(HandshakeTrafficSender::Server, &mut server_storage)
            .unwrap();
        let server_prk = HkdfPrk::import_exact(
            fixed_hash(suite),
            issuer.traffic_secrets().traffic_secrets().server_secret(),
        )
        .unwrap();
        let mut peer_storage = TrafficKeyMaterialStorage::new(suite).unwrap();
        let peer_material = derive_traffic_key_material(&server_prk, &mut peer_storage).unwrap();
        let mut peer_state = RecordProtectionState::new(peer_material).unwrap();

        let handshake_policy = Tls13HandshakePolicy::new(
            &[
                Tls13HandshakeType::EncryptedExtensions,
                Tls13HandshakeType::CertificateRequest,
                Tls13HandshakeType::Certificate,
                Tls13HandshakeType::CertificateVerify,
                Tls13HandshakeType::Finished,
            ],
            16,
        )
        .unwrap();
        let mut reassembly_storage = [0xa5; 128];
        let reassembler = RetryHandshakeRecordReassembler::new(
            server_state,
            handshake_policy,
            &mut reassembly_storage,
        )
        .unwrap();
        let mut ordered = RetryOrderedServerFlight::new(reassembler).unwrap();
        assert_eq!(
            ordered.state(),
            ServerFlightState::ExpectEncryptedExtensions
        );
        assert!(!ordered.client_authentication_requested());
        assert!(!ordered.is_complete());

        let plaintext = [
            8, 0, 0, 1, 0xa1, 13, 0, 0, 1, 0xa2, 11, 0, 0, 1, 0xa3, 15, 0, 0, 1, 0xa4, 20, 0, 0, 1,
            0xa5,
        ];
        let ciphertext_policy = Tls13CiphertextPolicy::new(1024).unwrap();
        let inner_policy =
            Tls13InnerPlaintextPolicy::new(&[TlsContentType::Handshake], 1008).unwrap();
        let mut scratch = [0xa5; 64];
        let mut wire = [0xcc; 96];
        let used = seal_tls13_record(
            &mut peer_state,
            &ciphertext_policy,
            &inner_policy,
            Tls13Plaintext::new(TlsContentType::Handshake, &plaintext, 2),
            &mut scratch,
            &mut wire,
        )
        .unwrap();
        let mut opened = [0x5a; 64];
        let mut received = Vec::new();
        {
            let result = ordered
                .open_and_accept(
                    &ciphertext_policy,
                    &inner_policy,
                    &wire[..used],
                    &mut opened,
                    |accepted| {
                        received.push((
                            accepted.message().handshake_type(),
                            accepted.message().body().to_vec(),
                            accepted.state(),
                            accepted.client_authentication_requested(),
                        ));
                    },
                )
                .unwrap();
            assert!(core::ptr::eq(
                result.traffic_secrets(),
                issuer.traffic_secrets()
            ));
            assert_eq!(result.selected_suite(), suite);
            assert_eq!(result.input(), &wire[..used]);
            assert_eq!(result.authenticated_content_length(), plaintext.len());
            assert_eq!(result.completed_messages(), 5);
            assert!(!result.message_in_progress());
            assert_eq!(result.records_used(), 1);
            assert_eq!(result.state(), ServerFlightState::Complete);
            assert!(result.client_authentication_requested());
            assert!(result.is_complete());
            let debug = format!("{result:?}");
            assert!(debug.contains("[RETAINED]"));
            assert!(debug.contains("[REDACTED]"));
            assert!(!debug.contains("a1"));
        }
        assert_eq!(
            received,
            vec![
                (
                    Tls13HandshakeType::EncryptedExtensions,
                    vec![0xa1],
                    ServerFlightState::ExpectCertificateRequestOrCertificate,
                    false,
                ),
                (
                    Tls13HandshakeType::CertificateRequest,
                    vec![0xa2],
                    ServerFlightState::ExpectCertificate,
                    true,
                ),
                (
                    Tls13HandshakeType::Certificate,
                    vec![0xa3],
                    ServerFlightState::ExpectCertificateVerify,
                    true,
                ),
                (
                    Tls13HandshakeType::CertificateVerify,
                    vec![0xa4],
                    ServerFlightState::ExpectFinished,
                    true,
                ),
                (
                    Tls13HandshakeType::Finished,
                    vec![0xa5],
                    ServerFlightState::Complete,
                    true,
                ),
            ]
        );
        assert_eq!(opened[..plaintext.len() + 3], [0; 28]);
        assert_eq!(ordered.state(), ServerFlightState::Complete);
        assert!(ordered.client_authentication_requested());
        assert!(ordered.is_complete());
        assert!(!ordered.is_failed());
        assert_eq!(ordered.records_used(), 1);

        let debug = format!("{ordered:?}");
        assert!(debug.contains("[RETAINED]"));
        assert!(!debug.contains("retry-key-material"));
    });
}

#[test]
fn ordering_failure_poison_closes_before_the_offending_callback() {
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
        let issuer = RetryHandshakeRecordProtectionStateIssuer::new(traffic);
        let mut server_storage = TrafficKeyMaterialStorage::new(suite).unwrap();
        let server_state = issuer
            .issue(HandshakeTrafficSender::Server, &mut server_storage)
            .unwrap();
        let server_prk = HkdfPrk::import_exact(
            fixed_hash(suite),
            issuer.traffic_secrets().traffic_secrets().server_secret(),
        )
        .unwrap();
        let mut peer_storage = TrafficKeyMaterialStorage::new(suite).unwrap();
        let peer_material = derive_traffic_key_material(&server_prk, &mut peer_storage).unwrap();
        let mut peer_state = RecordProtectionState::new(peer_material).unwrap();

        let handshake_policy = Tls13HandshakePolicy::new(
            &[
                Tls13HandshakeType::EncryptedExtensions,
                Tls13HandshakeType::Finished,
            ],
            16,
        )
        .unwrap();
        let mut reassembly_storage = [0xa5; 64];
        let reassembler = RetryHandshakeRecordReassembler::new(
            server_state,
            handshake_policy,
            &mut reassembly_storage,
        )
        .unwrap();
        let mut ordered = RetryOrderedServerFlight::new(reassembler).unwrap();

        let plaintext = [8, 0, 0, 0, 20, 0, 0, 0];
        let ciphertext_policy = Tls13CiphertextPolicy::new(1024).unwrap();
        let inner_policy =
            Tls13InnerPlaintextPolicy::new(&[TlsContentType::Handshake], 1008).unwrap();
        let mut scratch = [0xa5; 32];
        let mut wire = [0xcc; 64];
        let used = seal_tls13_record(
            &mut peer_state,
            &ciphertext_policy,
            &inner_policy,
            Tls13Plaintext::new(TlsContentType::Handshake, &plaintext, 0),
            &mut scratch,
            &mut wire,
        )
        .unwrap();
        let mut opened = [0x5a; 32];
        let mut received = Vec::new();
        let error = ordered
            .open_and_accept(
                &ciphertext_policy,
                &inner_policy,
                &wire[..used],
                &mut opened,
                |accepted| received.push(accepted.message().handshake_type()),
            )
            .unwrap_err();
        assert!(matches!(
            error,
            RetryOrderedServerFlightError::Ordering(ordering)
                if ordering.kind() == ServerFlightErrorKind::UnexpectedMessage
        ));
        assert_eq!(received, vec![Tls13HandshakeType::EncryptedExtensions]);
        assert_eq!(
            ordered.state(),
            ServerFlightState::ExpectCertificateRequestOrCertificate
        );
        assert!(ordered.is_failed());
        assert!(!ordered.message_in_progress());
        assert_eq!(ordered.records_used(), 1);
        assert_eq!(opened[..plaintext.len() + 1], [0; 9]);

        opened.fill(0x5a);
        assert_eq!(
            ordered
                .open_and_accept(
                    &ciphertext_policy,
                    &inner_policy,
                    &wire[..used],
                    &mut opened,
                    |_| panic!("a poisoned owner must not invoke the callback"),
                )
                .unwrap_err(),
            RetryOrderedServerFlightError::Reassembly(
                RetryHandshakeRecordReassemblyError::OwnerFailed
            )
        );
        assert!(opened.iter().all(|byte| *byte == 0x5a));
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
fn rejects_wrong_suite_without_consuming_sender_issuance() {
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

        let issuer = RetryHandshakeRecordProtectionStateIssuer::new(traffic);
        let issuance_error = match issuer.issue(HandshakeTrafficSender::Client, &mut output_storage)
        {
            Err(error) => error,
            Ok(_) => panic!("mismatched storage must not produce record state"),
        };
        assert_eq!(
            issuance_error,
            RetryHandshakeRecordProtectionStateBindingError::KeyMaterial(
                RetryHandshakeTrafficKeyMaterialBindingError::CipherSuiteMismatch
            )
        );
        assert!(!issuer.has_issued(HandshakeTrafficSender::Client));

        let mut matching_storage =
            TrafficKeyMaterialStorage::new(transcript.transcript().selected_suite()).unwrap();
        let state = match issuer.issue(HandshakeTrafficSender::Client, &mut matching_storage) {
            Ok(state) => state,
            Err(_) => panic!("failed issuance must remain retryable"),
        };
        assert_eq!(state.sender(), HandshakeTrafficSender::Client);
        assert!(issuer.has_issued(HandshakeTrafficSender::Client));
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

    let duplicate = RetryHandshakeRecordProtectionStateBindingError::SenderAlreadyIssued;
    assert_eq!(
        duplicate.to_string(),
        "TLS 1.3 retry handshake record state already issued for sender"
    );
    assert!(duplicate.source().is_none());

    let issuance_key = RetryHandshakeRecordProtectionStateBindingError::KeyMaterial(
        RetryHandshakeTrafficKeyMaterialBindingError::CipherSuiteMismatch,
    );
    assert_eq!(
        issuance_key.to_string(),
        "TLS 1.3 retry handshake record-state key derivation failed"
    );
    assert!(issuance_key.source().is_some());

    let record_state = RetryHandshakeRecordProtectionStateBindingError::RecordState(
        RecordProtectionStateError::UnsupportedSuiteParameters,
    );
    assert_eq!(
        record_state.to_string(),
        "TLS 1.3 retry handshake record-state composition failed"
    );
    assert!(record_state.source().is_some());
    assert!(!format!("{record_state:?}").contains("retry-key-material"));

    let nonempty_context = RetryCertificateRequestValidationError::NonEmptyContext;
    assert_eq!(
        nonempty_context.to_string(),
        "TLS 1.3 main-handshake CertificateRequest context is not empty"
    );
    assert!(nonempty_context.source().is_none());

    let invalid_signature_algorithms = RetryCertificateRequestValidationError::SignatureAlgorithms(
        tls13_signature_scheme_list::SignatureSchemeListError::EmptyList,
    );
    assert_eq!(
        invalid_signature_algorithms.to_string(),
        "TLS 1.3 CertificateRequest signature_algorithms is invalid"
    );
    assert!(invalid_signature_algorithms.source().is_some());
    assert!(!format!("{invalid_signature_algorithms:?}").contains("retry-key-material"));

    let empty_certificate = RetryServerCertificateValidationError::EmptyCertificateList;
    assert_eq!(
        empty_certificate.to_string(),
        "TLS 1.3 server Certificate list is empty"
    );
    assert!(empty_certificate.source().is_none());

    let invalid_entry_extensions = RetryServerCertificateValidationError::EntryExtensions(
        tls13_extension_block::ExtensionBlockError::BlockLengthMismatch,
    );
    assert_eq!(
        invalid_entry_extensions.to_string(),
        "TLS 1.3 server Certificate entry extensions are invalid"
    );
    assert!(invalid_entry_extensions.source().is_some());

    let leaf_mismatch = RetryServerCertificateLeafBindingError::LeafCertificateMismatch;
    assert_eq!(
        leaf_mismatch.to_string(),
        "TLS 1.3 retry server Certificate leaf and decoded X.509 certificate differ"
    );
    assert!(leaf_mismatch.source().is_none());
    assert!(!format!("{leaf_mismatch:?}").contains("apple"));

    let invalid_certificate_verify = RetryServerCertificateVerifyValidationError::Structure(
        CertificateVerifyMessageError::SignatureLengthMismatch,
    );
    assert_eq!(
        invalid_certificate_verify.to_string(),
        "TLS 1.3 retry CertificateVerify structure is invalid"
    );
    assert!(invalid_certificate_verify.source().is_some());
    assert!(!format!("{invalid_certificate_verify:?}").contains("aabbcc"));

    let signature_binding_errors = [
        (
            RetryServerCertificateVerifySignatureBindingError::WrongEndpointRole,
            "TLS 1.3 retry CertificateVerify signature verdict has the wrong endpoint role",
        ),
        (
            RetryServerCertificateVerifySignatureBindingError::TranscriptSnapshotMismatch,
            "TLS 1.3 retry CertificateVerify signature verdict has a different transcript snapshot",
        ),
        (
            RetryServerCertificateVerifySignatureBindingError::SignatureSchemeMismatch,
            "TLS 1.3 retry CertificateVerify signature verdict has a different authorized scheme",
        ),
        (
            RetryServerCertificateVerifySignatureBindingError::MessageMismatch,
            "TLS 1.3 retry CertificateVerify signature verdict has a different message",
        ),
    ];
    for (error, expected) in signature_binding_errors {
        assert_eq!(error.to_string(), expected);
        assert!(error.source().is_none());
        assert!(!format!("{error:?}").contains("7d4fa22e"));
    }

    for error in [
        OrderedRetryServerFinishedVerificationError::WrongMessageType,
        OrderedRetryServerFinishedVerificationError::WrongFlightState,
        OrderedRetryServerFinishedVerificationError::RetryEvidenceMismatch,
        OrderedRetryServerFinishedVerificationError::WrongEndpointRole,
        OrderedRetryServerFinishedVerificationError::CipherSuiteMismatch,
        OrderedRetryServerFinishedVerificationError::PeerVerifyDataLengthMismatch,
        OrderedRetryServerFinishedVerificationError::FinishedComputation,
        OrderedRetryServerFinishedVerificationError::VerifyDataMismatch,
    ] {
        let display = error.to_string();
        let debug = format!("{error:?}");
        assert!(error.source().is_none());
        assert!(!display.contains("retry-key-material"));
        assert!(!debug.contains("7d4fa22e"));
    }

    let unexpected = RetryHandshakeRecordBindingError::UnexpectedContentType;
    assert_eq!(
        unexpected.to_string(),
        "TLS 1.3 retry record authenticated with a non-handshake content type"
    );
    assert!(unexpected.source().is_none());

    let reassembly_record = RetryHandshakeRecordReassemblyError::Record(
        RetryHandshakeRecordBindingError::UnexpectedContentType,
    );
    assert_eq!(
        reassembly_record.to_string(),
        "TLS 1.3 retry record authentication failed during reassembly"
    );
    assert!(reassembly_record.source().is_some());

    let owner_failed = RetryHandshakeRecordReassemblyError::OwnerFailed;
    assert_eq!(
        owner_failed.to_string(),
        "TLS 1.3 retry handshake reassembly owner already failed"
    );
    assert!(owner_failed.source().is_none());

    let client_sender = RetryOrderedServerFlightError::ClientSender;
    assert_eq!(
        client_sender.to_string(),
        "TLS 1.3 ordered server-flight composition received client sender state"
    );
    assert!(client_sender.source().is_none());

    let non_fresh = RetryOrderedServerFlightError::NonFreshReassembler;
    assert_eq!(
        non_fresh.to_string(),
        "TLS 1.3 ordered server-flight composition received a non-fresh reassembler"
    );
    assert!(non_fresh.source().is_none());

    let ordered_reassembly =
        RetryOrderedServerFlightError::Reassembly(RetryHandshakeRecordReassemblyError::OwnerFailed);
    assert_eq!(
        ordered_reassembly.to_string(),
        "TLS 1.3 ordered server-flight authenticated reassembly failed"
    );
    assert!(ordered_reassembly.source().is_some());
    assert!(!format!("{ordered_reassembly:?}").contains("retry-key-material"));

    for error in [
        RetryOrderedServerFlightTranscriptBindingError::NonFreshOrderedFlight,
        RetryOrderedServerFlightTranscriptBindingError::TrafficSecretProofMismatch,
        RetryOrderedServerFlightTranscriptBindingError::CipherSuiteMismatch,
        RetryOrderedServerFlightTranscriptBindingError::SnapshotMismatch,
    ] {
        let display = error.to_string();
        let debug = format!("{error:?}");
        assert!(!display.contains("retry-key-material"));
        assert!(!debug.contains("retry-key-material"));
        assert!(error.source().is_none());
    }
}
