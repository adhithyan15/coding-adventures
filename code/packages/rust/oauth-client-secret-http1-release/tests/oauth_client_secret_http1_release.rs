use coding_adventures_oauth::{
    prepare_token_refresh, Audited, OAuthAuditError, OAuthAuditEvent, OAuthAuditSink, OAuthTraceId,
    ProviderConfig, ProviderId,
};
use coding_adventures_oauth_client_secret_custody::{
    ClientSecretAuditError, ClientSecretAuditEvent, ClientSecretAuditSink,
    ClientSecretAuthenticatedTokenRefresh, ClientSecretAuthentication,
    ClientSecretAuthenticationMethod, ClientSecretCustody, ClientSecretKey, ClientSecretReference,
    InMemoryClientSecretStore,
};
use coding_adventures_zeroize::Zeroizing;
use der_asn1::Asn1Limits;
use http1_fixed_request_body::{
    Http1FixedRequestBodyLimits, MAX_HTTP1_FIXED_REQUEST_CONTENT_LENGTH_BYTES,
};
use http1_request_header_field::{
    validate_http1_request_header_field, Http1RequestHeaderFieldLimits,
};
use http1_request_host_authority::{
    validate_http1_request_host_authority, Http1RequestHostAuthorityLimits,
};
use http1_request_line::{
    validate_http1_request_line, Http1RequestLineLimits, Http1RequestVersion,
};
use https_destination_name_binding::{
    bind_https_dns_destination, bind_https_host_header_field, BoundHttpsHostHeaderField,
};
use oauth_client_secret_http1_binding::{
    bind_client_secret_http1_request_material, ClientSecretHttp1AuditError,
    ClientSecretHttp1AuditEvent, ClientSecretHttp1AuditSink,
};
use oauth_client_secret_http1_release::{
    release_client_secret_http1_request, ClientSecretHttp1ReleaseAuditAction,
    ClientSecretHttp1ReleaseAuditError, ClientSecretHttp1ReleaseAuditEvent,
    ClientSecretHttp1ReleaseAuditFailure, ClientSecretHttp1ReleaseAuditOutcome,
    ClientSecretHttp1ReleaseAuditSink, ClientSecretHttp1ReleaseError,
    ClientSecretHttp1ReleasePolicy, ClientSecretHttp1ReleasePolicyError,
    MAX_CLIENT_SECRET_HTTP1_REQUEST_BYTES,
};
use tls13_server_name::TlsServerName;
use x509_certificate::{decode_x509_certificate, X509Certificate};

const SIGNATURE_OID: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];
const KEY_OID: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
const SUBJECT_ALT_NAME_OID: &[u8] = &[0x55, 0x1d, 0x11];

#[derive(Default)]
struct CustodyAudit;

impl ClientSecretAuditSink for CustodyAudit {
    fn publish(&mut self, _event: &ClientSecretAuditEvent) -> Result<(), ClientSecretAuditError> {
        Ok(())
    }
}

#[derive(Default)]
struct OAuthAudit;

impl OAuthAuditSink for OAuthAudit {
    fn publish(&mut self, _event: &OAuthAuditEvent) -> Result<(), OAuthAuditError> {
        Ok(())
    }
}

#[derive(Default)]
struct BindingAudit;

impl ClientSecretHttp1AuditSink for BindingAudit {
    fn publish(
        &mut self,
        _event: &ClientSecretHttp1AuditEvent,
    ) -> Result<(), ClientSecretHttp1AuditError> {
        Ok(())
    }
}

#[derive(Debug, Eq, PartialEq)]
struct RecordedReleaseAudit {
    provider: String,
    trace: OAuthTraceId,
    action: ClientSecretHttp1ReleaseAuditAction,
    outcome: ClientSecretHttp1ReleaseAuditOutcome,
    debug: String,
}

#[derive(Default)]
struct ReleaseAudit {
    events: Vec<RecordedReleaseAudit>,
    fail_at: Option<usize>,
}

impl ClientSecretHttp1ReleaseAuditSink for ReleaseAudit {
    fn publish(
        &mut self,
        event: &ClientSecretHttp1ReleaseAuditEvent<'_>,
    ) -> Result<(), ClientSecretHttp1ReleaseAuditError> {
        if self.fail_at == Some(self.events.len()) {
            return Err(ClientSecretHttp1ReleaseAuditError);
        }
        self.events.push(RecordedReleaseAudit {
            provider: event.provider().as_str().to_owned(),
            trace: event.trace(),
            action: event.action(),
            outcome: event.outcome(),
            debug: format!("{event:?}"),
        });
        Ok(())
    }
}

fn trace() -> OAuthTraceId {
    OAuthTraceId::new([0x42; 16])
}

fn provider() -> ProviderId {
    ProviderId::new("provider-a").expect("provider")
}

fn config() -> ProviderConfig {
    ProviderConfig::new(
        provider(),
        "https://authorize.example/oauth2/auth",
        "https://token.example/oauth2/token",
        "client-id",
        "http://127.0.0.1:53682/callback",
    )
    .expect("provider config")
}

fn release<T>(audited: Audited<T>) -> T {
    audited
        .publish_then_release(&mut OAuthAudit)
        .expect("OAuth audit")
}

fn authenticated_request(
    method: ClientSecretAuthenticationMethod,
) -> ClientSecretAuthenticatedTokenRefresh {
    let custody = ClientSecretCustody::new(InMemoryClientSecretStore::new());
    let key = ClientSecretKey::new(provider(), ClientSecretReference::new([0x51; 32]));
    let mut audit = CustodyAudit;
    custody
        .create(
            &key,
            Zeroizing::new("secret value".to_owned()),
            trace(),
            &mut audit,
        )
        .expect("create secret");
    let request = release(prepare_token_refresh(
        &config(),
        Zeroizing::new("refresh-token".to_owned()),
        &["files.read"],
        trace(),
    ));
    custody
        .authenticate_token_refresh(
            &config(),
            &ClientSecretAuthentication::new(key, method),
            request,
            &mut audit,
        )
        .expect("authenticate request")
}

fn body_limits() -> Http1FixedRequestBodyLimits {
    Http1FixedRequestBodyLimits::new(1024 * 1024).expect("body limits")
}

fn field_limits() -> Http1RequestHeaderFieldLimits {
    Http1RequestHeaderFieldLimits::new(64, 8 * 1024, 8 * 1024).expect("field limits")
}

fn line_limits() -> Http1RequestLineLimits {
    Http1RequestLineLimits::new(16, 4096, 4096).expect("line limits")
}

fn host_limits() -> Http1RequestHostAuthorityLimits {
    Http1RequestHostAuthorityLimits::new(253, 259).expect("host limits")
}

fn der(tag: u8, value: &[u8]) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(value.len() + 4);
    encoded.push(tag);
    match value.len() {
        length @ 0..=127 => encoded.push(length as u8),
        length @ 128..=255 => encoded.extend([0x81, length as u8]),
        length @ 256..=65_535 => encoded.extend([0x82, (length >> 8) as u8, length as u8]),
        _ => panic!("test value too large"),
    }
    encoded.extend(value);
    encoded
}

fn algorithm(oid: &[u8]) -> Vec<u8> {
    let mut fields = der(0x06, oid);
    fields.extend(der(0x05, &[]));
    der(0x30, &fields)
}

fn name(common_name: &[u8]) -> Vec<u8> {
    let mut attribute = der(0x06, &[0x55, 0x04, 0x03]);
    attribute.extend(der(0x0c, common_name));
    der(0x30, &der(0x31, &der(0x30, &attribute)))
}

fn certificate() -> Vec<u8> {
    let mut validity = der(0x17, b"260101000000Z");
    validity.extend(der(0x17, b"270101000000Z"));
    let mut public_key = algorithm(KEY_OID);
    public_key.extend(der(0x03, &[0x00, 0x30, 0x00]));
    let names = der(0x30, &der(0x82, b"token.example"));
    let mut extension = der(0x06, SUBJECT_ALT_NAME_OID);
    extension.extend(der(0x04, &names));

    let mut tbs = der(0xa0, &der(0x02, &[0x02]));
    tbs.extend(der(0x02, &[0x01]));
    tbs.extend(algorithm(SIGNATURE_OID));
    tbs.extend(name(b"issuer.example"));
    tbs.extend(der(0x30, &validity));
    tbs.extend(name(b"wrong.example"));
    tbs.extend(der(0x30, &public_key));
    tbs.extend(der(0xa3, &der(0x30, &der(0x30, &extension))));

    let mut fields = der(0x30, &tbs);
    fields.extend(algorithm(SIGNATURE_OID));
    fields.extend(der(0x03, &[0x00, 0xde, 0xad]));
    der(0x30, &fields)
}

fn destination<'certificate, 'encoded>(
    certificate: &'certificate X509Certificate<'encoded>,
) -> BoundHttpsHostHeaderField<'certificate, 'encoded, 'static, 'static, 'static> {
    let server_name = TlsServerName::new("token.example").expect("server name");
    let host = validate_http1_request_host_authority(b"token.example", None, host_limits())
        .expect("host authority");
    let destination =
        bind_https_dns_destination(certificate, server_name, host, Asn1Limits::default())
            .expect("destination");
    let host_field = validate_http1_request_header_field(b"Host", b"token.example", field_limits())
        .expect("Host field");
    bind_https_host_header_field(destination, host_field).expect("Host binding")
}

fn request_line(target: &'static [u8]) -> http1_request_line::Http1RequestLine<'static> {
    validate_http1_request_line(b"POST", target, Http1RequestVersion::Http11, line_limits())
        .expect("request line")
}

fn content_type(
    value: &'static [u8],
) -> http1_request_header_field::Http1RequestHeaderField<'static> {
    validate_http1_request_header_field(b"Content-Type", value, field_limits())
        .expect("Content-Type")
}

fn policy<'provider>(
    provider: &'provider ProviderId,
    limit: usize,
) -> ClientSecretHttp1ReleasePolicy<'provider> {
    ClientSecretHttp1ReleasePolicy::new(
        provider,
        b"POST",
        b"application/x-www-form-urlencoded",
        limit,
    )
    .expect("release policy")
}

#[test]
fn basic_releases_one_exact_complete_request_after_two_audits() {
    let authenticated = authenticated_request(ClientSecretAuthenticationMethod::ClientSecretBasic);
    let request = authenticated.request();
    let encoded_certificate = certificate();
    let certificate =
        decode_x509_certificate(&encoded_certificate, Asn1Limits::default()).expect("certificate");
    let destination = destination(&certificate);
    let mut length = [0u8; MAX_HTTP1_FIXED_REQUEST_CONTENT_LENGTH_BYTES];
    let material = bind_client_secret_http1_request_material(
        request,
        body_limits(),
        field_limits(),
        &mut length,
        &mut BindingAudit,
    )
    .expect("material");
    let expected_provider = provider();
    let mut audit = ReleaseAudit::default();

    let released = release_client_secret_http1_request(
        policy(&expected_provider, 4096),
        request_line(b"/oauth2/token"),
        &destination,
        material,
        content_type(b"application/x-www-form-urlencoded"),
        &mut audit,
    )
    .expect("complete request");

    let expected = format!(
        "POST /oauth2/token HTTP/1.1\r\nHost: token.example\r\nContent-Length: {}\r\nContent-Type: application/x-www-form-urlencoded\r\nAuthorization: {}\r\n\r\n{}",
        request.form_body().len(),
        request.authorization_header().expect("Basic authorization"),
        request.form_body(),
    );
    assert_eq!(released.provider(), request.provider());
    assert_eq!(released.trace(), trace());
    assert_eq!(released.request_line().target(), b"/oauth2/token");
    assert!(core::ptr::eq(released.destination(), &destination));
    assert_eq!(released.content_type().name(), b"Content-Type");
    assert_eq!(released.len(), expected.len());
    assert!(!released.is_empty());
    assert_eq!(audit.events.len(), 2);
    assert_eq!(
        audit.events[0].outcome,
        ClientSecretHttp1ReleaseAuditOutcome::Attempted
    );
    assert_eq!(
        audit.events[1].outcome,
        ClientSecretHttp1ReleaseAuditOutcome::Succeeded
    );
    assert!(audit.events.iter().all(|event| {
        event.provider == "provider-a"
            && event.trace == trace()
            && event.action == ClientSecretHttp1ReleaseAuditAction::ReleaseCompleteRequest
    }));
    assert_eq!(
        released.audited_bytes(&mut audit).unwrap(),
        expected.as_bytes()
    );
    assert_eq!(audit.events.len(), 3);
    assert_eq!(
        audit.events[2].action,
        ClientSecretHttp1ReleaseAuditAction::ReadCompleteRequest
    );
    assert_eq!(
        audit.events[2].outcome,
        ClientSecretHttp1ReleaseAuditOutcome::Succeeded
    );
}

#[test]
fn post_releases_body_without_authorization_field() {
    let authenticated = authenticated_request(ClientSecretAuthenticationMethod::ClientSecretPost);
    let request = authenticated.request();
    let encoded_certificate = certificate();
    let certificate = decode_x509_certificate(&encoded_certificate, Asn1Limits::default()).unwrap();
    let destination = destination(&certificate);
    let mut length = [0u8; MAX_HTTP1_FIXED_REQUEST_CONTENT_LENGTH_BYTES];
    let material = bind_client_secret_http1_request_material(
        request,
        body_limits(),
        field_limits(),
        &mut length,
        &mut BindingAudit,
    )
    .unwrap();
    let expected_provider = provider();
    let mut audit = ReleaseAudit::default();
    let released = release_client_secret_http1_request(
        policy(&expected_provider, 4096),
        request_line(b"/oauth2/token"),
        &destination,
        material,
        content_type(b"application/x-www-form-urlencoded"),
        &mut audit,
    )
    .unwrap();

    let wire = std::str::from_utf8(released.audited_bytes(&mut audit).unwrap()).unwrap();
    assert!(!wire.contains("Authorization:"));
    assert!(wire.ends_with(request.form_body()));
    assert!(wire.contains("\r\n\r\ngrant_type=refresh_token"));
}

#[test]
fn wire_borrow_audit_failure_withholds_bytes() {
    let authenticated = authenticated_request(ClientSecretAuthenticationMethod::ClientSecretPost);
    let encoded_certificate = certificate();
    let certificate = decode_x509_certificate(&encoded_certificate, Asn1Limits::default()).unwrap();
    let destination = destination(&certificate);
    let mut length = [0u8; MAX_HTTP1_FIXED_REQUEST_CONTENT_LENGTH_BYTES];
    let material = bind_client_secret_http1_request_material(
        authenticated.request(),
        body_limits(),
        field_limits(),
        &mut length,
        &mut BindingAudit,
    )
    .unwrap();
    let expected_provider = provider();
    let mut audit = ReleaseAudit::default();
    let released = release_client_secret_http1_request(
        policy(&expected_provider, 4096),
        request_line(b"/oauth2/token"),
        &destination,
        material,
        content_type(b"application/x-www-form-urlencoded"),
        &mut audit,
    )
    .unwrap();
    audit.fail_at = Some(2);

    let error = released
        .audited_bytes(&mut audit)
        .expect_err("byte-read audit must fail");

    assert_eq!(error, ClientSecretHttp1ReleaseError::Audit);
    assert_eq!(audit.events.len(), 2);
}

#[test]
fn attempted_audit_failure_prevents_credential_release() {
    let authenticated = authenticated_request(ClientSecretAuthenticationMethod::ClientSecretBasic);
    let encoded_certificate = certificate();
    let certificate = decode_x509_certificate(&encoded_certificate, Asn1Limits::default()).unwrap();
    let destination = destination(&certificate);
    let mut length = [0u8; MAX_HTTP1_FIXED_REQUEST_CONTENT_LENGTH_BYTES];
    let material = bind_client_secret_http1_request_material(
        authenticated.request(),
        body_limits(),
        field_limits(),
        &mut length,
        &mut BindingAudit,
    )
    .unwrap();
    let expected_provider = provider();
    let mut audit = ReleaseAudit {
        events: Vec::new(),
        fail_at: Some(0),
    };

    let error = release_client_secret_http1_request(
        policy(&expected_provider, 4096),
        request_line(b"/oauth2/token"),
        &destination,
        material,
        content_type(b"application/x-www-form-urlencoded"),
        &mut audit,
    )
    .expect_err("attempt audit must fail");

    assert_eq!(error, ClientSecretHttp1ReleaseError::Audit);
    assert!(audit.events.is_empty());
}

#[test]
fn terminal_audit_failure_withholds_prepared_request() {
    let authenticated = authenticated_request(ClientSecretAuthenticationMethod::ClientSecretBasic);
    let encoded_certificate = certificate();
    let certificate = decode_x509_certificate(&encoded_certificate, Asn1Limits::default()).unwrap();
    let destination = destination(&certificate);
    let mut length = [0u8; MAX_HTTP1_FIXED_REQUEST_CONTENT_LENGTH_BYTES];
    let material = bind_client_secret_http1_request_material(
        authenticated.request(),
        body_limits(),
        field_limits(),
        &mut length,
        &mut BindingAudit,
    )
    .unwrap();
    let expected_provider = provider();
    let mut audit = ReleaseAudit {
        events: Vec::new(),
        fail_at: Some(1),
    };

    let error = release_client_secret_http1_request(
        policy(&expected_provider, 4096),
        request_line(b"/oauth2/token"),
        &destination,
        material,
        content_type(b"application/x-www-form-urlencoded"),
        &mut audit,
    )
    .expect_err("success audit must fail");

    assert_eq!(error, ClientSecretHttp1ReleaseError::Audit);
    assert_eq!(audit.events.len(), 1);
    assert_eq!(
        audit.events[0].outcome,
        ClientSecretHttp1ReleaseAuditOutcome::Attempted
    );
}

#[test]
fn rejection_audit_failure_hides_the_underlying_reason() {
    let authenticated = authenticated_request(ClientSecretAuthenticationMethod::ClientSecretPost);
    let encoded_certificate = certificate();
    let certificate = decode_x509_certificate(&encoded_certificate, Asn1Limits::default()).unwrap();
    let destination = destination(&certificate);
    let mut length = [0u8; MAX_HTTP1_FIXED_REQUEST_CONTENT_LENGTH_BYTES];
    let material = bind_client_secret_http1_request_material(
        authenticated.request(),
        body_limits(),
        field_limits(),
        &mut length,
        &mut BindingAudit,
    )
    .unwrap();
    let expected_provider = provider();
    let mut audit = ReleaseAudit {
        events: Vec::new(),
        fail_at: Some(1),
    };

    let error = release_client_secret_http1_request(
        policy(&expected_provider, 4096),
        request_line(b"/wrong"),
        &destination,
        material,
        content_type(b"application/x-www-form-urlencoded"),
        &mut audit,
    )
    .expect_err("rejection audit must fail closed");

    assert_eq!(error, ClientSecretHttp1ReleaseError::Audit);
    assert_eq!(audit.events.len(), 1);
    assert_eq!(
        audit.events[0].outcome,
        ClientSecretHttp1ReleaseAuditOutcome::Attempted
    );
}

#[test]
fn mismatched_endpoint_is_audited_and_rejected() {
    let authenticated = authenticated_request(ClientSecretAuthenticationMethod::ClientSecretPost);
    let encoded_certificate = certificate();
    let certificate = decode_x509_certificate(&encoded_certificate, Asn1Limits::default()).unwrap();
    let destination = destination(&certificate);
    let mut length = [0u8; MAX_HTTP1_FIXED_REQUEST_CONTENT_LENGTH_BYTES];
    let material = bind_client_secret_http1_request_material(
        authenticated.request(),
        body_limits(),
        field_limits(),
        &mut length,
        &mut BindingAudit,
    )
    .unwrap();
    let expected_provider = provider();
    let mut audit = ReleaseAudit::default();

    let error = release_client_secret_http1_request(
        policy(&expected_provider, 4096),
        request_line(b"/wrong"),
        &destination,
        material,
        content_type(b"application/x-www-form-urlencoded"),
        &mut audit,
    )
    .expect_err("endpoint must bind destination and target");

    assert_eq!(error, ClientSecretHttp1ReleaseError::EndpointMismatch);
    assert_eq!(
        audit.events[1].outcome,
        ClientSecretHttp1ReleaseAuditOutcome::Failed(
            ClientSecretHttp1ReleaseAuditFailure::PolicyMismatch
        )
    );
}

#[test]
fn provider_media_type_and_length_policy_fail_closed() {
    let authenticated = authenticated_request(ClientSecretAuthenticationMethod::ClientSecretPost);
    let encoded_certificate = certificate();
    let certificate = decode_x509_certificate(&encoded_certificate, Asn1Limits::default()).unwrap();
    let destination = destination(&certificate);

    for (expected_provider, content_type_value, limit, expected_error, expected_failure) in [
        (
            ProviderId::new("provider-b").unwrap(),
            b"application/x-www-form-urlencoded".as_slice(),
            4096,
            ClientSecretHttp1ReleaseError::PolicyMismatch,
            ClientSecretHttp1ReleaseAuditFailure::PolicyMismatch,
        ),
        (
            provider(),
            b"application/json".as_slice(),
            4096,
            ClientSecretHttp1ReleaseError::EvidenceMismatch,
            ClientSecretHttp1ReleaseAuditFailure::EvidenceMismatch,
        ),
        (
            provider(),
            b"application/x-www-form-urlencoded".as_slice(),
            1,
            ClientSecretHttp1ReleaseError::RequestTooLong,
            ClientSecretHttp1ReleaseAuditFailure::ResourceLimit,
        ),
    ] {
        let mut length = [0u8; MAX_HTTP1_FIXED_REQUEST_CONTENT_LENGTH_BYTES];
        let material = bind_client_secret_http1_request_material(
            authenticated.request(),
            body_limits(),
            field_limits(),
            &mut length,
            &mut BindingAudit,
        )
        .unwrap();
        let mut audit = ReleaseAudit::default();
        let error = release_client_secret_http1_request(
            policy(&expected_provider, limit),
            request_line(b"/oauth2/token"),
            &destination,
            material,
            content_type(content_type_value),
            &mut audit,
        )
        .expect_err("mismatch must fail");
        assert_eq!(error, expected_error);
        assert_eq!(
            audit.events[1].outcome,
            ClientSecretHttp1ReleaseAuditOutcome::Failed(expected_failure)
        );
    }
}

#[test]
fn policy_debug_release_debug_and_errors_redact_payloads() {
    let expected_provider = provider();
    let release_policy = policy(&expected_provider, 4096);
    let policy_debug = format!("{release_policy:?}");
    assert!(!policy_debug.contains("provider-a"));
    assert!(!policy_debug.contains("POST"));
    assert!(!policy_debug.contains("form-urlencoded"));
    assert!(policy_debug.contains("[REDACTED]"));

    assert_eq!(
        ClientSecretHttp1ReleasePolicy::new(
            &expected_provider,
            b"",
            b"application/x-www-form-urlencoded",
            4096,
        )
        .expect_err("empty method"),
        ClientSecretHttp1ReleasePolicyError::InvalidMetadata
    );
    assert_eq!(
        ClientSecretHttp1ReleasePolicy::new(
            &expected_provider,
            b"POST",
            b"application/x-www-form-urlencoded",
            MAX_CLIENT_SECRET_HTTP1_REQUEST_BYTES + 1,
        )
        .expect_err("oversized limit"),
        ClientSecretHttp1ReleasePolicyError::InvalidLimits
    );

    let authenticated = authenticated_request(ClientSecretAuthenticationMethod::ClientSecretBasic);
    let encoded_certificate = certificate();
    let certificate = decode_x509_certificate(&encoded_certificate, Asn1Limits::default()).unwrap();
    let destination = destination(&certificate);
    let mut length = [0u8; MAX_HTTP1_FIXED_REQUEST_CONTENT_LENGTH_BYTES];
    let material = bind_client_secret_http1_request_material(
        authenticated.request(),
        body_limits(),
        field_limits(),
        &mut length,
        &mut BindingAudit,
    )
    .unwrap();
    let mut audit = ReleaseAudit::default();
    let released = release_client_secret_http1_request(
        release_policy,
        request_line(b"/oauth2/token"),
        &destination,
        material,
        content_type(b"application/x-www-form-urlencoded"),
        &mut audit,
    )
    .unwrap();
    let debug = format!("{released:?}");
    for forbidden in [
        "provider-a",
        "token.example",
        "refresh-token",
        "secret+value",
        "Basic ",
    ] {
        assert!(!debug.contains(forbidden));
    }
    assert!(debug.contains("[REDACTED]"));
    assert!(audit.events.iter().all(|event| {
        !event.debug.contains("provider-a")
            && !event.debug.contains(&"42".repeat(16))
            && event.debug.contains("[REDACTED]")
    }));
    for error in [
        ClientSecretHttp1ReleaseError::PolicyMismatch,
        ClientSecretHttp1ReleaseError::EndpointMismatch,
        ClientSecretHttp1ReleaseError::EvidenceMismatch,
        ClientSecretHttp1ReleaseError::RequestTooLong,
        ClientSecretHttp1ReleaseError::Allocation,
        ClientSecretHttp1ReleaseError::Audit,
    ] {
        let message = error.to_string();
        assert!(!message.contains("provider-a"));
        assert!(!message.contains("refresh-token"));
        assert!(!message.contains("token.example"));
    }
}
