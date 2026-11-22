use tls13_cipher_suite_offer::ClientCipherSuiteOffer;
use tls13_cipher_suite_policy::{CipherSuite, CipherSuitePolicy, CipherSuitePolicyEntry};
use tls13_client_hello_message::{decode_client_hello, ClientHelloLimits};
use tls13_extension_block::{ExtensionBlockLimits, ExtensionType};
use tls13_extension_context_policy::{ExtensionContextPolicy, STANDARD_TLS13_EXTENSION_RULES};
use tls13_key_share::{KeyShareErrorKind, KeyShareLimits};
use tls13_key_share_retry::RetryKeyShareError;
use tls13_named_group_policy::{NamedGroup, NamedGroupPolicy};
use tls13_server_hello_message::{
    decode_tls13_server_hello, Tls13ServerHelloLimits, TLS13_HELLO_RETRY_REQUEST_RANDOM,
};
use tls13_server_hello_retry_selection::{
    bind_final_server_key_share_message, bind_retry_key_share_messages,
    bind_server_hello_retry_selection, BoundServerHelloRetrySelection, RetryKeyShareMessageError,
    ServerHelloRetrySelectionError, KEY_SHARE_EXTENSION_TYPE,
};
use tls13_server_hello_selection::bind_server_hello_selection;
use tls13_server_hello_version_selection::{
    bind_server_hello_version_selection, BoundServerHelloVersionSelection,
};
use tls13_supported_groups::{NamedGroupListError, NamedGroupListLimits};
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
    client_body_with_extensions(random, session_id, suites, versions, &[])
}

fn client_body_with_extensions(
    random: [u8; 32],
    session_id: &[u8],
    suites: &[u16],
    versions: &[u16],
    extensions: &[(u16, &[u8])],
) -> Vec<u8> {
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
    let mut all_extensions = Vec::with_capacity(extensions.len() + 1);
    all_extensions.push((43, version_data.as_slice()));
    all_extensions.extend_from_slice(extensions);
    append_extensions(&mut body, &all_extensions);
    body
}

fn server_body(random: [u8; 32], session_id: &[u8], selected_suite: u16) -> Vec<u8> {
    server_body_with_extensions(random, session_id, selected_suite, &[])
}

fn server_body_with_extensions(
    random: [u8; 32],
    session_id: &[u8],
    selected_suite: u16,
    extensions: &[(u16, &[u8])],
) -> Vec<u8> {
    let mut body = vec![0x03, 0x03];
    body.extend_from_slice(&random);
    body.push(session_id.len() as u8);
    body.extend_from_slice(session_id);
    body.extend_from_slice(&selected_suite.to_be_bytes());
    body.push(0);
    let version_data = TLS13.to_be_bytes();
    let mut all_extensions = Vec::with_capacity(extensions.len() + 1);
    all_extensions.push((43, version_data.as_slice()));
    all_extensions.extend_from_slice(extensions);
    append_extensions(&mut body, &all_extensions);
    body
}

fn supported_groups_data(groups: &[u16]) -> Vec<u8> {
    let mut data = Vec::with_capacity(2 + groups.len() * 2);
    data.extend_from_slice(&((groups.len() * 2) as u16).to_be_bytes());
    for group in groups {
        data.extend_from_slice(&group.to_be_bytes());
    }
    data
}

fn client_key_share_data(entries: &[(u16, &[u8])]) -> Vec<u8> {
    let payload_length: usize = entries.iter().map(|(_, key)| 4 + key.len()).sum();
    let mut data = Vec::with_capacity(payload_length + 2);
    data.extend_from_slice(&(payload_length as u16).to_be_bytes());
    for (group, key) in entries {
        data.extend_from_slice(&group.to_be_bytes());
        data.extend_from_slice(&(key.len() as u16).to_be_bytes());
        data.extend_from_slice(key);
    }
    data
}

fn server_key_share_data(group: u16, key: &[u8]) -> Vec<u8> {
    let mut data = Vec::with_capacity(key.len() + 4);
    data.extend_from_slice(&group.to_be_bytes());
    data.extend_from_slice(&(key.len() as u16).to_be_bytes());
    data.extend_from_slice(key);
    data
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

type TestRetrySelection<'input> = BoundServerHelloRetrySelection<
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
) -> TestRetrySelection<'input> {
    bind_server_hello_retry_selection(
        version_selection(first_client, suites, retry_server, versions, 1_000),
        version_selection(second_client, suites, final_server, versions, 1_000),
    )
    .unwrap()
}

fn named_group_policy() -> NamedGroupPolicy {
    NamedGroupPolicy::new(&[NamedGroup::Secp256r1, NamedGroup::X25519]).unwrap()
}

fn bind_key_share_message_result(
    first_client_extensions: &[(u16, &[u8])],
    retry_extensions: &[(u16, &[u8])],
    second_client_extensions: &[(u16, &[u8])],
    final_server_extensions: &[(u16, &[u8])],
    bind_final: bool,
) -> Result<(), RetryKeyShareMessageError> {
    let suites = [0x1301];
    let versions = [TLS13];
    let first_client = client_body_with_extensions(
        CLIENT_RANDOM,
        SESSION_ID,
        &suites,
        &versions,
        first_client_extensions,
    );
    let retry_server = server_body_with_extensions(
        TLS13_HELLO_RETRY_REQUEST_RANDOM,
        SESSION_ID,
        0x1301,
        retry_extensions,
    );
    let second_client = client_body_with_extensions(
        CLIENT_RANDOM,
        SESSION_ID,
        &suites,
        &versions,
        second_client_extensions,
    );
    let final_server = server_body_with_extensions(
        ORDINARY_SERVER_RANDOM,
        SESSION_ID,
        0x1301,
        final_server_extensions,
    );
    let selection = retry_selection(
        &first_client,
        &suites,
        &retry_server,
        &versions,
        &second_client,
        &final_server,
    );
    let messages = bind_retry_key_share_messages(
        selection,
        named_group_policy(),
        NamedGroupListLimits::new(2).unwrap(),
        KeyShareLimits::new(2).unwrap(),
    )?;
    if bind_final {
        bind_final_server_key_share_message(&messages)?;
    }
    Ok(())
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
fn binds_retry_and_final_key_shares_to_the_exact_retained_messages() {
    let suites = [0x1301];
    let versions = [TLS13];
    let secp256r1_key = [0x31; 65];
    let retried_x25519_key = [0x42; 32];
    let server_x25519_key = [0x53; 32];
    let supported_groups = supported_groups_data(&[0x0017, 0x001d]);
    let initial_key_share = client_key_share_data(&[(0x0017, &secp256r1_key)]);
    let retry_key_share = 0x001du16.to_be_bytes();
    let retried_key_share = client_key_share_data(&[(0x001d, &retried_x25519_key)]);
    let final_key_share = server_key_share_data(0x001d, &server_x25519_key);
    let first_client = client_body_with_extensions(
        CLIENT_RANDOM,
        SESSION_ID,
        &suites,
        &versions,
        &[
            (10, &supported_groups),
            (KEY_SHARE_EXTENSION_TYPE, &initial_key_share),
        ],
    );
    let retry_server = server_body_with_extensions(
        TLS13_HELLO_RETRY_REQUEST_RANDOM,
        SESSION_ID,
        0x1301,
        &[(KEY_SHARE_EXTENSION_TYPE, &retry_key_share)],
    );
    let second_client = client_body_with_extensions(
        CLIENT_RANDOM,
        SESSION_ID,
        &suites,
        &versions,
        &[(KEY_SHARE_EXTENSION_TYPE, &retried_key_share)],
    );
    let final_server = server_body_with_extensions(
        ORDINARY_SERVER_RANDOM,
        SESSION_ID,
        0x1301,
        &[(KEY_SHARE_EXTENSION_TYPE, &final_key_share)],
    );
    let selection = retry_selection(
        &first_client,
        &suites,
        &retry_server,
        &versions,
        &second_client,
        &final_server,
    );
    let policy = named_group_policy();
    let messages = bind_retry_key_share_messages(
        selection,
        policy,
        NamedGroupListLimits::new(2).unwrap(),
        KeyShareLimits::new(2).unwrap(),
    )
    .unwrap();

    let key_share_type = ExtensionType::from_wire_value(KEY_SHARE_EXTENSION_TYPE);
    let supported_groups_type = ExtensionType::from_wire_value(10);
    let retained_initial_extensions = messages
        .selection()
        .retry()
        .selection()
        .client_hello()
        .extensions();
    let retained_retry_extensions = messages
        .selection()
        .retry()
        .selection()
        .server_hello()
        .extensions();
    let retained_second_extensions = messages
        .selection()
        .server_hello()
        .selection()
        .client_hello()
        .extensions();
    assert_eq!(messages.named_group_policy(), policy);
    assert_eq!(
        messages
            .transition()
            .selection()
            .supported_groups()
            .extension_data()
            .as_ptr(),
        retained_initial_extensions
            .find(supported_groups_type)
            .unwrap()
            .data()
            .as_ptr()
    );
    assert_eq!(
        messages
            .transition()
            .selection()
            .initial_client_shares()
            .extension_data()
            .as_ptr(),
        retained_initial_extensions
            .find(key_share_type)
            .unwrap()
            .data()
            .as_ptr()
    );
    assert_eq!(
        messages.transition().selection().extension_data().as_ptr(),
        retained_retry_extensions
            .find(key_share_type)
            .unwrap()
            .data()
            .as_ptr()
    );
    assert_eq!(
        messages
            .transition()
            .retried_client_shares()
            .extension_data()
            .as_ptr(),
        retained_second_extensions
            .find(key_share_type)
            .unwrap()
            .data()
            .as_ptr()
    );

    let final_messages = bind_final_server_key_share_message(&messages).unwrap();
    let retained_final_data = final_messages
        .messages()
        .selection()
        .server_hello()
        .selection()
        .server_hello()
        .extensions()
        .find(key_share_type)
        .unwrap()
        .data();
    assert!(core::ptr::eq(final_messages.messages(), &messages));
    assert_eq!(
        final_messages
            .server_share()
            .server_share()
            .key_exchange()
            .as_ptr(),
        retained_final_data.as_ptr().wrapping_add(4)
    );
    assert_eq!(
        final_messages.server_share().server_share().key_exchange(),
        server_x25519_key
    );

    for debug in [format!("{messages:?}"), format!("{final_messages:?}")] {
        assert!(debug.contains("[RETAINED]"));
        for disclosed in ["0x001d", "29", "424242", "535353"] {
            assert!(!debug.contains(disclosed));
        }
    }
}

#[test]
fn rejects_missing_key_share_message_extensions_before_detached_input_exists() {
    let secp256r1_key = [0x31; 65];
    let retried_x25519_key = [0x42; 32];
    let server_x25519_key = [0x53; 32];
    let supported_groups = supported_groups_data(&[0x0017, 0x001d]);
    let initial_key_share = client_key_share_data(&[(0x0017, &secp256r1_key)]);
    let retry_key_share = 0x001du16.to_be_bytes();
    let retried_key_share = client_key_share_data(&[(0x001d, &retried_x25519_key)]);
    let final_key_share = server_key_share_data(0x001d, &server_x25519_key);

    assert_eq!(
        bind_key_share_message_result(
            &[(KEY_SHARE_EXTENSION_TYPE, &initial_key_share)],
            &[(KEY_SHARE_EXTENSION_TYPE, &retry_key_share)],
            &[(KEY_SHARE_EXTENSION_TYPE, &retried_key_share)],
            &[(KEY_SHARE_EXTENSION_TYPE, &final_key_share)],
            false,
        )
        .unwrap_err(),
        RetryKeyShareMessageError::MissingInitialSupportedGroups
    );
    assert_eq!(
        bind_key_share_message_result(
            &[(10, &supported_groups)],
            &[],
            &[(KEY_SHARE_EXTENSION_TYPE, &retried_key_share)],
            &[(KEY_SHARE_EXTENSION_TYPE, &final_key_share)],
            false,
        )
        .unwrap_err(),
        RetryKeyShareMessageError::MissingInitialClientKeyShare
    );
    assert_eq!(
        bind_key_share_message_result(
            &[
                (10, &supported_groups),
                (KEY_SHARE_EXTENSION_TYPE, &initial_key_share),
            ],
            &[],
            &[(KEY_SHARE_EXTENSION_TYPE, &retried_key_share)],
            &[(KEY_SHARE_EXTENSION_TYPE, &final_key_share)],
            false,
        )
        .unwrap_err(),
        RetryKeyShareMessageError::MissingRetryKeyShare
    );
    assert_eq!(
        bind_key_share_message_result(
            &[
                (10, &supported_groups),
                (KEY_SHARE_EXTENSION_TYPE, &initial_key_share),
            ],
            &[(KEY_SHARE_EXTENSION_TYPE, &retry_key_share)],
            &[],
            &[],
            false,
        )
        .unwrap_err(),
        RetryKeyShareMessageError::MissingRetriedClientKeyShare
    );
    assert_eq!(
        bind_key_share_message_result(
            &[
                (10, &supported_groups),
                (KEY_SHARE_EXTENSION_TYPE, &initial_key_share),
            ],
            &[(KEY_SHARE_EXTENSION_TYPE, &retry_key_share)],
            &[(KEY_SHARE_EXTENSION_TYPE, &retried_key_share)],
            &[],
            true,
        )
        .unwrap_err(),
        RetryKeyShareMessageError::MissingFinalServerKeyShare
    );
}

#[test]
fn propagates_redacted_framing_and_transition_failures() {
    let secp256r1_key = [0x31; 65];
    let retried_x25519_key = [0x42; 32];
    let server_x25519_key = [0x53; 32];
    let supported_groups = supported_groups_data(&[0x0017, 0x001d]);
    let malformed_groups = [0x00, 0x03, 0x00, 0x17, 0x00];
    let initial_key_share = client_key_share_data(&[(0x0017, &secp256r1_key)]);
    let malformed_initial = [0x00, 0x01, 0x00];
    let retry_key_share = 0x001du16.to_be_bytes();
    let already_offered_retry = 0x0017u16.to_be_bytes();
    let retried_key_share = client_key_share_data(&[(0x001d, &retried_x25519_key)]);
    let malformed_retried = [0x00, 0x01, 0x00];
    let final_key_share = server_key_share_data(0x001d, &server_x25519_key);
    let wrong_final_key_share = server_key_share_data(0x0017, &secp256r1_key);

    assert_eq!(
        bind_key_share_message_result(
            &[
                (10, &malformed_groups),
                (KEY_SHARE_EXTENSION_TYPE, &initial_key_share),
            ],
            &[(KEY_SHARE_EXTENSION_TYPE, &retry_key_share)],
            &[(KEY_SHARE_EXTENSION_TYPE, &retried_key_share)],
            &[(KEY_SHARE_EXTENSION_TYPE, &final_key_share)],
            false,
        )
        .unwrap_err(),
        RetryKeyShareMessageError::InitialSupportedGroups(NamedGroupListError::OddListLength)
    );
    assert_eq!(
        bind_key_share_message_result(
            &[
                (10, &supported_groups),
                (KEY_SHARE_EXTENSION_TYPE, &malformed_initial),
            ],
            &[(KEY_SHARE_EXTENSION_TYPE, &retry_key_share)],
            &[(KEY_SHARE_EXTENSION_TYPE, &retried_key_share)],
            &[(KEY_SHARE_EXTENSION_TYPE, &final_key_share)],
            false,
        )
        .unwrap_err(),
        RetryKeyShareMessageError::InitialClientKeyShare(KeyShareErrorKind::TruncatedEntryHeader)
    );
    assert!(matches!(
        bind_key_share_message_result(
            &[
                (10, &supported_groups),
                (KEY_SHARE_EXTENSION_TYPE, &initial_key_share),
            ],
            &[(KEY_SHARE_EXTENSION_TYPE, &already_offered_retry)],
            &[(KEY_SHARE_EXTENSION_TYPE, &retried_key_share)],
            &[(KEY_SHARE_EXTENSION_TYPE, &final_key_share)],
            false,
        )
        .unwrap_err(),
        RetryKeyShareMessageError::RetryTransition(_)
    ));
    assert_eq!(
        bind_key_share_message_result(
            &[
                (10, &supported_groups),
                (KEY_SHARE_EXTENSION_TYPE, &initial_key_share),
            ],
            &[(KEY_SHARE_EXTENSION_TYPE, &retry_key_share)],
            &[(KEY_SHARE_EXTENSION_TYPE, &malformed_retried)],
            &[(KEY_SHARE_EXTENSION_TYPE, &final_key_share)],
            false,
        )
        .unwrap_err(),
        RetryKeyShareMessageError::RetriedClientKeyShare(KeyShareErrorKind::TruncatedEntryHeader)
    );
    assert_eq!(
        bind_key_share_message_result(
            &[
                (10, &supported_groups),
                (KEY_SHARE_EXTENSION_TYPE, &initial_key_share),
            ],
            &[(KEY_SHARE_EXTENSION_TYPE, &retry_key_share)],
            &[(KEY_SHARE_EXTENSION_TYPE, &retried_key_share)],
            &[(KEY_SHARE_EXTENSION_TYPE, &wrong_final_key_share)],
            true,
        )
        .unwrap_err(),
        RetryKeyShareMessageError::FinalServerShare(RetryKeyShareError::FinalServerShare(
            KeyShareErrorKind::ServerGroupNotOffered
        ))
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

    let message_errors = [
        (
            RetryKeyShareMessageError::MissingInitialSupportedGroups,
            "the first ClientHello is missing supported_groups",
        ),
        (
            RetryKeyShareMessageError::MissingInitialClientKeyShare,
            "the first ClientHello is missing key_share",
        ),
        (
            RetryKeyShareMessageError::MissingRetryKeyShare,
            "HelloRetryRequest is missing key_share",
        ),
        (
            RetryKeyShareMessageError::MissingRetriedClientKeyShare,
            "the second ClientHello is missing key_share",
        ),
        (
            RetryKeyShareMessageError::MissingFinalServerKeyShare,
            "the final ServerHello is missing key_share",
        ),
        (
            RetryKeyShareMessageError::InitialSupportedGroups(NamedGroupListError::OddListLength),
            "the first ClientHello supported_groups value is invalid",
        ),
        (
            RetryKeyShareMessageError::InitialClientKeyShare(
                KeyShareErrorKind::TruncatedEntryHeader,
            ),
            "the first ClientHello key_share value is invalid",
        ),
        (
            RetryKeyShareMessageError::RetriedClientKeyShare(
                KeyShareErrorKind::TruncatedEntryHeader,
            ),
            "the second ClientHello key_share value is invalid",
        ),
        (
            RetryKeyShareMessageError::RetryTransition(
                RetryKeyShareError::SelectedGroupAlreadyOffered,
            ),
            "the retained retry key-share transition is invalid",
        ),
        (
            RetryKeyShareMessageError::FinalServerShare(RetryKeyShareError::FinalServerShare(
                KeyShareErrorKind::ServerGroupNotOffered,
            )),
            "the retained final ServerHello key_share value is invalid",
        ),
    ];
    for (error, expected) in message_errors {
        assert_eq!(error.to_string(), expected);
        for disclosed in ["retry-session", "0x001d", "29", "424242", "535353"] {
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
