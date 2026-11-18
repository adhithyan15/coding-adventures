use tls13_cipher_suite_offer::ClientCipherSuiteOffer;
use tls13_cipher_suite_policy::{CipherSuite, CipherSuitePolicy, CipherSuitePolicyEntry, HkdfHash};
use tls13_client_hello_message::{decode_client_hello, ClientHelloLimits};
use tls13_extension_block::ExtensionBlockLimits;
use tls13_extension_context_policy::{ExtensionContextPolicy, STANDARD_TLS13_EXTENSION_RULES};
use tls13_retry_transcript_binding::bind_retry_transcript;
use tls13_server_hello_message::{
    decode_tls13_server_hello, Tls13ServerHelloLimits, TLS13_HELLO_RETRY_REQUEST_RANDOM,
};
use tls13_server_hello_retry_selection::{
    bind_server_hello_retry_selection, BoundServerHelloRetrySelection,
};
use tls13_server_hello_selection::bind_server_hello_selection;
use tls13_server_hello_version_selection::{
    bind_server_hello_version_selection, BoundServerHelloVersionSelection,
};
use tls13_supported_versions::ClientVersionOffer;
use tls13_transcript::{
    PendingTls13Transcript, SelectedTls13Transcript, TranscriptErrorKind, TranscriptPolicy,
};

const SESSION_ID: &[u8] = b"retry-transcript";
const CLIENT_RANDOM: [u8; 32] = [0x11; 32];
const ORDINARY_SERVER_RANDOM: [u8; 32] = [0x22; 32];
const TLS13: u16 = 0x0304;

fn cipher_policy() -> CipherSuitePolicy {
    CipherSuitePolicy::new(&[
        CipherSuitePolicyEntry::new(CipherSuite::TlsAes128GcmSha256, 1_000).unwrap(),
        CipherSuitePolicyEntry::new(CipherSuite::TlsAes256GcmSha384, 1_000).unwrap(),
    ])
    .unwrap()
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

fn client_body(selected_suite: u16) -> Vec<u8> {
    let mut body = vec![0x03, 0x03];
    body.extend_from_slice(&CLIENT_RANDOM);
    body.push(SESSION_ID.len() as u8);
    body.extend_from_slice(SESSION_ID);
    body.extend_from_slice(&2_u16.to_be_bytes());
    body.extend_from_slice(&selected_suite.to_be_bytes());
    body.extend_from_slice(&[1, 0]);
    let versions = [2, 0x03, 0x04];
    append_extensions(&mut body, &[(43, &versions)]);
    body
}

fn server_body(random: [u8; 32], selected_suite: u16) -> Vec<u8> {
    let mut body = vec![0x03, 0x03];
    body.extend_from_slice(&random);
    body.push(SESSION_ID.len() as u8);
    body.extend_from_slice(SESSION_ID);
    body.extend_from_slice(&selected_suite.to_be_bytes());
    body.push(0);
    let version = TLS13.to_be_bytes();
    append_extensions(&mut body, &[(43, &version)]);
    body
}

fn client_limits() -> ClientHelloLimits {
    ClientHelloLimits::new(4, ExtensionBlockLimits::new(128, 4).unwrap()).unwrap()
}

fn server_limits() -> Tls13ServerHelloLimits {
    Tls13ServerHelloLimits::new(256, 128, 4).unwrap()
}

fn version_selection<'client, 'offer, 'server, 'versions>(
    client_body: &'client [u8],
    offer: &'offer [u16],
    server_body: &'server [u8],
    versions: &'versions [u16],
) -> BoundServerHelloVersionSelection<'client, 'offer, 'server, 'versions, 'static> {
    let client = decode_client_hello(client_body, client_limits()).unwrap();
    let server = decode_tls13_server_hello(server_body, server_limits()).unwrap();
    let offer = ClientCipherSuiteOffer::new(offer, cipher_policy()).unwrap();
    let selection = bind_server_hello_selection(client, offer, server).unwrap();
    bind_server_hello_version_selection(
        selection,
        ClientVersionOffer::new(versions).unwrap(),
        ExtensionContextPolicy::new(&STANDARD_TLS13_EXTENSION_RULES).unwrap(),
    )
    .unwrap()
}

type TestSelection<'input> = BoundServerHelloRetrySelection<
    'input,
    'input,
    'input,
    'input,
    'static,
    'input,
    'input,
    'input,
    'input,
    'static,
>;

fn retry_selection<'input>(
    first_client: &'input [u8],
    suites: &'input [u16],
    retry_server: &'input [u8],
    versions: &'input [u16],
    second_client: &'input [u8],
    final_server: &'input [u8],
) -> TestSelection<'input> {
    bind_server_hello_retry_selection(
        version_selection(first_client, suites, retry_server, versions),
        version_selection(second_client, suites, final_server, versions),
    )
    .unwrap()
}

fn update_manual(transcript: &mut SelectedTls13Transcript, message_type: u8, body: &[u8]) {
    let length = u32::try_from(body.len()).unwrap().to_be_bytes();
    transcript
        .update(&[message_type, length[1], length[2], length[3]])
        .unwrap();
    transcript.update(body).unwrap();
}

fn assert_success(selected_suite: u16, expected_hash: HkdfHash) {
    let suites = [selected_suite];
    let versions = [TLS13];
    let first_client = client_body(selected_suite);
    let retry_server = server_body(TLS13_HELLO_RETRY_REQUEST_RANDOM, selected_suite);
    let second_client = client_body(selected_suite);
    let final_server = server_body(ORDINARY_SERVER_RANDOM, selected_suite);
    let selection = retry_selection(
        &first_client,
        &suites,
        &retry_server,
        &versions,
        &second_client,
        &final_server,
    );

    let policy = TranscriptPolicy::new(4_096).unwrap();
    let bound = bind_retry_transcript(&selection, policy).unwrap();

    let first_length = u32::try_from(first_client.len()).unwrap().to_be_bytes();
    let mut manual = PendingTls13Transcript::new(policy);
    manual
        .update(&[1, first_length[1], first_length[2], first_length[3]])
        .unwrap();
    manual.update(&first_client).unwrap();
    let mut manual = manual.select(selection.retry().selection().cipher_suite());
    manual.apply_hello_retry_request().unwrap();
    update_manual(&mut manual, 2, &retry_server);
    update_manual(&mut manual, 1, &second_client);
    update_manual(&mut manual, 2, &final_server);

    assert!(std::ptr::eq(bound.messages(), &selection));
    assert!(bound.transcript().hello_retry_request_applied());
    assert_eq!(bound.transcript().snapshot(), manual.snapshot());
    assert_eq!(bound.transcript().snapshot().algorithm(), expected_hash);
    assert_eq!(
        bound.transcript().observed_bytes(),
        u64::try_from(
            first_client.len() + retry_server.len() + second_client.len() + final_server.len() + 16
        )
        .unwrap()
    );
    assert_eq!(
        format!("{bound:?}"),
        "BoundRetryTranscript { messages: \"[RETAINED]\", transcript: \"[RETAINED]\" }"
    );
}

#[test]
fn exact_retry_messages_select_sha256_and_remain_attached() {
    assert_success(0x1301, HkdfHash::Sha256);
}

#[test]
fn retained_cipher_suite_selects_sha384_without_detached_algorithm_input() {
    assert_success(0x1302, HkdfHash::Sha384);
}

#[test]
fn transcript_limit_failure_is_stable_and_payload_free() {
    let suites = [0x1301];
    let versions = [TLS13];
    let first_client = client_body(0x1301);
    let retry_server = server_body(TLS13_HELLO_RETRY_REQUEST_RANDOM, 0x1301);
    let second_client = client_body(0x1301);
    let final_server = server_body(ORDINARY_SERVER_RANDOM, 0x1301);
    let selection = retry_selection(
        &first_client,
        &suites,
        &retry_server,
        &versions,
        &second_client,
        &final_server,
    );

    let error = bind_retry_transcript(&selection, TranscriptPolicy::new(4).unwrap()).unwrap_err();
    assert_eq!(error.kind(), TranscriptErrorKind::TranscriptLimitExceeded);
    let rendered = format!("{error:?} {error}");
    assert!(!rendered.contains("retry-transcript"));
    assert!(!rendered.contains("4865"));
}
