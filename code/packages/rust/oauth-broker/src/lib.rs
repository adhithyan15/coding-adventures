//! Provider-neutral, audit-first OAuth credential, refresh, revocation, and device orchestration.
//!
//! This crate composes the pure OAuth protocol core with storage-agnostic
//! credential custody. It owns policy and sequencing, but no clock, network,
//! browser, filesystem, vault, or provider-specific authority.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_oauth::{
    decode_device_authorization_response, decode_device_token_poll_response, decode_token_response,
    decode_token_revocation_response, prepare_device_authorization, prepare_device_token_poll,
    prepare_token_refresh, prepare_token_revocation, ConfidentialClientAuthenticationMethod,
    DeviceAuthorization, DeviceAuthorizationProfile, DeviceAuthorizationRequest, DevicePollResult,
    DevicePollingSession, DeviceTokenPollRequest, OAuthAuditSink, OAuthError, OAuthTraceId,
    ProviderConfig, ProviderId, RevocationTokenHint, TokenExchangeRequest, TokenRefreshRequest,
    TokenResponse, TokenResponseFormat, TokenRevocationRequest, TokenRevocationResponse,
    MAX_TOKEN_RESPONSE_BYTES, MAX_TOKEN_REVOCATION_RESPONSE_BYTES,
};
use coding_adventures_oauth_client_secret_custody::{
    ClientSecretAuditSink, ClientSecretAuthenticatedRequest, ClientSecretAuthentication,
    ClientSecretAuthenticationMethod, ClientSecretCustody, ClientSecretCustodyError,
    ClientSecretKey, ClientSecretStore,
};
use coding_adventures_oauth_credential_custody::{
    CredentialAuditSink, CredentialCustody, CredentialKey, CredentialMetadata, CredentialRevision,
    CredentialStore, CustodyError,
};
use coding_adventures_oauth_private_key_jwt::PrivateKeyJwtProfile;
use coding_adventures_oauth_private_key_signer::{PrivateKeyId, PrivateKeyJwtAlgorithm};
use coding_adventures_zeroize::Zeroizing;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Debug, Display, Formatter};

mod provider_data;

/// Largest accepted proactive-refresh window: one day.
pub const MAX_REFRESH_LEAD_SECONDS: u64 = 24 * 60 * 60;

const MAX_CLIENT_AUTHENTICATION_ALGORITHMS: usize = 128;

const MAX_CLIENT_AUTHENTICATION_ALGORITHM_BYTES: usize = 256;

/// Provider data plus broker-owned lifecycle policy.
#[derive(Clone, PartialEq, Eq)]
pub struct BrokerProvider {
    config: ProviderConfig,
    response_format: TokenResponseFormat,
    refresh_lead_seconds: u64,
    confidential_authentication_method: Option<ConfidentialClientAuthenticationMethod>,
    client_authentication_signing_algorithms: Vec<String>,
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
            confidential_authentication_method: None,
            client_authentication_signing_algorithms: Vec::new(),
        })
    }

    /// Validate one confidential-provider registration without acquiring
    /// credential, signer, storage, or I/O authority.
    ///
    /// `private_key_jwt` requires the exact non-empty provider-advertised
    /// algorithm set. Client-secret methods must supply no algorithms. Retained
    /// algorithm names are data and do not imply a concrete signer.
    pub fn new_confidential(
        config: ProviderConfig,
        response_format: TokenResponseFormat,
        refresh_lead_seconds: u64,
        authentication_method: ConfidentialClientAuthenticationMethod,
        signing_algorithms: Vec<String>,
    ) -> Result<Self, BrokerError> {
        validate_client_authentication_algorithms(authentication_method, &signing_algorithms)?;
        let mut provider = Self::new(config, response_format, refresh_lead_seconds)?;
        provider.confidential_authentication_method = Some(authentication_method);
        provider.client_authentication_signing_algorithms = signing_algorithms;
        Ok(provider)
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

    /// Return the selected closed confidential authentication method.
    ///
    /// `None` identifies the public-client profile selected by [`Self::new`].
    pub const fn confidential_authentication_method(
        &self,
    ) -> Option<ConfidentialClientAuthenticationMethod> {
        self.confidential_authentication_method
    }

    /// Borrow the exact provider-advertised JWT client-authentication algorithms.
    ///
    /// This is empty for public and client-secret profiles. Retention does not
    /// imply that any algorithm has a concrete signing implementation.
    pub fn client_authentication_signing_algorithms(&self) -> &[String] {
        &self.client_authentication_signing_algorithms
    }

    /// Bind one opaque client-secret key to this provider's retained method.
    ///
    /// The key's provider must match exactly. Public profiles and
    /// `private_key_jwt` profiles fail closed, so callers cannot choose or
    /// default the client-secret wire method. This pure binding performs no
    /// credential access and acquires no storage or transport authority.
    pub fn bind_client_secret_authentication(
        &self,
        key: ClientSecretKey,
    ) -> Result<ClientSecretAuthentication, BrokerError> {
        if key.provider() != self.provider() {
            return Err(BrokerError::BindingMismatch);
        }
        let method = match self.confidential_authentication_method {
            Some(ConfidentialClientAuthenticationMethod::ClientSecretBasic) => {
                ClientSecretAuthenticationMethod::ClientSecretBasic
            }
            Some(ConfidentialClientAuthenticationMethod::ClientSecretPost) => {
                ClientSecretAuthenticationMethod::ClientSecretPost
            }
            None | Some(ConfidentialClientAuthenticationMethod::PrivateKeyJwt) => {
                return Err(BrokerError::BindingMismatch);
            }
        };
        Ok(ClientSecretAuthentication::new(key, method))
    }

    /// Bind one opaque private key to this provider's retained JWT profile.
    ///
    /// The key provider and selected algorithm must match this profile
    /// exactly. Public and client-secret profiles fail closed. Constructing
    /// this data-only assertion profile neither invokes a signer nor implies
    /// that the selected algorithm has a concrete implementation.
    pub fn bind_private_key_jwt_profile(
        &self,
        key: PrivateKeyId,
        algorithm: PrivateKeyJwtAlgorithm,
        key_id: Option<String>,
        lifetime_seconds: u64,
    ) -> Result<PrivateKeyJwtProfile, BrokerError> {
        if key.provider() != self.provider()
            || self.confidential_authentication_method
                != Some(ConfidentialClientAuthenticationMethod::PrivateKeyJwt)
            || !self
                .client_authentication_signing_algorithms
                .iter()
                .any(|candidate| candidate == algorithm.as_str())
        {
            return Err(BrokerError::BindingMismatch);
        }
        let methods = [ConfidentialClientAuthenticationMethod::PrivateKeyJwt
            .as_str()
            .to_owned()];
        PrivateKeyJwtProfile::new(
            self.provider().clone(),
            self.config.client_id().to_owned(),
            self.config.token_endpoint(),
            &methods,
            &self.client_authentication_signing_algorithms,
            key,
            algorithm,
            key_id,
            lifetime_seconds,
        )
        .map_err(|_| BrokerError::InvalidPolicy)
    }
}

fn validate_client_authentication_algorithms(
    authentication_method: ConfidentialClientAuthenticationMethod,
    algorithms: &[String],
) -> Result<(), BrokerError> {
    if authentication_method != ConfidentialClientAuthenticationMethod::PrivateKeyJwt {
        return if algorithms.is_empty() {
            Ok(())
        } else {
            Err(BrokerError::InvalidPolicy)
        };
    }
    if algorithms.is_empty() || algorithms.len() > MAX_CLIENT_AUTHENTICATION_ALGORITHMS {
        return Err(BrokerError::InvalidPolicy);
    }
    let mut seen = BTreeSet::new();
    if algorithms.iter().any(|algorithm| {
        algorithm == "none"
            || algorithm.is_empty()
            || algorithm.len() > MAX_CLIENT_AUTHENTICATION_ALGORITHM_BYTES
            || !algorithm.bytes().all(|byte| matches!(byte, 0x21..=0x7e))
            || !seen.insert(algorithm.as_str())
    }) {
        return Err(BrokerError::InvalidPolicy);
    }
    Ok(())
}

impl Debug for BrokerProvider {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BrokerProvider")
            .field("provider", self.provider())
            .field("response_format", &self.response_format)
            .field("refresh_lead_seconds", &self.refresh_lead_seconds)
            .field(
                "confidential_authentication_method",
                &self.confidential_authentication_method,
            )
            .field(
                "client_authentication_signing_algorithm_count",
                &self.client_authentication_signing_algorithms.len(),
            )
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

/// Caller-injected authority for reading one static confidential-provider profile.
///
/// Implementations may read a file, vault object, embedded resource, or other
/// host-owned source. The broker publishes its provider- and trace-bound audit
/// intent before invoking this method, and the returned bytes remain
/// wipe-on-drop while they are decoded. This boundary reads provider policy,
/// never a client secret or signing key.
pub trait ConfidentialProviderDataSource {
    /// Read the profile selected by the exact requested provider identity.
    fn load_confidential_provider_data(
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

/// Bounded RFC 7009 endpoint response owned in wipe-on-drop storage.
pub struct TokenRevocationEndpointResponse {
    status: u16,
    body: Zeroizing<Vec<u8>>,
}

impl TokenRevocationEndpointResponse {
    /// Construct a syntactically valid bounded HTTP response boundary.
    ///
    /// An empty body is valid because RFC 7009 success bodies are ignored.
    pub fn new(status: u16, body: Zeroizing<Vec<u8>>) -> Result<Self, BrokerError> {
        if !(100..=599).contains(&status) || body.len() > MAX_TOKEN_REVOCATION_RESPONSE_BYTES {
            return Err(BrokerError::InvalidTransportResponse);
        }
        Ok(Self { status, body })
    }

    fn into_parts(self) -> (u16, Zeroizing<Vec<u8>>) {
        (self.status, self.body)
    }
}

impl Debug for TokenRevocationEndpointResponse {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TokenRevocationEndpointResponse")
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

/// Authorized provider-neutral transport for one client-secret-authenticated refresh.
pub trait OAuthClientSecretTokenTransport {
    /// Send one custody-built, zeroizing authenticated refresh request.
    fn send_client_secret_refresh(
        &mut self,
        request: &ClientSecretAuthenticatedRequest,
    ) -> Result<TokenEndpointResponse, TokenTransportError>;
}

/// Authorized provider-neutral transport for one client-secret-authenticated exchange.
pub trait OAuthClientSecretTokenExchangeTransport {
    /// Send one custody-built, zeroizing authenticated authorization-code exchange.
    fn send_client_secret_exchange(
        &mut self,
        request: &ClientSecretAuthenticatedRequest,
    ) -> Result<TokenEndpointResponse, TokenTransportError>;
}

/// Authorized provider-neutral transport for one client-secret-authenticated revocation.
pub trait OAuthClientSecretTokenRevocationTransport {
    /// Send one custody-built, zeroizing authenticated RFC 7009 request.
    fn send_client_secret_revocation(
        &mut self,
        request: &ClientSecretAuthenticatedRequest,
    ) -> Result<TokenRevocationEndpointResponse, TokenTransportError>;
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

/// Opaque caller-timed state for an RFC 8628 device-flow polling sequence.
///
/// The time values share an arbitrary caller-owned monotonic epoch. The broker
/// neither reads a clock nor sleeps; each step accepts one observed time and
/// performs at most one transport effect.
pub struct DeviceFlowPollSequence {
    session: DevicePollingSession,
    expires_at_seconds: u64,
    next_poll_at_seconds: u64,
}

impl DeviceFlowPollSequence {
    /// Start scheduling an opaque polling session on a caller-owned timeline.
    pub fn new(
        session: DevicePollingSession,
        started_at_seconds: u64,
    ) -> Result<Self, BrokerError> {
        let expires_at_seconds = started_at_seconds
            .checked_add(session.expires_in_seconds())
            .ok_or(BrokerError::Clock)?;
        let next_poll_at_seconds = started_at_seconds
            .checked_add(session.interval_seconds())
            .ok_or(BrokerError::Clock)?;
        Ok(Self {
            session,
            expires_at_seconds,
            next_poll_at_seconds,
        })
    }

    /// Return the provider bound to the opaque device code.
    pub fn provider(&self) -> &ProviderId {
        self.session.provider()
    }

    /// Return the trace shared by initiation and every later poll step.
    pub const fn trace(&self) -> OAuthTraceId {
        self.session.trace()
    }

    /// Return the earliest caller-timeline instant at which a poll is allowed.
    pub const fn next_poll_at_seconds(&self) -> u64 {
        self.next_poll_at_seconds
    }

    /// Return the caller-timeline instant at which local polling expires.
    pub const fn expires_at_seconds(&self) -> u64 {
        self.expires_at_seconds
    }
}

impl Debug for DeviceFlowPollSequence {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DeviceFlowPollSequence")
            .field("provider", self.session.provider())
            .field("trace", &self.session.trace())
            .field("session", &"<redacted>")
            .field("expires_at_seconds", &self.expires_at_seconds)
            .field("next_poll_at_seconds", &self.next_poll_at_seconds)
            .finish()
    }
}

/// One audited caller-driven device-flow sequencing outcome.
pub enum DeviceFlowStepResult {
    /// The caller-provided time has not reached the minimum polling interval.
    Waiting(DeviceFlowPollSequence),
    /// The provider has not yet received the resource owner's decision.
    Pending(DeviceFlowPollSequence),
    /// The provider increased the minimum delay for every later poll.
    SlowDown(DeviceFlowPollSequence),
    /// A transient transport failure may be retried at the next scheduled instant.
    TransportFailed(DeviceFlowPollSequence),
    /// The provider returned an audited token response.
    Authorized(TokenResponse),
    /// The resource owner denied authorization.
    Denied,
    /// The caller timeline or provider reported expiration.
    Expired,
}

impl DeviceFlowStepResult {
    /// Return the next absolute caller-timeline polling instant, when applicable.
    pub const fn next_poll_at_seconds(&self) -> Option<u64> {
        match self {
            Self::Waiting(sequence)
            | Self::Pending(sequence)
            | Self::SlowDown(sequence)
            | Self::TransportFailed(sequence) => Some(sequence.next_poll_at_seconds()),
            Self::Authorized(_) | Self::Denied | Self::Expired => None,
        }
    }
}

impl Debug for DeviceFlowStepResult {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Waiting(sequence) => formatter.debug_tuple("Waiting").field(sequence).finish(),
            Self::Pending(sequence) => formatter.debug_tuple("Pending").field(sequence).finish(),
            Self::SlowDown(sequence) => formatter.debug_tuple("SlowDown").field(sequence).finish(),
            Self::TransportFailed(sequence) => formatter
                .debug_tuple("TransportFailed")
                .field(sequence)
                .finish(),
            Self::Authorized(response) => {
                formatter.debug_tuple("Authorized").field(response).finish()
            }
            Self::Denied => formatter.write_str("Denied"),
            Self::Expired => formatter.write_str("Expired"),
        }
    }
}

/// One audited caller-driven device-flow outcome whose authorized response is
/// persisted under an exact opaque credential key before it can leave the broker.
pub enum DeviceFlowCredentialStepResult {
    /// The caller-provided time has not reached the minimum polling interval.
    Waiting(DeviceFlowPollSequence),
    /// The provider has not yet received the resource owner's decision.
    Pending(DeviceFlowPollSequence),
    /// The provider increased the minimum delay for every later poll.
    SlowDown(DeviceFlowPollSequence),
    /// A transient transport failure may be retried at the next scheduled instant.
    TransportFailed(DeviceFlowPollSequence),
    /// The authorized credential was durably stored without releasing token bytes.
    Stored(CredentialRevision),
    /// The resource owner denied authorization.
    Denied,
    /// The caller timeline or provider reported expiration.
    Expired,
}

impl DeviceFlowCredentialStepResult {
    /// Return the next absolute caller-timeline polling instant, when applicable.
    pub const fn next_poll_at_seconds(&self) -> Option<u64> {
        match self {
            Self::Waiting(sequence)
            | Self::Pending(sequence)
            | Self::SlowDown(sequence)
            | Self::TransportFailed(sequence) => Some(sequence.next_poll_at_seconds()),
            Self::Stored(_) | Self::Denied | Self::Expired => None,
        }
    }
}

impl Debug for DeviceFlowCredentialStepResult {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Waiting(sequence) => formatter.debug_tuple("Waiting").field(sequence).finish(),
            Self::Pending(sequence) => formatter.debug_tuple("Pending").field(sequence).finish(),
            Self::SlowDown(sequence) => formatter.debug_tuple("SlowDown").field(sequence).finish(),
            Self::TransportFailed(sequence) => formatter
                .debug_tuple("TransportFailed")
                .field(sequence)
                .finish(),
            Self::Stored(revision) => formatter.debug_tuple("Stored").field(revision).finish(),
            Self::Denied => formatter.write_str("Denied"),
            Self::Expired => formatter.write_str("Expired"),
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
    /// Read and decode one provider-bound static provider profile.
    ProviderDataLoad,
    /// Register or idempotently confirm one provider definition.
    ProviderRegister,
    /// Store an initial credential response under an opaque account key.
    CredentialCreate,
    /// Refresh and atomically rotate one account credential.
    Refresh,
    /// Authenticate and send one client-secret refresh request.
    ClientSecretRefresh,
    /// Refresh and atomically rotate one client-secret-authenticated credential.
    ClientSecretRefreshCredentialRotate,
    /// Authenticate and send one client-secret authorization-code exchange.
    ClientSecretExchange,
    /// Authenticate, send, and classify one client-secret RFC 7009 revocation.
    ClientSecretRevocation,
    /// Revoke the exact stored refresh token, then delete its credential revision.
    ClientSecretRevocationCredentialDelete,
    /// Exchange and persist one client-secret-authenticated credential response.
    ClientSecretExchangeCredentialCreate,
    /// Send one request through the injected token transport.
    TokenTransport,
    /// Send one RFC 7009 request through the injected revocation transport.
    TokenRevocationTransport,
    /// Classify one externally scheduled RFC 8628 device-token poll.
    DevicePoll,
    /// Send one device-token request through the injected transport.
    DeviceTokenTransport,
    /// Initiate one RFC 8628 device authorization operation.
    DeviceAuthorization,
    /// Send one device-authorization request through the injected transport.
    DeviceAuthorizationTransport,
    /// Advance one caller-timed RFC 8628 polling sequence by at most one effect.
    DeviceFlowStep,
    /// Persist an authorized device response under one opaque credential key.
    DeviceCredentialCreate,
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
    /// Client-secret custody failed.
    ClientSecretCustody,
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

/// Audit sink capable of recording a broker-composed client-secret request.
pub trait OAuthClientSecretBrokerAuditSink:
    BrokerAuditSink + OAuthAuditSink + ClientSecretAuditSink
{
}

impl<T> OAuthClientSecretBrokerAuditSink for T where
    T: BrokerAuditSink + OAuthAuditSink + ClientSecretAuditSink
{
}

/// Audit sink capable of recording client-secret exchange and credential creation.
pub trait OAuthClientSecretCredentialBrokerAuditSink:
    OAuthClientSecretBrokerAuditSink + CredentialAuditSink
{
}

impl<T> OAuthClientSecretCredentialBrokerAuditSink for T where
    T: OAuthClientSecretBrokerAuditSink + CredentialAuditSink
{
}

/// Injected effect authorities for one exchange-to-custody composition.
pub struct ClientSecretExchangeCredentialExecution<'a, C, T> {
    clock: &'a mut C,
    transport: &'a mut T,
}

/// Injected effect authorities for one confidential refresh-to-custody composition.
pub struct ClientSecretRefreshCredentialExecution<'a, C, T> {
    clock: &'a mut C,
    transport: &'a mut T,
}

impl<'a, C, T> ClientSecretRefreshCredentialExecution<'a, C, T> {
    /// Bind caller-owned time and transport authorities to one execution.
    pub fn new(clock: &'a mut C, transport: &'a mut T) -> Self {
        Self { clock, transport }
    }
}

impl<'a, C, T> ClientSecretExchangeCredentialExecution<'a, C, T> {
    /// Bind caller-owned time and transport authorities to one execution.
    pub fn new(clock: &'a mut C, transport: &'a mut T) -> Self {
        Self { clock, transport }
    }
}

/// Closed broker-audit publication failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BrokerAuditError;

/// Closed broker error with no provider-controlled or secret-bearing text.
#[derive(Clone, PartialEq, Eq)]
pub enum BrokerError {
    /// Broker lifecycle policy was outside its accepted bound.
    InvalidPolicy,
    /// Static provider data was malformed, unknown, or failed validation.
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
    /// Client-secret custody failed with a closed class.
    ClientSecretCustody(ClientSecretCustodyError),
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
            | Self::ClientSecretCustody(ClientSecretCustodyError::Audit)
            | Self::Protocol(OAuthError::Audit)
            | Self::Audit => None,
            Self::Custody(_) => Some(BrokerFailureClass::Custody),
            Self::ClientSecretCustody(_) => Some(BrokerFailureClass::ClientSecretCustody),
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
            Self::ClientSecretCustody(_) => "ClientSecretCustody(<redacted>)",
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

    /// Audit, load, exactly bind, decode, and register one confidential profile.
    ///
    /// The requested provider identity selects the caller-owned source before
    /// any read occurs. A profile naming another provider is rejected before
    /// registry mutation. Deployment-specific client and redirect values stay
    /// outside the static profile. The profile may select only the closed
    /// confidential authentication methods accepted by
    /// [`BrokerProvider::from_confidential_provider_data`]; this operation
    /// acquires no client-secret, signing-key, signer, filesystem, or vault
    /// authority.
    pub fn load_and_register_confidential_provider<D, A>(
        &mut self,
        requested_provider: &ProviderId,
        client_id: impl Into<String>,
        redirect_uri: impl Into<String>,
        trace: OAuthTraceId,
        source: &mut D,
        audit: &mut A,
    ) -> Result<(), BrokerError>
    where
        D: ConfidentialProviderDataSource,
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
            .load_confidential_provider_data(requested_provider)
            .map_err(|_| BrokerError::ProviderDataSource)
            .and_then(|body| {
                BrokerProvider::from_confidential_provider_data(body, client_id, redirect_uri)
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

    /// Authenticate, send, and decode one prepared client-secret refresh request.
    ///
    /// The registered provider's exact client ID, token endpoint, and retained
    /// `client_secret_basic` or `client_secret_post` method are checked before
    /// client-secret custody access. Custody owns secret disclosure and
    /// zeroizing wire construction; the broker audit-brackets the injected
    /// transport and releases only an audit-gated bounded token response.
    pub fn send_client_secret_refresh<SS, T, A>(
        &self,
        authentication: &ClientSecretAuthentication,
        request: TokenRefreshRequest,
        client_secret_custody: &ClientSecretCustody<SS>,
        transport: &mut T,
        audit: &mut A,
    ) -> Result<TokenResponse, BrokerError>
    where
        SS: ClientSecretStore,
        T: OAuthClientSecretTokenTransport,
        A: OAuthClientSecretBrokerAuditSink,
    {
        let provider = request.provider().clone();
        let trace = request.trace();
        publish_broker(
            audit,
            &provider,
            trace,
            BrokerAuditAction::ClientSecretRefresh,
            BrokerAuditOutcome::Attempted,
        )?;
        let result = self.send_client_secret_refresh_inner(
            authentication,
            request,
            client_secret_custody,
            transport,
            audit,
        );
        finish_broker(
            audit,
            &provider,
            trace,
            BrokerAuditAction::ClientSecretRefresh,
            result,
        )
    }

    /// Refresh one stored credential through exact retained client-secret policy.
    ///
    /// The opaque account key selects the registered provider and credential
    /// record. Its retained Basic/Post method and provider-bound secret key are
    /// validated before credential custody releases the exact refresh token and
    /// revision. The decoded response crosses the OAuth credential-release gate
    /// into revision-bound custody rotation, so failures and stale revisions
    /// retain the prior record. Time, transport, secret storage, and credential
    /// storage remain injected authorities.
    pub fn refresh_client_secret_and_rotate_credentials<SS, C, T, A>(
        &self,
        key: &CredentialKey,
        authentication: &ClientSecretAuthentication,
        trace: OAuthTraceId,
        client_secret_custody: &ClientSecretCustody<SS>,
        execution: ClientSecretRefreshCredentialExecution<'_, C, T>,
        audit: &mut A,
    ) -> Result<CredentialRevision, BrokerError>
    where
        SS: ClientSecretStore,
        C: BrokerClock,
        T: OAuthClientSecretTokenTransport,
        A: OAuthClientSecretCredentialBrokerAuditSink,
    {
        let ClientSecretRefreshCredentialExecution { clock, transport } = execution;
        let provider_id = key.provider().clone();
        publish_broker(
            audit,
            &provider_id,
            trace,
            BrokerAuditAction::ClientSecretRefreshCredentialRotate,
            BrokerAuditOutcome::Attempted,
        )?;
        let result = (|| {
            let provider = self.registered_provider(&provider_id)?.clone();
            let expected =
                provider.bind_client_secret_authentication(authentication.key().clone())?;
            if &expected != authentication {
                return Err(BrokerError::BindingMismatch);
            }
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
            let response = self.send_client_secret_refresh(
                authentication,
                request,
                client_secret_custody,
                transport,
                audit,
            )?;
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
        })();
        finish_broker(
            audit,
            &provider_id,
            trace,
            BrokerAuditAction::ClientSecretRefreshCredentialRotate,
            result,
        )
    }

    /// Authenticate, send, and decode one prepared client-secret token exchange.
    ///
    /// The request has already consumed and validated the authorization
    /// transaction. The registered provider's exact client ID, token endpoint,
    /// and retained `client_secret_basic` or `client_secret_post` method are
    /// checked before client-secret custody access. Custody owns secret
    /// disclosure and zeroizing wire construction; the broker audit-brackets
    /// the injected transport and releases only an audit-gated bounded token
    /// response.
    pub fn send_client_secret_exchange<SS, T, A>(
        &self,
        authentication: &ClientSecretAuthentication,
        request: TokenExchangeRequest,
        client_secret_custody: &ClientSecretCustody<SS>,
        transport: &mut T,
        audit: &mut A,
    ) -> Result<TokenResponse, BrokerError>
    where
        SS: ClientSecretStore,
        T: OAuthClientSecretTokenExchangeTransport,
        A: OAuthClientSecretBrokerAuditSink,
    {
        let provider = request.provider().clone();
        let trace = request.trace();
        publish_broker(
            audit,
            &provider,
            trace,
            BrokerAuditAction::ClientSecretExchange,
            BrokerAuditOutcome::Attempted,
        )?;
        let result = self.send_client_secret_exchange_inner(
            authentication,
            request,
            client_secret_custody,
            transport,
            audit,
        );
        finish_broker(
            audit,
            &provider,
            trace,
            BrokerAuditAction::ClientSecretExchange,
            result,
        )
    }

    /// Authenticate, send, and classify one client-secret RFC 7009 revocation.
    ///
    /// The registered provider's exact client ID, revocation endpoint, and
    /// retained `client_secret_basic` or `client_secret_post` method are
    /// checked before client-secret custody access. Custody owns secret
    /// disclosure and zeroizing wire construction; the broker audit-brackets
    /// the injected transport, and the OAuth core audit-gates response
    /// classification. This operation does not delete any local credential.
    pub fn send_client_secret_revocation<SS, T, A>(
        &self,
        authentication: &ClientSecretAuthentication,
        request: TokenRevocationRequest,
        client_secret_custody: &ClientSecretCustody<SS>,
        transport: &mut T,
        audit: &mut A,
    ) -> Result<TokenRevocationResponse, BrokerError>
    where
        SS: ClientSecretStore,
        T: OAuthClientSecretTokenRevocationTransport,
        A: OAuthClientSecretBrokerAuditSink,
    {
        let provider = request.provider().clone();
        let trace = request.trace();
        publish_broker(
            audit,
            &provider,
            trace,
            BrokerAuditAction::ClientSecretRevocation,
            BrokerAuditOutcome::Attempted,
        )?;
        let result = self.send_client_secret_revocation_inner(
            authentication,
            request,
            client_secret_custody,
            transport,
            audit,
        );
        finish_broker(
            audit,
            &provider,
            trace,
            BrokerAuditAction::ClientSecretRevocation,
            result,
        )
    }

    /// Revoke one credential's exact stored refresh token, then delete that revision.
    ///
    /// The opaque account key selects both the registered provider and the
    /// credential record. The retained client-secret method and key binding are
    /// checked before the refresh token is read. Credential custody then releases
    /// that token and its exact revision into a zeroizing RFC 7009 request. Local
    /// deletion is reachable only after exact HTTP 200 has passed the transport,
    /// OAuth response, and broker audit gates; every other outcome leaves the
    /// credential record intact. Account-key selection remains caller-owned.
    pub fn revoke_refresh_token_and_delete_credentials<SS, T, A>(
        &self,
        key: &CredentialKey,
        authentication: &ClientSecretAuthentication,
        trace: OAuthTraceId,
        client_secret_custody: &ClientSecretCustody<SS>,
        transport: &mut T,
        audit: &mut A,
    ) -> Result<(), BrokerError>
    where
        SS: ClientSecretStore,
        T: OAuthClientSecretTokenRevocationTransport,
        A: OAuthClientSecretCredentialBrokerAuditSink,
    {
        let provider_id = key.provider().clone();
        publish_broker(
            audit,
            &provider_id,
            trace,
            BrokerAuditAction::ClientSecretRevocationCredentialDelete,
            BrokerAuditOutcome::Attempted,
        )?;
        let result = (|| {
            let provider = self.registered_provider(&provider_id)?.clone();
            let expected =
                provider.bind_client_secret_authentication(authentication.key().clone())?;
            if &expected != authentication || provider.config().revocation_endpoint().is_none() {
                return Err(BrokerError::BindingMismatch);
            }
            let (token, revision) = self
                .custody
                .with_refresh_token(key, trace, audit, |token, revision| {
                    (Zeroizing::new(token.to_owned()), revision)
                })
                .map_err(map_custody_error)?;
            let request = prepare_token_revocation(
                provider.config(),
                token,
                RevocationTokenHint::RefreshToken,
                trace,
            )
            .publish_then_release(audit)
            .map_err(map_oauth_error)?;
            let response = self.send_client_secret_revocation(
                authentication,
                request,
                client_secret_custody,
                transport,
                audit,
            )?;
            if response.provider() != &provider_id || response.trace() != trace {
                return Err(BrokerError::BindingMismatch);
            }
            self.custody
                .delete(key, revision, trace, audit)
                .map_err(map_custody_error)
        })();
        finish_broker(
            audit,
            &provider_id,
            trace,
            BrokerAuditAction::ClientSecretRevocationCredentialDelete,
            result,
        )
    }

    /// Exchange and persist one client-secret-authenticated credential response.
    ///
    /// The opaque account key must name the request provider before any
    /// client-secret, transport, clock, or credential-custody access. The
    /// response remains inside the broker and crosses the OAuth credential
    /// release gate directly into audited credential creation. Only the opaque
    /// storage revision is released.
    pub fn exchange_client_secret_and_store_credentials<SS, C, T, A>(
        &self,
        key: &CredentialKey,
        authentication: &ClientSecretAuthentication,
        request: TokenExchangeRequest,
        client_secret_custody: &ClientSecretCustody<SS>,
        execution: ClientSecretExchangeCredentialExecution<'_, C, T>,
        audit: &mut A,
    ) -> Result<CredentialRevision, BrokerError>
    where
        SS: ClientSecretStore,
        C: BrokerClock,
        T: OAuthClientSecretTokenExchangeTransport,
        A: OAuthClientSecretCredentialBrokerAuditSink,
    {
        let ClientSecretExchangeCredentialExecution { clock, transport } = execution;
        let provider = request.provider().clone();
        let trace = request.trace();
        publish_broker(
            audit,
            &provider,
            trace,
            BrokerAuditAction::ClientSecretExchangeCredentialCreate,
            BrokerAuditOutcome::Attempted,
        )?;
        let result = (|| {
            if key.provider() != &provider {
                return Err(BrokerError::BindingMismatch);
            }
            let response = self.send_client_secret_exchange(
                authentication,
                request,
                client_secret_custody,
                transport,
                audit,
            )?;
            self.store_initial_response(key, response, trace, clock, audit)
        })();
        finish_broker(
            audit,
            &provider,
            trace,
            BrokerAuditAction::ClientSecretExchangeCredentialCreate,
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

    /// Advance one caller-timed RFC 8628 polling sequence by at most one poll.
    ///
    /// `observed_at_seconds` uses the same caller-owned monotonic epoch supplied
    /// to [`DeviceFlowPollSequence::new`]. Early calls return `Waiting` without
    /// preparing a request or invoking transport. Calls at or after local
    /// expiry return `Expired` without transport. Continuation responses and
    /// transient transport failures schedule the next attempt from the time of
    /// this effect, so a delayed caller cannot trigger catch-up polling.
    pub fn advance_device_flow<T, A>(
        &self,
        sequence: DeviceFlowPollSequence,
        observed_at_seconds: u64,
        transport: &mut T,
        audit: &mut A,
    ) -> Result<DeviceFlowStepResult, BrokerError>
    where
        T: OAuthDeviceTokenTransport,
        A: OAuthBrokerAuditSink,
    {
        let provider = sequence.provider().clone();
        let trace = sequence.trace();
        publish_broker(
            audit,
            &provider,
            trace,
            BrokerAuditAction::DeviceFlowStep,
            BrokerAuditOutcome::Attempted,
        )?;
        let result =
            self.advance_device_flow_inner(sequence, observed_at_seconds, transport, audit);
        finish_broker(
            audit,
            &provider,
            trace,
            BrokerAuditAction::DeviceFlowStep,
            result,
        )
    }

    /// Advance one caller-timed device flow and persist an authorized response.
    ///
    /// The caller-selected opaque credential key must name the sequence provider
    /// before any protocol or transport effect. Continuation outcomes retain only
    /// the opaque sequence. An authorized token response crosses the existing
    /// OAuth release and credential-custody audit gates, and this method returns
    /// only the resulting opaque revision. The caller retains account-identity,
    /// monotonic-time, clock, waiting, and transport authority.
    pub fn advance_device_flow_and_store<T, C, A>(
        &self,
        key: &CredentialKey,
        sequence: DeviceFlowPollSequence,
        observed_at_seconds: u64,
        transport: &mut T,
        clock: &mut C,
        audit: &mut A,
    ) -> Result<DeviceFlowCredentialStepResult, BrokerError>
    where
        T: OAuthDeviceTokenTransport,
        C: BrokerClock,
        A: OAuthBrokerAuditSink,
    {
        let provider = sequence.provider().clone();
        let trace = sequence.trace();
        publish_broker(
            audit,
            &provider,
            trace,
            BrokerAuditAction::DeviceCredentialCreate,
            BrokerAuditOutcome::Attempted,
        )?;
        let result = (|| {
            if key.provider() != &provider {
                return Err(BrokerError::BindingMismatch);
            }
            match self.advance_device_flow(sequence, observed_at_seconds, transport, audit)? {
                DeviceFlowStepResult::Waiting(sequence) => {
                    Ok(DeviceFlowCredentialStepResult::Waiting(sequence))
                }
                DeviceFlowStepResult::Pending(sequence) => {
                    Ok(DeviceFlowCredentialStepResult::Pending(sequence))
                }
                DeviceFlowStepResult::SlowDown(sequence) => {
                    Ok(DeviceFlowCredentialStepResult::SlowDown(sequence))
                }
                DeviceFlowStepResult::TransportFailed(sequence) => {
                    Ok(DeviceFlowCredentialStepResult::TransportFailed(sequence))
                }
                DeviceFlowStepResult::Authorized(response) => self
                    .store_initial_response(key, response, trace, clock, audit)
                    .map(DeviceFlowCredentialStepResult::Stored),
                DeviceFlowStepResult::Denied => Ok(DeviceFlowCredentialStepResult::Denied),
                DeviceFlowStepResult::Expired => Ok(DeviceFlowCredentialStepResult::Expired),
            }
        })();
        finish_broker(
            audit,
            &provider,
            trace,
            BrokerAuditAction::DeviceCredentialCreate,
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

    fn advance_device_flow_inner<T, A>(
        &self,
        sequence: DeviceFlowPollSequence,
        observed_at_seconds: u64,
        transport: &mut T,
        audit: &mut A,
    ) -> Result<DeviceFlowStepResult, BrokerError>
    where
        T: OAuthDeviceTokenTransport,
        A: OAuthBrokerAuditSink,
    {
        let registered = self.registered_provider(sequence.session.provider())?;
        if !sequence.session.is_bound_to(registered.config()) {
            return Err(BrokerError::BindingMismatch);
        }
        if observed_at_seconds >= sequence.expires_at_seconds {
            return Ok(DeviceFlowStepResult::Expired);
        }
        if observed_at_seconds < sequence.next_poll_at_seconds {
            return Ok(DeviceFlowStepResult::Waiting(sequence));
        }

        let expires_at_seconds = sequence.expires_at_seconds;
        match self.poll_device_once(sequence.session, transport, audit)? {
            BrokerDevicePollResult::Response(DevicePollResult::Pending(session)) => {
                reschedule_device_flow(session, expires_at_seconds, observed_at_seconds).map(
                    |sequence| {
                        sequence
                            .map_or(DeviceFlowStepResult::Expired, DeviceFlowStepResult::Pending)
                    },
                )
            }
            BrokerDevicePollResult::Response(DevicePollResult::SlowDown(session)) => {
                reschedule_device_flow(session, expires_at_seconds, observed_at_seconds).map(
                    |sequence| {
                        sequence.map_or(
                            DeviceFlowStepResult::Expired,
                            DeviceFlowStepResult::SlowDown,
                        )
                    },
                )
            }
            BrokerDevicePollResult::TransportFailed(session) => {
                reschedule_device_flow(session, expires_at_seconds, observed_at_seconds).map(
                    |sequence| {
                        sequence.map_or(
                            DeviceFlowStepResult::Expired,
                            DeviceFlowStepResult::TransportFailed,
                        )
                    },
                )
            }
            BrokerDevicePollResult::Response(DevicePollResult::Authorized(response)) => {
                Ok(DeviceFlowStepResult::Authorized(response))
            }
            BrokerDevicePollResult::Response(DevicePollResult::Denied) => {
                Ok(DeviceFlowStepResult::Denied)
            }
            BrokerDevicePollResult::Response(DevicePollResult::ExpiredToken) => {
                Ok(DeviceFlowStepResult::Expired)
            }
        }
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

    fn send_client_secret_refresh_inner<SS, T, A>(
        &self,
        authentication: &ClientSecretAuthentication,
        request: TokenRefreshRequest,
        client_secret_custody: &ClientSecretCustody<SS>,
        transport: &mut T,
        audit: &mut A,
    ) -> Result<TokenResponse, BrokerError>
    where
        SS: ClientSecretStore,
        T: OAuthClientSecretTokenTransport,
        A: OAuthClientSecretBrokerAuditSink,
    {
        let provider = self.registered_provider(request.provider())?;
        let expected = provider.bind_client_secret_authentication(authentication.key().clone())?;
        if &expected != authentication
            || request.client_id() != provider.config().client_id()
            || request.endpoint() != provider.config().token_endpoint()
        {
            return Err(BrokerError::BindingMismatch);
        }
        let authenticated = client_secret_custody
            .authenticate_token_refresh(provider.config(), authentication, request, audit)
            .map_err(map_client_secret_custody_error)?;
        let context = authenticated.response_context().clone();
        let wire_response =
            send_client_secret_refresh_audited(transport, authenticated.request(), audit)?;
        let (status, body) = wire_response.into_parts();
        decode_token_response(context, status, provider.response_format(), body)
            .publish_then_release(audit)
            .map_err(map_oauth_error)
    }

    fn send_client_secret_exchange_inner<SS, T, A>(
        &self,
        authentication: &ClientSecretAuthentication,
        request: TokenExchangeRequest,
        client_secret_custody: &ClientSecretCustody<SS>,
        transport: &mut T,
        audit: &mut A,
    ) -> Result<TokenResponse, BrokerError>
    where
        SS: ClientSecretStore,
        T: OAuthClientSecretTokenExchangeTransport,
        A: OAuthClientSecretBrokerAuditSink,
    {
        let provider = self.registered_provider(request.provider())?;
        let expected = provider.bind_client_secret_authentication(authentication.key().clone())?;
        if &expected != authentication
            || request.client_id() != provider.config().client_id()
            || request.endpoint() != provider.config().token_endpoint()
        {
            return Err(BrokerError::BindingMismatch);
        }
        let authenticated = client_secret_custody
            .authenticate_token_exchange(provider.config(), authentication, request, audit)
            .map_err(map_client_secret_custody_error)?;
        let context = authenticated.response_context().clone();
        let wire_response =
            send_client_secret_exchange_audited(transport, authenticated.request(), audit)?;
        let (status, body) = wire_response.into_parts();
        decode_token_response(context, status, provider.response_format(), body)
            .publish_then_release(audit)
            .map_err(map_oauth_error)
    }

    fn send_client_secret_revocation_inner<SS, T, A>(
        &self,
        authentication: &ClientSecretAuthentication,
        request: TokenRevocationRequest,
        client_secret_custody: &ClientSecretCustody<SS>,
        transport: &mut T,
        audit: &mut A,
    ) -> Result<TokenRevocationResponse, BrokerError>
    where
        SS: ClientSecretStore,
        T: OAuthClientSecretTokenRevocationTransport,
        A: OAuthClientSecretBrokerAuditSink,
    {
        let provider = self.registered_provider(request.provider())?;
        let expected = provider.bind_client_secret_authentication(authentication.key().clone())?;
        if &expected != authentication
            || request.client_id() != provider.config().client_id()
            || Some(request.endpoint()) != provider.config().revocation_endpoint()
        {
            return Err(BrokerError::BindingMismatch);
        }
        let authenticated = client_secret_custody
            .authenticate_token_revocation(provider.config(), authentication, request, audit)
            .map_err(map_client_secret_custody_error)?;
        let context = authenticated.response_context().clone();
        let wire_response =
            send_client_secret_revocation_audited(transport, authenticated.request(), audit)?;
        let (status, body) = wire_response.into_parts();
        decode_token_revocation_response(context, status, body)
            .publish_then_release(audit)
            .map_err(map_oauth_error)
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

fn reschedule_device_flow(
    session: DevicePollingSession,
    expires_at_seconds: u64,
    observed_at_seconds: u64,
) -> Result<Option<DeviceFlowPollSequence>, BrokerError> {
    let next_poll_at_seconds = observed_at_seconds
        .checked_add(session.interval_seconds())
        .ok_or(BrokerError::Clock)?;
    if next_poll_at_seconds >= expires_at_seconds {
        return Ok(None);
    }
    Ok(Some(DeviceFlowPollSequence {
        session,
        expires_at_seconds,
        next_poll_at_seconds,
    }))
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

fn send_client_secret_refresh_audited<T: OAuthClientSecretTokenTransport, A: BrokerAuditSink>(
    transport: &mut T,
    request: &ClientSecretAuthenticatedRequest,
    audit: &mut A,
) -> Result<TokenEndpointResponse, BrokerError> {
    publish_broker(
        audit,
        request.provider(),
        request.trace(),
        BrokerAuditAction::TokenTransport,
        BrokerAuditOutcome::Attempted,
    )?;
    let result = transport
        .send_client_secret_refresh(request)
        .map_err(|_| BrokerError::Transport);
    finish_broker(
        audit,
        request.provider(),
        request.trace(),
        BrokerAuditAction::TokenTransport,
        result,
    )
}

fn send_client_secret_exchange_audited<
    T: OAuthClientSecretTokenExchangeTransport,
    A: BrokerAuditSink,
>(
    transport: &mut T,
    request: &ClientSecretAuthenticatedRequest,
    audit: &mut A,
) -> Result<TokenEndpointResponse, BrokerError> {
    publish_broker(
        audit,
        request.provider(),
        request.trace(),
        BrokerAuditAction::TokenTransport,
        BrokerAuditOutcome::Attempted,
    )?;
    let result = transport
        .send_client_secret_exchange(request)
        .map_err(|_| BrokerError::Transport);
    finish_broker(
        audit,
        request.provider(),
        request.trace(),
        BrokerAuditAction::TokenTransport,
        result,
    )
}

fn send_client_secret_revocation_audited<
    T: OAuthClientSecretTokenRevocationTransport,
    A: BrokerAuditSink,
>(
    transport: &mut T,
    request: &ClientSecretAuthenticatedRequest,
    audit: &mut A,
) -> Result<TokenRevocationEndpointResponse, BrokerError> {
    publish_broker(
        audit,
        request.provider(),
        request.trace(),
        BrokerAuditAction::TokenRevocationTransport,
        BrokerAuditOutcome::Attempted,
    )?;
    let result = transport
        .send_client_secret_revocation(request)
        .map_err(|_| BrokerError::Transport);
    finish_broker(
        audit,
        request.provider(),
        request.trace(),
        BrokerAuditAction::TokenRevocationTransport,
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

fn map_client_secret_custody_error(error: ClientSecretCustodyError) -> BrokerError {
    if error == ClientSecretCustodyError::Audit {
        BrokerError::Audit
    } else {
        BrokerError::ClientSecretCustody(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_oauth::{
        begin_authorization, complete_authorization, decode_authorization_server_metadata,
        decode_device_authorization_response, prepare_authorization_server_metadata,
        prepare_device_authorization, prepare_token_refresh, prepare_token_revocation,
        DeviceAuthorizationProfile, EntropySource, OAuthAuditAction, OAuthAuditError,
        OAuthAuditEvent, OAuthAuditOutcome, ProviderTokenRevocationError, RevocationTokenHint,
    };
    use coding_adventures_oauth_client_secret_custody::{
        ClientSecretAuditAction, ClientSecretAuditError, ClientSecretAuditEvent,
        ClientSecretAuditOutcome, ClientSecretReference, InMemoryClientSecretStore,
    };
    use coding_adventures_oauth_credential_custody::{
        AccountId, CredentialAuditAction, CredentialAuditError, CredentialAuditEvent,
        CredentialAuditOutcome, InMemoryCredentialStore,
    };
    use coding_adventures_oauth_private_key_signer::PrivateKeyReference;
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::rc::Rc;

    #[derive(Default)]
    struct RecordingAudit {
        broker: Vec<BrokerAuditEvent>,
        oauth: Vec<OAuthAuditEvent>,
        custody: Vec<CredentialAuditEvent>,
        client_secret: Vec<ClientSecretAuditEvent>,
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
                    (BrokerAuditAction::ClientSecretRefresh, BrokerAuditOutcome::Attempted) => {
                        "refresh-attempted"
                    }
                    (BrokerAuditAction::ClientSecretRefresh, BrokerAuditOutcome::Succeeded) => {
                        "refresh-succeeded"
                    }
                    (BrokerAuditAction::ClientSecretRefresh, BrokerAuditOutcome::Failed(_)) => {
                        "refresh-failed"
                    }
                    (
                        BrokerAuditAction::ClientSecretRefreshCredentialRotate,
                        BrokerAuditOutcome::Attempted,
                    ) => "refresh-rotate-attempted",
                    (
                        BrokerAuditAction::ClientSecretRefreshCredentialRotate,
                        BrokerAuditOutcome::Succeeded,
                    ) => "refresh-rotate-succeeded",
                    (
                        BrokerAuditAction::ClientSecretRefreshCredentialRotate,
                        BrokerAuditOutcome::Failed(_),
                    ) => "refresh-rotate-failed",
                    (BrokerAuditAction::ClientSecretExchange, BrokerAuditOutcome::Attempted) => {
                        "exchange-attempted"
                    }
                    (BrokerAuditAction::ClientSecretExchange, BrokerAuditOutcome::Succeeded) => {
                        "exchange-succeeded"
                    }
                    (BrokerAuditAction::ClientSecretExchange, BrokerAuditOutcome::Failed(_)) => {
                        "exchange-failed"
                    }
                    (BrokerAuditAction::ClientSecretRevocation, BrokerAuditOutcome::Attempted) => {
                        "revocation-attempted"
                    }
                    (BrokerAuditAction::ClientSecretRevocation, BrokerAuditOutcome::Succeeded) => {
                        "revocation-succeeded"
                    }
                    (BrokerAuditAction::ClientSecretRevocation, BrokerAuditOutcome::Failed(_)) => {
                        "revocation-failed"
                    }
                    (
                        BrokerAuditAction::ClientSecretRevocationCredentialDelete,
                        BrokerAuditOutcome::Attempted,
                    ) => "revoke-delete-attempted",
                    (
                        BrokerAuditAction::ClientSecretRevocationCredentialDelete,
                        BrokerAuditOutcome::Succeeded,
                    ) => "revoke-delete-succeeded",
                    (
                        BrokerAuditAction::ClientSecretRevocationCredentialDelete,
                        BrokerAuditOutcome::Failed(_),
                    ) => "revoke-delete-failed",
                    (
                        BrokerAuditAction::ClientSecretExchangeCredentialCreate,
                        BrokerAuditOutcome::Attempted,
                    ) => "exchange-store-attempted",
                    (
                        BrokerAuditAction::ClientSecretExchangeCredentialCreate,
                        BrokerAuditOutcome::Succeeded,
                    ) => "exchange-store-succeeded",
                    (
                        BrokerAuditAction::ClientSecretExchangeCredentialCreate,
                        BrokerAuditOutcome::Failed(_),
                    ) => "exchange-store-failed",
                    (BrokerAuditAction::CredentialCreate, BrokerAuditOutcome::Attempted) => {
                        "credential-create-attempted"
                    }
                    (BrokerAuditAction::CredentialCreate, BrokerAuditOutcome::Succeeded) => {
                        "credential-create-succeeded"
                    }
                    (BrokerAuditAction::CredentialCreate, BrokerAuditOutcome::Failed(_)) => {
                        "credential-create-failed"
                    }
                    (BrokerAuditAction::TokenTransport, BrokerAuditOutcome::Attempted) => {
                        "transport-attempted"
                    }
                    (BrokerAuditAction::TokenTransport, BrokerAuditOutcome::Succeeded) => {
                        "transport-succeeded"
                    }
                    (BrokerAuditAction::TokenTransport, BrokerAuditOutcome::Failed(_)) => {
                        "transport-failed"
                    }
                    (
                        BrokerAuditAction::TokenRevocationTransport,
                        BrokerAuditOutcome::Attempted,
                    ) => "revocation-transport-attempted",
                    (
                        BrokerAuditAction::TokenRevocationTransport,
                        BrokerAuditOutcome::Succeeded,
                    ) => "revocation-transport-succeeded",
                    (
                        BrokerAuditAction::TokenRevocationTransport,
                        BrokerAuditOutcome::Failed(_),
                    ) => "revocation-transport-failed",
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
            if let Some(order) = &self.order {
                let label = match (event.action(), event.outcome()) {
                    (CredentialAuditAction::Create, CredentialAuditOutcome::Attempted) => {
                        "credential-store-attempted"
                    }
                    (CredentialAuditAction::Create, CredentialAuditOutcome::Succeeded) => {
                        "credential-store-succeeded"
                    }
                    (CredentialAuditAction::Create, CredentialAuditOutcome::Failed(_)) => {
                        "credential-store-failed"
                    }
                    (CredentialAuditAction::RefreshToken, CredentialAuditOutcome::Attempted) => {
                        "refresh-token-access-attempted"
                    }
                    (CredentialAuditAction::RefreshToken, CredentialAuditOutcome::Succeeded) => {
                        "refresh-token-access-succeeded"
                    }
                    (CredentialAuditAction::RefreshToken, CredentialAuditOutcome::Failed(_)) => {
                        "refresh-token-access-failed"
                    }
                    (CredentialAuditAction::Rotate, CredentialAuditOutcome::Attempted) => {
                        "credential-rotate-attempted"
                    }
                    (CredentialAuditAction::Rotate, CredentialAuditOutcome::Succeeded) => {
                        "credential-rotate-succeeded"
                    }
                    (CredentialAuditAction::Rotate, CredentialAuditOutcome::Failed(_)) => {
                        "credential-rotate-failed"
                    }
                    (CredentialAuditAction::Delete, CredentialAuditOutcome::Attempted) => {
                        "credential-delete-attempted"
                    }
                    (CredentialAuditAction::Delete, CredentialAuditOutcome::Succeeded) => {
                        "credential-delete-succeeded"
                    }
                    (CredentialAuditAction::Delete, CredentialAuditOutcome::Failed(_)) => {
                        "credential-delete-failed"
                    }
                    _ => "other-credential-audit",
                };
                order.borrow_mut().push(label);
            }
            self.custody.push(event.clone());
            Ok(())
        }
    }

    impl ClientSecretAuditSink for RecordingAudit {
        fn publish(
            &mut self,
            event: &ClientSecretAuditEvent,
        ) -> Result<(), ClientSecretAuditError> {
            if let Some(order) = &self.order {
                let label = match (event.action(), event.outcome()) {
                    (ClientSecretAuditAction::Access, ClientSecretAuditOutcome::Attempted) => {
                        "secret-access-attempted"
                    }
                    (ClientSecretAuditAction::Access, ClientSecretAuditOutcome::Succeeded) => {
                        "secret-access-succeeded"
                    }
                    _ => "other-secret-audit",
                };
                order.borrow_mut().push(label);
            }
            self.client_secret.push(event.clone());
            Ok(())
        }
    }

    struct FixedClock(u64);

    impl BrokerClock for FixedClock {
        fn now_unix_seconds(&mut self) -> Result<u64, BrokerClockError> {
            Ok(self.0)
        }
    }

    struct FixedEntropy([u8; 64]);

    impl EntropySource for FixedEntropy {
        fn fill(&mut self, destination: &mut [u8]) -> Result<(), OAuthError> {
            destination.copy_from_slice(&self.0[..destination.len()]);
            Ok(())
        }
    }

    struct CountingClock {
        now: u64,
        calls: usize,
    }

    impl BrokerClock for CountingClock {
        fn now_unix_seconds(&mut self) -> Result<u64, BrokerClockError> {
            self.calls += 1;
            Ok(self.now)
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

    struct MockClientSecretTransport {
        response: Option<TokenEndpointResponse>,
        expected_authorization_header: bool,
        expected_refresh: &'static str,
        calls: usize,
        order: Rc<RefCell<Vec<&'static str>>>,
    }

    impl OAuthClientSecretTokenTransport for MockClientSecretTransport {
        fn send_client_secret_refresh(
            &mut self,
            request: &ClientSecretAuthenticatedRequest,
        ) -> Result<TokenEndpointResponse, TokenTransportError> {
            self.calls += 1;
            self.order.borrow_mut().push("transport-effect");
            assert_eq!(request.provider().as_str(), "fixture-confidential");
            assert_eq!(
                request.endpoint(),
                "https://token.fixture-confidential.example/token"
            );
            assert_eq!(
                request.authorization_header().is_some(),
                self.expected_authorization_header
            );
            assert!(request
                .form_body()
                .contains(&format!("refresh_token={}", self.expected_refresh)));
            assert_eq!(
                request.form_body().contains("client_secret=client+secret"),
                !self.expected_authorization_header
            );
            self.response.take().ok_or(TokenTransportError)
        }
    }

    struct MockClientSecretExchangeTransport {
        response: Option<TokenEndpointResponse>,
        expected_authorization_header: bool,
        calls: usize,
        order: Rc<RefCell<Vec<&'static str>>>,
    }

    impl OAuthClientSecretTokenExchangeTransport for MockClientSecretExchangeTransport {
        fn send_client_secret_exchange(
            &mut self,
            request: &ClientSecretAuthenticatedRequest,
        ) -> Result<TokenEndpointResponse, TokenTransportError> {
            self.calls += 1;
            self.order.borrow_mut().push("transport-effect");
            assert_eq!(request.provider().as_str(), "fixture-confidential");
            assert_eq!(
                request.endpoint(),
                "https://token.fixture-confidential.example/token"
            );
            assert_eq!(
                request.authorization_header().is_some(),
                self.expected_authorization_header
            );
            assert!(request
                .form_body()
                .contains("grant_type=authorization_code"));
            assert!(request.form_body().contains("code=code-value"));
            assert_eq!(
                request.form_body().contains("client_secret=client+secret"),
                !self.expected_authorization_header
            );
            self.response.take().ok_or(TokenTransportError)
        }
    }

    struct MockClientSecretRevocationTransport {
        response: Option<Result<TokenRevocationEndpointResponse, TokenTransportError>>,
        expected_authorization_header: bool,
        expected_token: &'static str,
        expected_hint: RevocationTokenHint,
        calls: usize,
        order: Rc<RefCell<Vec<&'static str>>>,
    }

    impl OAuthClientSecretTokenRevocationTransport for MockClientSecretRevocationTransport {
        fn send_client_secret_revocation(
            &mut self,
            request: &ClientSecretAuthenticatedRequest,
        ) -> Result<TokenRevocationEndpointResponse, TokenTransportError> {
            self.calls += 1;
            self.order.borrow_mut().push("revocation-transport-effect");
            assert_eq!(request.provider().as_str(), "fixture-confidential");
            assert_eq!(
                request.endpoint(),
                "https://token.fixture-confidential.example/revoke"
            );
            assert_eq!(
                request.authorization_header().is_some(),
                self.expected_authorization_header
            );
            assert!(request
                .form_body()
                .contains(&format!("token={}", self.expected_token)));
            assert!(request.form_body().contains(&format!(
                "token_type_hint={}",
                match self.expected_hint {
                    RevocationTokenHint::AccessToken => "access_token",
                    RevocationTokenHint::RefreshToken => "refresh_token",
                }
            )));
            assert_eq!(
                request.form_body().contains("client_secret=client+secret"),
                !self.expected_authorization_header
            );
            self.response.take().unwrap_or(Err(TokenTransportError))
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

    impl ConfidentialProviderDataSource for MockProviderDataSource {
        fn load_confidential_provider_data(
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

    fn prepared_exchange(
        config: &ProviderConfig,
        operation_trace: OAuthTraceId,
        audit: &mut impl OAuthAuditSink,
    ) -> TokenExchangeRequest {
        let begin = begin_authorization(
            config,
            &["files.read"],
            operation_trace,
            &mut FixedEntropy([0x29; 64]),
        )
        .publish_then_release(audit)
        .unwrap();
        let state = begin
            .url()
            .as_str()
            .split('&')
            .find_map(|parameter| parameter.strip_prefix("state="))
            .unwrap()
            .to_owned();
        let (_, transaction) = begin.into_parts();
        complete_authorization(
            transaction,
            &format!("{}?code=code-value&state={state}", config.redirect_uri()),
        )
        .publish_then_release(audit)
        .unwrap()
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

    fn confidential_provider_data(name: &str) -> Zeroizing<Vec<u8>> {
        Zeroizing::new(
            format!(
                r#"{{
                    "schema_version":1,
                    "provider":"{name}",
                    "authorization_endpoint":"https://auth.{name}.example/authorize",
                    "token_endpoint":"https://token.{name}.example/token",
                    "mix_up_defense":{{"kind":"authorization_response_issuer","issuer":"https://auth.{name}.example"}},
                    "authorization_extra_parameters":{{}},
                    "token_response_format":"json",
                    "token_endpoint_auth_method":"private_key_jwt",
                    "token_endpoint_auth_signing_alg_values_supported":["EdDSA","RS256"],
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
    fn confidential_provider_data_load_retains_policy_after_audited_source_read() {
        let custody = CredentialCustody::new(InMemoryCredentialStore::new());
        let mut broker = OAuthBroker::new(custody);
        let requested = ProviderId::new("fixture-confidential").unwrap();
        let order = Rc::new(RefCell::new(Vec::new()));
        let mut source =
            MockProviderDataSource::successful(confidential_provider_data("fixture-confidential"));
        source.order = Some(Rc::clone(&order));
        let mut audit = RecordingAudit {
            order: Some(Rc::clone(&order)),
            ..RecordingAudit::default()
        };

        broker
            .load_and_register_confidential_provider(
                &requested,
                "fixture-service-client",
                "https://service.fixture.example/oauth/callback",
                trace(32),
                &mut source,
                &mut audit,
            )
            .unwrap();

        assert_eq!(source.calls, 1);
        assert_eq!(
            source.requested.as_slice(),
            std::slice::from_ref(&requested)
        );
        let provider = broker.registered_provider(&requested).unwrap();
        assert_eq!(provider.config().client_id(), "fixture-service-client");
        assert_eq!(
            provider.confidential_authentication_method(),
            Some(ConfidentialClientAuthenticationMethod::PrivateKeyJwt)
        );
        assert_eq!(
            provider.client_authentication_signing_algorithms(),
            ["EdDSA", "RS256"]
        );
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
            .all(|event| { event.provider() == &requested && event.trace() == trace(32) }));
    }

    #[test]
    fn confidential_provider_binds_only_its_exact_retained_client_secret_method() {
        let provider = ProviderId::new("fixture-confidential").unwrap();
        let key = || ClientSecretKey::new(provider.clone(), ClientSecretReference::new([0x51; 32]));

        for (retained, expected) in [
            (
                ConfidentialClientAuthenticationMethod::ClientSecretBasic,
                ClientSecretAuthenticationMethod::ClientSecretBasic,
            ),
            (
                ConfidentialClientAuthenticationMethod::ClientSecretPost,
                ClientSecretAuthenticationMethod::ClientSecretPost,
            ),
        ] {
            let policy = BrokerProvider::new_confidential(
                config("fixture-confidential"),
                TokenResponseFormat::Json,
                300,
                retained,
                Vec::new(),
            )
            .unwrap();
            let authentication = policy
                .bind_client_secret_authentication(key())
                .expect("retained client-secret method");
            assert_eq!(authentication.key().provider(), &provider);
            assert_eq!(authentication.method(), expected);
        }

        let wrong_provider_key = ClientSecretKey::new(
            ProviderId::new("other-provider").unwrap(),
            ClientSecretReference::new([0x52; 32]),
        );
        let basic = BrokerProvider::new_confidential(
            config("fixture-confidential"),
            TokenResponseFormat::Json,
            300,
            ConfidentialClientAuthenticationMethod::ClientSecretBasic,
            Vec::new(),
        )
        .unwrap();
        assert_eq!(
            basic.bind_client_secret_authentication(wrong_provider_key),
            Err(BrokerError::BindingMismatch)
        );

        let public = policy("fixture-confidential", 300);
        assert_eq!(
            public.bind_client_secret_authentication(key()),
            Err(BrokerError::BindingMismatch)
        );
        let private_key_jwt = BrokerProvider::new_confidential(
            config("fixture-confidential"),
            TokenResponseFormat::Json,
            300,
            ConfidentialClientAuthenticationMethod::PrivateKeyJwt,
            vec!["EdDSA".to_owned()],
        )
        .unwrap();
        assert_eq!(
            private_key_jwt.bind_client_secret_authentication(key()),
            Err(BrokerError::BindingMismatch)
        );
    }

    #[test]
    fn confidential_provider_binds_only_its_exact_retained_private_key_profile() {
        let provider = ProviderId::new("fixture-confidential").unwrap();
        let key = || PrivateKeyId::new(provider.clone(), PrivateKeyReference::new([0x61; 32]));
        let algorithm = |name| PrivateKeyJwtAlgorithm::new(name).unwrap();
        let private_key_jwt = BrokerProvider::new_confidential(
            config("fixture-confidential"),
            TokenResponseFormat::Json,
            300,
            ConfidentialClientAuthenticationMethod::PrivateKeyJwt,
            vec!["EdDSA".to_owned(), "RS256".to_owned()],
        )
        .unwrap();

        let profile = private_key_jwt
            .bind_private_key_jwt_profile(
                key(),
                algorithm("EdDSA"),
                Some("active-key".to_owned()),
                60,
            )
            .expect("retained private-key JWT profile");
        assert_eq!(profile.provider(), &provider);
        assert_eq!(profile.algorithm().as_str(), "EdDSA");

        let wrong_provider_key = PrivateKeyId::new(
            ProviderId::new("other-provider").unwrap(),
            PrivateKeyReference::new([0x62; 32]),
        );
        assert!(matches!(
            private_key_jwt.bind_private_key_jwt_profile(
                wrong_provider_key,
                algorithm("EdDSA"),
                None,
                60,
            ),
            Err(BrokerError::BindingMismatch)
        ));
        assert!(matches!(
            private_key_jwt.bind_private_key_jwt_profile(key(), algorithm("ES256"), None, 60),
            Err(BrokerError::BindingMismatch)
        ));

        let public = policy("fixture-confidential", 300);
        assert!(matches!(
            public.bind_private_key_jwt_profile(key(), algorithm("EdDSA"), None, 60),
            Err(BrokerError::BindingMismatch)
        ));
        let client_secret = BrokerProvider::new_confidential(
            config("fixture-confidential"),
            TokenResponseFormat::Json,
            300,
            ConfidentialClientAuthenticationMethod::ClientSecretBasic,
            Vec::new(),
        )
        .unwrap();
        assert!(matches!(
            client_secret.bind_private_key_jwt_profile(key(), algorithm("EdDSA"), None, 60),
            Err(BrokerError::BindingMismatch)
        ));
        assert!(matches!(
            private_key_jwt.bind_private_key_jwt_profile(
                key(),
                algorithm("RS256"),
                Some(String::new()),
                60,
            ),
            Err(BrokerError::InvalidPolicy)
        ));
        assert!(matches!(
            private_key_jwt.bind_private_key_jwt_profile(key(), algorithm("RS256"), None, 0),
            Err(BrokerError::InvalidPolicy)
        ));
    }

    #[test]
    fn client_secret_refresh_uses_exact_retained_method_and_audit_order() {
        for (method, expects_basic_header) in [
            (
                ConfidentialClientAuthenticationMethod::ClientSecretBasic,
                true,
            ),
            (
                ConfidentialClientAuthenticationMethod::ClientSecretPost,
                false,
            ),
        ] {
            let provider_config = config("fixture-confidential");
            let secret_key = ClientSecretKey::new(
                provider_config.provider().clone(),
                ClientSecretReference::new([0x71; 32]),
            );
            let provider = BrokerProvider::new_confidential(
                provider_config.clone(),
                TokenResponseFormat::Json,
                300,
                method,
                Vec::new(),
            )
            .unwrap();
            let authentication = provider
                .bind_client_secret_authentication(secret_key.clone())
                .unwrap();
            let mut broker =
                OAuthBroker::new(CredentialCustody::new(InMemoryCredentialStore::new()));
            let order = Rc::new(RefCell::new(Vec::new()));
            let mut audit = RecordingAudit {
                order: Some(order.clone()),
                ..RecordingAudit::default()
            };
            broker
                .register_provider(provider, trace(40), &mut audit)
                .unwrap();
            let client_secret_custody = ClientSecretCustody::new(InMemoryClientSecretStore::new());
            client_secret_custody
                .create(
                    &secret_key,
                    Zeroizing::new("client secret".to_owned()),
                    trace(40),
                    &mut audit,
                )
                .unwrap();
            let request = prepare_token_refresh(
                &provider_config,
                Zeroizing::new("refresh-secret".to_owned()),
                &[],
                trace(41),
            )
            .publish_then_release(&mut audit)
            .unwrap();
            order.borrow_mut().clear();
            let mut transport = MockClientSecretTransport {
                response: Some(
                    TokenEndpointResponse::new(
                        200,
                        Zeroizing::new(
                            br#"{"access_token":"fresh","token_type":"Bearer"}"#.to_vec(),
                        ),
                    )
                    .unwrap(),
                ),
                expected_authorization_header: expects_basic_header,
                expected_refresh: "refresh-secret",
                calls: 0,
                order: order.clone(),
            };

            let response = broker
                .send_client_secret_refresh(
                    &authentication,
                    request,
                    &client_secret_custody,
                    &mut transport,
                    &mut audit,
                )
                .unwrap();

            assert_eq!(response.provider(), provider_config.provider());
            assert_eq!(response.trace(), trace(41));
            assert_eq!(response.token_type(), "Bearer");
            assert_eq!(transport.calls, 1);
            assert_eq!(
                order.borrow().as_slice(),
                [
                    "refresh-attempted",
                    "secret-access-attempted",
                    "secret-access-succeeded",
                    "transport-attempted",
                    "transport-effect",
                    "transport-succeeded",
                    "refresh-succeeded",
                ]
            );
        }
    }

    #[test]
    fn client_secret_revocation_uses_exact_retained_method_and_audit_order() {
        for (method, expects_basic_header) in [
            (
                ConfidentialClientAuthenticationMethod::ClientSecretBasic,
                true,
            ),
            (
                ConfidentialClientAuthenticationMethod::ClientSecretPost,
                false,
            ),
        ] {
            let provider_config = config("fixture-confidential")
                .with_revocation_endpoint("https://token.fixture-confidential.example/revoke")
                .unwrap();
            let secret_key = ClientSecretKey::new(
                provider_config.provider().clone(),
                ClientSecretReference::new([0x77; 32]),
            );
            let provider = BrokerProvider::new_confidential(
                provider_config.clone(),
                TokenResponseFormat::Json,
                300,
                method,
                Vec::new(),
            )
            .unwrap();
            let authentication = provider
                .bind_client_secret_authentication(secret_key.clone())
                .unwrap();
            let mut broker =
                OAuthBroker::new(CredentialCustody::new(InMemoryCredentialStore::new()));
            let order = Rc::new(RefCell::new(Vec::new()));
            let mut audit = RecordingAudit {
                order: Some(order.clone()),
                ..RecordingAudit::default()
            };
            broker
                .register_provider(provider, trace(53), &mut audit)
                .unwrap();
            let client_secret_custody = ClientSecretCustody::new(InMemoryClientSecretStore::new());
            client_secret_custody
                .create(
                    &secret_key,
                    Zeroizing::new("client secret".to_owned()),
                    trace(53),
                    &mut audit,
                )
                .unwrap();
            let request = prepare_token_revocation(
                &provider_config,
                Zeroizing::new("access-secret".to_owned()),
                RevocationTokenHint::AccessToken,
                trace(54),
            )
            .publish_then_release(&mut audit)
            .unwrap();
            let oauth_events_before = audit.oauth.len();
            order.borrow_mut().clear();
            let mut transport = MockClientSecretRevocationTransport {
                response: Some(Ok(TokenRevocationEndpointResponse::new(
                    200,
                    Zeroizing::new(Vec::new()),
                )
                .unwrap())),
                expected_authorization_header: expects_basic_header,
                expected_token: "access-secret",
                expected_hint: RevocationTokenHint::AccessToken,
                calls: 0,
                order: order.clone(),
            };

            let response = broker
                .send_client_secret_revocation(
                    &authentication,
                    request,
                    &client_secret_custody,
                    &mut transport,
                    &mut audit,
                )
                .unwrap();

            assert_eq!(response.provider(), provider_config.provider());
            assert_eq!(response.trace(), trace(54));
            assert_eq!(transport.calls, 1);
            assert_eq!(
                audit.oauth[oauth_events_before..]
                    .iter()
                    .map(OAuthAuditEvent::action)
                    .collect::<Vec<_>>(),
                vec![OAuthAuditAction::TokenRevocationResponseClassify]
            );
            assert_eq!(
                order.borrow().as_slice(),
                [
                    "revocation-attempted",
                    "secret-access-attempted",
                    "secret-access-succeeded",
                    "revocation-transport-attempted",
                    "revocation-transport-effect",
                    "revocation-transport-succeeded",
                    "revocation-succeeded",
                ]
            );
        }
    }

    #[test]
    fn client_secret_revocation_binding_and_retryable_failure_stay_closed() {
        let provider_config = config("fixture-confidential")
            .with_revocation_endpoint("https://token.fixture-confidential.example/revoke")
            .unwrap();
        let secret_key = ClientSecretKey::new(
            provider_config.provider().clone(),
            ClientSecretReference::new([0x78; 32]),
        );
        let provider = BrokerProvider::new_confidential(
            provider_config.clone(),
            TokenResponseFormat::Json,
            300,
            ConfidentialClientAuthenticationMethod::ClientSecretBasic,
            Vec::new(),
        )
        .unwrap();
        let authentication = provider
            .bind_client_secret_authentication(secret_key.clone())
            .unwrap();
        let mut broker = OAuthBroker::new(CredentialCustody::new(InMemoryCredentialStore::new()));
        let order = Rc::new(RefCell::new(Vec::new()));
        let mut audit = RecordingAudit {
            order: Some(order.clone()),
            ..RecordingAudit::default()
        };
        broker
            .register_provider(provider, trace(55), &mut audit)
            .unwrap();
        let client_secret_custody = ClientSecretCustody::new(InMemoryClientSecretStore::new());
        client_secret_custody
            .create(
                &secret_key,
                Zeroizing::new("client secret".to_owned()),
                trace(55),
                &mut audit,
            )
            .unwrap();

        let other_endpoint_config = config("fixture-confidential")
            .with_revocation_endpoint("https://other.fixture-confidential.example/revoke")
            .unwrap();
        let mismatched = prepare_token_revocation(
            &other_endpoint_config,
            Zeroizing::new("access-secret".to_owned()),
            RevocationTokenHint::AccessToken,
            trace(56),
        )
        .publish_then_release(&mut audit)
        .unwrap();
        let secret_events_before = audit.client_secret.len();
        order.borrow_mut().clear();
        let mut transport = MockClientSecretRevocationTransport {
            response: None,
            expected_authorization_header: true,
            expected_token: "access-secret",
            expected_hint: RevocationTokenHint::AccessToken,
            calls: 0,
            order: order.clone(),
        };
        assert!(matches!(
            broker.send_client_secret_revocation(
                &authentication,
                mismatched,
                &client_secret_custody,
                &mut transport,
                &mut audit,
            ),
            Err(BrokerError::BindingMismatch)
        ));
        assert_eq!(audit.client_secret.len(), secret_events_before);
        assert_eq!(transport.calls, 0);
        assert_eq!(
            order.borrow().as_slice(),
            ["revocation-attempted", "revocation-failed"]
        );

        let retryable = prepare_token_revocation(
            &provider_config,
            Zeroizing::new("access-secret".to_owned()),
            RevocationTokenHint::AccessToken,
            trace(57),
        )
        .publish_then_release(&mut audit)
        .unwrap();
        order.borrow_mut().clear();
        let mut transport = MockClientSecretRevocationTransport {
            response: Some(Ok(TokenRevocationEndpointResponse::new(
                503,
                Zeroizing::new(Vec::new()),
            )
            .unwrap())),
            expected_authorization_header: true,
            expected_token: "access-secret",
            expected_hint: RevocationTokenHint::AccessToken,
            calls: 0,
            order: order.clone(),
        };
        assert!(matches!(
            broker.send_client_secret_revocation(
                &authentication,
                retryable,
                &client_secret_custody,
                &mut transport,
                &mut audit,
            ),
            Err(BrokerError::Protocol(OAuthError::TokenRevocationEndpoint(
                ProviderTokenRevocationError::TemporarilyUnavailable
            )))
        ));
        assert_eq!(transport.calls, 1);
        assert_eq!(
            order.borrow().as_slice(),
            [
                "revocation-attempted",
                "secret-access-attempted",
                "secret-access-succeeded",
                "revocation-transport-attempted",
                "revocation-transport-effect",
                "revocation-transport-succeeded",
                "revocation-failed",
            ]
        );
    }

    #[test]
    fn exact_stored_refresh_token_is_revoked_before_credential_delete() {
        let provider_config = config("fixture-confidential")
            .with_revocation_endpoint("https://token.fixture-confidential.example/revoke")
            .unwrap();
        let secret_key = ClientSecretKey::new(
            provider_config.provider().clone(),
            ClientSecretReference::new([0x79; 32]),
        );
        let provider = BrokerProvider::new_confidential(
            provider_config.clone(),
            TokenResponseFormat::Json,
            300,
            ConfidentialClientAuthenticationMethod::ClientSecretBasic,
            Vec::new(),
        )
        .unwrap();
        let authentication = provider
            .bind_client_secret_authentication(secret_key.clone())
            .unwrap();
        let key = key("fixture-confidential");
        let order = Rc::new(RefCell::new(Vec::new()));
        let mut audit = RecordingAudit {
            order: Some(order.clone()),
            ..RecordingAudit::default()
        };
        let response = decoded_refresh_response(
            &provider_config,
            trace(58),
            r#"{"access_token":"access-one","refresh_token":"refresh-one","token_type":"Bearer"}"#,
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
                CredentialMetadata::new("Bearer", None, Vec::new()).unwrap(),
                trace(58),
                &mut audit,
            )
            .unwrap();
        let mut broker = OAuthBroker::new(custody);
        broker
            .register_provider(provider, trace(58), &mut audit)
            .unwrap();
        let client_secret_custody = ClientSecretCustody::new(InMemoryClientSecretStore::new());
        client_secret_custody
            .create(
                &secret_key,
                Zeroizing::new("client secret".to_owned()),
                trace(58),
                &mut audit,
            )
            .unwrap();

        order.borrow_mut().clear();
        let custody_events_before = audit.custody.len();
        let client_secret_events_before = audit.client_secret.len();
        let wrong_authentication = ClientSecretAuthentication::new(
            secret_key.clone(),
            ClientSecretAuthenticationMethod::ClientSecretPost,
        );
        let mut rejected_transport = MockClientSecretRevocationTransport {
            response: None,
            expected_authorization_header: true,
            expected_token: "refresh-one",
            expected_hint: RevocationTokenHint::RefreshToken,
            calls: 0,
            order: order.clone(),
        };
        assert_eq!(
            broker.revoke_refresh_token_and_delete_credentials(
                &key,
                &wrong_authentication,
                trace(59),
                &client_secret_custody,
                &mut rejected_transport,
                &mut audit,
            ),
            Err(BrokerError::BindingMismatch)
        );
        assert_eq!(audit.custody.len(), custody_events_before);
        assert_eq!(audit.client_secret.len(), client_secret_events_before);
        assert_eq!(rejected_transport.calls, 0);
        assert_eq!(
            order.borrow().as_slice(),
            ["revoke-delete-attempted", "revoke-delete-failed"]
        );

        order.borrow_mut().clear();
        let custody_events_before = audit.custody.len();
        let mut retryable_transport = MockClientSecretRevocationTransport {
            response: Some(Ok(TokenRevocationEndpointResponse::new(
                503,
                Zeroizing::new(Vec::new()),
            )
            .unwrap())),
            expected_authorization_header: true,
            expected_token: "refresh-one",
            expected_hint: RevocationTokenHint::RefreshToken,
            calls: 0,
            order: order.clone(),
        };
        assert!(matches!(
            broker.revoke_refresh_token_and_delete_credentials(
                &key,
                &authentication,
                trace(60),
                &client_secret_custody,
                &mut retryable_transport,
                &mut audit,
            ),
            Err(BrokerError::Protocol(OAuthError::TokenRevocationEndpoint(
                ProviderTokenRevocationError::TemporarilyUnavailable
            )))
        ));
        assert!(audit.custody[custody_events_before..]
            .iter()
            .all(|event| event.action() != CredentialAuditAction::Delete));
        assert_eq!(retryable_transport.calls, 1);
        assert_eq!(
            broker
                .custody
                .with_access_token(&key, trace(60), &mut audit, |token| token.to_owned())
                .unwrap(),
            "access-one"
        );

        order.borrow_mut().clear();
        let custody_events_before = audit.custody.len();
        let mut successful_transport = MockClientSecretRevocationTransport {
            response: Some(Ok(TokenRevocationEndpointResponse::new(
                200,
                Zeroizing::new(Vec::new()),
            )
            .unwrap())),
            expected_authorization_header: true,
            expected_token: "refresh-one",
            expected_hint: RevocationTokenHint::RefreshToken,
            calls: 0,
            order: order.clone(),
        };
        broker
            .revoke_refresh_token_and_delete_credentials(
                &key,
                &authentication,
                trace(61),
                &client_secret_custody,
                &mut successful_transport,
                &mut audit,
            )
            .unwrap();
        assert_eq!(successful_transport.calls, 1);
        assert_eq!(
            audit.custody[custody_events_before..]
                .iter()
                .map(|event| (event.action(), event.outcome()))
                .collect::<Vec<_>>(),
            vec![
                (
                    CredentialAuditAction::RefreshToken,
                    CredentialAuditOutcome::Attempted,
                ),
                (
                    CredentialAuditAction::RefreshToken,
                    CredentialAuditOutcome::Succeeded,
                ),
                (
                    CredentialAuditAction::Delete,
                    CredentialAuditOutcome::Attempted,
                ),
                (
                    CredentialAuditAction::Delete,
                    CredentialAuditOutcome::Succeeded,
                ),
            ]
        );
        assert_eq!(
            order.borrow().as_slice(),
            [
                "revoke-delete-attempted",
                "refresh-token-access-attempted",
                "refresh-token-access-succeeded",
                "revocation-attempted",
                "secret-access-attempted",
                "secret-access-succeeded",
                "revocation-transport-attempted",
                "revocation-transport-effect",
                "revocation-transport-succeeded",
                "revocation-succeeded",
                "credential-delete-attempted",
                "credential-delete-succeeded",
                "revoke-delete-succeeded",
            ]
        );
        assert_eq!(
            broker
                .custody
                .with_access_token(&key, trace(61), &mut audit, |_| ()),
            Err(CustodyError::NotFound)
        );
    }

    #[test]
    fn client_secret_exchange_uses_exact_retained_method_and_audit_order() {
        for (method, expects_basic_header) in [
            (
                ConfidentialClientAuthenticationMethod::ClientSecretBasic,
                true,
            ),
            (
                ConfidentialClientAuthenticationMethod::ClientSecretPost,
                false,
            ),
        ] {
            let provider_config = config("fixture-confidential");
            let secret_key = ClientSecretKey::new(
                provider_config.provider().clone(),
                ClientSecretReference::new([0x73; 32]),
            );
            let provider = BrokerProvider::new_confidential(
                provider_config.clone(),
                TokenResponseFormat::Json,
                300,
                method,
                Vec::new(),
            )
            .unwrap();
            let authentication = provider
                .bind_client_secret_authentication(secret_key.clone())
                .unwrap();
            let mut broker =
                OAuthBroker::new(CredentialCustody::new(InMemoryCredentialStore::new()));
            let order = Rc::new(RefCell::new(Vec::new()));
            let mut audit = RecordingAudit {
                order: Some(order.clone()),
                ..RecordingAudit::default()
            };
            broker
                .register_provider(provider, trace(45), &mut audit)
                .unwrap();
            let client_secret_custody = ClientSecretCustody::new(InMemoryClientSecretStore::new());
            client_secret_custody
                .create(
                    &secret_key,
                    Zeroizing::new("client secret".to_owned()),
                    trace(45),
                    &mut audit,
                )
                .unwrap();
            let request = prepared_exchange(&provider_config, trace(46), &mut audit);
            order.borrow_mut().clear();
            let mut transport = MockClientSecretExchangeTransport {
                response: Some(
                    TokenEndpointResponse::new(
                        200,
                        Zeroizing::new(
                            br#"{"access_token":"initial","token_type":"Bearer"}"#.to_vec(),
                        ),
                    )
                    .unwrap(),
                ),
                expected_authorization_header: expects_basic_header,
                calls: 0,
                order: order.clone(),
            };

            let response = broker
                .send_client_secret_exchange(
                    &authentication,
                    request,
                    &client_secret_custody,
                    &mut transport,
                    &mut audit,
                )
                .unwrap();

            assert_eq!(response.provider(), provider_config.provider());
            assert_eq!(response.trace(), trace(46));
            assert_eq!(response.token_type(), "Bearer");
            assert_eq!(transport.calls, 1);
            assert_eq!(
                order.borrow().as_slice(),
                [
                    "exchange-attempted",
                    "secret-access-attempted",
                    "secret-access-succeeded",
                    "transport-attempted",
                    "transport-effect",
                    "transport-succeeded",
                    "exchange-succeeded",
                ]
            );
        }
    }

    #[test]
    fn client_secret_exchange_persists_without_releasing_credentials() {
        let provider_config = config("fixture-confidential");
        let secret_key = ClientSecretKey::new(
            provider_config.provider().clone(),
            ClientSecretReference::new([0x75; 32]),
        );
        let provider = BrokerProvider::new_confidential(
            provider_config.clone(),
            TokenResponseFormat::Json,
            300,
            ConfidentialClientAuthenticationMethod::ClientSecretBasic,
            Vec::new(),
        )
        .unwrap();
        let authentication = provider
            .bind_client_secret_authentication(secret_key.clone())
            .unwrap();
        let mut broker = OAuthBroker::new(CredentialCustody::new(InMemoryCredentialStore::new()));
        let order = Rc::new(RefCell::new(Vec::new()));
        let mut audit = RecordingAudit {
            order: Some(order.clone()),
            ..RecordingAudit::default()
        };
        broker
            .register_provider(provider, trace(49), &mut audit)
            .unwrap();
        let client_secret_custody = ClientSecretCustody::new(InMemoryClientSecretStore::new());
        client_secret_custody
            .create(
                &secret_key,
                Zeroizing::new("client secret".to_owned()),
                trace(49),
                &mut audit,
            )
            .unwrap();
        let request = prepared_exchange(&provider_config, trace(50), &mut audit);
        let credential_key = key("fixture-confidential");
        let oauth_events_before = audit.oauth.len();
        let custody_events_before = audit.custody.len();
        order.borrow_mut().clear();
        let mut transport = MockClientSecretExchangeTransport {
            response: Some(
                TokenEndpointResponse::new(
                    200,
                    Zeroizing::new(
                        br#"{"access_token":"initial-access","refresh_token":"initial-refresh","token_type":"Bearer","expires_in":3600}"#.to_vec(),
                    ),
                )
                .unwrap(),
            ),
            expected_authorization_header: true,
            calls: 0,
            order: order.clone(),
        };
        let mut clock = CountingClock {
            now: 1_000,
            calls: 0,
        };

        let revision = broker
            .exchange_client_secret_and_store_credentials(
                &credential_key,
                &authentication,
                request,
                &client_secret_custody,
                ClientSecretExchangeCredentialExecution::new(&mut clock, &mut transport),
                &mut audit,
            )
            .unwrap();

        assert_eq!(transport.calls, 1);
        assert_eq!(clock.calls, 1);
        assert!(!format!("{revision:?}").contains("initial-access"));
        assert_eq!(
            audit.oauth[oauth_events_before..]
                .iter()
                .map(OAuthAuditEvent::action)
                .collect::<Vec<_>>(),
            vec![
                OAuthAuditAction::TokenResponseDecode,
                OAuthAuditAction::TokenCredentialRelease,
            ]
        );
        assert_eq!(
            audit.custody[custody_events_before..]
                .iter()
                .map(CredentialAuditEvent::action)
                .collect::<Vec<_>>(),
            vec![CredentialAuditAction::Create, CredentialAuditAction::Create]
        );
        assert_eq!(
            order.borrow().as_slice(),
            [
                "exchange-store-attempted",
                "exchange-attempted",
                "secret-access-attempted",
                "secret-access-succeeded",
                "transport-attempted",
                "transport-effect",
                "transport-succeeded",
                "exchange-succeeded",
                "credential-create-attempted",
                "credential-store-attempted",
                "credential-store-succeeded",
                "credential-create-succeeded",
                "exchange-store-succeeded",
            ]
        );

        let access = broker
            .with_access_token(
                &credential_key,
                trace(50),
                &mut FixedClock(1_001),
                &mut MockTransport::new("initial-refresh", &[]),
                &mut RecordingAudit::default(),
                str::to_owned,
            )
            .unwrap();
        assert_eq!(access, "initial-access");
    }

    #[test]
    fn client_secret_exchange_persistence_rejects_key_before_all_effects() {
        let provider_config = config("fixture-confidential");
        let secret_key = ClientSecretKey::new(
            provider_config.provider().clone(),
            ClientSecretReference::new([0x76; 32]),
        );
        let provider = BrokerProvider::new_confidential(
            provider_config.clone(),
            TokenResponseFormat::Json,
            300,
            ConfidentialClientAuthenticationMethod::ClientSecretBasic,
            Vec::new(),
        )
        .unwrap();
        let authentication = provider
            .bind_client_secret_authentication(secret_key.clone())
            .unwrap();
        let mut broker = OAuthBroker::new(CredentialCustody::new(InMemoryCredentialStore::new()));
        let order = Rc::new(RefCell::new(Vec::new()));
        let mut audit = RecordingAudit {
            order: Some(order.clone()),
            ..RecordingAudit::default()
        };
        broker
            .register_provider(provider, trace(51), &mut audit)
            .unwrap();
        let client_secret_custody = ClientSecretCustody::new(InMemoryClientSecretStore::new());
        client_secret_custody
            .create(
                &secret_key,
                Zeroizing::new("client secret".to_owned()),
                trace(51),
                &mut audit,
            )
            .unwrap();
        let request = prepared_exchange(&provider_config, trace(52), &mut audit);
        let secret_events_before = audit.client_secret.len();
        let custody_events_before = audit.custody.len();
        order.borrow_mut().clear();
        let mut transport = MockClientSecretExchangeTransport {
            response: None,
            expected_authorization_header: true,
            calls: 0,
            order: order.clone(),
        };
        let mut clock = CountingClock { now: 0, calls: 0 };

        assert!(matches!(
            broker.exchange_client_secret_and_store_credentials(
                &key("other-provider"),
                &authentication,
                request,
                &client_secret_custody,
                ClientSecretExchangeCredentialExecution::new(&mut clock, &mut transport),
                &mut audit,
            ),
            Err(BrokerError::BindingMismatch)
        ));
        assert_eq!(audit.client_secret.len(), secret_events_before);
        assert_eq!(audit.custody.len(), custody_events_before);
        assert_eq!(transport.calls, 0);
        assert_eq!(clock.calls, 0);
        assert_eq!(
            order.borrow().as_slice(),
            ["exchange-store-attempted", "exchange-store-failed"]
        );
    }

    #[test]
    fn client_secret_exchange_binding_failure_precedes_secret_and_transport() {
        let provider_config = config("fixture-confidential");
        let secret_key = ClientSecretKey::new(
            provider_config.provider().clone(),
            ClientSecretReference::new([0x74; 32]),
        );
        let provider = BrokerProvider::new_confidential(
            provider_config.clone(),
            TokenResponseFormat::Json,
            300,
            ConfidentialClientAuthenticationMethod::ClientSecretBasic,
            Vec::new(),
        )
        .unwrap();
        let mut broker = OAuthBroker::new(CredentialCustody::new(InMemoryCredentialStore::new()));
        let order = Rc::new(RefCell::new(Vec::new()));
        let mut audit = RecordingAudit {
            order: Some(order.clone()),
            ..RecordingAudit::default()
        };
        broker
            .register_provider(provider, trace(47), &mut audit)
            .unwrap();
        let client_secret_custody = ClientSecretCustody::new(InMemoryClientSecretStore::new());
        client_secret_custody
            .create(
                &secret_key,
                Zeroizing::new("client secret".to_owned()),
                trace(47),
                &mut audit,
            )
            .unwrap();
        let authentication = ClientSecretAuthentication::new(
            secret_key,
            ClientSecretAuthenticationMethod::ClientSecretPost,
        );
        let request = prepared_exchange(&provider_config, trace(48), &mut audit);
        order.borrow_mut().clear();
        let secret_events_before = audit.client_secret.len();
        let mut transport = MockClientSecretExchangeTransport {
            response: None,
            expected_authorization_header: true,
            calls: 0,
            order: order.clone(),
        };

        assert!(matches!(
            broker.send_client_secret_exchange(
                &authentication,
                request,
                &client_secret_custody,
                &mut transport,
                &mut audit,
            ),
            Err(BrokerError::BindingMismatch)
        ));
        assert_eq!(audit.client_secret.len(), secret_events_before);
        assert_eq!(transport.calls, 0);
        assert_eq!(
            order.borrow().as_slice(),
            ["exchange-attempted", "exchange-failed"]
        );
    }

    #[test]
    fn client_secret_refresh_failures_prevent_unaudited_secret_or_transport_effects() {
        let provider_config = config("fixture-confidential");
        let secret_key = ClientSecretKey::new(
            provider_config.provider().clone(),
            ClientSecretReference::new([0x72; 32]),
        );
        let provider = BrokerProvider::new_confidential(
            provider_config.clone(),
            TokenResponseFormat::Json,
            300,
            ConfidentialClientAuthenticationMethod::ClientSecretBasic,
            Vec::new(),
        )
        .unwrap();
        let mut broker = OAuthBroker::new(CredentialCustody::new(InMemoryCredentialStore::new()));
        let order = Rc::new(RefCell::new(Vec::new()));
        let mut audit = RecordingAudit {
            order: Some(order.clone()),
            ..RecordingAudit::default()
        };
        broker
            .register_provider(provider, trace(42), &mut audit)
            .unwrap();
        let client_secret_custody = ClientSecretCustody::new(InMemoryClientSecretStore::new());
        client_secret_custody
            .create(
                &secret_key,
                Zeroizing::new("client secret".to_owned()),
                trace(42),
                &mut audit,
            )
            .unwrap();
        let authentication = ClientSecretAuthentication::new(
            secret_key,
            ClientSecretAuthenticationMethod::ClientSecretPost,
        );
        let request = prepare_token_refresh(
            &provider_config,
            Zeroizing::new("refresh-secret".to_owned()),
            &[],
            trace(43),
        )
        .publish_then_release(&mut audit)
        .unwrap();
        order.borrow_mut().clear();
        let secret_events_before = audit.client_secret.len();
        let mut transport = MockClientSecretTransport {
            response: None,
            expected_authorization_header: true,
            expected_refresh: "refresh-secret",
            calls: 0,
            order: order.clone(),
        };

        assert!(matches!(
            broker.send_client_secret_refresh(
                &authentication,
                request,
                &client_secret_custody,
                &mut transport,
                &mut audit,
            ),
            Err(BrokerError::BindingMismatch)
        ));
        assert_eq!(audit.client_secret.len(), secret_events_before);
        assert_eq!(transport.calls, 0);
        assert_eq!(
            order.borrow().as_slice(),
            ["refresh-attempted", "refresh-failed"]
        );

        let authentication = ClientSecretAuthentication::new(
            authentication.key().clone(),
            ClientSecretAuthenticationMethod::ClientSecretBasic,
        );
        let request = prepare_token_refresh(
            &provider_config,
            Zeroizing::new("refresh-secret".to_owned()),
            &[],
            trace(44),
        )
        .publish_then_release(&mut audit)
        .unwrap();
        order.borrow_mut().clear();
        audit.broker_calls = 0;
        audit.fail_broker_on = Some(2);
        let secret_events_before = audit.client_secret.len();
        assert!(matches!(
            broker.send_client_secret_refresh(
                &authentication,
                request,
                &client_secret_custody,
                &mut transport,
                &mut audit,
            ),
            Err(BrokerError::Audit)
        ));
        assert_eq!(audit.client_secret.len(), secret_events_before + 2);
        assert_eq!(transport.calls, 0);
        assert_eq!(
            order.borrow().as_slice(),
            [
                "refresh-attempted",
                "secret-access-attempted",
                "secret-access-succeeded",
            ]
        );
    }

    #[test]
    fn stored_client_secret_refresh_rotates_exact_revision_after_all_audit_gates() {
        let provider_config = config("fixture-confidential");
        let secret_key = ClientSecretKey::new(
            provider_config.provider().clone(),
            ClientSecretReference::new([0x7a; 32]),
        );
        let provider = BrokerProvider::new_confidential(
            provider_config.clone(),
            TokenResponseFormat::Json,
            300,
            ConfidentialClientAuthenticationMethod::ClientSecretBasic,
            Vec::new(),
        )
        .unwrap();
        let authentication = provider
            .bind_client_secret_authentication(secret_key.clone())
            .unwrap();
        let credential_key = key("fixture-confidential");
        let order = Rc::new(RefCell::new(Vec::new()));
        let mut audit = RecordingAudit {
            order: Some(order.clone()),
            ..RecordingAudit::default()
        };
        let response = decoded_refresh_response(
            &provider_config,
            trace(62),
            r#"{"access_token":"access-one","refresh_token":"refresh-one","token_type":"Bearer","scope":"mail.read profile"}"#,
            &mut audit,
        );
        let credentials = response
            .release_credentials()
            .publish_then_release(&mut audit)
            .unwrap();
        let custody = CredentialCustody::new(InMemoryCredentialStore::new());
        let initial_revision = custody
            .create(
                &credential_key,
                credentials,
                CredentialMetadata::new(
                    "Bearer",
                    Some(2_000),
                    vec!["mail.read".to_owned(), "profile".to_owned()],
                )
                .unwrap(),
                trace(62),
                &mut audit,
            )
            .unwrap();
        let mut broker = OAuthBroker::new(custody);
        broker
            .register_provider(provider, trace(62), &mut audit)
            .unwrap();
        let client_secret_custody = ClientSecretCustody::new(InMemoryClientSecretStore::new());
        client_secret_custody
            .create(
                &secret_key,
                Zeroizing::new("client secret".to_owned()),
                trace(62),
                &mut audit,
            )
            .unwrap();

        order.borrow_mut().clear();
        let custody_events_before = audit.custody.len();
        let secret_events_before = audit.client_secret.len();
        let wrong_authentication = ClientSecretAuthentication::new(
            secret_key.clone(),
            ClientSecretAuthenticationMethod::ClientSecretPost,
        );
        let mut rejected_transport = MockClientSecretTransport {
            response: None,
            expected_authorization_header: true,
            expected_refresh: "refresh-one",
            calls: 0,
            order: order.clone(),
        };
        let mut rejected_clock = CountingClock { now: 0, calls: 0 };
        assert_eq!(
            broker.refresh_client_secret_and_rotate_credentials(
                &credential_key,
                &wrong_authentication,
                trace(63),
                &client_secret_custody,
                ClientSecretRefreshCredentialExecution::new(
                    &mut rejected_clock,
                    &mut rejected_transport,
                ),
                &mut audit,
            ),
            Err(BrokerError::BindingMismatch)
        );
        assert_eq!(audit.custody.len(), custody_events_before);
        assert_eq!(audit.client_secret.len(), secret_events_before);
        assert_eq!(rejected_transport.calls, 0);
        assert_eq!(rejected_clock.calls, 0);
        assert_eq!(
            order.borrow().as_slice(),
            ["refresh-rotate-attempted", "refresh-rotate-failed"]
        );

        order.borrow_mut().clear();
        let custody_events_before = audit.custody.len();
        let mut failed_transport = MockClientSecretTransport {
            response: None,
            expected_authorization_header: true,
            expected_refresh: "refresh-one",
            calls: 0,
            order: order.clone(),
        };
        let mut unused_clock = CountingClock { now: 0, calls: 0 };
        assert_eq!(
            broker.refresh_client_secret_and_rotate_credentials(
                &credential_key,
                &authentication,
                trace(64),
                &client_secret_custody,
                ClientSecretRefreshCredentialExecution::new(
                    &mut unused_clock,
                    &mut failed_transport,
                ),
                &mut audit,
            ),
            Err(BrokerError::Transport)
        );
        assert_eq!(failed_transport.calls, 1);
        assert_eq!(unused_clock.calls, 0);
        assert!(audit.custody[custody_events_before..]
            .iter()
            .all(|event| event.action() != CredentialAuditAction::Rotate));
        assert_eq!(
            broker
                .custody
                .with_access_token(&credential_key, trace(64), &mut audit, str::to_owned)
                .unwrap(),
            "access-one"
        );

        order.borrow_mut().clear();
        let custody_events_before = audit.custody.len();
        let mut transport = MockClientSecretTransport {
            response: Some(
                TokenEndpointResponse::new(
                    200,
                    Zeroizing::new(
                        br#"{"access_token":"access-two","refresh_token":"refresh-two","token_type":"Bearer","expires_in":3600}"#.to_vec(),
                    ),
                )
                .unwrap(),
            ),
            expected_authorization_header: true,
            expected_refresh: "refresh-one",
            calls: 0,
            order: order.clone(),
        };
        let mut clock = CountingClock {
            now: 1_000,
            calls: 0,
        };
        let revision = broker
            .refresh_client_secret_and_rotate_credentials(
                &credential_key,
                &authentication,
                trace(65),
                &client_secret_custody,
                ClientSecretRefreshCredentialExecution::new(&mut clock, &mut transport),
                &mut audit,
            )
            .unwrap();

        assert_ne!(revision, initial_revision);
        assert_eq!(transport.calls, 1);
        assert_eq!(clock.calls, 1);
        assert_eq!(
            audit.custody[custody_events_before..]
                .iter()
                .map(|event| (event.action(), event.outcome()))
                .collect::<Vec<_>>(),
            vec![
                (
                    CredentialAuditAction::RefreshToken,
                    CredentialAuditOutcome::Attempted,
                ),
                (
                    CredentialAuditAction::RefreshToken,
                    CredentialAuditOutcome::Succeeded,
                ),
                (
                    CredentialAuditAction::Rotate,
                    CredentialAuditOutcome::Attempted,
                ),
                (
                    CredentialAuditAction::Rotate,
                    CredentialAuditOutcome::Succeeded,
                ),
            ]
        );
        assert_eq!(
            order.borrow().as_slice(),
            [
                "refresh-rotate-attempted",
                "refresh-token-access-attempted",
                "refresh-token-access-succeeded",
                "refresh-attempted",
                "secret-access-attempted",
                "secret-access-succeeded",
                "transport-attempted",
                "transport-effect",
                "transport-succeeded",
                "refresh-succeeded",
                "credential-rotate-attempted",
                "credential-rotate-succeeded",
                "refresh-rotate-succeeded",
            ]
        );
        assert_eq!(
            broker
                .custody
                .with_access_token(&credential_key, trace(65), &mut audit, str::to_owned)
                .unwrap(),
            "access-two"
        );
        assert_eq!(
            broker
                .custody
                .with_refresh_token(&credential_key, trace(65), &mut audit, |token, _| {
                    token.to_owned()
                })
                .unwrap(),
            "refresh-two"
        );
        assert_eq!(
            broker
                .custody
                .with_metadata(&credential_key, trace(65), &mut audit, |metadata| {
                    (
                        metadata.expires_at_unix_seconds(),
                        metadata.scopes().to_vec(),
                    )
                })
                .unwrap(),
            (
                Some(4_600),
                vec!["mail.read".to_owned(), "profile".to_owned()],
            )
        );
    }

    #[test]
    fn confidential_provider_source_binding_and_audit_fail_closed_before_registration() {
        let custody = CredentialCustody::new(InMemoryCredentialStore::new());
        let mut broker = OAuthBroker::new(custody);
        let requested = ProviderId::new("fixture-confidential").unwrap();

        let mut source = MockProviderDataSource::failed();
        let mut audit = RecordingAudit::default();
        assert_eq!(
            broker.load_and_register_confidential_provider(
                &requested,
                "fixture-service-client",
                "https://service.fixture.example/oauth/callback",
                trace(33),
                &mut source,
                &mut audit,
            ),
            Err(BrokerError::ProviderDataSource)
        );
        assert_eq!(source.calls, 1);
        assert_eq!(broker.provider_count(), 0);
        assert_eq!(
            audit.broker.last().unwrap().outcome(),
            BrokerAuditOutcome::Failed(BrokerFailureClass::ProviderData)
        );

        let mut source =
            MockProviderDataSource::successful(confidential_provider_data("other-confidential"));
        let mut audit = RecordingAudit::default();
        assert_eq!(
            broker.load_and_register_confidential_provider(
                &requested,
                "fixture-service-client",
                "https://service.fixture.example/oauth/callback",
                trace(34),
                &mut source,
                &mut audit,
            ),
            Err(BrokerError::BindingMismatch)
        );
        assert_eq!(source.calls, 1);
        assert_eq!(broker.provider_count(), 0);
        assert_eq!(
            audit.broker.last().unwrap().outcome(),
            BrokerAuditOutcome::Failed(BrokerFailureClass::InvalidInput)
        );
        assert!(!audit.broker.iter().any(|event| {
            event.trace() == trace(34) && event.action() == BrokerAuditAction::ProviderRegister
        }));

        let mut source =
            MockProviderDataSource::successful(confidential_provider_data("fixture-confidential"));
        let mut audit = RecordingAudit {
            fail_broker_on: Some(1),
            ..RecordingAudit::default()
        };
        assert_eq!(
            broker.load_and_register_confidential_provider(
                &requested,
                "fixture-service-client",
                "https://service.fixture.example/oauth/callback",
                trace(35),
                &mut source,
                &mut audit,
            ),
            Err(BrokerError::Audit)
        );
        assert_eq!(source.calls, 0);
        assert_eq!(broker.provider_count(), 0);

        let mut source =
            MockProviderDataSource::successful(confidential_provider_data("fixture-confidential"));
        let mut audit = RecordingAudit {
            fail_broker_on: Some(2),
            ..RecordingAudit::default()
        };
        assert_eq!(
            broker.load_and_register_confidential_provider(
                &requested,
                "fixture-service-client",
                "https://service.fixture.example/oauth/callback",
                trace(36),
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
    fn caller_timed_device_sequence_waits_polls_and_applies_slow_down() {
        let custody = CredentialCustody::new(InMemoryCredentialStore::new());
        let mut broker = OAuthBroker::new(custody);
        let mut setup_audit = RecordingAudit::default();
        broker
            .register_provider(policy("fixture", 300), trace(40), &mut setup_audit)
            .unwrap();
        let session = device_session("fixture", trace(40), &mut setup_audit);
        let sequence = DeviceFlowPollSequence::new(session, 100).unwrap();
        assert_eq!(sequence.next_poll_at_seconds(), 105);
        assert_eq!(sequence.expires_at_seconds(), 1_000);
        assert!(!format!("{sequence:?}").contains("device-secret"));

        let mut audit = RecordingAudit::default();
        let mut transport = MockDeviceTransport::new(vec![
            MockDeviceTransport::json(400, r#"{"error":"authorization_pending"}"#),
            MockDeviceTransport::json(400, r#"{"error":"slow_down"}"#),
            MockDeviceTransport::json(
                200,
                r#"{"access_token":"device-access","token_type":"Bearer","expires_in":3600}"#,
            ),
        ]);

        let waiting = broker
            .advance_device_flow(sequence, 104, &mut transport, &mut audit)
            .unwrap();
        assert_eq!(transport.calls, 0);
        assert_eq!(waiting.next_poll_at_seconds(), Some(105));
        let DeviceFlowStepResult::Waiting(sequence) = waiting else {
            panic!("expected an early waiting result");
        };

        let pending = broker
            .advance_device_flow(sequence, 105, &mut transport, &mut audit)
            .unwrap();
        assert_eq!(transport.calls, 1);
        assert_eq!(pending.next_poll_at_seconds(), Some(110));
        let DeviceFlowStepResult::Pending(sequence) = pending else {
            panic!("expected an authorization-pending result");
        };

        let waiting = broker
            .advance_device_flow(sequence, 109, &mut transport, &mut audit)
            .unwrap();
        assert_eq!(transport.calls, 1);
        let DeviceFlowStepResult::Waiting(sequence) = waiting else {
            panic!("expected a second early waiting result");
        };

        let slow_down = broker
            .advance_device_flow(sequence, 110, &mut transport, &mut audit)
            .unwrap();
        assert_eq!(transport.calls, 2);
        assert_eq!(slow_down.next_poll_at_seconds(), Some(120));
        let DeviceFlowStepResult::SlowDown(sequence) = slow_down else {
            panic!("expected a slow-down result");
        };

        let authorized = broker
            .advance_device_flow(sequence, 120, &mut transport, &mut audit)
            .unwrap();
        let DeviceFlowStepResult::Authorized(response) = authorized else {
            panic!("expected an authorized result");
        };
        assert_eq!(response.provider().as_str(), "fixture");
        assert_eq!(response.trace(), trace(40));
        assert_eq!(transport.calls, 3);
        assert_eq!(
            audit
                .broker
                .iter()
                .filter(|event| event.action() == BrokerAuditAction::DeviceFlowStep)
                .count(),
            10
        );
    }

    #[test]
    fn authorized_device_response_is_stored_without_token_release() {
        let custody = CredentialCustody::new(InMemoryCredentialStore::new());
        let mut broker = OAuthBroker::new(custody);
        let mut setup_audit = RecordingAudit::default();
        broker
            .register_provider(policy("fixture", 300), trace(46), &mut setup_audit)
            .unwrap();
        let session = device_session("fixture", trace(46), &mut setup_audit);
        let sequence = DeviceFlowPollSequence::new(session, 0).unwrap();
        let credential_key = key("fixture");
        let mut transport = MockDeviceTransport::new(vec![MockDeviceTransport::json(
            200,
            r#"{"access_token":"device-access","refresh_token":"device-refresh","token_type":"Bearer","expires_in":3600}"#,
        )]);
        let mut clock = CountingClock {
            now: 1_000,
            calls: 0,
        };
        let mut audit = RecordingAudit::default();

        let result = broker
            .advance_device_flow_and_store(
                &credential_key,
                sequence,
                5,
                &mut transport,
                &mut clock,
                &mut audit,
            )
            .unwrap();

        assert!(matches!(result, DeviceFlowCredentialStepResult::Stored(_)));
        assert_eq!(transport.calls, 1);
        assert_eq!(clock.calls, 1);
        assert!(!format!("{result:?}").contains("device-access"));
        let access = broker
            .with_access_token(
                &credential_key,
                trace(46),
                &mut FixedClock(1_001),
                &mut MockTransport::new("device-refresh", &[]),
                &mut audit,
                str::to_owned,
            )
            .unwrap();
        assert_eq!(access, "device-access");
        assert_eq!(
            audit
                .broker
                .iter()
                .filter(|event| event.action() == BrokerAuditAction::DeviceCredentialCreate)
                .map(BrokerAuditEvent::outcome)
                .collect::<Vec<_>>(),
            vec![BrokerAuditOutcome::Attempted, BrokerAuditOutcome::Succeeded]
        );
    }

    #[test]
    fn device_credential_binding_precedes_transport_and_clock() {
        let custody = CredentialCustody::new(InMemoryCredentialStore::new());
        let mut broker = OAuthBroker::new(custody);
        let mut setup_audit = RecordingAudit::default();
        broker
            .register_provider(policy("fixture", 300), trace(47), &mut setup_audit)
            .unwrap();
        let session = device_session("fixture", trace(47), &mut setup_audit);
        let sequence = DeviceFlowPollSequence::new(session, 0).unwrap();
        let mut transport = MockDeviceTransport::new(vec![]);
        let mut clock = CountingClock {
            now: 1_000,
            calls: 0,
        };
        let mut audit = RecordingAudit::default();

        assert!(matches!(
            broker.advance_device_flow_and_store(
                &key("other"),
                sequence,
                5,
                &mut transport,
                &mut clock,
                &mut audit,
            ),
            Err(BrokerError::BindingMismatch)
        ));
        assert_eq!(transport.calls, 0);
        assert_eq!(clock.calls, 0);
        assert!(audit.oauth.is_empty());
        assert!(audit.custody.is_empty());
        assert_eq!(
            audit.broker.last().unwrap().outcome(),
            BrokerAuditOutcome::Failed(BrokerFailureClass::InvalidInput)
        );
    }

    #[test]
    fn device_credential_continuation_does_not_access_clock_or_custody() {
        let custody = CredentialCustody::new(InMemoryCredentialStore::new());
        let mut broker = OAuthBroker::new(custody);
        let mut setup_audit = RecordingAudit::default();
        broker
            .register_provider(policy("fixture", 300), trace(48), &mut setup_audit)
            .unwrap();
        let session = device_session("fixture", trace(48), &mut setup_audit);
        let sequence = DeviceFlowPollSequence::new(session, 0).unwrap();
        let mut transport = MockDeviceTransport::new(vec![]);
        let mut clock = CountingClock {
            now: 1_000,
            calls: 0,
        };
        let mut audit = RecordingAudit::default();

        let result = broker
            .advance_device_flow_and_store(
                &key("fixture"),
                sequence,
                4,
                &mut transport,
                &mut clock,
                &mut audit,
            )
            .unwrap();

        assert!(matches!(result, DeviceFlowCredentialStepResult::Waiting(_)));
        assert_eq!(result.next_poll_at_seconds(), Some(5));
        assert_eq!(transport.calls, 0);
        assert_eq!(clock.calls, 0);
        assert!(audit.oauth.is_empty());
        assert!(audit.custody.is_empty());
    }

    #[test]
    fn caller_timed_device_sequence_stops_at_local_expiry_without_transport() {
        let custody = CredentialCustody::new(InMemoryCredentialStore::new());
        let mut broker = OAuthBroker::new(custody);
        let mut setup_audit = RecordingAudit::default();
        broker
            .register_provider(policy("fixture", 300), trace(41), &mut setup_audit)
            .unwrap();
        let session = device_session("fixture", trace(41), &mut setup_audit);
        let sequence = DeviceFlowPollSequence::new(session, 100).unwrap();
        let mut audit = RecordingAudit::default();
        let mut transport = MockDeviceTransport::new(vec![]);

        let result = broker
            .advance_device_flow(sequence, 1_000, &mut transport, &mut audit)
            .unwrap();

        assert!(matches!(result, DeviceFlowStepResult::Expired));
        assert_eq!(transport.calls, 0);
        assert!(audit.oauth.is_empty());
        assert_eq!(
            audit
                .broker
                .iter()
                .map(|event| (event.action(), event.outcome()))
                .collect::<Vec<_>>(),
            vec![
                (
                    BrokerAuditAction::DeviceFlowStep,
                    BrokerAuditOutcome::Attempted,
                ),
                (
                    BrokerAuditAction::DeviceFlowStep,
                    BrokerAuditOutcome::Succeeded,
                ),
            ]
        );
    }

    #[test]
    fn caller_timed_device_sequence_audit_failures_withhold_state_and_transport() {
        let custody = CredentialCustody::new(InMemoryCredentialStore::new());
        let mut broker = OAuthBroker::new(custody);
        let mut setup_audit = RecordingAudit::default();
        broker
            .register_provider(policy("fixture", 300), trace(44), &mut setup_audit)
            .unwrap();

        for fail_broker_on in [1, 2] {
            let session = device_session("fixture", trace(44), &mut setup_audit);
            let sequence = DeviceFlowPollSequence::new(session, 0).unwrap();
            let mut audit = RecordingAudit {
                fail_broker_on: Some(fail_broker_on),
                ..RecordingAudit::default()
            };
            let mut transport = MockDeviceTransport::new(vec![]);

            assert!(matches!(
                broker.advance_device_flow(sequence, 0, &mut transport, &mut audit),
                Err(BrokerError::Audit)
            ));
            assert_eq!(transport.calls, 0);
            assert!(audit.oauth.is_empty());
        }
    }

    #[test]
    fn caller_timed_device_sequence_rejects_timeline_overflow() {
        let mut audit = RecordingAudit::default();
        let session = device_session("fixture", trace(45), &mut audit);
        assert!(matches!(
            DeviceFlowPollSequence::new(session, u64::MAX - 1),
            Err(BrokerError::Clock)
        ));
    }

    #[test]
    fn caller_timed_device_sequence_reschedules_transport_failure() {
        let custody = CredentialCustody::new(InMemoryCredentialStore::new());
        let mut broker = OAuthBroker::new(custody);
        let mut setup_audit = RecordingAudit::default();
        broker
            .register_provider(policy("fixture", 300), trace(42), &mut setup_audit)
            .unwrap();
        let session = device_session("fixture", trace(42), &mut setup_audit);
        let sequence = DeviceFlowPollSequence::new(session, 0).unwrap();
        let mut audit = RecordingAudit::default();
        let mut transport = MockDeviceTransport::new(vec![Err(TokenTransportError)]);

        let result = broker
            .advance_device_flow(sequence, 5, &mut transport, &mut audit)
            .unwrap();

        assert_eq!(result.next_poll_at_seconds(), Some(10));
        assert!(matches!(result, DeviceFlowStepResult::TransportFailed(_)));
        assert_eq!(transport.calls, 1);
    }

    #[test]
    fn caller_timed_device_sequence_rejects_binding_before_waiting_or_transport() {
        let custody = CredentialCustody::new(InMemoryCredentialStore::new());
        let mut broker = OAuthBroker::new(custody);
        let mut setup_audit = RecordingAudit::default();
        broker
            .register_provider(policy("other", 300), trace(43), &mut setup_audit)
            .unwrap();
        let session = device_session("fixture", trace(43), &mut setup_audit);
        let sequence = DeviceFlowPollSequence::new(session, 0).unwrap();
        let mut audit = RecordingAudit::default();
        let mut transport = MockDeviceTransport::new(vec![]);

        assert!(matches!(
            broker.advance_device_flow(sequence, 0, &mut transport, &mut audit),
            Err(BrokerError::ProviderNotRegistered)
        ));
        assert_eq!(transport.calls, 0);
        assert!(audit.oauth.is_empty());
        assert_eq!(
            audit.broker.last().unwrap().outcome(),
            BrokerAuditOutcome::Failed(BrokerFailureClass::ProviderNotRegistered)
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
        assert!(matches!(
            TokenRevocationEndpointResponse::new(99, Zeroizing::new(Vec::new())),
            Err(BrokerError::InvalidTransportResponse)
        ));
        assert!(matches!(
            TokenRevocationEndpointResponse::new(
                200,
                Zeroizing::new(vec![b'x'; MAX_TOKEN_REVOCATION_RESPONSE_BYTES + 1]),
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
            "{broker:?} {:?} {:?} {:?}",
            BrokerError::Transport,
            TokenEndpointResponse::new(200, Zeroizing::new(b"top-secret".to_vec())).unwrap(),
            TokenRevocationEndpointResponse::new(
                200,
                Zeroizing::new(b"revocation-secret".to_vec()),
            )
            .unwrap()
        );
        assert!(!debug.contains("top-secret"));
        assert!(!debug.contains("revocation-secret"));
        assert!(debug.contains("<redacted>"));
    }
}
