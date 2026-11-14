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
use http1_fixed_request_body::{
    Http1FixedRequestBodyLimits, MAX_HTTP1_FIXED_REQUEST_CONTENT_LENGTH_BYTES,
};
use http1_request_header_field::Http1RequestHeaderFieldLimits;
use oauth_client_secret_http1_binding::{
    bind_client_secret_http1_request_material, ClientSecretHttp1AuditAction,
    ClientSecretHttp1AuditError, ClientSecretHttp1AuditEvent, ClientSecretHttp1AuditOutcome,
    ClientSecretHttp1AuditSink, ClientSecretHttp1BindingError,
};

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
struct BindingAudit {
    events: Vec<ClientSecretHttp1AuditEvent>,
    fail_at: Option<usize>,
}

impl ClientSecretHttp1AuditSink for BindingAudit {
    fn publish(
        &mut self,
        event: &ClientSecretHttp1AuditEvent,
    ) -> Result<(), ClientSecretHttp1AuditError> {
        if self.fail_at == Some(self.events.len()) {
            return Err(ClientSecretHttp1AuditError);
        }
        self.events.push(event.clone());
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

#[test]
fn basic_constructs_exact_body_length_and_authorization_after_audit() {
    let authenticated = authenticated_request(ClientSecretAuthenticationMethod::ClientSecretBasic);
    let request = authenticated.request();
    let mut length_output = [0xa5; MAX_HTTP1_FIXED_REQUEST_CONTENT_LENGTH_BYTES];
    let mut audit = BindingAudit::default();

    let bound = bind_client_secret_http1_request_material(
        request,
        body_limits(),
        field_limits(),
        &mut length_output,
        &mut audit,
    )
    .expect("bind Basic material");

    assert!(core::ptr::eq(bound.request(), request));
    assert!(core::ptr::eq(
        bound.body().body().body(),
        request.form_body().as_bytes()
    ));
    assert_eq!(
        bound.body().field().value(),
        request.form_body().len().to_string().as_bytes()
    );
    let authorization = bound.authorization_field().expect("Authorization");
    assert_eq!(authorization.name(), b"Authorization");
    assert!(core::ptr::eq(
        authorization.value(),
        request
            .authorization_header()
            .expect("Basic value")
            .as_bytes()
    ));
    assert_eq!(audit.events.len(), 2);
    assert!(audit
        .events
        .iter()
        .all(|event| event.provider() == request.provider() && event.trace() == trace()));
    assert_eq!(
        audit.events[0].action(),
        ClientSecretHttp1AuditAction::BindRequestMaterial
    );
    assert_eq!(
        audit.events[0].outcome(),
        ClientSecretHttp1AuditOutcome::Attempted
    );
    assert_eq!(
        audit.events[1].outcome(),
        ClientSecretHttp1AuditOutcome::Succeeded
    );
}

#[test]
fn post_constructs_exact_body_without_authorization() {
    let authenticated = authenticated_request(ClientSecretAuthenticationMethod::ClientSecretPost);
    let request = authenticated.request();
    let mut length_output = [0xa5; MAX_HTTP1_FIXED_REQUEST_CONTENT_LENGTH_BYTES];
    let mut audit = BindingAudit::default();

    let bound = bind_client_secret_http1_request_material(
        request,
        body_limits(),
        field_limits(),
        &mut length_output,
        &mut audit,
    )
    .expect("bind Post material");

    assert!(bound.authorization_field().is_none());
    assert!(core::ptr::eq(
        bound.body().body().body(),
        request.form_body().as_bytes()
    ));
    assert_eq!(
        audit.events[1].outcome(),
        ClientSecretHttp1AuditOutcome::Succeeded
    );
}

#[test]
fn body_rejection_restores_output_and_is_audited() {
    let authenticated = authenticated_request(ClientSecretAuthenticationMethod::ClientSecretPost);
    let request = authenticated.request();
    let original = [0xa5; MAX_HTTP1_FIXED_REQUEST_CONTENT_LENGTH_BYTES];
    let mut length_output = original;
    let mut audit = BindingAudit::default();

    let error = bind_client_secret_http1_request_material(
        request,
        Http1FixedRequestBodyLimits::new(1).expect("small body limit"),
        field_limits(),
        &mut length_output,
        &mut audit,
    )
    .expect_err("body must exceed limit");

    assert_eq!(error, ClientSecretHttp1BindingError::BodyRejected);
    assert_eq!(length_output, original);
    assert_eq!(audit.events.len(), 2);
    assert_eq!(
        audit.events[1].outcome(),
        ClientSecretHttp1AuditOutcome::Rejected
    );
}

#[test]
fn content_length_rejection_restores_output_and_is_audited() {
    let authenticated = authenticated_request(ClientSecretAuthenticationMethod::ClientSecretPost);
    let request = authenticated.request();
    let original = [0xa5; MAX_HTTP1_FIXED_REQUEST_CONTENT_LENGTH_BYTES];
    let mut length_output = original;
    let limits =
        Http1RequestHeaderFieldLimits::new(64, 8 * 1024, 3).expect("deliberately short line limit");
    let mut audit = BindingAudit::default();

    let error = bind_client_secret_http1_request_material(
        request,
        body_limits(),
        limits,
        &mut length_output,
        &mut audit,
    )
    .expect_err("Content-Length line must exceed limit");

    assert_eq!(error, ClientSecretHttp1BindingError::ContentLengthRejected);
    assert_eq!(length_output, original);
    assert_eq!(
        audit.events[1].outcome(),
        ClientSecretHttp1AuditOutcome::Rejected
    );
}

#[test]
fn authorization_rejection_restores_output_and_is_audited() {
    let authenticated = authenticated_request(ClientSecretAuthenticationMethod::ClientSecretBasic);
    let request = authenticated.request();
    let original = [0xa5; MAX_HTTP1_FIXED_REQUEST_CONTENT_LENGTH_BYTES];
    let mut length_output = original;
    let limits = Http1RequestHeaderFieldLimits::new(64, 8, 8 * 1024)
        .expect("short Authorization value limit");
    let mut audit = BindingAudit::default();

    let error = bind_client_secret_http1_request_material(
        request,
        body_limits(),
        limits,
        &mut length_output,
        &mut audit,
    )
    .expect_err("Authorization must exceed limit");

    assert_eq!(error, ClientSecretHttp1BindingError::AuthorizationRejected);
    assert_eq!(length_output, original);
    assert_eq!(
        audit.events[1].outcome(),
        ClientSecretHttp1AuditOutcome::Rejected
    );
}

#[test]
fn attempted_audit_failure_prevents_construction_and_preserves_output() {
    let authenticated = authenticated_request(ClientSecretAuthenticationMethod::ClientSecretPost);
    let request = authenticated.request();
    let original = [0xa5; MAX_HTTP1_FIXED_REQUEST_CONTENT_LENGTH_BYTES];
    let mut length_output = original;
    let mut audit = BindingAudit {
        events: Vec::new(),
        fail_at: Some(0),
    };

    let error = bind_client_secret_http1_request_material(
        request,
        body_limits(),
        field_limits(),
        &mut length_output,
        &mut audit,
    )
    .expect_err("attempt audit failure");

    assert_eq!(error, ClientSecretHttp1BindingError::Audit);
    assert!(audit.events.is_empty());
    assert_eq!(length_output, original);
}

#[test]
fn terminal_success_audit_failure_restores_output_and_withholds_proof() {
    let authenticated = authenticated_request(ClientSecretAuthenticationMethod::ClientSecretPost);
    let request = authenticated.request();
    let original = [0xa5; MAX_HTTP1_FIXED_REQUEST_CONTENT_LENGTH_BYTES];
    let mut length_output = original;
    let mut audit = BindingAudit {
        events: Vec::new(),
        fail_at: Some(1),
    };

    let error = bind_client_secret_http1_request_material(
        request,
        body_limits(),
        field_limits(),
        &mut length_output,
        &mut audit,
    )
    .expect_err("terminal audit failure");

    assert_eq!(error, ClientSecretHttp1BindingError::Audit);
    assert_eq!(audit.events.len(), 1);
    assert_eq!(
        audit.events[0].outcome(),
        ClientSecretHttp1AuditOutcome::Attempted
    );
    assert_eq!(length_output, original);
}

#[test]
fn terminal_rejection_audit_failure_restores_output_and_hides_reason() {
    let authenticated = authenticated_request(ClientSecretAuthenticationMethod::ClientSecretPost);
    let request = authenticated.request();
    let original = [0xa5; MAX_HTTP1_FIXED_REQUEST_CONTENT_LENGTH_BYTES];
    let mut length_output = original;
    let mut audit = BindingAudit {
        events: Vec::new(),
        fail_at: Some(1),
    };

    let error = bind_client_secret_http1_request_material(
        request,
        Http1FixedRequestBodyLimits::new(1).expect("small body limit"),
        field_limits(),
        &mut length_output,
        &mut audit,
    )
    .expect_err("terminal rejection audit failure");

    assert_eq!(error, ClientSecretHttp1BindingError::Audit);
    assert_eq!(audit.events.len(), 1);
    assert_eq!(length_output, original);
}

#[test]
fn debug_and_errors_do_not_retain_sensitive_payloads() {
    let authenticated = authenticated_request(ClientSecretAuthenticationMethod::ClientSecretBasic);
    let request = authenticated.request();
    let mut length_output = [0xa5; MAX_HTTP1_FIXED_REQUEST_CONTENT_LENGTH_BYTES];
    let mut audit = BindingAudit::default();
    let bound = bind_client_secret_http1_request_material(
        request,
        body_limits(),
        field_limits(),
        &mut length_output,
        &mut audit,
    )
    .expect("bind");

    let debug = format!("{bound:?}");
    assert!(!debug.contains("refresh-token"));
    assert!(!debug.contains("secret+value"));
    assert!(!debug.contains("Basic "));
    assert!(!debug.contains("token.example"));
    assert!(debug.contains("[REDACTED]"));
    for error in [
        ClientSecretHttp1BindingError::BodyRejected,
        ClientSecretHttp1BindingError::ContentLengthRejected,
        ClientSecretHttp1BindingError::AuthorizationRejected,
        ClientSecretHttp1BindingError::Audit,
    ] {
        let message = error.to_string();
        assert!(!message.contains("refresh-token"));
        assert!(!message.contains("secret value"));
        assert!(!message.contains("token.example"));
    }
}
