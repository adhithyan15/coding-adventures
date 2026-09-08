//! Provider-neutral, audit-first OAuth broker orchestration.
//!
//! This crate composes the pure OAuth protocol core with storage-agnostic
//! credential custody. It owns policy and sequencing, but no clock, network,
//! browser, filesystem, vault, or provider-specific authority.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_oauth::{
    decode_token_response, prepare_token_refresh, OAuthAuditSink, OAuthError, OAuthTraceId,
    ProviderConfig, ProviderId, TokenRefreshRequest, TokenResponse, TokenResponseFormat,
    MAX_TOKEN_RESPONSE_BYTES,
};
use coding_adventures_oauth_credential_custody::{
    CredentialAuditSink, CredentialCustody, CredentialKey, CredentialMetadata, CredentialRevision,
    CredentialStore, CustodyError,
};
use coding_adventures_zeroize::Zeroizing;
use std::collections::BTreeMap;
use std::fmt::{self, Debug, Display, Formatter};

/// Largest accepted proactive-refresh window: one day.
pub const MAX_REFRESH_LEAD_SECONDS: u64 = 24 * 60 * 60;

/// Provider data plus broker-owned lifecycle policy.
#[derive(Clone, PartialEq, Eq)]
pub struct BrokerProvider {
    config: ProviderConfig,
    response_format: TokenResponseFormat,
    refresh_lead_seconds: u64,
}

impl BrokerProvider {
    /// Validate one provider registration without acquiring any I/O authority.
    pub fn new(
        config: ProviderConfig,
        response_format: TokenResponseFormat,
        refresh_lead_seconds: u64,
    ) -> Result<Self, BrokerError> {
        if refresh_lead_seconds > MAX_REFRESH_LEAD_SECONDS {
            return Err(BrokerError::InvalidPolicy);
        }
        Ok(Self {
            config,
            response_format,
            refresh_lead_seconds,
        })
    }

    /// Return the provider identifier.
    pub fn provider(&self) -> &ProviderId {
        self.config.provider()
    }

    /// Borrow the validated pure-protocol configuration.
    pub const fn config(&self) -> &ProviderConfig {
        &self.config
    }

    /// Return the response wire format selected by provider data.
    pub const fn response_format(&self) -> TokenResponseFormat {
        self.response_format
    }

    /// Return the proactive-refresh lead time.
    pub const fn refresh_lead_seconds(&self) -> u64 {
        self.refresh_lead_seconds
    }
}

impl Debug for BrokerProvider {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BrokerProvider")
            .field("provider", self.provider())
            .field("response_format", &self.response_format)
            .field("refresh_lead_seconds", &self.refresh_lead_seconds)
            .field("config", &"<redacted>")
            .finish()
    }
}

/// Caller-injected trusted wall clock.
pub trait BrokerClock {
    /// Return current Unix time or a closed failure.
    fn now_unix_seconds(&mut self) -> Result<u64, BrokerClockError>;
}

/// Closed clock failure without platform diagnostics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BrokerClockError;

/// Bounded token-endpoint response owned in wipe-on-drop storage.
pub struct TokenEndpointResponse {
    status: u16,
    body: Zeroizing<Vec<u8>>,
}

impl TokenEndpointResponse {
    /// Construct a syntactically valid HTTP response boundary.
    pub fn new(status: u16, body: Zeroizing<Vec<u8>>) -> Result<Self, BrokerError> {
        if !(100..=599).contains(&status)
            || body.is_empty()
            || body.len() > MAX_TOKEN_RESPONSE_BYTES
        {
            return Err(BrokerError::InvalidTransportResponse);
        }
        Ok(Self { status, body })
    }

    fn into_parts(self) -> (u16, Zeroizing<Vec<u8>>) {
        (self.status, self.body)
    }
}

impl Debug for TokenEndpointResponse {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TokenEndpointResponse")
            .field("status", &self.status)
            .field("body", &"<redacted>")
            .finish()
    }
}

/// Authorized provider-neutral token transport.
pub trait OAuthTokenTransport {
    /// Send one already validated refresh request and return bounded owned bytes.
    fn send_refresh(
        &mut self,
        request: &TokenRefreshRequest,
    ) -> Result<TokenEndpointResponse, TokenTransportError>;
}

/// Closed transport failure without endpoint or provider response text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TokenTransportError;

/// Broker boundary recorded durably around configuration and external effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerAuditAction {
    /// Register or idempotently confirm one provider definition.
    ProviderRegister,
    /// Store an initial credential response under an opaque account key.
    CredentialCreate,
    /// Refresh and atomically rotate one account credential.
    Refresh,
    /// Send one request through the injected token transport.
    TokenTransport,
}

/// Closed privacy-safe broker result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerAuditOutcome {
    /// Durable intent before an effect or orchestrated access.
    Attempted,
    /// Durable success before the broker releases its result.
    Succeeded,
    /// Closed failure classification.
    Failed(BrokerFailureClass),
}

/// Closed broker failure class safe for durable audit storage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerFailureClass {
    /// Provider or broker policy input was invalid.
    InvalidInput,
    /// No provider registration matched the requested key.
    ProviderNotRegistered,
    /// A provider identifier was redefined with different data.
    ProviderConflict,
    /// Clock access or time arithmetic failed.
    Clock,
    /// Credential custody failed.
    Custody,
    /// The injected token transport failed.
    Transport,
    /// OAuth request preparation or response decoding failed.
    Protocol,
}

/// Privacy-safe broker event containing no endpoint, scope, token, or body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerAuditEvent {
    provider: ProviderId,
    trace: OAuthTraceId,
    action: BrokerAuditAction,
    outcome: BrokerAuditOutcome,
}

impl BrokerAuditEvent {
    /// Return the provider identity.
    pub const fn provider(&self) -> &ProviderId {
        &self.provider
    }

    /// Return the caller-owned operation trace.
    pub const fn trace(&self) -> OAuthTraceId {
        self.trace
    }

    /// Return the stable broker action.
    pub const fn action(&self) -> BrokerAuditAction {
        self.action
    }

    /// Return the closed outcome.
    pub const fn outcome(&self) -> BrokerAuditOutcome {
        self.outcome
    }
}

/// Durable publication for broker-owned boundaries.
pub trait BrokerAuditSink {
    /// Persist one privacy-safe broker event or fail closed.
    fn publish(&mut self, event: &BrokerAuditEvent) -> Result<(), BrokerAuditError>;
}

/// Audit sink capable of recording every layer composed by the broker.
pub trait OAuthBrokerAuditSink: BrokerAuditSink + OAuthAuditSink + CredentialAuditSink {}

impl<T> OAuthBrokerAuditSink for T where T: BrokerAuditSink + OAuthAuditSink + CredentialAuditSink {}

/// Closed broker-audit publication failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BrokerAuditError;

/// Closed broker error with no provider-controlled or secret-bearing text.
#[derive(Clone, PartialEq, Eq)]
pub enum BrokerError {
    /// Broker lifecycle policy was outside its accepted bound.
    InvalidPolicy,
    /// A transport returned an impossible HTTP status.
    InvalidTransportResponse,
    /// The provider has not been registered.
    ProviderNotRegistered,
    /// Existing provider data differs from a repeated registration.
    ProviderConflict,
    /// The credential key or response belongs to another provider or trace.
    BindingMismatch,
    /// Clock access or expiry arithmetic failed.
    Clock,
    /// Credential custody failed with a closed class.
    Custody(CustodyError),
    /// OAuth protocol preparation or decoding failed with a closed class.
    Protocol(OAuthError),
    /// The injected token transport failed.
    Transport,
    /// Durable audit publication failed and the result was withheld.
    Audit,
}

impl BrokerError {
    fn failure_class(&self) -> Option<BrokerFailureClass> {
        match self {
            Self::InvalidPolicy | Self::InvalidTransportResponse | Self::BindingMismatch => {
                Some(BrokerFailureClass::InvalidInput)
            }
            Self::ProviderNotRegistered => Some(BrokerFailureClass::ProviderNotRegistered),
            Self::ProviderConflict => Some(BrokerFailureClass::ProviderConflict),
            Self::Clock => Some(BrokerFailureClass::Clock),
            Self::Custody(CustodyError::Audit)
            | Self::Protocol(OAuthError::Audit)
            | Self::Audit => None,
            Self::Custody(_) => Some(BrokerFailureClass::Custody),
            Self::Protocol(_) => Some(BrokerFailureClass::Protocol),
            Self::Transport => Some(BrokerFailureClass::Transport),
        }
    }
}

impl Debug for BrokerError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidPolicy => "InvalidPolicy",
            Self::InvalidTransportResponse => "InvalidTransportResponse",
            Self::ProviderNotRegistered => "ProviderNotRegistered",
            Self::ProviderConflict => "ProviderConflict",
            Self::BindingMismatch => "BindingMismatch",
            Self::Clock => "Clock",
            Self::Custody(_) => "Custody(<redacted>)",
            Self::Protocol(_) => "Protocol(<redacted>)",
            Self::Transport => "Transport",
            Self::Audit => "Audit",
        })
    }
}

impl Display for BrokerError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("oauth broker: ")?;
        Debug::fmt(self, formatter)
    }
}

impl std::error::Error for BrokerError {}

/// Provider-neutral broker over one injected credential store.
pub struct OAuthBroker<S: CredentialStore> {
    providers: BTreeMap<ProviderId, BrokerProvider>,
    custody: CredentialCustody<S>,
}

impl<S: CredentialStore> OAuthBroker<S> {
    /// Construct an empty registry over an existing custody boundary.
    pub const fn new(custody: CredentialCustody<S>) -> Self {
        Self {
            providers: BTreeMap::new(),
            custody,
        }
    }

    /// Return the number of registered provider configurations.
    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }

    /// Audit and register one provider; an exact repeat is idempotent.
    pub fn register_provider<A: BrokerAuditSink>(
        &mut self,
        provider: BrokerProvider,
        trace: OAuthTraceId,
        audit: &mut A,
    ) -> Result<(), BrokerError> {
        let provider_id = provider.provider().clone();
        publish_broker(
            audit,
            &provider_id,
            trace,
            BrokerAuditAction::ProviderRegister,
            BrokerAuditOutcome::Attempted,
        )?;
        let result = match self.providers.get(&provider_id) {
            Some(existing) if existing == &provider => Ok(()),
            Some(_) => Err(BrokerError::ProviderConflict),
            None => {
                self.providers.insert(provider_id.clone(), provider);
                Ok(())
            }
        };
        finish_broker(
            audit,
            &provider_id,
            trace,
            BrokerAuditAction::ProviderRegister,
            result,
        )
    }

    /// Store one decoded initial token response under an opaque account key.
    ///
    /// The response must already have passed the OAuth decoder's audit gate.
    /// Credential bytes cross its second release gate here, then custody creates
    /// the record only if absent.
    pub fn store_initial_response<C: BrokerClock, A: OAuthBrokerAuditSink>(
        &self,
        key: &CredentialKey,
        response: TokenResponse,
        trace: OAuthTraceId,
        clock: &mut C,
        audit: &mut A,
    ) -> Result<CredentialRevision, BrokerError> {
        publish_broker(
            audit,
            key.provider(),
            trace,
            BrokerAuditAction::CredentialCreate,
            BrokerAuditOutcome::Attempted,
        )?;
        let result = (|| {
            self.registered_provider(key.provider())?;
            validate_response_binding(key, &response, trace)?;
            let now = clock.now_unix_seconds().map_err(|_| BrokerError::Clock)?;
            let metadata = response_metadata(&response, now, &[])?;
            let credentials = response
                .release_credentials()
                .publish_then_release(audit)
                .map_err(map_oauth_error)?;
            self.custody
                .create(key, credentials, metadata, trace, audit)
                .map_err(map_custody_error)
        })();
        finish_broker(
            audit,
            key.provider(),
            trace,
            BrokerAuditAction::CredentialCreate,
            result,
        )
    }

    /// Release a usable access token to one closure, refreshing first when due.
    pub fn with_access_token<R, C, T, A>(
        &self,
        key: &CredentialKey,
        trace: OAuthTraceId,
        clock: &mut C,
        transport: &mut T,
        audit: &mut A,
        use_token: impl FnOnce(&str) -> R,
    ) -> Result<R, BrokerError>
    where
        C: BrokerClock,
        T: OAuthTokenTransport,
        A: OAuthBrokerAuditSink,
    {
        let provider = self.registered_provider(key.provider())?.clone();
        let metadata = self
            .custody
            .with_metadata(key, trace, audit, Clone::clone)
            .map_err(map_custody_error)?;
        if let Some(expires_at) = metadata.expires_at_unix_seconds() {
            let now = clock.now_unix_seconds().map_err(|_| BrokerError::Clock)?;
            let refresh_at = now
                .checked_add(provider.refresh_lead_seconds)
                .ok_or(BrokerError::Clock)?;
            if expires_at <= refresh_at {
                self.force_refresh(key, trace, clock, transport, audit)?;
            }
        }
        self.custody
            .with_access_token(key, trace, audit, use_token)
            .map_err(map_custody_error)
    }

    /// Refresh one credential now and atomically retain or rotate its refresh token.
    pub fn force_refresh<C, T, A>(
        &self,
        key: &CredentialKey,
        trace: OAuthTraceId,
        clock: &mut C,
        transport: &mut T,
        audit: &mut A,
    ) -> Result<CredentialRevision, BrokerError>
    where
        C: BrokerClock,
        T: OAuthTokenTransport,
        A: OAuthBrokerAuditSink,
    {
        publish_broker(
            audit,
            key.provider(),
            trace,
            BrokerAuditAction::Refresh,
            BrokerAuditOutcome::Attempted,
        )?;
        let result = self.force_refresh_inner(key, trace, clock, transport, audit);
        finish_broker(
            audit,
            key.provider(),
            trace,
            BrokerAuditAction::Refresh,
            result,
        )
    }

    fn force_refresh_inner<C, T, A>(
        &self,
        key: &CredentialKey,
        trace: OAuthTraceId,
        clock: &mut C,
        transport: &mut T,
        audit: &mut A,
    ) -> Result<CredentialRevision, BrokerError>
    where
        C: BrokerClock,
        T: OAuthTokenTransport,
        A: OAuthBrokerAuditSink,
    {
        let provider = self.registered_provider(key.provider())?.clone();
        let material = self
            .custody
            .with_refresh_material(key, trace, audit, |token, revision, metadata| {
                RefreshMaterial {
                    token: Zeroizing::new(token.to_owned()),
                    revision,
                    scopes: metadata.scopes().to_vec(),
                }
            })
            .map_err(map_custody_error)?;
        let requested_scopes: Vec<&str> = material.scopes.iter().map(String::as_str).collect();
        let request =
            prepare_token_refresh(provider.config(), material.token, &requested_scopes, trace)
                .publish_then_release(audit)
                .map_err(map_oauth_error)?;
        let context = request.response_context();
        let wire_response =
            send_refresh_audited(transport, &request, key.provider(), trace, audit)?;
        let (status, body) = wire_response.into_parts();
        let response = decode_token_response(context, status, provider.response_format(), body)
            .publish_then_release(audit)
            .map_err(map_oauth_error)?;
        validate_response_binding(key, &response, trace)?;
        let now = clock.now_unix_seconds().map_err(|_| BrokerError::Clock)?;
        let metadata = response_metadata(&response, now, &material.scopes)?;
        let credentials = response
            .release_credentials()
            .publish_then_release(audit)
            .map_err(map_oauth_error)?;
        self.custody
            .rotate(key, material.revision, credentials, metadata, trace, audit)
            .map_err(map_custody_error)
    }

    fn registered_provider(&self, provider: &ProviderId) -> Result<&BrokerProvider, BrokerError> {
        self.providers
            .get(provider)
            .ok_or(BrokerError::ProviderNotRegistered)
    }
}

impl<S: CredentialStore> Debug for OAuthBroker<S> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OAuthBroker")
            .field("provider_count", &self.providers.len())
            .field("custody", &"<redacted>")
            .finish()
    }
}

struct RefreshMaterial {
    token: Zeroizing<String>,
    revision: CredentialRevision,
    scopes: Vec<String>,
}

fn response_metadata(
    response: &TokenResponse,
    now: u64,
    fallback_scopes: &[String],
) -> Result<CredentialMetadata, BrokerError> {
    let expires_at = response
        .expires_in_seconds()
        .map(|seconds| now.checked_add(seconds).ok_or(BrokerError::Clock))
        .transpose()?;
    let scopes = if response.scopes().is_empty() {
        fallback_scopes.to_vec()
    } else {
        response.scopes().to_vec()
    };
    CredentialMetadata::new(response.token_type().to_owned(), expires_at, scopes)
        .map_err(map_custody_error)
}

fn validate_response_binding(
    key: &CredentialKey,
    response: &TokenResponse,
    trace: OAuthTraceId,
) -> Result<(), BrokerError> {
    if response.provider() != key.provider() || response.trace() != trace {
        return Err(BrokerError::BindingMismatch);
    }
    Ok(())
}

fn send_refresh_audited<T: OAuthTokenTransport, A: BrokerAuditSink>(
    transport: &mut T,
    request: &TokenRefreshRequest,
    provider: &ProviderId,
    trace: OAuthTraceId,
    audit: &mut A,
) -> Result<TokenEndpointResponse, BrokerError> {
    publish_broker(
        audit,
        provider,
        trace,
        BrokerAuditAction::TokenTransport,
        BrokerAuditOutcome::Attempted,
    )?;
    let result = transport
        .send_refresh(request)
        .map_err(|_| BrokerError::Transport);
    finish_broker(
        audit,
        provider,
        trace,
        BrokerAuditAction::TokenTransport,
        result,
    )
}

fn publish_broker<A: BrokerAuditSink>(
    audit: &mut A,
    provider: &ProviderId,
    trace: OAuthTraceId,
    action: BrokerAuditAction,
    outcome: BrokerAuditOutcome,
) -> Result<(), BrokerError> {
    audit
        .publish(&BrokerAuditEvent {
            provider: provider.clone(),
            trace,
            action,
            outcome,
        })
        .map_err(|_| BrokerError::Audit)
}

fn finish_broker<T, A: BrokerAuditSink>(
    audit: &mut A,
    provider: &ProviderId,
    trace: OAuthTraceId,
    action: BrokerAuditAction,
    result: Result<T, BrokerError>,
) -> Result<T, BrokerError> {
    match result {
        Ok(value) => {
            publish_broker(
                audit,
                provider,
                trace,
                action,
                BrokerAuditOutcome::Succeeded,
            )?;
            Ok(value)
        }
        Err(error) => {
            if let Some(failure) = error.failure_class() {
                publish_broker(
                    audit,
                    provider,
                    trace,
                    action,
                    BrokerAuditOutcome::Failed(failure),
                )?;
            }
            Err(error)
        }
    }
}

fn map_custody_error(error: CustodyError) -> BrokerError {
    if error == CustodyError::Audit {
        BrokerError::Audit
    } else {
        BrokerError::Custody(error)
    }
}

fn map_oauth_error(error: OAuthError) -> BrokerError {
    if error == OAuthError::Audit {
        BrokerError::Audit
    } else {
        BrokerError::Protocol(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_oauth::{
        prepare_token_refresh, OAuthAuditError, OAuthAuditEvent, OAuthAuditOutcome,
    };
    use coding_adventures_oauth_credential_custody::{
        AccountId, CredentialAuditAction, CredentialAuditError, CredentialAuditEvent,
        CredentialAuditOutcome, InMemoryCredentialStore,
    };
    use std::collections::VecDeque;

    #[derive(Default)]
    struct RecordingAudit {
        broker: Vec<BrokerAuditEvent>,
        oauth: Vec<OAuthAuditEvent>,
        custody: Vec<CredentialAuditEvent>,
        broker_calls: usize,
        fail_broker_on: Option<usize>,
    }

    impl BrokerAuditSink for RecordingAudit {
        fn publish(&mut self, event: &BrokerAuditEvent) -> Result<(), BrokerAuditError> {
            self.broker_calls += 1;
            if self.fail_broker_on == Some(self.broker_calls) {
                return Err(BrokerAuditError);
            }
            self.broker.push(event.clone());
            Ok(())
        }
    }

    impl OAuthAuditSink for RecordingAudit {
        fn publish(&mut self, event: &OAuthAuditEvent) -> Result<(), OAuthAuditError> {
            self.oauth.push(event.clone());
            Ok(())
        }
    }

    impl CredentialAuditSink for RecordingAudit {
        fn publish(&mut self, event: &CredentialAuditEvent) -> Result<(), CredentialAuditError> {
            self.custody.push(event.clone());
            Ok(())
        }
    }

    struct FixedClock(u64);

    impl BrokerClock for FixedClock {
        fn now_unix_seconds(&mut self) -> Result<u64, BrokerClockError> {
            Ok(self.0)
        }
    }

    struct MockTransport {
        responses: VecDeque<TokenEndpointResponse>,
        expected_refresh: &'static str,
        calls: usize,
    }

    impl MockTransport {
        fn new(expected_refresh: &'static str, bodies: &[&str]) -> Self {
            Self {
                responses: bodies
                    .iter()
                    .map(|body| {
                        TokenEndpointResponse::new(200, Zeroizing::new(body.as_bytes().to_vec()))
                            .unwrap()
                    })
                    .collect(),
                expected_refresh,
                calls: 0,
            }
        }
    }

    impl OAuthTokenTransport for MockTransport {
        fn send_refresh(
            &mut self,
            request: &TokenRefreshRequest,
        ) -> Result<TokenEndpointResponse, TokenTransportError> {
            self.calls += 1;
            assert!(request
                .form_body()
                .contains(&format!("refresh_token={}", self.expected_refresh)));
            self.responses.pop_front().ok_or(TokenTransportError)
        }
    }

    fn trace(byte: u8) -> OAuthTraceId {
        OAuthTraceId::new([byte; 16])
    }

    fn config(name: &str) -> ProviderConfig {
        ProviderConfig::new(
            ProviderId::new(name).unwrap(),
            format!("https://auth.{name}.example/authorize"),
            format!("https://token.{name}.example/token"),
            format!("{name}-public-client"),
            "http://127.0.0.1:49152/callback",
        )
        .unwrap()
        .with_distinct_redirect_uri()
    }

    fn policy(name: &str, lead: u64) -> BrokerProvider {
        BrokerProvider::new(config(name), TokenResponseFormat::Json, lead).unwrap()
    }

    fn key(name: &str) -> CredentialKey {
        CredentialKey::new(ProviderId::new(name).unwrap(), AccountId::new([0x31; 32]))
    }

    fn decoded_refresh_response(
        config: &ProviderConfig,
        trace: OAuthTraceId,
        body: &str,
        audit: &mut impl OAuthAuditSink,
    ) -> TokenResponse {
        let request = prepare_token_refresh(
            config,
            Zeroizing::new("seed-refresh".to_owned()),
            &[],
            trace,
        )
        .publish_then_release(audit)
        .unwrap();
        decode_token_response(
            request.response_context(),
            200,
            TokenResponseFormat::Json,
            Zeroizing::new(body.as_bytes().to_vec()),
        )
        .publish_then_release(audit)
        .unwrap()
    }

    fn broker_with_credential(
        expiry: u64,
    ) -> (
        OAuthBroker<InMemoryCredentialStore>,
        RecordingAudit,
        CredentialKey,
    ) {
        let config = config("fixture");
        let key = key("fixture");
        let mut audit = RecordingAudit::default();
        let response = decoded_refresh_response(
            &config,
            trace(1),
            r#"{"access_token":"access-one","refresh_token":"refresh-one","token_type":"Bearer","expires_in":3600,"scope":"mail.read profile"}"#,
            &mut audit,
        );
        let credentials = response
            .release_credentials()
            .publish_then_release(&mut audit)
            .unwrap();
        let custody = CredentialCustody::new(InMemoryCredentialStore::new());
        custody
            .create(
                &key,
                credentials,
                CredentialMetadata::new(
                    "Bearer",
                    Some(expiry),
                    vec!["mail.read".to_owned(), "profile".to_owned()],
                )
                .unwrap(),
                trace(1),
                &mut audit,
            )
            .unwrap();
        let mut broker = OAuthBroker::new(custody);
        broker
            .register_provider(
                BrokerProvider::new(config, TokenResponseFormat::Json, 300).unwrap(),
                trace(1),
                &mut audit,
            )
            .unwrap();
        (broker, audit, key)
    }

    #[test]
    fn registry_is_data_driven_idempotent_and_conflict_closed() {
        let custody = CredentialCustody::new(InMemoryCredentialStore::new());
        let mut broker = OAuthBroker::new(custody);
        let mut audit = RecordingAudit::default();
        for index in 0..128 {
            let name = format!("provider-{index}");
            broker
                .register_provider(policy(&name, 300), trace(2), &mut audit)
                .unwrap();
        }
        assert_eq!(broker.provider_count(), 128);
        broker
            .register_provider(policy("provider-7", 300), trace(2), &mut audit)
            .unwrap();
        assert_eq!(broker.provider_count(), 128);
        assert_eq!(
            broker.register_provider(policy("provider-7", 301), trace(2), &mut audit),
            Err(BrokerError::ProviderConflict)
        );
        assert_eq!(
            audit.broker.last().unwrap().outcome(),
            BrokerAuditOutcome::Failed(BrokerFailureClass::ProviderConflict)
        );
    }

    #[test]
    fn fresh_access_uses_no_transport_and_is_audited_before_disclosure() {
        let (broker, mut audit, key) = broker_with_credential(2_000);
        let mut clock = FixedClock(1_000);
        let mut transport = MockTransport::new("refresh-one", &[]);
        let observed = broker
            .with_access_token(
                &key,
                trace(3),
                &mut clock,
                &mut transport,
                &mut audit,
                str::to_owned,
            )
            .unwrap();
        assert_eq!(observed, "access-one");
        assert_eq!(transport.calls, 0);
        let actions: Vec<_> = audit
            .custody
            .iter()
            .map(CredentialAuditEvent::action)
            .collect();
        assert!(actions.ends_with(&[
            CredentialAuditAction::Metadata,
            CredentialAuditAction::Metadata,
            CredentialAuditAction::AccessToken,
            CredentialAuditAction::AccessToken,
        ]));
        assert_eq!(
            audit.custody.last().unwrap().outcome(),
            CredentialAuditOutcome::Succeeded
        );
    }

    #[test]
    fn expiring_access_refreshes_and_retains_the_rotating_credential_atomically() {
        let (broker, mut audit, key) = broker_with_credential(1_100);
        let mut clock = FixedClock(1_000);
        let mut transport = MockTransport::new(
            "refresh-one",
            &[
                r#"{"access_token":"access-two","token_type":"Bearer","expires_in":3600}"#,
                r#"{"access_token":"access-three","token_type":"Bearer","expires_in":3600}"#,
            ],
        );
        let observed = broker
            .with_access_token(
                &key,
                trace(4),
                &mut clock,
                &mut transport,
                &mut audit,
                str::to_owned,
            )
            .unwrap();
        assert_eq!(observed, "access-two");
        broker
            .force_refresh(&key, trace(5), &mut clock, &mut transport, &mut audit)
            .unwrap();
        let latest = broker
            .with_access_token(
                &key,
                trace(6),
                &mut clock,
                &mut transport,
                &mut audit,
                str::to_owned,
            )
            .unwrap();
        assert_eq!(latest, "access-three");
        assert_eq!(transport.calls, 2);
        assert!(audit.broker.iter().any(|event| {
            event.action() == BrokerAuditAction::TokenTransport
                && event.outcome() == BrokerAuditOutcome::Attempted
        }));
        assert!(audit
            .oauth
            .iter()
            .any(|event| { event.outcome() == OAuthAuditOutcome::Succeeded }));
    }

    #[test]
    fn transport_audit_failure_prevents_the_external_effect() {
        let (broker, _, key) = broker_with_credential(1_000);
        let mut audit = RecordingAudit {
            fail_broker_on: Some(2),
            ..RecordingAudit::default()
        };
        let mut clock = FixedClock(1_000);
        let mut transport = MockTransport::new("refresh-one", &[]);
        assert_eq!(
            broker.force_refresh(&key, trace(7), &mut clock, &mut transport, &mut audit,),
            Err(BrokerError::Audit)
        );
        assert_eq!(transport.calls, 0);
    }

    #[test]
    fn initial_response_is_bound_to_provider_and_trace() {
        let custody = CredentialCustody::new(InMemoryCredentialStore::new());
        let mut broker = OAuthBroker::new(custody);
        let mut audit = RecordingAudit::default();
        broker
            .register_provider(policy("fixture", 300), trace(8), &mut audit)
            .unwrap();
        let other = config("other");
        let response = decoded_refresh_response(
            &other,
            trace(8),
            r#"{"access_token":"do-not-store","refresh_token":"refresh","token_type":"Bearer"}"#,
            &mut audit,
        );
        assert_eq!(
            broker.store_initial_response(
                &key("fixture"),
                response,
                trace(8),
                &mut FixedClock(1_000),
                &mut audit,
            ),
            Err(BrokerError::BindingMismatch)
        );
        assert_eq!(
            audit.broker.last().unwrap().outcome(),
            BrokerAuditOutcome::Failed(BrokerFailureClass::InvalidInput)
        );
    }

    #[test]
    fn invalid_policy_time_overflow_and_debug_surfaces_fail_closed() {
        assert_eq!(
            BrokerProvider::new(
                config("fixture"),
                TokenResponseFormat::Json,
                MAX_REFRESH_LEAD_SECONDS + 1,
            ),
            Err(BrokerError::InvalidPolicy)
        );
        assert!(matches!(
            TokenEndpointResponse::new(0, Zeroizing::new(b"secret".to_vec())),
            Err(BrokerError::InvalidTransportResponse)
        ));
        assert!(matches!(
            TokenEndpointResponse::new(
                200,
                Zeroizing::new(vec![b'x'; MAX_TOKEN_RESPONSE_BYTES + 1]),
            ),
            Err(BrokerError::InvalidTransportResponse)
        ));
        let (broker, mut audit, key) = broker_with_credential(u64::MAX);
        let mut transport = MockTransport::new("refresh-one", &[]);
        assert_eq!(
            broker.with_access_token(
                &key,
                trace(9),
                &mut FixedClock(u64::MAX),
                &mut transport,
                &mut audit,
                |_| (),
            ),
            Err(BrokerError::Clock)
        );
        let debug = format!(
            "{broker:?} {:?} {:?}",
            BrokerError::Transport,
            TokenEndpointResponse::new(200, Zeroizing::new(b"top-secret".to_vec())).unwrap()
        );
        assert!(!debug.contains("top-secret"));
        assert!(debug.contains("<redacted>"));
    }
}
