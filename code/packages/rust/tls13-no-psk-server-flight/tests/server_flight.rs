use tls13_handshake::Tls13HandshakeType;
use tls13_no_psk_server_flight::{
    NoPskServerFlight, ServerFlightErrorKind, ServerFlightState,
};

#[test]
fn accepts_server_flight_without_client_authentication_request() {
    let mut flight = NoPskServerFlight::new();

    assert_eq!(
        flight.accept(Tls13HandshakeType::EncryptedExtensions),
        Ok(ServerFlightState::ExpectCertificateRequestOrCertificate)
    );
    assert_eq!(
        flight.accept(Tls13HandshakeType::Certificate),
        Ok(ServerFlightState::ExpectCertificateVerify)
    );
    assert_eq!(
        flight.accept(Tls13HandshakeType::CertificateVerify),
        Ok(ServerFlightState::ExpectFinished)
    );
    assert_eq!(
        flight.accept(Tls13HandshakeType::Finished),
        Ok(ServerFlightState::Complete)
    );

    assert!(flight.is_complete());
    assert!(!flight.client_authentication_requested());
}

#[test]
fn accepts_one_certificate_request_and_retains_client_authentication_requirement() {
    let mut flight = NoPskServerFlight::new();
    for handshake_type in [
        Tls13HandshakeType::EncryptedExtensions,
        Tls13HandshakeType::CertificateRequest,
        Tls13HandshakeType::Certificate,
        Tls13HandshakeType::CertificateVerify,
        Tls13HandshakeType::Finished,
    ] {
        flight.accept(handshake_type).expect("ordered message");
    }

    assert!(flight.is_complete());
    assert!(flight.client_authentication_requested());
}

#[test]
fn unexpected_message_does_not_advance_or_set_client_authentication() {
    let mut flight = NoPskServerFlight::new();
    let error = flight
        .accept(Tls13HandshakeType::CertificateRequest)
        .expect_err("server flight must start with EncryptedExtensions");

    assert_eq!(error.kind(), ServerFlightErrorKind::UnexpectedMessage);
    assert_eq!(flight.state(), ServerFlightState::ExpectEncryptedExtensions);
    assert!(!flight.client_authentication_requested());
}

#[test]
fn duplicate_certificate_request_fails_without_advancing() {
    let mut flight = NoPskServerFlight::new();
    flight
        .accept(Tls13HandshakeType::EncryptedExtensions)
        .unwrap();
    flight
        .accept(Tls13HandshakeType::CertificateRequest)
        .unwrap();

    let error = flight
        .accept(Tls13HandshakeType::CertificateRequest)
        .expect_err("CertificateRequest appears at most once");
    assert_eq!(error.kind(), ServerFlightErrorKind::UnexpectedMessage);
    assert_eq!(flight.state(), ServerFlightState::ExpectCertificate);
    assert!(flight.client_authentication_requested());
}

#[test]
fn premature_finished_fails_without_advancing() {
    let mut flight = NoPskServerFlight::new();
    flight
        .accept(Tls13HandshakeType::EncryptedExtensions)
        .unwrap();

    let error = flight
        .accept(Tls13HandshakeType::Finished)
        .expect_err("certificate authentication cannot be skipped on no-PSK path");
    assert_eq!(error.kind(), ServerFlightErrorKind::UnexpectedMessage);
    assert_eq!(
        flight.state(),
        ServerFlightState::ExpectCertificateRequestOrCertificate
    );
}

#[test]
fn complete_flight_rejects_further_messages_without_mutation() {
    let mut flight = NoPskServerFlight::new();
    for handshake_type in [
        Tls13HandshakeType::EncryptedExtensions,
        Tls13HandshakeType::Certificate,
        Tls13HandshakeType::CertificateVerify,
        Tls13HandshakeType::Finished,
    ] {
        flight.accept(handshake_type).unwrap();
    }

    let error = flight
        .accept(Tls13HandshakeType::NewSessionTicket)
        .expect_err("post-handshake messages belong to another state owner");
    assert_eq!(error.kind(), ServerFlightErrorKind::AlreadyComplete);
    assert_eq!(flight.state(), ServerFlightState::Complete);
}

#[test]
fn diagnostics_retain_no_received_message_type() {
    let mut flight = NoPskServerFlight::new();
    let error = flight
        .accept(Tls13HandshakeType::KeyUpdate)
        .expect_err("unexpected message fails");

    assert_eq!(
        error.to_string(),
        "TLS 1.3 no-PSK server-flight error UnexpectedMessage"
    );
    assert!(!format!("{error:?}").contains("KeyUpdate"));
}
