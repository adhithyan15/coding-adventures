//! Provider-neutral, audit-first OAuth credential, refresh, and device-poll orchestration.
//!
//! This crate composes the pure OAuth protocol core with storage-agnostic
//! credential custody. It owns policy and sequencing, but no clock, network,
//! browser, filesystem, vault, or provider-specific authority.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_oauth::{
    decode_device_authorization_response, decode_device_token_poll_response, decode_token_response,
    prepare_device_authorization, prepare_device_token_poll, prepare_token_refresh,
    DeviceAuthorization, DeviceAuthorizationProfile, DeviceAuthorizationRequest, DevicePollResult,
    DevicePollingSession, DeviceTokenPollRequest, OAuthAuditSink, OAuthError, OAuthTraceId,
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

mod provider_data;

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

/// Caller-injected authority for reading one static public-provider profile.
///
/// Implementations may read a file, vault object, embedded resource, or other
/// host-owned source. The broker publishes its provider- and trace-bound audit
/// intent before invoking this method, and the returned bytes remain
/// wipe-on-drop while they are decoded.
pub trait PublicProviderDataSource {
    /// Read the profile selected by the exact requested provider identity.
    fn load_public_provider_data(
        &mut self,
        provider: &ProviderId,
    ) -> Result<Zeroizing<Vec<u8>>, ProviderDataSourceError>;
}

/// Closed provider-data source failure without path or backend diagnostics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProviderDataSourceError;

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

/// Bounded device-authorization endpoint response owned in wipe-on-drop storage.
pub struct DeviceAuthorizationEndpointResponse {
    status: u16,
    content_type: String,
    body: Zeroizing<Vec<u8>>,
}

impl DeviceAuthorizationEndpointResponse {
    /// Construct a syntactically valid HTTP response boundary.
    pub fn new(
        status: u16,
        content_type: impl Into<String>,
        body: Zeroizing<Vec<u8>>,
    ) -> Result<Self, BrokerError> {
        let content_type = content_type.into();
        if !(100..=599).contains(&status)
            || content_type.is_empty()
            || content_type.len() > 256
            || !content_type.is_ascii()
            || body.is_empty()
            || body.len() > MAX_TOKEN_RESPONSE_BYTES
        {
            return Err(BrokerError::InvalidTransportResponse);
        }
        Ok(Self {
            status,
            content_type,
            body,
        })
    }

    fn into_parts(self) -> (u16, String, Zeroizing<Vec<u8>>) {
        (self.status, self.content_type, self.body)
    }
}

impl Debug for DeviceAuthorizationEndpointResponse {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DeviceAuthorizationEndpointResponse")
            .field("status", &self.status)
            .field("content_type", &"<redacted>")
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

/// Authorized provider-neutral transport for one RFC 8628 token poll.
pub trait OAuthDeviceTokenTransport {
    /// Send one already validated device-code request and return bounded owned bytes.
    fn send_device_poll(
        &mut self,
        request: &DeviceTokenPollRequest,
    ) -> Result<TokenEndpointResponse, TokenTransportError>;
}

/// Authorized provider-neutral transport for RFC 8628 initiation.
pub trait OAuthDeviceAuthorizationTransport {
    /// Send one already validated device-authorization request.
    fn send_device_authorization(
        &mut self,
        request: &DeviceAuthorizationRequest,
    ) -> Result<DeviceAuthorizationEndpointResponse, TokenTransportError>;
}

/// One broker-mediated device poll result.
///
/// A transient transport failure returns the opaque session so the caller can
/// schedule another attempt without exposing or cloning the device code.
pub enum BrokerDevicePollResult {
    /// The provider returned a response classified by the OAuth protocol core.
    Response(DevicePollResult),
    /// The external transport failed before any provider response existed.
    TransportFailed(DevicePollingSession),
}

impl BrokerDevicePollResult {
    /// Return the minimum caller-owned delay before retrying, when applicable.
    pub const fn retry_after_seconds(&self) -> Option<u64> {
        match self {
            Self::Response(result) => result.retry_after_seconds(),
            Self::TransportFailed(session) => Some(session.interval_seconds()),
        }
    }
}

impl Debug for BrokerDevicePollResult {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Response(result) => formatter.debug_tuple("Response").field(result).finish(),
            Self::TransportFailed(session) => formatter
                .debug_tuple("TransportFailed")
                .field(session)
                .finish(),
        }
    }
}

/// Closed transport failure without endpoint or provider response text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TokenTransportError;

/// Broker boundary recorded durably around configuration and external effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerAuditAction {
    /// Read and decode one provider-bound static public-provider profile.
    ProviderDataLoad,
    /// Register or idempotently confirm one provider definition.
    ProviderRegister,
    /// Store an initial credential response under an opaque account key.
    CredentialCreate,
    /// Refresh and atomically rotate one account credential.
    Refresh,
    /// Send one request through the injected token transport.
    TokenTransport,
    /// Classify one externally scheduled RFC 8628 device-token poll.
    DevicePoll,
    /// Send one device-token request through the injected transport.
    DeviceTokenTransport,
    /// Initiate one RFC 8628 device authorization operation.
    DeviceAuthorization,
    /// Send one device-authorization request through the injected transport.
    DeviceAuthorizationTransport,
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
    /// The injected provider-data source failed.
    ProviderData,
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
    /// Static public-provider data was malformed, unknown, or failed validation.
    InvalidProviderData,
    /// The injected provider-data source failed without releasing diagnostics.
    ProviderDataSource,
    /// A transport returned an impossible bounded HTTP response.
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
            Self::InvalidPolicy
            | Self::InvalidProviderData
            | Self::InvalidTransportResponse
            | Self::BindingMismatch => Some(BrokerFailureClass::InvalidInput),
            Self::ProviderDataSource => Some(BrokerFailureClass::ProviderData),
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
            Self::InvalidProviderData => "InvalidProviderData",
            Self::ProviderDataSource => "ProviderDataSource",
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

    /// Audit, load, exactly bind, decode, and register one public-provider profile.
    ///
    /// The requested provider identity selects the caller-owned source before
    /// any read occurs. A profile naming another provider is rejected before
    /// registry mutation. Deployment-specific client and redirect values stay
    /// outside the static profile, and no concrete filesystem or vault
    /// authority is acquired by the broker.
    pub fn load_and_register_public_provider<D, A>(
        &mut self,
        requested_provider: &ProviderId,
        client_id: impl Into<String>,
        redirect_uri: impl Into<String>,
        trace: OAuthTraceId,
        source: &mut D,
        audit: &mut A,
    ) -> Result<(), BrokerError>
    where
        D: PublicProviderDataSource,
        A: BrokerAuditSink,
    {
        publish_broker(
            audit,
            requested_provider,
            trace,
            BrokerAuditAction::ProviderDataLoad,
            BrokerAuditOutcome::Attempted,
        )?;
        let result = source
            .load_public_provider_data(requested_provider)
            .map_err(|_| BrokerError::ProviderDataSource)
            .and_then(|body| {
                BrokerProvider::from_public_provider_data(body, client_id, redirect_uri)
            })
            .and_then(|provider| {
                if provider.provider() == requested_provider {
                    Ok(provider)
                } else {
                    Err(BrokerError::BindingMismatch)
                }
            });
        let provider = finish_broker(
            audit,
            requested_provider,
            trace,
            BrokerAuditAction::ProviderDataLoad,
            result,
        )?;
        self.register_provider(provider, trace, audit)
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
                let config = provider.config();
                let redirect_conflict = self.providers.values().any(|existing| {
                    existing.config().redirect_uri() == config.redirect_uri()
                        && (existing.config().uses_distinct_redirect_uri()
                            || config.uses_distinct_redirect_uri())
                });
                if redirect_conflict {
                    Err(BrokerError::ProviderConflict)
                } else {
                    self.providers.insert(provider_id.clone(), provider);
                    Ok(())
                }
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

    /// Execute one externally scheduled RFC 8628 poll without sleeping or reading a clock.
    ///
    /// The opaque session must exactly match a registered provider, client, and
    /// token endpoint before request preparation or transport. A transport
    /// failure returns the session for a caller-scheduled retry; all response
    /// classification remains in the pure OAuth protocol core.
    pub fn poll_device_once<T, A>(
        &self,
        session: DevicePollingSession,
        transport: &mut T,
        audit: &mut A,
    ) -> Result<BrokerDevicePollResult, BrokerError>
    where
        T: OAuthDeviceTokenTransport,
        A: OAuthBrokerAuditSink,
    {
        let provider = session.provider().clone();
        let trace = session.trace();
        publish_broker(
            audit,
            &provider,
            trace,
            BrokerAuditAction::DevicePoll,
            BrokerAuditOutcome::Attempted,
        )?;
        let result = self.poll_device_once_inner(session, transport, audit);
        finish_broker(
            audit,
            &provider,
            trace,
            BrokerAuditAction::DevicePoll,
            result,
        )
    }

    /// Initiate one RFC 8628 device flow through an injected transport.
    ///
    /// The metadata-derived profile must exactly match a registered provider,
    /// client, and token endpoint before protocol preparation or transport.
    /// The caller retains UI, clock, waiting, and full-loop authority.
    pub fn begin_device_authorization<T, A>(
        &self,
        profile: &DeviceAuthorizationProfile,
        requested_scopes: &[&str],
        trace: OAuthTraceId,
        transport: &mut T,
        audit: &mut A,
    ) -> Result<DeviceAuthorization, BrokerError>
    where
        T: OAuthDeviceAuthorizationTransport,
        A: OAuthBrokerAuditSink,
    {
        let provider = profile.provider().clone();
        publish_broker(
            audit,
            &provider,
            trace,
            BrokerAuditAction::DeviceAuthorization,
            BrokerAuditOutcome::Attempted,
        )?;
        let result = (|| {
            let registered = self.registered_provider(&provider)?;
            if !profile.is_bound_to(registered.config()) {
                return Err(BrokerError::BindingMismatch);
            }
            let request = prepare_device_authorization(profile, requested_scopes, trace)
                .publish_then_release(audit)
                .map_err(map_oauth_error)?;
            let context = request.response_context();
            let response = send_device_authorization_audited(transport, &request, audit)?;
            let (status, content_type, body) = response.into_parts();
            decode_device_authorization_response(context, status, &content_type, body)
                .publish_then_release(audit)
                .map_err(map_oauth_error)
        })();
        finish_broker(
            audit,
            &provider,
            trace,
            BrokerAuditAction::DeviceAuthorization,
            result,
        )
    }

    fn poll_device_once_inner<T, A>(
        &self,
        session: DevicePollingSession,
        transport: &mut T,
        audit: &mut A,
    ) -> Result<BrokerDevicePollResult, BrokerError>
    where
        T: OAuthDeviceTokenTransport,
        A: OAuthBrokerAuditSink,
    {
        let provider = self.registered_provider(session.provider())?.clone();
        if !session.is_bound_to(provider.config()) {
            return Err(BrokerError::BindingMismatch);
        }
        let trace = session.trace();
        let request = prepare_device_token_poll(session)
            .publish_then_release(audit)
            .map_err(map_oauth_error)?;
        publish_broker(
            audit,
            request.provider(),
            trace,
            BrokerAuditAction::DeviceTokenTransport,
            BrokerAuditOutcome::Attempted,
        )?;
        let wire_response = match transport.send_device_poll(&request) {
            Ok(response) => {
                publish_broker(
                    audit,
                    request.provider(),
                    trace,
                    BrokerAuditAction::DeviceTokenTransport,
                    BrokerAuditOutcome::Succeeded,
                )?;
                response
            }
            Err(_) => {
                publish_broker(
                    audit,
                    request.provider(),
                    trace,
                    BrokerAuditAction::DeviceTokenTransport,
                    BrokerAuditOutcome::Failed(BrokerFailureClass::Transport),
                )?;
                return Ok(BrokerDevicePollResult::TransportFailed(
                    request.into_session(),
                ));
            }
        };
        let context = request.response_context();
        let (status, body) = wire_response.into_parts();
        let response =
            decode_device_token_poll_response(context, status, provider.response_format(), body)
                .publish_then_release(audit)
                .map_err(map_oauth_error)?;
        Ok(BrokerDevicePollResult::Response(response))
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

fn send_device_authorization_audited<T: OAuthDeviceAuthorizationTransport, A: BrokerAuditSink>(
    transport: &mut T,
    request: &DeviceAuthorizationRequest,
    audit: &mut A,
) -> Result<DeviceAuthorizationEndpointResponse, BrokerError> {
    publish_broker(
        audit,
        request.provider(),
        request.trace(),
        BrokerAuditAction::DeviceAuthorizationTransport,
        BrokerAuditOutcome::Attempted,
    )?;
    let result = transport
        .send_device_authorization(request)
        .map_err(|_| BrokerError::Transport);
    finish_broker(
        audit,
        request.provider(),
        request.trace(),
        BrokerAuditAction::DeviceAuthorizationTransport,
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
        decode_authorization_server_metadata, decode_device_authorization_response,
        prepare_authorization_server_metadata, prepare_device_authorization, prepare_token_refresh,
        DeviceAuthorizationProfile, OAuthAuditError, OAuthAuditEvent, OAuthAuditOutcome,
    };
    use coding_adventures_oauth_credential_custody::{
        AccountId, CredentialAuditAction, CredentialAuditError, CredentialAuditEvent,
        CredentialAuditOutcome, InMemoryCredentialStore,
    };
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::rc::Rc;

    #[derive(Default)]
    struct RecordingAudit {
        broker: Vec<BrokerAuditEvent>,
        oauth: Vec<OAuthAuditEvent>,
        custody: Vec<CredentialAuditEvent>,
        broker_calls: usize,
        fail_broker_on: Option<usize>,
        order: Option<Rc<RefCell<Vec<&'static str>>>>,
    }

    impl BrokerAuditSink for RecordingAudit {
        fn publish(&mut self, event: &BrokerAuditEvent) -> Result<(), BrokerAuditError> {
            self.broker_calls += 1;
            if self.fail_broker_on == Some(self.broker_calls) {
                return Err(BrokerAuditError);
            }
            if let Some(order) = &self.order {
                let label = match (event.action(), event.outcome()) {
                    (BrokerAuditAction::ProviderDataLoad, BrokerAuditOutcome::Attempted) => {
                        "load-attempted"
                    }
                    (BrokerAuditAction::ProviderDataLoad, BrokerAuditOutcome::Succeeded) => {
                        "load-succeeded"
                    }
                    (BrokerAuditAction::ProviderDataLoad, BrokerAuditOutcome::Failed(_)) => {
                        "load-failed"
                    }
                    (BrokerAuditAction::ProviderRegister, BrokerAuditOutcome::Attempted) => {
                        "register-attempted"
                    }
                    (BrokerAuditAction::ProviderRegister, BrokerAuditOutcome::Succeeded) => {
                        "register-succeeded"
                    }
                    (BrokerAuditAction::ProviderRegister, BrokerAuditOutcome::Failed(_)) => {
                        "register-failed"
                    }
                    _ => "other-broker-audit",
                };
                order.borrow_mut().push(label);
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

    struct MockDeviceTransport {
        responses: VecDeque<Result<TokenEndpointResponse, TokenTransportError>>,
        calls: usize,
    }

    impl MockDeviceTransport {
        fn new(responses: Vec<Result<TokenEndpointResponse, TokenTransportError>>) -> Self {
            Self {
                responses: responses.into(),
                calls: 0,
            }
        }

        fn json(status: u16, body: &str) -> Result<TokenEndpointResponse, TokenTransportError> {
            Ok(
                TokenEndpointResponse::new(status, Zeroizing::new(body.as_bytes().to_vec()))
                    .unwrap(),
            )
        }
    }

    impl OAuthDeviceTokenTransport for MockDeviceTransport {
        fn send_device_poll(
            &mut self,
            request: &DeviceTokenPollRequest,
        ) -> Result<TokenEndpointResponse, TokenTransportError> {
            self.calls += 1;
            assert_eq!(request.provider().as_str(), "fixture");
            assert_eq!(request.client_id(), "fixture-public-client");
            assert_eq!(request.endpoint(), "https://token.fixture.example/token");
            assert!(request.form_body().contains("device_code=device-secret"));
            self.responses
                .pop_front()
                .unwrap_or(Err(TokenTransportError))
        }
    }

    struct MockDeviceAuthorizationTransport {
        response: Option<Result<DeviceAuthorizationEndpointResponse, TokenTransportError>>,
        calls: usize,
    }

    impl MockDeviceAuthorizationTransport {
        fn json(body: &str) -> Self {
            Self {
                response: Some(Ok(DeviceAuthorizationEndpointResponse::new(
                    200,
                    "application/json",
                    Zeroizing::new(body.as_bytes().to_vec()),
                )
                .unwrap())),
                calls: 0,
            }
        }

        fn failed() -> Self {
            Self {
                response: Some(Err(TokenTransportError)),
                calls: 0,
            }
        }
    }

    impl OAuthDeviceAuthorizationTransport for MockDeviceAuthorizationTransport {
        fn send_device_authorization(
            &mut self,
            request: &DeviceAuthorizationRequest,
        ) -> Result<DeviceAuthorizationEndpointResponse, TokenTransportError> {
            self.calls += 1;
            assert_eq!(request.provider().as_str(), "fixture");
            assert_eq!(request.client_id(), "fixture-public-client");
            assert_eq!(
                request.endpoint(),
                "https://device.fixture.example/authorize"
            );
            assert_eq!(request.content_type(), "application/x-www-form-urlencoded");
            assert_eq!(
                request.form_body(),
                "client_id=fixture-public-client&scope=files.read"
            );
            self.response.take().unwrap_or(Err(TokenTransportError))
        }
    }

    struct MockProviderDataSource {
        response: Option<Result<Zeroizing<Vec<u8>>, ProviderDataSourceError>>,
        calls: usize,
        requested: Vec<ProviderId>,
        order: Option<Rc<RefCell<Vec<&'static str>>>>,
    }

    impl MockProviderDataSource {
        fn successful(body: Zeroizing<Vec<u8>>) -> Self {
            Self {
                response: Some(Ok(body)),
                calls: 0,
                requested: Vec::new(),
                order: None,
            }
        }

        fn failed() -> Self {
            Self {
                response: Some(Err(ProviderDataSourceError)),
                calls: 0,
                requested: Vec::new(),
                order: None,
            }
        }
    }

    impl PublicProviderDataSource for MockProviderDataSource {
        fn load_public_provider_data(
            &mut self,
            provider: &ProviderId,
        ) -> Result<Zeroizing<Vec<u8>>, ProviderDataSourceError> {
            self.calls += 1;
            self.requested.push(provider.clone());
            if let Some(order) = &self.order {
                order.borrow_mut().push("source-read");
            }
            self.response.take().unwrap_or(Err(ProviderDataSourceError))
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
            format!("http://127.0.0.1:49152/{name}/callback"),
        )
        .unwrap()
        .with_distinct_redirect_uri()
    }

    fn issuer_policy(name: &str, redirect_uri: &str) -> BrokerProvider {
        let config = ProviderConfig::new(
            ProviderId::new(name).unwrap(),
            format!("https://auth.{name}.example/authorize"),
            format!("https://token.{name}.example/token"),
            format!("{name}-public-client"),
            redirect_uri,
        )
        .unwrap()
        .with_expected_issuer(format!("https://auth.{name}.example"))
        .unwrap();
        BrokerProvider::new(config, TokenResponseFormat::Json, 300).unwrap()
    }

    fn policy(name: &str, lead: u64) -> BrokerProvider {
        BrokerProvider::new(config(name), TokenResponseFormat::Json, lead).unwrap()
    }

    fn public_provider_data(name: &str) -> Zeroizing<Vec<u8>> {
        Zeroizing::new(
            format!(
                r#"{{
                    "schema_version":1,
                    "provider":"{name}",
                    "authorization_endpoint":"https://auth.{name}.example/authorize",
                    "token_endpoint":"https://token.{name}.example/token",
                    "mix_up_defense":{{"kind":"distinct_redirect_uri"}},
                    "authorization_extra_parameters":{{}},
                    "token_response_format":"json",
                    "refresh_lead_seconds":300
                }}"#
            )
            .into_bytes(),
        )
    }

    fn device_profile(
        name: &str,
        operation_trace: OAuthTraceId,
        audit: &mut RecordingAudit,
    ) -> DeviceAuthorizationProfile {
        let issuer = format!("https://login.{name}.example/tenant");
        let metadata_request = prepare_authorization_server_metadata(
            ProviderId::new(name).unwrap(),
            &issuer,
            operation_trace,
        )
        .publish_then_release(audit)
        .unwrap();
        let metadata_body = format!(
            r#"{{
                "issuer":"{issuer}",
                "authorization_endpoint":"https://auth.{name}.example/authorize",
                "token_endpoint":"https://token.{name}.example/token",
                "device_authorization_endpoint":"https://device.{name}.example/authorize",
                "response_types_supported":["code"],
                "grant_types_supported":["authorization_code","urn:ietf:params:oauth:grant-type:device_code"],
                "token_endpoint_auth_methods_supported":["none"],
                "code_challenge_methods_supported":["S256"]
            }}"#
        );
        let metadata = decode_authorization_server_metadata(
            metadata_request.response_context(),
            200,
            "application/json",
            Zeroizing::new(metadata_body.into_bytes()),
        )
        .publish_then_release(audit)
        .unwrap();
        DeviceAuthorizationProfile::from_metadata(&metadata, format!("{name}-public-client"))
            .unwrap()
    }

    fn device_session(
        name: &str,
        operation_trace: OAuthTraceId,
        audit: &mut RecordingAudit,
    ) -> DevicePollingSession {
        let profile = device_profile(name, operation_trace, audit);
        let request = prepare_device_authorization(&profile, &["files.read"], operation_trace)
            .publish_then_release(audit)
            .unwrap();
        let body = br#"{
            "device_code":"device-secret",
            "user_code":"ABCD-EFGH",
            "verification_uri":"https://device.fixture.example/activate",
            "expires_in":900,
            "interval":5
        }"#;
        let authorization = decode_device_authorization_response(
            request.response_context(),
            200,
            "application/json",
            Zeroizing::new(body.to_vec()),
        )
        .publish_then_release(audit)
        .unwrap();
        authorization.into_parts().1
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
    fn provider_data_load_is_bound_and_audited_before_source_and_registration() {
        let custody = CredentialCustody::new(InMemoryCredentialStore::new());
        let mut broker = OAuthBroker::new(custody);
        let requested = ProviderId::new("fixture").unwrap();
        let order = Rc::new(RefCell::new(Vec::new()));
        let mut source = MockProviderDataSource::successful(public_provider_data("fixture"));
        source.order = Some(Rc::clone(&order));
        let mut audit = RecordingAudit {
            order: Some(Rc::clone(&order)),
            ..RecordingAudit::default()
        };

        broker
            .load_and_register_public_provider(
                &requested,
                "fixture-client",
                "http://127.0.0.1:49152/fixture/callback",
                trace(27),
                &mut source,
                &mut audit,
            )
            .unwrap();

        assert_eq!(source.calls, 1);
        assert_eq!(
            source.requested.as_slice(),
            std::slice::from_ref(&requested)
        );
        assert_eq!(broker.provider_count(), 1);
        assert_eq!(
            order.borrow().as_slice(),
            [
                "load-attempted",
                "source-read",
                "load-succeeded",
                "register-attempted",
                "register-succeeded",
            ]
        );
        assert!(audit
            .broker
            .iter()
            .all(|event| { event.provider() == &requested && event.trace() == trace(27) }));
    }

    #[test]
    fn provider_data_source_and_identity_failures_never_register() {
        let custody = CredentialCustody::new(InMemoryCredentialStore::new());
        let mut broker = OAuthBroker::new(custody);
        let requested = ProviderId::new("fixture").unwrap();
        let mut audit = RecordingAudit::default();
        let mut failed_source = MockProviderDataSource::failed();
        assert_eq!(
            broker.load_and_register_public_provider(
                &requested,
                "fixture-client",
                "http://127.0.0.1:49152/fixture/callback",
                trace(28),
                &mut failed_source,
                &mut audit,
            ),
            Err(BrokerError::ProviderDataSource)
        );
        assert_eq!(failed_source.calls, 1);
        assert_eq!(broker.provider_count(), 0);
        assert_eq!(
            audit.broker.last().unwrap().outcome(),
            BrokerAuditOutcome::Failed(BrokerFailureClass::ProviderData)
        );

        let mut mismatched_source =
            MockProviderDataSource::successful(public_provider_data("other"));
        assert_eq!(
            broker.load_and_register_public_provider(
                &requested,
                "fixture-client",
                "http://127.0.0.1:49152/fixture/callback",
                trace(29),
                &mut mismatched_source,
                &mut audit,
            ),
            Err(BrokerError::BindingMismatch)
        );
        assert_eq!(mismatched_source.calls, 1);
        assert_eq!(broker.provider_count(), 0);
        assert_eq!(
            audit.broker.last().unwrap().outcome(),
            BrokerAuditOutcome::Failed(BrokerFailureClass::InvalidInput)
        );
        assert!(!audit.broker.iter().any(|event| {
            event.trace() == trace(29) && event.action() == BrokerAuditAction::ProviderRegister
        }));
    }

    #[test]
    fn provider_data_audit_failures_prevent_read_or_registration() {
        let custody = CredentialCustody::new(InMemoryCredentialStore::new());
        let mut broker = OAuthBroker::new(custody);
        let requested = ProviderId::new("fixture").unwrap();
        let mut source = MockProviderDataSource::successful(public_provider_data("fixture"));
        let mut audit = RecordingAudit {
            fail_broker_on: Some(1),
            ..RecordingAudit::default()
        };
        assert_eq!(
            broker.load_and_register_public_provider(
                &requested,
                "fixture-client",
                "http://127.0.0.1:49152/fixture/callback",
                trace(30),
                &mut source,
                &mut audit,
            ),
            Err(BrokerError::Audit)
        );
        assert_eq!(source.calls, 0);
        assert_eq!(broker.provider_count(), 0);

        let mut source = MockProviderDataSource::successful(public_provider_data("fixture"));
        let mut audit = RecordingAudit {
            fail_broker_on: Some(2),
            ..RecordingAudit::default()
        };
        assert_eq!(
            broker.load_and_register_public_provider(
                &requested,
                "fixture-client",
                "http://127.0.0.1:49152/fixture/callback",
                trace(31),
                &mut source,
                &mut audit,
            ),
            Err(BrokerError::Audit)
        );
        assert_eq!(source.calls, 1);
        assert_eq!(broker.provider_count(), 0);
        assert!(!audit
            .broker
            .iter()
            .any(|event| event.action() == BrokerAuditAction::ProviderRegister));
    }

    #[test]
    fn registry_enforces_distinct_redirect_ownership_in_both_orders() {
        let shared = "http://127.0.0.1:49152/shared/callback";

        let custody = CredentialCustody::new(InMemoryCredentialStore::new());
        let mut broker = OAuthBroker::new(custody);
        let mut audit = RecordingAudit::default();
        let distinct = ProviderConfig::new(
            ProviderId::new("distinct-first").unwrap(),
            "https://auth.distinct-first.example/authorize",
            "https://token.distinct-first.example/token",
            "distinct-first-client",
            shared,
        )
        .unwrap()
        .with_distinct_redirect_uri();
        broker
            .register_provider(
                BrokerProvider::new(distinct, TokenResponseFormat::Json, 300).unwrap(),
                trace(21),
                &mut audit,
            )
            .unwrap();
        assert_eq!(
            broker.register_provider(
                issuer_policy("issuer-second", shared),
                trace(22),
                &mut audit,
            ),
            Err(BrokerError::ProviderConflict)
        );
        assert_eq!(broker.provider_count(), 1);

        let custody = CredentialCustody::new(InMemoryCredentialStore::new());
        let mut broker = OAuthBroker::new(custody);
        broker
            .register_provider(issuer_policy("issuer-first", shared), trace(23), &mut audit)
            .unwrap();
        let distinct = ProviderConfig::new(
            ProviderId::new("distinct-second").unwrap(),
            "https://auth.distinct-second.example/authorize",
            "https://token.distinct-second.example/token",
            "distinct-second-client",
            shared,
        )
        .unwrap()
        .with_distinct_redirect_uri();
        assert_eq!(
            broker.register_provider(
                BrokerProvider::new(distinct, TokenResponseFormat::Json, 300).unwrap(),
                trace(24),
                &mut audit,
            ),
            Err(BrokerError::ProviderConflict)
        );
        assert_eq!(broker.provider_count(), 1);
        assert_eq!(
            audit.broker.last().unwrap().outcome(),
            BrokerAuditOutcome::Failed(BrokerFailureClass::ProviderConflict)
        );
    }

    #[test]
    fn registry_allows_shared_redirect_when_both_providers_validate_issuer() {
        let shared = "http://127.0.0.1:49152/shared/callback";
        let custody = CredentialCustody::new(InMemoryCredentialStore::new());
        let mut broker = OAuthBroker::new(custody);
        let mut audit = RecordingAudit::default();
        broker
            .register_provider(issuer_policy("issuer-one", shared), trace(25), &mut audit)
            .unwrap();
        broker
            .register_provider(issuer_policy("issuer-two", shared), trace(26), &mut audit)
            .unwrap();
        assert_eq!(broker.provider_count(), 2);
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
    fn device_authorization_is_registry_bound_and_audited_around_transport() {
        let custody = CredentialCustody::new(InMemoryCredentialStore::new());
        let mut broker = OAuthBroker::new(custody);
        let mut setup_audit = RecordingAudit::default();
        broker
            .register_provider(policy("fixture", 300), trace(32), &mut setup_audit)
            .unwrap();
        let profile = device_profile("fixture", trace(32), &mut setup_audit);
        let mut audit = RecordingAudit::default();
        let mut transport = MockDeviceAuthorizationTransport::json(
            r#"{
                "device_code":"device-secret",
                "user_code":"ABCD-EFGH",
                "verification_uri":"https://device.fixture.example/activate",
                "expires_in":900,
                "interval":5
            }"#,
        );

        let authorization = broker
            .begin_device_authorization(
                &profile,
                &["files.read"],
                trace(32),
                &mut transport,
                &mut audit,
            )
            .unwrap();

        assert_eq!(transport.calls, 1);
        assert_eq!(authorization.polling().provider().as_str(), "fixture");
        assert_eq!(authorization.polling().trace(), trace(32));
        assert_eq!(authorization.verification().user_code(), "ABCD-EFGH");
        assert_eq!(
            audit
                .broker
                .iter()
                .map(|event| (event.action(), event.outcome()))
                .collect::<Vec<_>>(),
            vec![
                (
                    BrokerAuditAction::DeviceAuthorization,
                    BrokerAuditOutcome::Attempted,
                ),
                (
                    BrokerAuditAction::DeviceAuthorizationTransport,
                    BrokerAuditOutcome::Attempted,
                ),
                (
                    BrokerAuditAction::DeviceAuthorizationTransport,
                    BrokerAuditOutcome::Succeeded,
                ),
                (
                    BrokerAuditAction::DeviceAuthorization,
                    BrokerAuditOutcome::Succeeded,
                ),
            ]
        );
        assert_eq!(audit.oauth.len(), 2);
        assert!(audit
            .oauth
            .iter()
            .all(|event| { event.provider().as_str() == "fixture" && event.trace() == trace(32) }));
    }

    #[test]
    fn device_authorization_binding_and_transport_audits_fail_closed() {
        let custody = CredentialCustody::new(InMemoryCredentialStore::new());
        let mut broker = OAuthBroker::new(custody);
        let mut setup_audit = RecordingAudit::default();
        let mismatched = ProviderConfig::new(
            ProviderId::new("fixture").unwrap(),
            "https://auth.fixture.example/authorize",
            "https://token.fixture.example/token",
            "different-public-client",
            "http://127.0.0.1:49152/callback",
        )
        .unwrap()
        .with_distinct_redirect_uri();
        broker
            .register_provider(
                BrokerProvider::new(mismatched, TokenResponseFormat::Json, 300).unwrap(),
                trace(33),
                &mut setup_audit,
            )
            .unwrap();
        let profile = device_profile("fixture", trace(33), &mut setup_audit);
        let mut audit = RecordingAudit::default();
        let mut transport = MockDeviceAuthorizationTransport::failed();
        assert!(matches!(
            broker.begin_device_authorization(
                &profile,
                &["files.read"],
                trace(33),
                &mut transport,
                &mut audit,
            ),
            Err(BrokerError::BindingMismatch)
        ));
        assert_eq!(transport.calls, 0);
        assert!(audit.oauth.is_empty());

        let custody = CredentialCustody::new(InMemoryCredentialStore::new());
        let mut broker = OAuthBroker::new(custody);
        broker
            .register_provider(policy("fixture", 300), trace(34), &mut setup_audit)
            .unwrap();
        let profile = device_profile("fixture", trace(34), &mut setup_audit);
        let mut audit = RecordingAudit {
            fail_broker_on: Some(2),
            ..RecordingAudit::default()
        };
        assert!(matches!(
            broker.begin_device_authorization(
                &profile,
                &["files.read"],
                trace(34),
                &mut transport,
                &mut audit,
            ),
            Err(BrokerError::Audit)
        ));
        assert_eq!(transport.calls, 0);
    }

    #[test]
    fn device_authorization_transport_failure_and_result_audit_are_closed() {
        let custody = CredentialCustody::new(InMemoryCredentialStore::new());
        let mut broker = OAuthBroker::new(custody);
        let mut setup_audit = RecordingAudit::default();
        broker
            .register_provider(policy("fixture", 300), trace(35), &mut setup_audit)
            .unwrap();
        let profile = device_profile("fixture", trace(35), &mut setup_audit);
        let mut audit = RecordingAudit::default();
        let mut transport = MockDeviceAuthorizationTransport::failed();
        assert!(matches!(
            broker.begin_device_authorization(
                &profile,
                &["files.read"],
                trace(35),
                &mut transport,
                &mut audit,
            ),
            Err(BrokerError::Transport)
        ));
        assert_eq!(transport.calls, 1);
        assert!(audit.broker.iter().any(|event| {
            event.action() == BrokerAuditAction::DeviceAuthorizationTransport
                && event.outcome() == BrokerAuditOutcome::Failed(BrokerFailureClass::Transport)
        }));

        let profile = device_profile("fixture", trace(36), &mut setup_audit);
        let mut audit = RecordingAudit {
            fail_broker_on: Some(3),
            ..RecordingAudit::default()
        };
        let mut transport = MockDeviceAuthorizationTransport::json(
            r#"{"device_code":"secret","user_code":"CODE","verification_uri":"https://device.fixture.example/activate","expires_in":900}"#,
        );
        assert!(matches!(
            broker.begin_device_authorization(
                &profile,
                &["files.read"],
                trace(36),
                &mut transport,
                &mut audit,
            ),
            Err(BrokerError::Audit)
        ));
        assert_eq!(transport.calls, 1);
        assert_eq!(audit.oauth.len(), 1);
    }

    #[test]
    fn device_authorization_response_boundary_is_bounded_and_redacted() {
        assert!(DeviceAuthorizationEndpointResponse::new(
            99,
            "application/json",
            Zeroizing::new(vec![1]),
        )
        .is_err());
        assert!(DeviceAuthorizationEndpointResponse::new(
            200,
            "application/json",
            Zeroizing::new(vec![0; MAX_TOKEN_RESPONSE_BYTES + 1]),
        )
        .is_err());
        let response = DeviceAuthorizationEndpointResponse::new(
            200,
            "application/json",
            Zeroizing::new(b"device-secret".to_vec()),
        )
        .unwrap();
        let debug = format!("{response:?}");
        assert!(!debug.contains("device-secret"));
        assert!(!debug.contains("application/json"));
    }

    #[test]
    fn one_step_device_polling_is_bound_audited_and_caller_scheduled() {
        let custody = CredentialCustody::new(InMemoryCredentialStore::new());
        let mut broker = OAuthBroker::new(custody);
        let mut audit = RecordingAudit::default();
        broker
            .register_provider(policy("fixture", 300), trace(10), &mut audit)
            .unwrap();
        let session = device_session("fixture", trace(10), &mut audit);
        let mut transport = MockDeviceTransport::new(vec![
            MockDeviceTransport::json(400, r#"{"error":"authorization_pending"}"#),
            MockDeviceTransport::json(400, r#"{"error":"slow_down"}"#),
            MockDeviceTransport::json(
                200,
                r#"{"access_token":"device-access","refresh_token":"device-refresh","token_type":"Bearer","expires_in":3600}"#,
            ),
        ]);

        let pending = broker
            .poll_device_once(session, &mut transport, &mut audit)
            .unwrap();
        assert_eq!(pending.retry_after_seconds(), Some(5));
        let BrokerDevicePollResult::Response(DevicePollResult::Pending(session)) = pending else {
            panic!("expected pending device poll");
        };
        let slow_down = broker
            .poll_device_once(session, &mut transport, &mut audit)
            .unwrap();
        assert_eq!(slow_down.retry_after_seconds(), Some(10));
        let BrokerDevicePollResult::Response(DevicePollResult::SlowDown(session)) = slow_down
        else {
            panic!("expected slow-down device poll");
        };
        let authorized = broker
            .poll_device_once(session, &mut transport, &mut audit)
            .unwrap();
        let BrokerDevicePollResult::Response(DevicePollResult::Authorized(response)) = authorized
        else {
            panic!("expected authorized device poll");
        };
        assert_eq!(response.provider().as_str(), "fixture");
        assert_eq!(response.trace(), trace(10));
        assert_eq!(transport.calls, 3);
        assert_eq!(
            audit
                .broker
                .iter()
                .filter(|event| event.action() == BrokerAuditAction::DeviceTokenTransport)
                .count(),
            6
        );
        assert_eq!(
            audit.broker.last().unwrap().outcome(),
            BrokerAuditOutcome::Succeeded
        );
    }

    #[test]
    fn device_transport_failure_returns_only_the_opaque_retry_session() {
        let custody = CredentialCustody::new(InMemoryCredentialStore::new());
        let mut broker = OAuthBroker::new(custody);
        let mut setup_audit = RecordingAudit::default();
        broker
            .register_provider(policy("fixture", 300), trace(11), &mut setup_audit)
            .unwrap();
        let session = device_session("fixture", trace(11), &mut setup_audit);
        let mut audit = RecordingAudit::default();
        let mut transport = MockDeviceTransport::new(vec![Err(TokenTransportError)]);
        let failed = broker
            .poll_device_once(session, &mut transport, &mut audit)
            .unwrap();
        assert_eq!(failed.retry_after_seconds(), Some(5));
        let debug = format!("{failed:?}");
        assert!(!debug.contains("device-secret"));
        let BrokerDevicePollResult::TransportFailed(session) = failed else {
            panic!("expected recoverable transport failure");
        };
        assert_eq!(session.provider().as_str(), "fixture");
        assert_eq!(session.trace(), trace(11));
        assert!(audit.broker.iter().any(|event| {
            event.action() == BrokerAuditAction::DeviceTokenTransport
                && event.outcome() == BrokerAuditOutcome::Failed(BrokerFailureClass::Transport)
        }));
        assert_eq!(
            audit.broker.last().unwrap().outcome(),
            BrokerAuditOutcome::Succeeded
        );
    }

    #[test]
    fn device_session_binding_is_checked_before_protocol_or_transport() {
        let custody = CredentialCustody::new(InMemoryCredentialStore::new());
        let mut broker = OAuthBroker::new(custody);
        let mut setup_audit = RecordingAudit::default();
        let mismatched = ProviderConfig::new(
            ProviderId::new("fixture").unwrap(),
            "https://auth.fixture.example/authorize",
            "https://token.fixture.example/token",
            "different-public-client",
            "http://127.0.0.1:49152/callback",
        )
        .unwrap()
        .with_distinct_redirect_uri();
        broker
            .register_provider(
                BrokerProvider::new(mismatched, TokenResponseFormat::Json, 300).unwrap(),
                trace(12),
                &mut setup_audit,
            )
            .unwrap();
        let session = device_session("fixture", trace(12), &mut setup_audit);
        let mut audit = RecordingAudit::default();
        let mut transport = MockDeviceTransport::new(vec![]);
        assert!(matches!(
            broker.poll_device_once(session, &mut transport, &mut audit),
            Err(BrokerError::BindingMismatch)
        ));
        assert_eq!(transport.calls, 0);
        assert!(audit.oauth.is_empty());
        assert_eq!(
            audit.broker.last().unwrap().outcome(),
            BrokerAuditOutcome::Failed(BrokerFailureClass::InvalidInput)
        );
    }

    #[test]
    fn device_transport_audit_failure_prevents_the_external_effect() {
        let custody = CredentialCustody::new(InMemoryCredentialStore::new());
        let mut broker = OAuthBroker::new(custody);
        let mut setup_audit = RecordingAudit::default();
        broker
            .register_provider(policy("fixture", 300), trace(13), &mut setup_audit)
            .unwrap();
        let session = device_session("fixture", trace(13), &mut setup_audit);
        let mut audit = RecordingAudit {
            fail_broker_on: Some(2),
            ..RecordingAudit::default()
        };
        let mut transport = MockDeviceTransport::new(vec![]);
        assert!(matches!(
            broker.poll_device_once(session, &mut transport, &mut audit),
            Err(BrokerError::Audit)
        ));
        assert_eq!(transport.calls, 0);
    }

    #[test]
    fn device_transport_result_audit_failure_withholds_the_response() {
        let custody = CredentialCustody::new(InMemoryCredentialStore::new());
        let mut broker = OAuthBroker::new(custody);
        let mut setup_audit = RecordingAudit::default();
        broker
            .register_provider(policy("fixture", 300), trace(14), &mut setup_audit)
            .unwrap();
        let session = device_session("fixture", trace(14), &mut setup_audit);
        let mut audit = RecordingAudit {
            fail_broker_on: Some(3),
            ..RecordingAudit::default()
        };
        let mut transport = MockDeviceTransport::new(vec![MockDeviceTransport::json(
            400,
            r#"{"error":"authorization_pending"}"#,
        )]);
        assert!(matches!(
            broker.poll_device_once(session, &mut transport, &mut audit),
            Err(BrokerError::Audit)
        ));
        assert_eq!(transport.calls, 1);
        assert!(!audit.oauth.iter().any(|event| {
            event.action()
                == coding_adventures_oauth::OAuthAuditAction::DeviceTokenPollResponseClassify
        }));
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
