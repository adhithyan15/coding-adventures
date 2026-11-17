use std::error::Error;

use tls13_cipher_suite_offer::ClientCipherSuiteOffer;
use tls13_cipher_suite_policy::{CipherSuite, CipherSuitePolicy, CipherSuitePolicyEntry};
use tls13_client_hello_message::{decode_client_hello, ClientHelloLimits};
use tls13_extension_block::ExtensionBlockLimits;
use tls13_extension_context_policy::{
    ExtensionContextPolicy, ExtensionContextPolicyError, STANDARD_TLS13_EXTENSION_RULES,
};
use tls13_server_hello_message::{
    decode_tls13_server_hello, Tls13ServerHelloKind, Tls13ServerHelloLimits,
    TLS13_HELLO_RETRY_REQUEST_RANDOM,
};
use tls13_server_hello_selection::{bind_server_hello_selection, BoundServerHelloSelection};
use tls13_server_hello_version_selection::{
    bind_server_hello_version_selection, ServerHelloVersionSelectionError,
};
use tls13_supported_versions::{ClientVersionOffer, VersionError};

const SESSION_ID: &[u8] = b"version-session";
const SUITES: &[u16] = &[0x1301];
const CLIENT_TLS13_TLS12: &[u8] = &[4, 0x03, 0x04, 0x03, 0x03];
const CLIENT_TLS14_TLS13: &[u8] = &[4, 0x03, 0x05, 0x03, 0x04];
const SERVER_TLS13: &[u8] = &[0x03, 0x04];

fn cipher_policy() -> CipherSuitePolicy {
    let entries = [CipherSuitePolicyEntry::new(CipherSuite::TlsAes128GcmSha256, 1_000).unwrap()];
    CipherSuitePolicy::new(&entries).unwrap()
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

fn client_body(extensions: &[(u16, &[u8])]) -> Vec<u8> {
    let mut body = vec![0x03, 0x03];
    body.extend_from_slice(&[0x11; 32]);
    body.push(SESSION_ID.len() as u8);
    body.extend_from_slice(SESSION_ID);
    body.extend_from_slice(&2_u16.to_be_bytes());
    body.extend_from_slice(&SUITES[0].to_be_bytes());
    body.extend_from_slice(&[1, 0]);
    append_extensions(&mut body, extensions);
    body
}

fn server_body(random: [u8; 32], extensions: &[(u16, &[u8])]) -> Vec<u8> {
    let mut body = vec![0x03, 0x03];
    body.extend_from_slice(&random);
    body.push(SESSION_ID.len() as u8);
    body.extend_from_slice(SESSION_ID);
    body.extend_from_slice(&SUITES[0].to_be_bytes());
    body.push(0);
    append_extensions(&mut body, extensions);
    body
}

fn client_limits() -> ClientHelloLimits {
    ClientHelloLimits::new(8, ExtensionBlockLimits::new(128, 8).unwrap()).unwrap()
}

fn server_limits() -> Tls13ServerHelloLimits {
    Tls13ServerHelloLimits::new(256, 128, 8).unwrap()
}

fn prior_selection<'client, 'offer, 'server>(
    client_bytes: &'client [u8],
    suites: &'offer [u16],
    server_bytes: &'server [u8],
) -> BoundServerHelloSelection<'client, 'offer, 'server> {
    let client = decode_client_hello(client_bytes, client_limits()).unwrap();
    let server = decode_tls13_server_hello(server_bytes, server_limits()).unwrap();
    let offer = ClientCipherSuiteOffer::new(suites, cipher_policy()).unwrap();
    bind_server_hello_selection(client, offer, server).unwrap()
}

fn standard_policy() -> ExtensionContextPolicy<'static> {
    ExtensionContextPolicy::new(&STANDARD_TLS13_EXTENSION_RULES).unwrap()
}

#[test]
fn binds_an_ordinary_exact_tls13_selection_and_retains_prior_evidence() {
    let client_bytes = client_body(&[(43, CLIENT_TLS13_TLS12)]);
    let server_bytes = server_body([0x22; 32], &[(43, SERVER_TLS13)]);
    let selection = prior_selection(&client_bytes, SUITES, &server_bytes);
    let client_pointer = selection.client_hello().body().as_ptr();
    let server_pointer = selection.server_hello().encoded().as_ptr();
    let versions = [0x0304, 0x0303];
    let offer = ClientVersionOffer::new(&versions).unwrap();
    let policy = standard_policy();

    let bound = bind_server_hello_version_selection(selection, offer, policy).unwrap();

    assert_eq!(bound.kind(), Tls13ServerHelloKind::ServerHello);
    assert_eq!(bound.version().wire_id(), 0x0304);
    assert_eq!(bound.version_offer(), offer);
    assert_eq!(bound.extension_policy(), policy);
    assert_eq!(
        bound.selection().client_hello().body().as_ptr(),
        client_pointer
    );
    assert_eq!(
        bound.selection().server_hello().encoded().as_ptr(),
        server_pointer
    );
}

#[test]
fn binds_retry_context_and_its_unsolicited_cookie_without_advancing_state() {
    let client_bytes = client_body(&[(43, CLIENT_TLS13_TLS12)]);
    let server_bytes = server_body(
        TLS13_HELLO_RETRY_REQUEST_RANDOM,
        &[(43, SERVER_TLS13), (44, b"retry-cookie")],
    );
    let selection = prior_selection(&client_bytes, SUITES, &server_bytes);
    let versions = [0x0304, 0x0303];

    let bound = bind_server_hello_version_selection(
        selection,
        ClientVersionOffer::new(&versions).unwrap(),
        standard_policy(),
    )
    .unwrap();

    assert_eq!(bound.kind(), Tls13ServerHelloKind::HelloRetryRequest);
    assert_eq!(bound.version().wire_id(), 0x0304);
}

#[test]
fn rejects_every_detached_client_version_offer_difference() {
    let client_bytes = client_body(&[(43, CLIENT_TLS13_TLS12)]);
    let server_bytes = server_body([0x22; 32], &[(43, SERVER_TLS13)]);
    let detached = [&[0x0304][..], &[0x0303, 0x0304][..], &[0x0304, 0x0302][..]];

    for versions in detached {
        let selection = prior_selection(&client_bytes, SUITES, &server_bytes);
        let offer = ClientVersionOffer::new(versions).unwrap();
        assert_eq!(
            bind_server_hello_version_selection(selection, offer, standard_policy()).unwrap_err(),
            ServerHelloVersionSelectionError::ClientVersionOfferMismatch
        );
    }
}

#[test]
fn requires_supported_versions_in_both_exact_messages() {
    let no_versions_client = client_body(&[(51, &[]), (10, &[])]);
    let no_versions_server = server_body([0x22; 32], &[(51, &[0, 0])]);
    let versions = [0x0304];
    assert_eq!(
        bind_server_hello_version_selection(
            prior_selection(&no_versions_client, SUITES, &no_versions_server),
            ClientVersionOffer::new(&versions).unwrap(),
            standard_policy(),
        )
        .unwrap_err(),
        ServerHelloVersionSelectionError::MissingClientSupportedVersions
    );

    let client_bytes = client_body(&[(43, &[2, 0x03, 0x04]), (51, &[])]);
    assert_eq!(
        bind_server_hello_version_selection(
            prior_selection(&client_bytes, SUITES, &no_versions_server),
            ClientVersionOffer::new(&versions).unwrap(),
            standard_policy(),
        )
        .unwrap_err(),
        ServerHelloVersionSelectionError::MissingServerSupportedVersions
    );
}

#[test]
fn applies_the_actual_ordinary_or_retry_extension_context() {
    let client_bytes = client_body(&[(43, CLIENT_TLS13_TLS12), (44, b"offered-cookie")]);
    let server_bytes = server_body([0x22; 32], &[(43, SERVER_TLS13), (44, b"ordinary-cookie")]);
    let versions = [0x0304, 0x0303];

    assert_eq!(
        bind_server_hello_version_selection(
            prior_selection(&client_bytes, SUITES, &server_bytes),
            ClientVersionOffer::new(&versions).unwrap(),
            standard_policy(),
        )
        .unwrap_err(),
        ServerHelloVersionSelectionError::ExtensionPolicy(
            ExtensionContextPolicyError::ForbiddenContext
        )
    );
}

#[test]
fn rejects_a_server_response_extension_absent_from_the_exact_clienthello() {
    let client_bytes = client_body(&[(43, CLIENT_TLS13_TLS12)]);
    let server_bytes = server_body([0x22; 32], &[(43, SERVER_TLS13), (51, &[0, 0])]);
    let versions = [0x0304, 0x0303];

    assert_eq!(
        bind_server_hello_version_selection(
            prior_selection(&client_bytes, SUITES, &server_bytes),
            ClientVersionOffer::new(&versions).unwrap(),
            standard_policy(),
        )
        .unwrap_err(),
        ServerHelloVersionSelectionError::ExtensionPolicy(
            ExtensionContextPolicyError::UnsolicitedExtension
        )
    );
}

#[test]
fn preserves_server_version_selection_failure_categories() {
    let cases = [
        (
            CLIENT_TLS13_TLS12,
            &[0x03, 0x05][..],
            &[0x0304, 0x0303][..],
            VersionError::UnofferedServerSelection,
        ),
        (
            CLIENT_TLS13_TLS12,
            &[0x03, 0x03][..],
            &[0x0304, 0x0303][..],
            VersionError::ServerSelectedPriorVersion,
        ),
        (
            CLIENT_TLS14_TLS13,
            &[0x03, 0x05][..],
            &[0x0305, 0x0304][..],
            VersionError::UnsupportedServerSelection,
        ),
        (
            CLIENT_TLS13_TLS12,
            &[0x03, 0x04, 0x00][..],
            &[0x0304, 0x0303][..],
            VersionError::MalformedServerSelection,
        ),
    ];

    for (client_data, server_data, versions, expected) in cases {
        let client_bytes = client_body(&[(43, client_data)]);
        let server_bytes = server_body([0x22; 32], &[(43, server_data)]);
        let error = bind_server_hello_version_selection(
            prior_selection(&client_bytes, SUITES, &server_bytes),
            ClientVersionOffer::new(versions).unwrap(),
            standard_policy(),
        )
        .unwrap_err();
        assert_eq!(
            error,
            ServerHelloVersionSelectionError::VersionSelection(expected)
        );
    }
}

#[test]
fn diagnostics_are_stable_redacted_and_preserve_nested_sources() {
    let errors = [
        (
            ServerHelloVersionSelectionError::ExtensionPolicy(
                ExtensionContextPolicyError::ForbiddenContext,
            ),
            "TLS extension context policy rejected ServerHello evidence",
        ),
        (
            ServerHelloVersionSelectionError::MissingClientSupportedVersions,
            "the exact ClientHello is missing supported_versions",
        ),
        (
            ServerHelloVersionSelectionError::MissingServerSupportedVersions,
            "the exact server message is missing supported_versions",
        ),
        (
            ServerHelloVersionSelectionError::ClientVersionOfferMismatch,
            "version offer does not match the exact ClientHello",
        ),
        (
            ServerHelloVersionSelectionError::VersionSelection(
                VersionError::UnofferedServerSelection,
            ),
            "server supported_versions selection was not authorized",
        ),
    ];
    for (error, expected) in errors {
        assert_eq!(error.to_string(), expected);
        for disclosed in ["version-session", "0x0304", "772", "retry-cookie"] {
            assert!(!format!("{error:?}").contains(disclosed));
        }
    }
    assert_eq!(
        errors[0].0.source().unwrap().to_string(),
        "TLS extension is forbidden in this message context"
    );
    assert_eq!(
        errors[4].0.source().unwrap().to_string(),
        "server selected an unoffered TLS version"
    );

    let client_bytes = client_body(&[(43, CLIENT_TLS13_TLS12)]);
    let server_bytes = server_body([0x22; 32], &[(43, SERVER_TLS13)]);
    let versions = [0x0304, 0x0303];
    let debug = format!(
        "{:?}",
        bind_server_hello_version_selection(
            prior_selection(&client_bytes, SUITES, &server_bytes),
            ClientVersionOffer::new(&versions).unwrap(),
            standard_policy(),
        )
        .unwrap()
    );
    assert!(debug.contains("[RETAINED]"));
    assert!(debug.contains("[POLICY-BOUND]"));
    for disclosed in ["version-session", "0x0304", "772"] {
        assert!(!debug.contains(disclosed));
    }
}
