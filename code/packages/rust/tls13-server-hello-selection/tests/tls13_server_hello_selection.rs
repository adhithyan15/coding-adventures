use std::error::Error;

use tls13_cipher_suite_offer::{CipherSuiteOfferError, ClientCipherSuiteOffer};
use tls13_cipher_suite_policy::{
    CipherSuite, CipherSuitePolicy, CipherSuitePolicyEntry, PolicyError,
};
use tls13_client_hello_message::{decode_client_hello, ClientHelloLimits};
use tls13_extension_block::ExtensionBlockLimits;
use tls13_server_hello_message::{
    decode_tls13_server_hello, Tls13ServerHelloKind, Tls13ServerHelloLimits,
    TLS13_HELLO_RETRY_REQUEST_RANDOM,
};
use tls13_server_hello_selection::{bind_server_hello_selection, ServerHelloSelectionError};

const SESSION_ID: &[u8] = b"client-session";

fn policy(suites: &[CipherSuite]) -> CipherSuitePolicy {
    let entries: Vec<_> = suites
        .iter()
        .copied()
        .map(|suite| CipherSuitePolicyEntry::new(suite, 1_000).unwrap())
        .collect();
    CipherSuitePolicy::new(&entries).unwrap()
}

fn client_body(session_id: &[u8], suites: &[u16]) -> Vec<u8> {
    let mut body = vec![0x03, 0x03];
    body.extend_from_slice(&[0x11; 32]);
    body.push(session_id.len() as u8);
    body.extend_from_slice(session_id);
    body.extend_from_slice(&((suites.len() * 2) as u16).to_be_bytes());
    for suite in suites {
        body.extend_from_slice(&suite.to_be_bytes());
    }
    body.extend_from_slice(&[1, 0]);
    body.extend_from_slice(&[0, 8, 0, 43, 0, 4, 0x03, 0x04, 0x03, 0x03]);
    body
}

fn server_body(session_id: &[u8], suite: u16, random: [u8; 32]) -> Vec<u8> {
    let mut body = vec![0x03, 0x03];
    body.extend_from_slice(&random);
    body.push(session_id.len() as u8);
    body.extend_from_slice(session_id);
    body.extend_from_slice(&suite.to_be_bytes());
    body.push(0);
    body.extend_from_slice(&[0, 6, 0, 43, 0, 2, 0x03, 0x04]);
    body
}

fn client_limits() -> ClientHelloLimits {
    ClientHelloLimits::new(8, ExtensionBlockLimits::new(64, 8).unwrap()).unwrap()
}

fn server_limits() -> Tls13ServerHelloLimits {
    Tls13ServerHelloLimits::new(256, 64, 8).unwrap()
}

#[test]
fn binds_an_ordinary_server_selection_to_the_exact_client_evidence() {
    let suites = [0x1302, 0x1301];
    let client_bytes = client_body(SESSION_ID, &suites);
    let server_bytes = server_body(SESSION_ID, 0x1301, [0x22; 32]);
    let client = decode_client_hello(&client_bytes, client_limits()).unwrap();
    let server = decode_tls13_server_hello(&server_bytes, server_limits()).unwrap();
    let offer = ClientCipherSuiteOffer::new(
        &suites,
        policy(&[
            CipherSuite::TlsAes128GcmSha256,
            CipherSuite::TlsAes256GcmSha384,
        ]),
    )
    .unwrap();
    let client_body_pointer = client.body().as_ptr();
    let server_body_pointer = server.encoded().as_ptr();

    let bound = bind_server_hello_selection(client, offer, server).unwrap();

    assert_eq!(bound.kind(), Tls13ServerHelloKind::ServerHello);
    assert_eq!(bound.client_hello().body().as_ptr(), client_body_pointer);
    assert_eq!(bound.server_hello().encoded().as_ptr(), server_body_pointer);
    assert_eq!(bound.client_offer(), offer);
    assert_eq!(
        bound.cipher_suite().parameters(),
        CipherSuite::TlsAes128GcmSha256.parameters()
    );
    assert_eq!(bound.cipher_suite().record_limit(), 1_000);
}

#[test]
fn binds_the_exact_retry_shape_without_advancing_retry_state() {
    let suites = [0x1301];
    let client_bytes = client_body(SESSION_ID, &suites);
    let server_bytes = server_body(SESSION_ID, 0x1301, TLS13_HELLO_RETRY_REQUEST_RANDOM);
    let client = decode_client_hello(&client_bytes, client_limits()).unwrap();
    let server = decode_tls13_server_hello(&server_bytes, server_limits()).unwrap();
    let offer =
        ClientCipherSuiteOffer::new(&suites, policy(&[CipherSuite::TlsAes128GcmSha256])).unwrap();

    let bound = bind_server_hello_selection(client, offer, server).unwrap();
    assert_eq!(bound.kind(), Tls13ServerHelloKind::HelloRetryRequest);
}

#[test]
fn rejects_detached_offer_evidence_with_any_vector_difference() {
    let client_suites = [0x1302, 0x1301];
    let client_bytes = client_body(SESSION_ID, &client_suites);
    let server_bytes = server_body(SESSION_ID, 0x1301, [0x22; 32]);
    let suite_policy = policy(&[
        CipherSuite::TlsAes128GcmSha256,
        CipherSuite::TlsAes256GcmSha384,
    ]);

    for detached in [&[0x1301][..], &[0x1301, 0x1302][..], &[0x1302, 0x1303][..]] {
        let client = decode_client_hello(&client_bytes, client_limits()).unwrap();
        let server = decode_tls13_server_hello(&server_bytes, server_limits()).unwrap();
        let offer = ClientCipherSuiteOffer::new(detached, suite_policy).unwrap();
        assert_eq!(
            bind_server_hello_selection(client, offer, server).unwrap_err(),
            ServerHelloSelectionError::ClientOfferMismatch
        );
    }
}

#[test]
fn rejects_a_different_legacy_session_id_echo() {
    let suites = [0x1301];
    let client_bytes = client_body(SESSION_ID, &suites);
    let server_bytes = server_body(b"other-session", 0x1301, [0x22; 32]);
    let client = decode_client_hello(&client_bytes, client_limits()).unwrap();
    let server = decode_tls13_server_hello(&server_bytes, server_limits()).unwrap();
    let offer =
        ClientCipherSuiteOffer::new(&suites, policy(&[CipherSuite::TlsAes128GcmSha256])).unwrap();

    assert_eq!(
        bind_server_hello_selection(client, offer, server).unwrap_err(),
        ServerHelloSelectionError::LegacySessionIdEchoMismatch
    );
}

#[test]
fn preserves_unoffered_and_policy_rejected_selection_categories() {
    let client_suites = [0x1301, 0x1302];
    let client_bytes = client_body(SESSION_ID, &client_suites);
    let client = decode_client_hello(&client_bytes, client_limits()).unwrap();

    let unoffered_bytes = server_body(SESSION_ID, 0x1303, [0x22; 32]);
    let unoffered = decode_tls13_server_hello(&unoffered_bytes, server_limits()).unwrap();
    let offer = ClientCipherSuiteOffer::new(
        &client_suites,
        policy(&[
            CipherSuite::TlsAes128GcmSha256,
            CipherSuite::TlsAes256GcmSha384,
        ]),
    )
    .unwrap();
    assert_eq!(
        bind_server_hello_selection(client, offer, unoffered).unwrap_err(),
        ServerHelloSelectionError::CipherSuiteSelection(
            CipherSuiteOfferError::UnofferedServerSelection
        )
    );

    let client = decode_client_hello(&client_bytes, client_limits()).unwrap();
    let rejected_bytes = server_body(SESSION_ID, 0x1302, [0x22; 32]);
    let rejected = decode_tls13_server_hello(&rejected_bytes, server_limits()).unwrap();
    let offer =
        ClientCipherSuiteOffer::new(&client_suites, policy(&[CipherSuite::TlsAes128GcmSha256]))
            .unwrap();
    assert_eq!(
        bind_server_hello_selection(client, offer, rejected).unwrap_err(),
        ServerHelloSelectionError::CipherSuiteSelection(
            CipherSuiteOfferError::PolicyRejectedServerSelection(
                PolicyError::DisallowedCipherSuite
            )
        )
    );
}

#[test]
fn diagnostics_are_stable_and_retain_no_handshake_payloads() {
    let errors = [
        (
            ServerHelloSelectionError::ClientOfferMismatch,
            "cipher-suite offer does not match the exact ClientHello",
        ),
        (
            ServerHelloSelectionError::LegacySessionIdEchoMismatch,
            "ServerHello legacy session ID does not match the ClientHello",
        ),
        (
            ServerHelloSelectionError::CipherSuiteSelection(
                CipherSuiteOfferError::UnofferedServerSelection,
            ),
            "ServerHello cipher-suite selection was not authorized",
        ),
    ];
    for (error, expected) in errors {
        assert_eq!(error.to_string(), expected);
        assert!(!format!("{error:?}").contains("client-session"));
    }

    let nested = errors[2].0.source().expect("selection source");
    assert_eq!(
        nested.to_string(),
        "server selected an unoffered TLS cipher suite"
    );

    let suites = [0x1301];
    let client_bytes = client_body(SESSION_ID, &suites);
    let server_bytes = server_body(SESSION_ID, 0x1301, [0x22; 32]);
    let client = decode_client_hello(&client_bytes, client_limits()).unwrap();
    let server = decode_tls13_server_hello(&server_bytes, server_limits()).unwrap();
    let offer =
        ClientCipherSuiteOffer::new(&suites, policy(&[CipherSuite::TlsAes128GcmSha256])).unwrap();
    let debug = format!(
        "{:?}",
        bind_server_hello_selection(client, offer, server).unwrap()
    );
    assert!(debug.contains("[POLICY-BOUND]"));
    assert!(debug.contains("[RETAINED]"));
    for disclosed in ["client-session", "4865", "0x1301"] {
        assert!(!debug.contains(disclosed));
    }
}
