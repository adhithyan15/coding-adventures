//! Provider-neutral OAuth 2.0 installed-app primitives.
//!
//! This crate implements the security-sensitive pure protocol core and owns no
//! I/O authority. A host injects entropy, persists the returned privacy-safe
//! audit descriptor, opens the authorization URL, receives the exact callback,
//! and sends the prepared token request over an independently authorized HTTPS
//! transport. Provider behavior is configuration data, never a code branch.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_sha256::sha256;
use coding_adventures_zeroize::Zeroizing;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Debug, Display, Formatter};
use url_parser::Url;

mod token;

pub use token::*;

const ENTROPY_BYTES: usize = 32;
const MAX_ENDPOINT_BYTES: usize = 2_048;
const MAX_CLIENT_ID_BYTES: usize = 1_024;
const MAX_PROVIDER_ID_BYTES: usize = 64;
const MAX_SCOPE_BYTES: usize = 256;
const MAX_SCOPES: usize = 64;
const MAX_EXTRA_PARAMETERS: usize = 32;
const MAX_PARAMETER_BYTES: usize = 1_024;
const MAX_CALLBACK_BYTES: usize = 16 * 1024;
const MAX_AUTHORIZATION_CODE_BYTES: usize = 4_096;
const TRACE_BYTES: usize = 16;

const RESERVED_AUTHORIZATION_PARAMETERS: [&str; 8] = [
    "client_id",
    "redirect_uri",
    "response_type",
    "scope",
    "state",
    "code_challenge",
    "code_challenge_method",
    "resource",
];

/// A caller-owned cryptographically secure entropy source.
pub trait EntropySource {
    /// Fill the complete destination or fail without returning partial output.
    fn fill(&mut self, destination: &mut [u8]) -> Result<(), OAuthError>;
}

/// Caller-supplied correlation identity shared by one authorization ceremony.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct OAuthTraceId([u8; TRACE_BYTES]);

impl OAuthTraceId {
    /// Construct a trace identifier from exact caller-owned random bytes.
    pub const fn new(bytes: [u8; TRACE_BYTES]) -> Self {
        Self(bytes)
    }

    /// Borrow exact bytes for the durable audit system's trace field.
    pub const fn as_bytes(&self) -> &[u8; TRACE_BYTES] {
        &self.0
    }
}

impl Debug for OAuthTraceId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("OAuthTraceId(")?;
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        formatter.write_str(")")
    }
}

/// A stable, non-secret provider identifier used by configuration and audit.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProviderId(String);

impl ProviderId {
    /// Validate a bounded lowercase provider identifier.
    pub fn new(value: impl Into<String>) -> Result<Self, OAuthError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.len() <= MAX_PROVIDER_ID_BYTES
            && value.bytes().enumerate().all(|(index, byte)| match byte {
                b'a'..=b'z' | b'0'..=b'9' => true,
                b'.' | b'_' | b'-' => index > 0,
                _ => false,
            });
        if !valid {
            return Err(OAuthError::InvalidConfiguration(
                ConfigurationViolation::ProviderId,
            ));
        }
        Ok(Self(value))
    }

    /// Borrow the identifier for a provider registry or privacy-safe audit row.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Debug for ProviderId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("ProviderId").field(&self.0).finish()
    }
}

/// Authorization-server mix-up defense selected explicitly per provider.
#[derive(Clone, PartialEq, Eq)]
pub enum MixUpDefense {
    /// Require and exactly validate the RFC 9207 `iss` response parameter.
    AuthorizationResponseIssuer(String),
    /// Assert a redirect URI is unique to this provider in the host registry.
    DistinctRedirectUri,
}

impl Debug for MixUpDefense {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::AuthorizationResponseIssuer(_) => "AuthorizationResponseIssuer(<redacted>)",
            Self::DistinctRedirectUri => "DistinctRedirectUri",
        })
    }
}

/// Static provider data for a public installed-app Authorization Code client.
#[derive(Clone)]
pub struct ProviderConfig {
    provider: ProviderId,
    authorization_endpoint: String,
    token_endpoint: String,
    client_id: String,
    redirect_uri: String,
    revocation_endpoint: Option<String>,
    mix_up_defense: Option<MixUpDefense>,
    authorization_extra_parameters: BTreeMap<String, String>,
}

impl ProviderConfig {
    /// Construct a strict public-client configuration.
    pub fn new(
        provider: ProviderId,
        authorization_endpoint: impl Into<String>,
        token_endpoint: impl Into<String>,
        client_id: impl Into<String>,
        redirect_uri: impl Into<String>,
    ) -> Result<Self, OAuthError> {
        let authorization_endpoint = authorization_endpoint.into();
        let token_endpoint = token_endpoint.into();
        let client_id = client_id.into();
        let redirect_uri = redirect_uri.into();

        validate_https_endpoint(&authorization_endpoint)?;
        validate_https_endpoint(&token_endpoint)?;
        validate_client_id(&client_id)?;
        validate_redirect_uri(&redirect_uri)?;

        Ok(Self {
            provider,
            authorization_endpoint,
            token_endpoint,
            client_id,
            redirect_uri,
            revocation_endpoint: None,
            mix_up_defense: None,
            authorization_extra_parameters: BTreeMap::new(),
        })
    }

    /// Require an exact RFC 9207 authorization-response issuer value.
    pub fn with_expected_issuer(mut self, issuer: impl Into<String>) -> Result<Self, OAuthError> {
        let issuer = issuer.into();
        validate_issuer(&issuer)?;
        self.mix_up_defense = Some(MixUpDefense::AuthorizationResponseIssuer(issuer));
        Ok(self)
    }

    /// Declare that this provider owns a redirect URI no other provider uses.
    ///
    /// A registry that accepts this mode must reject duplicate redirect URIs.
    /// Prefer [`Self::with_expected_issuer`] when the server supports RFC 9207.
    pub fn with_distinct_redirect_uri(mut self) -> Self {
        self.mix_up_defense = Some(MixUpDefense::DistinctRedirectUri);
        self
    }

    /// Configure an optional RFC 7009 token revocation endpoint.
    pub fn with_revocation_endpoint(
        mut self,
        endpoint: impl Into<String>,
    ) -> Result<Self, OAuthError> {
        let endpoint = endpoint.into();
        validate_https_endpoint(&endpoint)?;
        self.revocation_endpoint = Some(endpoint);
        Ok(self)
    }

    /// Add bounded provider-defined authorization parameters.
    ///
    /// Core protocol names are reserved and cannot be overridden. The sorted
    /// map makes authorization URLs deterministic without provider branches.
    pub fn with_authorization_extra_parameters(
        mut self,
        parameters: BTreeMap<String, String>,
    ) -> Result<Self, OAuthError> {
        if parameters.len() > MAX_EXTRA_PARAMETERS {
            return Err(OAuthError::InvalidConfiguration(
                ConfigurationViolation::ExtraParameter,
            ));
        }
        for (key, value) in &parameters {
            if !valid_parameter(key)
                || !valid_parameter(value)
                || RESERVED_AUTHORIZATION_PARAMETERS.contains(&key.as_str())
            {
                return Err(OAuthError::InvalidConfiguration(
                    ConfigurationViolation::ExtraParameter,
                ));
            }
        }
        self.authorization_extra_parameters = parameters;
        Ok(self)
    }

    /// Return the stable provider identifier.
    pub fn provider(&self) -> &ProviderId {
        &self.provider
    }
}

impl Debug for ProviderConfig {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderConfig")
            .field("provider", &self.provider)
            .field("authorization_endpoint", &"<redacted>")
            .field("token_endpoint", &"<redacted>")
            .field("client_id", &"<redacted>")
            .field("redirect_uri", &"<redacted>")
            .field(
                "has_revocation_endpoint",
                &self.revocation_endpoint.is_some(),
            )
            .field("mix_up_defense", &self.mix_up_defense)
            .field(
                "authorization_extra_parameter_count",
                &self.authorization_extra_parameters.len(),
            )
            .finish()
    }
}

/// A browser-safe authorization URL whose debug output is always redacted.
pub struct AuthorizationUrl(String);

impl AuthorizationUrl {
    /// Borrow the URL for the host's external-user-agent opener.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Debug for AuthorizationUrl {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("AuthorizationUrl(<redacted>)")
    }
}

/// One-use state retained between authorization begin and callback receipt.
pub struct AuthorizationTransaction {
    provider: ProviderId,
    trace: OAuthTraceId,
    token_endpoint: String,
    client_id: String,
    redirect_uri: String,
    mix_up_defense: MixUpDefense,
    state: Zeroizing<String>,
    pkce_verifier: PkceVerifier,
}

impl AuthorizationTransaction {
    /// Return the provider identity for transaction routing and audit.
    pub fn provider(&self) -> &ProviderId {
        &self.provider
    }
}

impl Debug for AuthorizationTransaction {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthorizationTransaction")
            .field("provider", &self.provider)
            .field("trace", &self.trace)
            .field("token_endpoint", &"<redacted>")
            .field("client_id", &"<redacted>")
            .field("redirect_uri", &"<redacted>")
            .field("mix_up_defense", &self.mix_up_defense)
            .field("state", &"<redacted>")
            .field("pkce_verifier", &self.pkce_verifier)
            .finish()
    }
}

/// Successful authorization preparation plus the one-use transaction secret.
pub struct AuthorizationRequest {
    url: AuthorizationUrl,
    transaction: AuthorizationTransaction,
}

impl AuthorizationRequest {
    /// Borrow the authorization URL before consuming this value into its parts.
    pub fn url(&self) -> &AuthorizationUrl {
        &self.url
    }

    /// Split browser output from the transaction a host must retain securely.
    pub fn into_parts(self) -> (AuthorizationUrl, AuthorizationTransaction) {
        (self.url, self.transaction)
    }
}

impl Debug for AuthorizationRequest {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthorizationRequest")
            .field("url", &self.url)
            .field("transaction", &self.transaction)
            .finish()
    }
}

/// A PKCE verifier held in wipe-on-drop storage.
pub struct PkceVerifier(Zeroizing<String>);

impl PkceVerifier {
    /// Validate an RFC 7636 verifier for deterministic fixtures or restoration.
    pub fn new(value: impl Into<String>) -> Result<Self, OAuthError> {
        let value = value.into();
        if !valid_pkce_verifier(&value) {
            return Err(OAuthError::InvalidConfiguration(
                ConfigurationViolation::PkceVerifier,
            ));
        }
        Ok(Self(Zeroizing::new(value)))
    }

    fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Debug for PkceVerifier {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("PkceVerifier(<redacted>)")
    }
}

/// Derive an RFC 7636 `S256` code challenge.
pub fn pkce_s256_challenge(verifier: &PkceVerifier) -> String {
    base64_url_no_pad(&sha256(verifier.as_str().as_bytes()))
}

/// Prepared public-client token exchange with secret-bearing form body.
pub struct TokenExchangeRequest {
    provider: ProviderId,
    trace: OAuthTraceId,
    endpoint: String,
    form_body: Zeroizing<String>,
}

impl TokenExchangeRequest {
    /// Return the provider identifier for transport authorization and audit.
    pub fn provider(&self) -> &ProviderId {
        &self.provider
    }

    /// Borrow the validated HTTPS token endpoint.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Borrow the secret-bearing form body only for an authorized transport.
    pub fn form_body(&self) -> &str {
        self.form_body.as_str()
    }

    /// The exact media type required for the form body.
    pub const fn content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }
}

impl Debug for TokenExchangeRequest {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TokenExchangeRequest")
            .field("provider", &self.provider)
            .field("endpoint", &"<redacted>")
            .field("form_body", &"<redacted>")
            .finish()
    }
}

/// Security-relevant OAuth action represented in an audit descriptor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OAuthAuditAction {
    /// An authorization URL and transaction were prepared.
    AuthorizationBegin,
    /// An authorization callback was validated and an exchange was prepared.
    AuthorizationComplete,
    /// A refresh grant request was prepared for transport.
    TokenRefreshPrepare,
    /// A token endpoint response was decoded and classified.
    TokenResponseDecode,
    /// Parsed credential material was released to its next custodian.
    TokenCredentialRelease,
    /// An RFC 7009 revocation request was prepared for transport.
    TokenRevocationPrepare,
}

/// Privacy-safe outcome stored without callback, code, token, URL, or scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OAuthAuditOutcome {
    /// The pure preparation completed successfully.
    Succeeded,
    /// The authorization server reported that the resource owner denied access.
    Denied,
    /// Preparation failed closed.
    Failed(OAuthFailureClass),
}

/// Closed failure class suitable for privacy-safe audit storage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OAuthFailureClass {
    /// Requested scopes or provider configuration were invalid.
    InvalidInput,
    /// The caller's entropy source failed.
    Entropy,
    /// Callback URI, parameters, state, or issuer validation failed.
    InvalidCallback,
    /// The provider returned a non-denial OAuth error.
    Provider,
    /// Durable audit publication failed before result release.
    Audit,
}

/// A privacy-safe descriptor that a host persists before effect or disclosure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OAuthAuditEvent {
    provider: ProviderId,
    trace: OAuthTraceId,
    action: OAuthAuditAction,
    outcome: OAuthAuditOutcome,
}

impl OAuthAuditEvent {
    /// Return the provider identifier; no endpoint or account is included.
    pub fn provider(&self) -> &ProviderId {
        &self.provider
    }

    /// Return the caller-owned correlation identity for this ceremony.
    pub const fn trace(&self) -> OAuthTraceId {
        self.trace
    }

    /// Return the stable action.
    pub const fn action(&self) -> OAuthAuditAction {
        self.action
    }

    /// Return the closed outcome.
    pub const fn outcome(&self) -> OAuthAuditOutcome {
        self.outcome
    }
}

/// Every protocol attempt returns its audit descriptor beside its result.
pub struct Audited<T> {
    audit: OAuthAuditEvent,
    result: Result<T, OAuthError>,
}

/// Durable privacy-safe audit publication required before a result is released.
pub trait OAuthAuditSink {
    /// Persist `event` durably or fail closed.
    fn publish(&mut self, event: &OAuthAuditEvent) -> Result<(), OAuthAuditError>;
}

/// Closed audit-publication failure without sink diagnostics or secret data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OAuthAuditError;

impl<T> Audited<T> {
    /// Borrow the descriptor a host must durably publish before using success.
    pub fn audit(&self) -> &OAuthAuditEvent {
        &self.audit
    }

    /// Publish the descriptor durably, then and only then release the result.
    pub fn publish_then_release<S: OAuthAuditSink>(self, sink: &mut S) -> Result<T, OAuthError> {
        sink.publish(&self.audit).map_err(|_| OAuthError::Audit)?;
        self.result
    }

    /// Report success without exposing the result.
    pub fn is_success(&self) -> bool {
        self.result.is_ok()
    }
}

impl<T> Debug for Audited<T> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Audited")
            .field("audit", &self.audit)
            .field("success", &self.result.is_ok())
            .field("result", &"<redacted>")
            .finish()
    }
}

/// Prepare a mandatory-`S256` installed-app authorization request.
pub fn begin_authorization<E: EntropySource>(
    config: &ProviderConfig,
    requested_scopes: &[&str],
    trace: OAuthTraceId,
    entropy: &mut E,
) -> Audited<AuthorizationRequest> {
    let action = OAuthAuditAction::AuthorizationBegin;
    let result = prepare_authorization(config, requested_scopes, trace, entropy);
    audited(config.provider.clone(), trace, action, result)
}

fn prepare_authorization<E: EntropySource>(
    config: &ProviderConfig,
    requested_scopes: &[&str],
    trace: OAuthTraceId,
    entropy: &mut E,
) -> Result<AuthorizationRequest, OAuthError> {
    validate_scopes(requested_scopes)?;
    let mix_up_defense = config
        .mix_up_defense
        .clone()
        .ok_or(OAuthError::InvalidConfiguration(
            ConfigurationViolation::MixUpDefense,
        ))?;

    let mut random = Zeroizing::new([0_u8; ENTROPY_BYTES * 2]);
    entropy.fill(random.as_mut_slice())?;
    let state = Zeroizing::new(base64_url_no_pad(&random[..ENTROPY_BYTES]));
    let pkce_verifier = PkceVerifier::new(base64_url_no_pad(&random[ENTROPY_BYTES..]))?;
    let challenge = pkce_s256_challenge(&pkce_verifier);
    let scope = requested_scopes.join(" ");

    let mut parameters = vec![
        ("client_id", config.client_id.as_str()),
        ("redirect_uri", config.redirect_uri.as_str()),
        ("response_type", "code"),
        ("scope", scope.as_str()),
        ("state", state.as_str()),
        ("code_challenge", challenge.as_str()),
        ("code_challenge_method", "S256"),
    ];
    parameters.extend(
        config
            .authorization_extra_parameters
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str())),
    );

    let query = render_form(parameters);
    let url = AuthorizationUrl(format!("{}?{query}", config.authorization_endpoint));
    let transaction = AuthorizationTransaction {
        provider: config.provider.clone(),
        trace,
        token_endpoint: config.token_endpoint.clone(),
        client_id: config.client_id.clone(),
        redirect_uri: config.redirect_uri.clone(),
        mix_up_defense,
        state,
        pkce_verifier,
    };
    Ok(AuthorizationRequest { url, transaction })
}

/// Validate a callback and prepare the exact Authorization Code token request.
///
/// The transaction is consumed even on failure, structurally preventing local
/// callback replay through this API.
pub fn complete_authorization(
    transaction: AuthorizationTransaction,
    callback_uri: &str,
) -> Audited<TokenExchangeRequest> {
    let provider = transaction.provider.clone();
    let trace = transaction.trace;
    let result = prepare_token_exchange(transaction, callback_uri);
    audited(
        provider,
        trace,
        OAuthAuditAction::AuthorizationComplete,
        result,
    )
}

fn prepare_token_exchange(
    transaction: AuthorizationTransaction,
    callback_uri: &str,
) -> Result<TokenExchangeRequest, OAuthError> {
    if callback_uri.is_empty()
        || callback_uri.len() > MAX_CALLBACK_BYTES
        || callback_uri.contains('#')
    {
        return Err(OAuthError::InvalidCallback(CallbackViolation::Uri));
    }
    let (base, query) = callback_uri
        .split_once('?')
        .ok_or(OAuthError::InvalidCallback(CallbackViolation::Query))?;
    if base != transaction.redirect_uri || query.is_empty() {
        return Err(OAuthError::InvalidCallback(CallbackViolation::Uri));
    }
    let parameters = parse_form(query)?;
    let state = required_parameter(&parameters, "state")?;
    if !constant_time_equal(state.as_bytes(), transaction.state.as_bytes()) {
        return Err(OAuthError::InvalidCallback(CallbackViolation::State));
    }
    if let MixUpDefense::AuthorizationResponseIssuer(expected_issuer) = &transaction.mix_up_defense
    {
        let issuer = required_parameter(&parameters, "iss")?;
        if !constant_time_equal(issuer.as_bytes(), expected_issuer.as_bytes()) {
            return Err(OAuthError::InvalidCallback(CallbackViolation::Issuer));
        }
    }
    if let Some(error) = parameters.get("error") {
        return if error == "access_denied" {
            Err(OAuthError::ProviderDenied)
        } else {
            Err(OAuthError::ProviderError)
        };
    }
    let code = required_parameter(&parameters, "code")?;
    if code.is_empty()
        || code.len() > MAX_AUTHORIZATION_CODE_BYTES
        || code.chars().any(char::is_control)
    {
        return Err(OAuthError::InvalidCallback(CallbackViolation::Code));
    }

    let form_body = render_secret_form([
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", transaction.redirect_uri.as_str()),
        ("client_id", transaction.client_id.as_str()),
        ("code_verifier", transaction.pkce_verifier.as_str()),
    ]);
    Ok(TokenExchangeRequest {
        provider: transaction.provider,
        trace: transaction.trace,
        endpoint: transaction.token_endpoint,
        form_body,
    })
}

fn audited<T>(
    provider: ProviderId,
    trace: OAuthTraceId,
    action: OAuthAuditAction,
    result: Result<T, OAuthError>,
) -> Audited<T> {
    let outcome = match result.as_ref() {
        Ok(_) => OAuthAuditOutcome::Succeeded,
        Err(OAuthError::ProviderDenied) => OAuthAuditOutcome::Denied,
        Err(error) => OAuthAuditOutcome::Failed(error.failure_class()),
    };
    Audited {
        audit: OAuthAuditEvent {
            provider,
            trace,
            action,
            outcome,
        },
        result,
    }
}

/// Closed provider/configuration violation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigurationViolation {
    /// Provider identifier grammar failed.
    ProviderId,
    /// An endpoint was not a bounded, credential-free HTTPS URL.
    Endpoint,
    /// The public client identifier was invalid.
    ClientId,
    /// Redirect URI was not a bounded loopback HTTP or claimed HTTPS URI.
    RedirectUri,
    /// Expected issuer was invalid.
    Issuer,
    /// No explicit authorization-server mix-up defense was configured.
    MixUpDefense,
    /// A provider extra parameter was invalid or attempted to override core data.
    ExtraParameter,
    /// Requested scope grammar or bounds failed.
    Scope,
    /// PKCE verifier grammar or bounds failed.
    PkceVerifier,
    /// A token, token response, or revocation input failed strict bounds.
    TokenInput,
}

/// Closed callback validation violation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallbackViolation {
    /// Callback base URI did not exactly match the configured redirect.
    Uri,
    /// Query encoding, bounds, duplicates, or required parameters were invalid.
    Query,
    /// State did not match the one-use transaction.
    State,
    /// Authorization-server issuer was absent or mismatched.
    Issuer,
    /// Authorization code was absent or malformed.
    Code,
}

/// Closed, attacker-text-free OAuth protocol error.
#[derive(Clone, PartialEq, Eq)]
pub enum OAuthError {
    /// Provider or request configuration was invalid.
    InvalidConfiguration(ConfigurationViolation),
    /// Caller-provided entropy failed.
    Entropy,
    /// Authorization callback validation failed.
    InvalidCallback(CallbackViolation),
    /// Resource owner denied authorization.
    ProviderDenied,
    /// Authorization server returned another OAuth error code.
    ProviderError,
    /// The token endpoint response was malformed or internally inconsistent.
    InvalidTokenResponse(TokenResponseViolation),
    /// The token endpoint returned a bounded, classified OAuth error code.
    TokenEndpoint(ProviderTokenError),
    /// Durable audit publication failed; the wrapped result was not released.
    Audit,
}

impl OAuthError {
    fn failure_class(&self) -> OAuthFailureClass {
        match self {
            Self::InvalidConfiguration(_) => OAuthFailureClass::InvalidInput,
            Self::Entropy => OAuthFailureClass::Entropy,
            Self::InvalidCallback(_) => OAuthFailureClass::InvalidCallback,
            Self::ProviderDenied
            | Self::ProviderError
            | Self::InvalidTokenResponse(_)
            | Self::TokenEndpoint(_) => OAuthFailureClass::Provider,
            Self::Audit => OAuthFailureClass::Audit,
        }
    }
}

impl Debug for OAuthError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfiguration(reason) => formatter
                .debug_tuple("InvalidConfiguration")
                .field(reason)
                .finish(),
            Self::InvalidCallback(reason) => formatter
                .debug_tuple("InvalidCallback")
                .field(reason)
                .finish(),
            Self::Entropy => formatter.write_str("Entropy"),
            Self::ProviderDenied => formatter.write_str("ProviderDenied"),
            Self::ProviderError => formatter.write_str("ProviderError"),
            Self::InvalidTokenResponse(reason) => formatter
                .debug_tuple("InvalidTokenResponse")
                .field(reason)
                .finish(),
            Self::TokenEndpoint(code) => {
                formatter.debug_tuple("TokenEndpoint").field(code).finish()
            }
            Self::Audit => formatter.write_str("Audit"),
        }
    }
}

impl Display for OAuthError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidConfiguration(_) => "oauth: invalid configuration",
            Self::Entropy => "oauth: entropy unavailable",
            Self::InvalidCallback(_) => "oauth: invalid authorization callback",
            Self::ProviderDenied => "oauth: authorization denied",
            Self::ProviderError => "oauth: provider rejected authorization",
            Self::InvalidTokenResponse(_) => "oauth: invalid token response",
            Self::TokenEndpoint(_) => "oauth: token endpoint rejected request",
            Self::Audit => "oauth: audit publication failed",
        })
    }
}

impl std::error::Error for OAuthError {}

fn validate_https_endpoint(value: &str) -> Result<(), OAuthError> {
    if value.is_empty() || value.len() > MAX_ENDPOINT_BYTES {
        return Err(OAuthError::InvalidConfiguration(
            ConfigurationViolation::Endpoint,
        ));
    }
    let parsed = Url::parse(value)
        .map_err(|_| OAuthError::InvalidConfiguration(ConfigurationViolation::Endpoint))?;
    if parsed.scheme != "https"
        || parsed.host.is_none()
        || parsed.userinfo.is_some()
        || parsed.query.is_some()
        || parsed.fragment.is_some()
    {
        return Err(OAuthError::InvalidConfiguration(
            ConfigurationViolation::Endpoint,
        ));
    }
    Ok(())
}

fn validate_redirect_uri(value: &str) -> Result<(), OAuthError> {
    if value.is_empty() || value.len() > MAX_ENDPOINT_BYTES {
        return Err(OAuthError::InvalidConfiguration(
            ConfigurationViolation::RedirectUri,
        ));
    }
    let parsed = Url::parse(value)
        .map_err(|_| OAuthError::InvalidConfiguration(ConfigurationViolation::RedirectUri))?;
    let host = parsed.host.as_deref();
    let loopback_http = parsed.scheme == "http" && matches!(host, Some("127.0.0.1") | Some("::1"));
    let claimed_https = parsed.scheme == "https" && host.is_some();
    if (!loopback_http && !claimed_https)
        || parsed.userinfo.is_some()
        || parsed.query.is_some()
        || parsed.fragment.is_some()
        || parsed.path.is_empty()
    {
        return Err(OAuthError::InvalidConfiguration(
            ConfigurationViolation::RedirectUri,
        ));
    }
    Ok(())
}

fn validate_issuer(value: &str) -> Result<(), OAuthError> {
    if value.is_empty() || value.len() > MAX_ENDPOINT_BYTES {
        return Err(OAuthError::InvalidConfiguration(
            ConfigurationViolation::Issuer,
        ));
    }
    let parsed = Url::parse(value)
        .map_err(|_| OAuthError::InvalidConfiguration(ConfigurationViolation::Issuer))?;
    if parsed.scheme != "https"
        || parsed.host.is_none()
        || parsed.userinfo.is_some()
        || parsed.query.is_some()
        || parsed.fragment.is_some()
    {
        return Err(OAuthError::InvalidConfiguration(
            ConfigurationViolation::Issuer,
        ));
    }
    Ok(())
}

fn validate_client_id(value: &str) -> Result<(), OAuthError> {
    if value.is_empty() || value.len() > MAX_CLIENT_ID_BYTES || value.chars().any(char::is_control)
    {
        return Err(OAuthError::InvalidConfiguration(
            ConfigurationViolation::ClientId,
        ));
    }
    Ok(())
}

fn valid_parameter(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_PARAMETER_BYTES && !value.chars().any(char::is_control)
}

fn validate_scopes(scopes: &[&str]) -> Result<(), OAuthError> {
    if scopes.is_empty() || scopes.len() > MAX_SCOPES {
        return Err(OAuthError::InvalidConfiguration(
            ConfigurationViolation::Scope,
        ));
    }
    let mut unique = BTreeSet::new();
    for scope in scopes {
        let valid = !scope.is_empty()
            && scope.len() <= MAX_SCOPE_BYTES
            && scope
                .bytes()
                .all(|byte| matches!(byte, 0x21 | 0x23..=0x5b | 0x5d..=0x7e));
        if !valid || !unique.insert(*scope) {
            return Err(OAuthError::InvalidConfiguration(
                ConfigurationViolation::Scope,
            ));
        }
    }
    Ok(())
}

fn valid_pkce_verifier(value: &str) -> bool {
    (43..=128).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~'))
}

fn base64_url_no_pad(input: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut output = String::with_capacity((input.len() * 4).div_ceil(3));
    let (chunks, remainder) = input.as_chunks::<3>();
    for chunk in chunks {
        let value = (u32::from(chunk[0]) << 16) | (u32::from(chunk[1]) << 8) | u32::from(chunk[2]);
        output.push(char::from(ALPHABET[((value >> 18) & 0x3f) as usize]));
        output.push(char::from(ALPHABET[((value >> 12) & 0x3f) as usize]));
        output.push(char::from(ALPHABET[((value >> 6) & 0x3f) as usize]));
        output.push(char::from(ALPHABET[(value & 0x3f) as usize]));
    }
    match remainder {
        [first] => {
            let value = u16::from(*first) << 8;
            output.push(char::from(ALPHABET[((value >> 10) & 0x3f) as usize]));
            output.push(char::from(ALPHABET[((value >> 4) & 0x3f) as usize]));
        }
        [first, second] => {
            let value = (u32::from(*first) << 16) | (u32::from(*second) << 8);
            output.push(char::from(ALPHABET[((value >> 18) & 0x3f) as usize]));
            output.push(char::from(ALPHABET[((value >> 12) & 0x3f) as usize]));
            output.push(char::from(ALPHABET[((value >> 6) & 0x3f) as usize]));
        }
        [] => {}
        _ => unreachable!("chunks_exact remainder is shorter than three"),
    }
    output
}

fn render_form<'a, I>(parameters: I) -> String
where
    I: IntoIterator<Item = (&'a str, &'a str)>,
{
    let mut output = String::new();
    append_form(&mut output, parameters);
    output
}

fn render_secret_form<'a, I>(parameters: I) -> Zeroizing<String>
where
    I: IntoIterator<Item = (&'a str, &'a str)>,
{
    let mut output = Zeroizing::new(String::new());
    append_form(&mut output, parameters);
    output
}

fn append_form<'a, I>(output: &mut String, parameters: I)
where
    I: IntoIterator<Item = (&'a str, &'a str)>,
{
    for (index, (key, value)) in parameters.into_iter().enumerate() {
        if index > 0 {
            output.push('&');
        }
        append_form_encoded(output, key);
        output.push('=');
        append_form_encoded(output, value);
    }
}

#[cfg(test)]
fn form_encode(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    append_form_encoded(&mut output, value);
    output
}

fn append_form_encoded(output: &mut String, value: &str) {
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            output.push(char::from(byte));
        } else {
            const HEX: &[u8; 16] = b"0123456789ABCDEF";
            output.push('%');
            output.push(char::from(HEX[(byte >> 4) as usize]));
            output.push(char::from(HEX[(byte & 0x0f) as usize]));
        }
    }
}

fn parse_form(query: &str) -> Result<BTreeMap<String, String>, OAuthError> {
    let mut parameters = BTreeMap::new();
    for pair in query.split('&') {
        let (encoded_key, encoded_value) = pair
            .split_once('=')
            .ok_or(OAuthError::InvalidCallback(CallbackViolation::Query))?;
        let key = form_decode(encoded_key)?;
        let value = form_decode(encoded_value)?;
        if key.is_empty()
            || key.len() > MAX_PARAMETER_BYTES
            || value.len() > MAX_AUTHORIZATION_CODE_BYTES
            || parameters.insert(key, value).is_some()
        {
            return Err(OAuthError::InvalidCallback(CallbackViolation::Query));
        }
    }
    Ok(parameters)
}

fn form_decode(value: &str) -> Result<String, OAuthError> {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                output.push(b' ');
                index += 1;
            }
            b'%' if index + 2 < bytes.len() => {
                let high = hex_value(bytes[index + 1])?;
                let low = hex_value(bytes[index + 2])?;
                output.push((high << 4) | low);
                index += 3;
            }
            b'%' => return Err(OAuthError::InvalidCallback(CallbackViolation::Query)),
            byte => {
                output.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8(output).map_err(|_| OAuthError::InvalidCallback(CallbackViolation::Query))
}

fn hex_value(byte: u8) -> Result<u8, OAuthError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(OAuthError::InvalidCallback(CallbackViolation::Query)),
    }
}

fn required_parameter<'a>(
    parameters: &'a BTreeMap<String, String>,
    key: &str,
) -> Result<&'a str, OAuthError> {
    parameters
        .get(key)
        .map(String::as_str)
        .ok_or(OAuthError::InvalidCallback(CallbackViolation::Query))
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    let mut difference = left.len() ^ right.len();
    let maximum = left.len().max(right.len());
    for index in 0..maximum {
        let left_byte = left.get(index).copied().unwrap_or(0);
        let right_byte = right.get(index).copied().unwrap_or(0);
        difference |= usize::from(left_byte ^ right_byte);
    }
    difference == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixedEntropy {
        bytes: [u8; ENTROPY_BYTES * 2],
    }

    impl FixedEntropy {
        fn ascending() -> Self {
            let mut bytes = [0_u8; ENTROPY_BYTES * 2];
            for (index, byte) in bytes.iter_mut().enumerate() {
                *byte = index as u8;
            }
            Self { bytes }
        }
    }

    impl EntropySource for FixedEntropy {
        fn fill(&mut self, destination: &mut [u8]) -> Result<(), OAuthError> {
            destination.copy_from_slice(&self.bytes[..destination.len()]);
            Ok(())
        }
    }

    #[derive(Default)]
    struct RecordingAuditSink {
        events: Vec<OAuthAuditEvent>,
        fail: bool,
    }

    impl OAuthAuditSink for RecordingAuditSink {
        fn publish(&mut self, event: &OAuthAuditEvent) -> Result<(), OAuthAuditError> {
            if self.fail {
                return Err(OAuthAuditError);
            }
            self.events.push(event.clone());
            Ok(())
        }
    }

    fn trace() -> OAuthTraceId {
        OAuthTraceId::new([0x42; TRACE_BYTES])
    }

    fn release<T>(audited: Audited<T>) -> Result<T, OAuthError> {
        audited.publish_then_release(&mut RecordingAuditSink::default())
    }

    fn config() -> ProviderConfig {
        ProviderConfig::new(
            ProviderId::new("fixture").unwrap(),
            "https://authorize.example/oauth2/auth",
            "https://token.example/oauth2/token",
            "public client/id",
            "http://127.0.0.1:53682/callback",
        )
        .unwrap()
        .with_expected_issuer("https://issuer.example")
        .unwrap()
    }

    fn begin() -> AuthorizationRequest {
        begin_authorization(
            &config(),
            &["files.read", "files.write"],
            trace(),
            &mut FixedEntropy::ascending(),
        )
        .publish_then_release(&mut RecordingAuditSink::default())
        .unwrap()
    }

    #[test]
    fn rfc_7636_s256_vector_matches() {
        let verifier = PkceVerifier::new("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk").unwrap();
        assert_eq!(
            pkce_s256_challenge(&verifier),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn authorization_request_is_deterministic_mandatory_s256_and_audited() {
        let mut extras = BTreeMap::new();
        extras.insert("access_type".to_string(), "offline".to_string());
        extras.insert("prompt".to_string(), "consent select".to_string());
        let config = config()
            .with_authorization_extra_parameters(extras)
            .unwrap();
        let audited = begin_authorization(
            &config,
            &["files.read", "files.write"],
            trace(),
            &mut FixedEntropy::ascending(),
        );
        assert_eq!(audited.audit().provider().as_str(), "fixture");
        assert_eq!(
            audited.audit().action(),
            OAuthAuditAction::AuthorizationBegin
        );
        assert_eq!(audited.audit().outcome(), OAuthAuditOutcome::Succeeded);
        assert_eq!(audited.audit().trace(), trace());
        let request = release(audited).unwrap();
        let url = request.url().as_str();
        assert!(
            url.starts_with("https://authorize.example/oauth2/auth?client_id=public%20client%2Fid")
        );
        assert!(url.contains("response_type=code"));
        assert!(url.contains("scope=files.read%20files.write"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("access_type=offline&prompt=consent%20select"));
        assert!(!url.contains("code_verifier"));
    }

    #[test]
    fn callback_is_exact_state_and_issuer_bound_then_prepares_exchange() {
        let request = begin();
        let (_, transaction) = request.into_parts();
        let state = base64_url_no_pad(&(0_u8..32).collect::<Vec<_>>());
        let callback = format!(
            "http://127.0.0.1:53682/callback?code=secret%2Fcode&state={state}&iss=https%3A%2F%2Fissuer.example"
        );
        let audited = complete_authorization(transaction, &callback);
        assert_eq!(
            audited.audit().action(),
            OAuthAuditAction::AuthorizationComplete
        );
        assert_eq!(audited.audit().outcome(), OAuthAuditOutcome::Succeeded);
        let exchange = release(audited).unwrap();
        assert_eq!(exchange.provider().as_str(), "fixture");
        assert_eq!(exchange.endpoint(), "https://token.example/oauth2/token");
        assert_eq!(exchange.content_type(), "application/x-www-form-urlencoded");
        assert!(exchange
            .form_body()
            .contains("grant_type=authorization_code"));
        assert!(exchange.form_body().contains("code=secret%2Fcode"));
        assert!(exchange
            .form_body()
            .contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A53682%2Fcallback"));
        assert!(exchange
            .form_body()
            .contains("client_id=public%20client%2Fid"));
        assert!(exchange.form_body().contains("code_verifier="));
    }

    #[test]
    fn callback_state_mismatch_fails_with_privacy_safe_audit() {
        let (_, transaction) = begin().into_parts();
        let audited = complete_authorization(
            transaction,
            "http://127.0.0.1:53682/callback?code=secret&state=wrong&iss=https%3A%2F%2Fissuer.example",
        );
        assert_eq!(
            audited.audit().outcome(),
            OAuthAuditOutcome::Failed(OAuthFailureClass::InvalidCallback)
        );
        assert_eq!(
            release(audited).unwrap_err(),
            OAuthError::InvalidCallback(CallbackViolation::State)
        );
    }

    #[test]
    fn callback_rejects_redirect_mixup_issuer_mixup_and_duplicate_keys() {
        let (_, transaction) = begin().into_parts();
        let wrong_redirect = complete_authorization(
            transaction,
            "http://127.0.0.1:53683/callback?code=x&state=x&iss=https%3A%2F%2Fissuer.example",
        );
        assert_eq!(
            release(wrong_redirect).unwrap_err(),
            OAuthError::InvalidCallback(CallbackViolation::Uri)
        );

        let (_, transaction) = begin().into_parts();
        let state = base64_url_no_pad(&(0_u8..32).collect::<Vec<_>>());
        let wrong_issuer = complete_authorization(
            transaction,
            &format!("http://127.0.0.1:53682/callback?code=x&state={state}&iss=https%3A%2F%2Fevil.example"),
        );
        assert_eq!(
            release(wrong_issuer).unwrap_err(),
            OAuthError::InvalidCallback(CallbackViolation::Issuer)
        );

        let (_, transaction) = begin().into_parts();
        let duplicate = complete_authorization(
            transaction,
            &format!("http://127.0.0.1:53682/callback?code=x&state={state}&state={state}&iss=https%3A%2F%2Fissuer.example"),
        );
        assert_eq!(
            release(duplicate).unwrap_err(),
            OAuthError::InvalidCallback(CallbackViolation::Query)
        );
    }

    #[test]
    fn provider_denial_is_closed_and_distinct_in_audit() {
        let (_, transaction) = begin().into_parts();
        let state = base64_url_no_pad(&(0_u8..32).collect::<Vec<_>>());
        let audited = complete_authorization(
            transaction,
            &format!("http://127.0.0.1:53682/callback?error=access_denied&error_description=attacker+text&state={state}&iss=https%3A%2F%2Fissuer.example"),
        );
        assert_eq!(audited.audit().outcome(), OAuthAuditOutcome::Denied);
        assert_eq!(release(audited).unwrap_err(), OAuthError::ProviderDenied);
    }

    #[test]
    fn configuration_and_scope_grammar_fail_closed() {
        assert_eq!(
            ProviderId::new("Google"),
            Err(OAuthError::InvalidConfiguration(
                ConfigurationViolation::ProviderId
            ))
        );
        assert!(ProviderConfig::new(
            ProviderId::new("bad-endpoint").unwrap(),
            "http://authorize.example/auth",
            "https://token.example/token",
            "client",
            "http://127.0.0.1:53682/callback",
        )
        .is_err());
        let audited = begin_authorization(
            &config(),
            &["valid", "invalid scope"],
            trace(),
            &mut FixedEntropy::ascending(),
        );
        assert_eq!(
            audited.audit().outcome(),
            OAuthAuditOutcome::Failed(OAuthFailureClass::InvalidInput)
        );

        let no_mix_up_defense = ProviderConfig::new(
            ProviderId::new("distinct").unwrap(),
            "https://authorize.example/auth",
            "https://token.example/token",
            "client",
            "http://127.0.0.1:53683/distinct-callback",
        )
        .unwrap();
        let audited = begin_authorization(
            &no_mix_up_defense,
            &["valid"],
            trace(),
            &mut FixedEntropy::ascending(),
        );
        assert_eq!(
            release(audited).unwrap_err(),
            OAuthError::InvalidConfiguration(ConfigurationViolation::MixUpDefense)
        );
    }

    #[test]
    fn distinct_redirect_mode_and_audit_failure_both_fail_or_release_exactly() {
        let distinct = ProviderConfig::new(
            ProviderId::new("distinct").unwrap(),
            "https://authorize.example/auth",
            "https://token.example/token",
            "client",
            "http://127.0.0.1:53683/distinct-callback",
        )
        .unwrap()
        .with_distinct_redirect_uri();
        let audited = begin_authorization(
            &distinct,
            &["files.read"],
            trace(),
            &mut FixedEntropy::ascending(),
        );
        let mut unavailable = RecordingAuditSink {
            events: Vec::new(),
            fail: true,
        };
        assert_eq!(
            audited.publish_then_release(&mut unavailable).unwrap_err(),
            OAuthError::Audit
        );
        assert!(unavailable.events.is_empty());

        let request = release(begin_authorization(
            &distinct,
            &["files.read"],
            trace(),
            &mut FixedEntropy::ascending(),
        ))
        .unwrap();
        let (_, transaction) = request.into_parts();
        let state = base64_url_no_pad(&(0_u8..32).collect::<Vec<_>>());
        let exchange = release(complete_authorization(
            transaction,
            &format!("http://127.0.0.1:53683/distinct-callback?code=ok&state={state}"),
        ))
        .unwrap();
        assert_eq!(exchange.provider().as_str(), "distinct");
    }

    #[test]
    fn forged_denial_must_pass_state_and_issuer_before_it_is_observable() {
        let (_, transaction) = begin().into_parts();
        let audited = complete_authorization(
            transaction,
            "http://127.0.0.1:53682/callback?error=access_denied&state=wrong&iss=https%3A%2F%2Fissuer.example",
        );
        assert_eq!(
            audited.audit().outcome(),
            OAuthAuditOutcome::Failed(OAuthFailureClass::InvalidCallback)
        );
        assert_eq!(
            release(audited).unwrap_err(),
            OAuthError::InvalidCallback(CallbackViolation::State)
        );
    }

    #[test]
    fn diagnostics_never_expose_transaction_or_exchange_secrets() {
        let request = begin();
        let debug = format!("{request:?}");
        assert!(!debug.contains("AAECAw"));
        assert!(!debug.contains("authorize.example"));
        let (_, transaction) = request.into_parts();
        let state = base64_url_no_pad(&(0_u8..32).collect::<Vec<_>>());
        let exchange = complete_authorization(
            transaction,
            &format!("http://127.0.0.1:53682/callback?code=super-secret-code&state={state}&iss=https%3A%2F%2Fissuer.example"),
        )
        .publish_then_release(&mut RecordingAuditSink::default())
        .unwrap();
        let debug = format!("{exchange:?}");
        assert!(!debug.contains("super-secret-code"));
        assert!(!debug.contains("token.example"));
        assert!(!debug.contains("code_verifier"));
    }

    #[test]
    fn base64url_and_form_codecs_cover_tail_and_unicode_cases() {
        assert_eq!(base64_url_no_pad(b""), "");
        assert_eq!(base64_url_no_pad(b"f"), "Zg");
        assert_eq!(base64_url_no_pad(b"fo"), "Zm8");
        assert_eq!(base64_url_no_pad(b"foo"), "Zm9v");
        assert_eq!(form_encode("a b/c~"), "a%20b%2Fc~");
        assert_eq!(form_decode("snowman%3D%E2%98%83").unwrap(), "snowman=☃");
        assert!(form_decode("%GG").is_err());
    }
}
