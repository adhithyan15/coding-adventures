use tls13_cipher_suite_offer::ClientCipherSuiteOffer;
use tls13_cipher_suite_policy::{CipherSuite, CipherSuitePolicy, CipherSuitePolicyEntry};
use tls13_client_hello_message::{decode_client_hello, ClientHelloLimits};
use tls13_extension_block::ExtensionBlockLimits;
use tls13_extension_context_policy::{ExtensionContextPolicy, STANDARD_TLS13_EXTENSION_RULES};
use tls13_server_hello_message::{
    decode_tls13_server_hello, Tls13ServerHelloLimits, TLS13_HELLO_RETRY_REQUEST_RANDOM,
};
use tls13_server_hello_retry_selection::{
    bind_server_hello_retry_selection, ServerHelloRetrySelectionError,
};
use tls13_server_hello_selection::bind_server_hello_selection;
use tls13_server_hello_version_selection::{
    bind_server_hello_version_selection, BoundServerHelloVersionSelection,
};
use tls13_supported_versions::ClientVersionOffer;

const SESSION_ID: &[u8] = b"retry-session";
const OTHER_SESSION_ID: &[u8] = b"other-session";
const CLIENT_RANDOM: [u8; 32] = [0x11; 32];
const OTHER_CLIENT_RANDOM: [u8; 32] = [0x12; 32];
const ORDINARY_SERVER_RANDOM: [u8; 32] = [0x22; 32];
const TLS13: u16 = 0x0304;

fn cipher_policy(record_limit: u64) -> CipherSuitePolicy {
    let entries = [
        CipherSuitePolicyEntry::new(CipherSuite::TlsAes128GcmSha256, record_limit).unwrap(),
        CipherSuitePolicyEntry::new(CipherSuite::TlsAes256GcmSha384, record_limit).unwrap(),
        CipherSuitePolicyEntry::new(CipherSuite::TlsChacha20Poly1305Sha256, record_limit).unwrap(),
    ];
    CipherSuitePolicy::new(&entries).unwrap()
}

fn version_extension_data(versions: &[u16]) -> Vec<u8> {
    let mut data = Vec::with_capacity(1 + versions.len() * 2);
    data.push((versions.len() * 2) as u8);
    for version in versions {
        data.extend_from_slice(&version.to_be_bytes());
    }
    data
}

fn append_extensions(body: &mut Vec<u8>, extensions: &[(u16, &[u8])]) {
    let payload_len: usize = extensions.iter().map(|(_, data)| 4 + data.len()).sum();
    body.extend_from_slice(&(payload_len as u16).to_be_bytes());
    for (extension_type, data) in extensions {
        body.extend_from_slice(&extension_type.to_be_bytes());
        body.extend_from_slice(&(data.len() as u16).to_be_bytes());
        body.extend_from_slice(data);
    }
}

fn client_body(random: [u8; 32], session_id: &[u8], suites: &[u16], versions: &[u16]) -> Vec<u8> {
    let mut body = vec![0x03, 0x03];
    body.extend_from_slice(&random);
    body.push(session_id.len() as u8);
    body.extend_from_slice(session_id);
    body.extend_from_slice(&((suites.len() * 2) as u16).to_be_bytes());
    for suite in suites {
        body.extend_from_slice(&suite.to_be_bytes());
    }
    body.extend_from_slice(&[1, 0]);
    let version_data = version_extension_data(versions);
    append_extensions(&mut body, &[(43, &version_data), (21, &[])]);
    body
}

fn server_body(random: [u8; 32], session_id: &[u8], selected_suite: u16) -> Vec<u8> {
    let mut body = vec![0x03, 0x03];
    body.extend_from_slice(&random);
    body.push(session_id.len() as u8);
    body.extend_from_slice(session_id);
    body.extend_from_slice(&selected_suite.to_be_bytes());
    body.push(0);
    append_extensions(&mut body, &[(43, &TLS13.to_be_bytes())]);
    body
}

fn client_limits() -> ClientHelloLimits {
    ClientHelloLimits::new(8, ExtensionBlockLimits::new(128, 8).unwrap()).unwrap()
}

fn server_limits() -> Tls13ServerHelloLimits {
    Tls13ServerHelloLimits::new(256, 128, 8).unwrap()
}

fn version_selection_with_policy<'client, 'cipher_offer, 'server, 'version_offer, 'rules>(
    client_bytes: &'client [u8],
    suites: &'cipher_offer [u16],
    server_bytes: &'server [u8],
    versions: &'version_offer [u16],
    record_limit: u64,
    extension_policy: ExtensionContextPolicy<'rules>,
) -> BoundServerHelloVersionSelection<'client, 'cipher_offer, 'server, 'version_offer, 'rules> {
    let client = decode_client_hello(client_bytes, client_limits()).unwrap();
    let server = decode_tls13_server_hello(server_bytes, server_limits()).unwrap();
    let cipher_offer = ClientCipherSuiteOffer::new(suites, cipher_policy(record_limit)).unwrap();
    let selection = bind_server_hello_selection(client, cipher_offer, server).unwrap();
    bind_server_hello_version_selection(
        selection,
        ClientVersionOffer::new(versions).unwrap(),
        extension_policy,
    )
    .unwrap()
}

fn version_selection<'client, 'cipher_offer, 'server, 'version_offer>(
    client_bytes: &'client [u8],
    suites: &'cipher_offer [u16],
    server_bytes: &'server [u8],
    versions: &'version_offer [u16],
    record_limit: u64,
) -> BoundServerHelloVersionSelection<'client, 'cipher_offer, 'server, 'version_offer, 'static> {
    version_selection_with_policy(
        client_bytes,
        suites,
        server_bytes,
        versions,
        record_limit,
        ExtensionContextPolicy::new(&STANDARD_TLS13_EXTENSION_RULES).unwrap(),
    )
}

#[test]
fn binds_retry_to_final_selection_and_retains_both_exact_proofs() {
    let suites = [0x1301, 0x1302];
    let versions = [TLS13, 0x0303];
    let first_client = client_body(CLIENT_RANDOM, SESSION_ID, &suites, &versions);
    let second_client = client_body(CLIENT_RANDOM, SESSION_ID, &suites, &versions);
    let retry_server = server_body(TLS13_HELLO_RETRY_REQUEST_RANDOM, SESSION_ID, 0x1301);
    let final_server = server_body(ORDINARY_SERVER_RANDOM, SESSION_ID, 0x1301);
    let retry = version_selection(&first_client, &suites, &retry_server, &versions, 1_000);
    let retry_pointer = retry.selection().server_hello().encoded().as_ptr();
    let server_hello = version_selection(&second_client, &suites, &final_server, &versions, 1_000);
    let final_pointer = server_hello.selection().server_hello().encoded().as_ptr();

    let bound = bind_server_hello_retry_selection(retry, server_hello).unwrap();

    assert_eq!(
        bound.retry().selection().server_hello().encoded().as_ptr(),
        retry_pointer
    );
    assert_eq!(
        bound
            .server_hello()
            .selection()
            .server_hello()
            .encoded()
            .as_ptr(),
        final_pointer
    );
    assert_eq!(bound.retry().version().wire_id(), TLS13);
}

#[test]
fn requires_retry_first_and_ordinary_server_hello_second() {
    let suites = [0x1301];
    let versions = [TLS13];
    let client = client_body(CLIENT_RANDOM, SESSION_ID, &suites, &versions);
    let retry_server = server_body(TLS13_HELLO_RETRY_REQUEST_RANDOM, SESSION_ID, 0x1301);
    let ordinary_server = server_body(ORDINARY_SERVER_RANDOM, SESSION_ID, 0x1301);

    assert_eq!(
        bind_server_hello_retry_selection(
            version_selection(&client, &suites, &ordinary_server, &versions, 1_000),
            version_selection(&client, &suites, &ordinary_server, &versions, 1_000),
        )
        .unwrap_err(),
        ServerHelloRetrySelectionError::ExpectedHelloRetryRequest
    );
    assert_eq!(
        bind_server_hello_retry_selection(
            version_selection(&client, &suites, &retry_server, &versions, 1_000),
            version_selection(&client, &suites, &retry_server, &versions, 1_000),
        )
        .unwrap_err(),
        ServerHelloRetrySelectionError::ExpectedServerHello
    );
}

#[test]
fn rejects_client_random_or_legacy_session_id_changes() {
    let suites = [0x1301];
    let versions = [TLS13];
    let first_client = client_body(CLIENT_RANDOM, SESSION_ID, &suites, &versions);
    let changed_random = client_body(OTHER_CLIENT_RANDOM, SESSION_ID, &suites, &versions);
    let changed_session = client_body(CLIENT_RANDOM, OTHER_SESSION_ID, &suites, &versions);
    let retry_server = server_body(TLS13_HELLO_RETRY_REQUEST_RANDOM, SESSION_ID, 0x1301);
    let final_server = server_body(ORDINARY_SERVER_RANDOM, SESSION_ID, 0x1301);
    let changed_session_server = server_body(ORDINARY_SERVER_RANDOM, OTHER_SESSION_ID, 0x1301);

    assert_eq!(
        bind_server_hello_retry_selection(
            version_selection(&first_client, &suites, &retry_server, &versions, 1_000),
            version_selection(&changed_random, &suites, &final_server, &versions, 1_000),
        )
        .unwrap_err(),
        ServerHelloRetrySelectionError::ClientRandomChanged
    );
    assert_eq!(
        bind_server_hello_retry_selection(
            version_selection(&first_client, &suites, &retry_server, &versions, 1_000),
            version_selection(
                &changed_session,
                &suites,
                &changed_session_server,
                &versions,
                1_000,
            ),
        )
        .unwrap_err(),
        ServerHelloRetrySelectionError::LegacySessionIdChanged
    );
}

#[test]
fn rejects_cipher_suite_offer_value_order_or_policy_changes() {
    let first_suites = [0x1301, 0x1302];
    let changed_suites = [0x1301, 0x1303];
    let versions = [TLS13];
    let first_client = client_body(CLIENT_RANDOM, SESSION_ID, &first_suites, &versions);
    let changed_client = client_body(CLIENT_RANDOM, SESSION_ID, &changed_suites, &versions);
    let retry_server = server_body(TLS13_HELLO_RETRY_REQUEST_RANDOM, SESSION_ID, 0x1301);
    let final_server = server_body(ORDINARY_SERVER_RANDOM, SESSION_ID, 0x1301);

    assert_eq!(
        bind_server_hello_retry_selection(
            version_selection(
                &first_client,
                &first_suites,
                &retry_server,
                &versions,
                1_000,
            ),
            version_selection(
                &changed_client,
                &changed_suites,
                &final_server,
                &versions,
                1_000,
            ),
        )
        .unwrap_err(),
        ServerHelloRetrySelectionError::CipherSuiteOfferChanged
    );
    assert_eq!(
        bind_server_hello_retry_selection(
            version_selection(
                &first_client,
                &first_suites,
                &retry_server,
                &versions,
                1_000,
            ),
            version_selection(
                &first_client,
                &first_suites,
                &final_server,
                &versions,
                2_000,
            ),
        )
        .unwrap_err(),
        ServerHelloRetrySelectionError::CipherSuiteOfferChanged
    );
}

#[test]
fn rejects_supported_version_offer_changes() {
    let suites = [0x1301];
    let first_versions = [TLS13, 0x0303];
    let changed_versions = [TLS13, 0x0302];
    let first_client = client_body(CLIENT_RANDOM, SESSION_ID, &suites, &first_versions);
    let changed_client = client_body(CLIENT_RANDOM, SESSION_ID, &suites, &changed_versions);
    let retry_server = server_body(TLS13_HELLO_RETRY_REQUEST_RANDOM, SESSION_ID, 0x1301);
    let final_server = server_body(ORDINARY_SERVER_RANDOM, SESSION_ID, 0x1301);

    assert_eq!(
        bind_server_hello_retry_selection(
            version_selection(
                &first_client,
                &suites,
                &retry_server,
                &first_versions,
                1_000,
            ),
            version_selection(
                &changed_client,
                &suites,
                &final_server,
                &changed_versions,
                1_000,
            ),
        )
        .unwrap_err(),
        ServerHelloRetrySelectionError::VersionOfferChanged
    );
}

#[test]
fn rejects_extension_context_policy_substitution() {
    let suites = [0x1301];
    let versions = [TLS13];
    let client = client_body(CLIENT_RANDOM, SESSION_ID, &suites, &versions);
    let retry_server = server_body(TLS13_HELLO_RETRY_REQUEST_RANDOM, SESSION_ID, 0x1301);
    let final_server = server_body(ORDINARY_SERVER_RANDOM, SESSION_ID, 0x1301);
    let mut reordered_rules = STANDARD_TLS13_EXTENSION_RULES;
    reordered_rules.swap(0, 1);
    let reordered_policy = ExtensionContextPolicy::new(&reordered_rules).unwrap();

    assert_eq!(
        bind_server_hello_retry_selection(
            version_selection(&client, &suites, &retry_server, &versions, 1_000),
            version_selection_with_policy(
                &client,
                &suites,
                &final_server,
                &versions,
                1_000,
                reordered_policy,
            ),
        )
        .unwrap_err(),
        ServerHelloRetrySelectionError::ExtensionPolicyChanged
    );
}

#[test]
fn rejects_a_changed_policy_authorized_cipher_suite_selection() {
    let suites = [0x1301, 0x1302];
    let versions = [TLS13];
    let client = client_body(CLIENT_RANDOM, SESSION_ID, &suites, &versions);
    let retry_server = server_body(TLS13_HELLO_RETRY_REQUEST_RANDOM, SESSION_ID, 0x1301);
    let final_server = server_body(ORDINARY_SERVER_RANDOM, SESSION_ID, 0x1302);

    assert_eq!(
        bind_server_hello_retry_selection(
            version_selection(&client, &suites, &retry_server, &versions, 1_000),
            version_selection(&client, &suites, &final_server, &versions, 1_000),
        )
        .unwrap_err(),
        ServerHelloRetrySelectionError::CipherSuiteSelectionChanged
    );
}

#[test]
fn diagnostics_are_stable_and_redact_all_compared_evidence() {
    let errors = [
        (
            ServerHelloRetrySelectionError::ExpectedHelloRetryRequest,
            "the first retained server message is not HelloRetryRequest",
        ),
        (
            ServerHelloRetrySelectionError::ExpectedServerHello,
            "the final retained server message is not ServerHello",
        ),
        (
            ServerHelloRetrySelectionError::ClientRandomChanged,
            "the retained ClientHello random changed after retry",
        ),
        (
            ServerHelloRetrySelectionError::LegacySessionIdChanged,
            "the retained ClientHello legacy session ID changed after retry",
        ),
        (
            ServerHelloRetrySelectionError::CipherSuiteOfferChanged,
            "the retained cipher-suite offer changed after retry",
        ),
        (
            ServerHelloRetrySelectionError::VersionOfferChanged,
            "the retained version offer changed after retry",
        ),
        (
            ServerHelloRetrySelectionError::ExtensionPolicyChanged,
            "the retained extension-context policy changed after retry",
        ),
        (
            ServerHelloRetrySelectionError::CipherSuiteSelectionChanged,
            "the policy-authorized cipher suite changed after retry",
        ),
        (
            ServerHelloRetrySelectionError::VersionSelectionChanged,
            "the selected TLS version changed after retry",
        ),
    ];
    for (error, expected) in errors {
        assert_eq!(error.to_string(), expected);
        for disclosed in ["retry-session", "0x1301", "4865", "0x0304", "772"] {
            assert!(!format!("{error:?}").contains(disclosed));
        }
    }

    let suites = [0x1301];
    let versions = [TLS13];
    let client = client_body(CLIENT_RANDOM, SESSION_ID, &suites, &versions);
    let retry_server = server_body(TLS13_HELLO_RETRY_REQUEST_RANDOM, SESSION_ID, 0x1301);
    let final_server = server_body(ORDINARY_SERVER_RANDOM, SESSION_ID, 0x1301);
    let debug = format!(
        "{:?}",
        bind_server_hello_retry_selection(
            version_selection(&client, &suites, &retry_server, &versions, 1_000),
            version_selection(&client, &suites, &final_server, &versions, 1_000),
        )
        .unwrap()
    );
    assert!(debug.contains("[RETAINED]"));
    assert!(debug.contains("[POLICY-BOUND]"));
    for disclosed in ["retry-session", "0x1301", "4865", "0x0304", "772"] {
        assert!(!debug.contains(disclosed));
    }
}
