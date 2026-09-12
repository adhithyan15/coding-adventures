//! RFC 8628 device authorization and caller-driven polling primitives.

use super::{
    audited, audited_with_outcome, json_nesting_within_limit, render_secret_form, valid_uri_text,
    validate_client_id, validate_scopes, Audited, AuthorizationServerMetadata,
    ConfigurationViolation, OAuthAuditAction, OAuthAuditOutcome, OAuthError, OAuthTraceId,
    ProviderConfig, ProviderId, ProviderTokenError, TokenResponse, TokenResponseContext,
    TokenResponseFormat, TokenResponseViolation, MAX_ENDPOINT_BYTES, MAX_JSON_NESTING,
};
use crate::token::{decode_token_response_inner, zeroize_json};
use coding_adventures_bounded_json::{JsonNumber, JsonValue};
use coding_adventures_zeroize::Zeroizing;
use std::collections::BTreeSet;
use std::fmt::{self, Debug, Formatter};
use url_parser::Url;

const DEVICE_GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";
const DEFAULT_POLL_INTERVAL_SECONDS: u64 = 5;
const SLOW_DOWN_INCREMENT_SECONDS: u64 = 5;
const MAX_DEVICE_RESPONSE_BYTES: usize = 128 * 1024;
const MAX_DEVICE_RESPONSE_FIELDS: usize = 32;
const MAX_DEVICE_CODE_BYTES: usize = 4 * 1024;
const MAX_USER_CODE_BYTES: usize = 256;
const MAX_DEVICE_LIFETIME_SECONDS: u64 = 24 * 60 * 60;
const MAX_POLL_INTERVAL_SECONDS: u64 = 60 * 60;
const MAX_CONTENT_TYPE_BYTES: usize = 256;

/// Metadata-derived public-client configuration for RFC 8628 initiation.
pub struct DeviceAuthorizationProfile {
    provider: ProviderId,
    device_authorization_endpoint: String,
    token_endpoint: String,
    client_id: String,
}

impl DeviceAuthorizationProfile {
    /// Derive a device profile only from exact advertised provider capabilities.
    ///
    /// The metadata must advertise the RFC 8628 grant, a device authorization
    /// endpoint, and public-client token authentication method `none`. No
    /// default endpoint, grant, or authentication method is inferred.
    pub fn from_metadata(
        metadata: &AuthorizationServerMetadata,
        client_id: impl Into<String>,
    ) -> Result<Self, OAuthError> {
        let client_id = client_id.into();
        validate_client_id(&client_id)?;
        let endpoint =
            metadata
                .device_authorization_endpoint()
                .ok_or(OAuthError::InvalidConfiguration(
                    ConfigurationViolation::DeviceAuthorization,
                ))?;
        let supports_grant = metadata
            .grant_types_supported()
            .iter()
            .any(|grant| grant == DEVICE_GRANT_TYPE);
        let supports_public_client = metadata
            .token_endpoint_auth_methods_supported()
            .iter()
            .any(|method| method == "none");
        if !supports_grant || !supports_public_client {
            return Err(OAuthError::InvalidConfiguration(
                ConfigurationViolation::DeviceAuthorization,
            ));
        }
        Ok(Self {
            provider: metadata.provider().clone(),
            device_authorization_endpoint: endpoint.to_owned(),
            token_endpoint: metadata.token_endpoint().to_owned(),
            client_id,
        })
    }

    /// Return the provider identity retained from validated metadata.
    pub fn provider(&self) -> &ProviderId {
        &self.provider
    }
}

impl Debug for DeviceAuthorizationProfile {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DeviceAuthorizationProfile")
            .field("provider", &self.provider)
            .field("device_authorization_endpoint", &"<redacted>")
            .field("token_endpoint", &"<redacted>")
            .field("client_id", &"<redacted>")
            .finish()
    }
}

/// Prepared RFC 8628 device authorization request.
pub struct DeviceAuthorizationRequest {
    provider: ProviderId,
    trace: OAuthTraceId,
    client_id: String,
    endpoint: String,
    token_endpoint: String,
    form_body: Zeroizing<String>,
}

impl DeviceAuthorizationRequest {
    /// Return the provider identifier for transport authorization and audit.
    pub fn provider(&self) -> &ProviderId {
        &self.provider
    }

    /// Return the caller-owned correlation identity.
    pub const fn trace(&self) -> OAuthTraceId {
        self.trace
    }

    /// Return the exact client identity bound to this request.
    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    /// Borrow the metadata-validated device authorization endpoint.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Borrow the form body only after durable audit release.
    pub fn form_body(&self) -> &str {
        self.form_body.as_str()
    }

    /// Return the exact request media type.
    pub const fn content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }

    /// Bind a response decoder to the exact provider, client, token endpoint, and trace.
    pub fn response_context(&self) -> DeviceAuthorizationResponseContext {
        DeviceAuthorizationResponseContext {
            provider: self.provider.clone(),
            trace: self.trace,
            client_id: self.client_id.clone(),
            token_endpoint: self.token_endpoint.clone(),
        }
    }
}

impl Debug for DeviceAuthorizationRequest {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DeviceAuthorizationRequest")
            .field("provider", &self.provider)
            .field("trace", &self.trace)
            .field("client_id", &"<redacted>")
            .field("endpoint", &"<redacted>")
            .field("token_endpoint", &"<redacted>")
            .field("form_body", &"<redacted>")
            .finish()
    }
}

/// Non-secret binding between one device request and its response.
pub struct DeviceAuthorizationResponseContext {
    provider: ProviderId,
    trace: OAuthTraceId,
    client_id: String,
    token_endpoint: String,
}

impl DeviceAuthorizationResponseContext {
    /// Return the provider expected to own the response.
    pub fn provider(&self) -> &ProviderId {
        &self.provider
    }

    /// Return the correlation identity inherited from the request.
    pub const fn trace(&self) -> OAuthTraceId {
        self.trace
    }
}

impl Debug for DeviceAuthorizationResponseContext {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DeviceAuthorizationResponseContext")
            .field("provider", &self.provider)
            .field("trace", &self.trace)
            .field("client_id", &"<redacted>")
            .field("token_endpoint", &"<redacted>")
            .finish()
    }
}

/// User-facing verification data released only after response audit succeeds.
pub struct DeviceVerification {
    user_code: Zeroizing<String>,
    verification_uri: String,
    verification_uri_complete: Option<Zeroizing<String>>,
    expires_in_seconds: u64,
}

impl DeviceVerification {
    /// Borrow the short code that the user enters at the verification URI.
    pub fn user_code(&self) -> &str {
        self.user_code.as_str()
    }

    /// Borrow the validated HTTPS verification URI.
    pub fn verification_uri(&self) -> &str {
        &self.verification_uri
    }

    /// Borrow the optional validated HTTPS URI with the user code embedded.
    pub fn verification_uri_complete(&self) -> Option<&str> {
        self.verification_uri_complete
            .as_ref()
            .map(|uri| uri.as_str())
    }

    /// Return the relative lifetime; a caller-owned clock determines expiry.
    pub const fn expires_in_seconds(&self) -> u64 {
        self.expires_in_seconds
    }
}

impl Debug for DeviceVerification {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DeviceVerification")
            .field("user_code", &"<redacted>")
            .field("verification_uri", &"<redacted>")
            .field(
                "has_verification_uri_complete",
                &self.verification_uri_complete.is_some(),
            )
            .field("expires_in_seconds", &self.expires_in_seconds)
            .finish()
    }
}

/// Opaque state retained for a later caller-driven polling state machine.
pub struct DevicePollingSession {
    provider: ProviderId,
    trace: OAuthTraceId,
    client_id: String,
    token_endpoint: String,
    device_code: Zeroizing<String>,
    expires_in_seconds: u64,
    interval_seconds: u64,
}

impl DevicePollingSession {
    /// Return the provider identity bound to the device code.
    pub fn provider(&self) -> &ProviderId {
        &self.provider
    }

    /// Return the trace shared by initiation and later polling.
    pub const fn trace(&self) -> OAuthTraceId {
        self.trace
    }

    /// Return the initial minimum polling interval from RFC 8628.
    pub const fn interval_seconds(&self) -> u64 {
        self.interval_seconds
    }

    /// Return the relative session lifetime for a caller-owned clock.
    pub const fn expires_in_seconds(&self) -> u64 {
        self.expires_in_seconds
    }

    /// Return whether this session exactly matches one registered public client.
    ///
    /// The device code stays opaque while a broker can still reject a session
    /// from another provider, client, or token endpoint before transport.
    pub fn is_bound_to(&self, config: &ProviderConfig) -> bool {
        self.provider == config.provider
            && self.client_id == config.client_id
            && self.token_endpoint == config.token_endpoint
    }
}

impl Debug for DevicePollingSession {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DevicePollingSession")
            .field("provider", &self.provider)
            .field("trace", &self.trace)
            .field("client_id", &"<redacted>")
            .field("token_endpoint", &"<redacted>")
            .field("device_code", &"<redacted>")
            .field("expires_in_seconds", &self.expires_in_seconds)
            .field("interval_seconds", &self.interval_seconds)
            .finish()
    }
}

/// One RFC 8628 token request prepared for an externally scheduled poll.
pub struct DeviceTokenPollRequest {
    session: DevicePollingSession,
    form_body: Zeroizing<String>,
}

impl DeviceTokenPollRequest {
    /// Return the provider identifier for transport authorization and audit.
    pub fn provider(&self) -> &ProviderId {
        &self.session.provider
    }

    /// Return the trace shared with device authorization initiation.
    pub const fn trace(&self) -> OAuthTraceId {
        self.session.trace
    }

    /// Return the exact client identity bound to this poll.
    pub fn client_id(&self) -> &str {
        &self.session.client_id
    }

    /// Borrow the metadata-validated token endpoint.
    pub fn endpoint(&self) -> &str {
        &self.session.token_endpoint
    }

    /// Borrow the secret-bearing form body only after durable audit release.
    pub fn form_body(&self) -> &str {
        self.form_body.as_str()
    }

    /// Return the exact request media type.
    pub const fn content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }

    /// Consume the sent request into the only context accepted by the response classifier.
    pub fn response_context(self) -> DeviceTokenPollResponseContext {
        DeviceTokenPollResponseContext {
            session: self.session,
        }
    }

    /// Recover the opaque session when transport fails before a response exists.
    ///
    /// The caller still owns retry scheduling and must prepare and audit a new
    /// request before another transport attempt.
    pub fn into_session(self) -> DevicePollingSession {
        self.session
    }
}

impl Debug for DeviceTokenPollRequest {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DeviceTokenPollRequest")
            .field("provider", &self.session.provider)
            .field("trace", &self.session.trace)
            .field("client_id", &"<redacted>")
            .field("endpoint", &"<redacted>")
            .field("form_body", &"<redacted>")
            .finish()
    }
}

/// One-use response binding created from the exact poll request sent on the wire.
pub struct DeviceTokenPollResponseContext {
    session: DevicePollingSession,
}

impl DeviceTokenPollResponseContext {
    /// Return the provider expected to own the response.
    pub fn provider(&self) -> &ProviderId {
        &self.session.provider
    }

    /// Return the trace shared by initiation, poll preparation, and classification.
    pub const fn trace(&self) -> OAuthTraceId {
        self.session.trace
    }
}

impl Debug for DeviceTokenPollResponseContext {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DeviceTokenPollResponseContext")
            .field("provider", &self.session.provider)
            .field("trace", &self.session.trace)
            .field("client_id", &"<redacted>")
            .field("token_endpoint", &"<redacted>")
            .field("device_code", &"<redacted>")
            .finish()
    }
}

/// Audited RFC 8628 transition produced from one token-endpoint response.
pub enum DevicePollResult {
    /// Authorization remains incomplete; retry no sooner than the retained interval.
    Pending(DevicePollingSession),
    /// The provider requested an additional five-second delay for all later polls.
    SlowDown(DevicePollingSession),
    /// Authorization completed and yielded a normal audited token response.
    Authorized(TokenResponse),
    /// The resource owner denied the request.
    Denied,
    /// The device code expired at the authorization server.
    ExpiredToken,
}

impl DevicePollResult {
    /// Return the minimum caller-owned delay before the next poll, when applicable.
    pub const fn retry_after_seconds(&self) -> Option<u64> {
        match self {
            Self::Pending(session) | Self::SlowDown(session) => Some(session.interval_seconds()),
            Self::Authorized(_) | Self::Denied | Self::ExpiredToken => None,
        }
    }
}

impl Debug for DevicePollResult {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending(session) => formatter.debug_tuple("Pending").field(session).finish(),
            Self::SlowDown(session) => formatter.debug_tuple("SlowDown").field(session).finish(),
            Self::Authorized(response) => {
                formatter.debug_tuple("Authorized").field(response).finish()
            }
            Self::Denied => formatter.write_str("Denied"),
            Self::ExpiredToken => formatter.write_str("ExpiredToken"),
        }
    }
}

/// Validated device authorization result split into display and polling state.
pub struct DeviceAuthorization {
    verification: DeviceVerification,
    polling: DevicePollingSession,
}

impl DeviceAuthorization {
    /// Borrow the data the host may present to the user.
    pub fn verification(&self) -> &DeviceVerification {
        &self.verification
    }

    /// Borrow the opaque polling state without exposing its device code.
    pub fn polling(&self) -> &DevicePollingSession {
        &self.polling
    }

    /// Transfer the user-facing data and opaque polling session without cloning secrets.
    pub fn into_parts(self) -> (DeviceVerification, DevicePollingSession) {
        (self.verification, self.polling)
    }
}

impl Debug for DeviceAuthorization {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DeviceAuthorization")
            .field("verification", &self.verification)
            .field("polling", &self.polling)
            .finish()
    }
}

/// Closed structural reason a device authorization response was rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceAuthorizationResponseViolation {
    /// The HTTP response was not exactly a successful `200` response.
    Status,
    /// The response did not declare the JSON media type.
    MediaType,
    /// The body was empty, oversized, non-UTF-8, or invalid JSON.
    Encoding,
    /// The top-level object, duplicate fields, or field types were invalid.
    Shape,
    /// A device or user code was absent, empty, oversized, or contained controls.
    Code,
    /// A verification URI was absent or failed strict HTTPS policy.
    VerificationUri,
    /// The lifetime or polling interval was absent, zero, or outside safe bounds.
    Timing,
}

/// Prepare an RFC 8628 public-client device authorization request.
pub fn prepare_device_authorization(
    profile: &DeviceAuthorizationProfile,
    requested_scopes: &[&str],
    trace: OAuthTraceId,
) -> Audited<DeviceAuthorizationRequest> {
    let result = (|| {
        validate_scopes(requested_scopes)?;
        let scope = requested_scopes.join(" ");
        Ok(DeviceAuthorizationRequest {
            provider: profile.provider.clone(),
            trace,
            client_id: profile.client_id.clone(),
            endpoint: profile.device_authorization_endpoint.clone(),
            token_endpoint: profile.token_endpoint.clone(),
            form_body: render_secret_form([
                ("client_id", profile.client_id.as_str()),
                ("scope", scope.as_str()),
            ]),
        })
    })();
    audited(
        profile.provider.clone(),
        trace,
        OAuthAuditAction::DeviceAuthorizationPrepare,
        result,
    )
}

/// Decode a bounded RFC 8628 device authorization response.
pub fn decode_device_authorization_response(
    context: DeviceAuthorizationResponseContext,
    status: u16,
    content_type: &str,
    body: Zeroizing<Vec<u8>>,
) -> Audited<DeviceAuthorization> {
    let result = decode_response_inner(&context, status, content_type, &body);
    audited(
        context.provider,
        context.trace,
        OAuthAuditAction::DeviceAuthorizationResponseDecode,
        result,
    )
}

/// Prepare one device-code token request without sleeping or opening a socket.
///
/// The caller owns scheduling and must wait at least
/// [`DevicePollingSession::interval_seconds`] before each call. The opaque
/// session moves into the request so a sent request can become exactly one
/// response context. Only the zeroizing form body contains the transient wire
/// copy of the device code.
pub fn prepare_device_token_poll(session: DevicePollingSession) -> Audited<DeviceTokenPollRequest> {
    let provider = session.provider.clone();
    let trace = session.trace;
    let result = Ok(DeviceTokenPollRequest {
        form_body: render_secret_form([
            ("grant_type", DEVICE_GRANT_TYPE),
            ("device_code", session.device_code.as_str()),
            ("client_id", session.client_id.as_str()),
        ]),
        session,
    });
    audited(
        provider,
        trace,
        OAuthAuditAction::DeviceTokenPollPrepare,
        result,
    )
}

/// Decode and classify one RFC 8628 token response without sleeping or reading a clock.
///
/// `Pending` retains the current minimum interval. `SlowDown` increases it by
/// exactly five seconds for every later request. Only those continuation
/// variants return the opaque device code to the caller.
pub fn decode_device_token_poll_response(
    context: DeviceTokenPollResponseContext,
    status: u16,
    format: TokenResponseFormat,
    body: Zeroizing<Vec<u8>>,
) -> Audited<DevicePollResult> {
    let provider = context.session.provider.clone();
    let trace = context.session.trace;
    let token_context = TokenResponseContext::device_code(provider.clone(), trace);
    let result = (|| match decode_token_response_inner(&token_context, status, format, &body) {
        Ok(response) => Ok(DevicePollResult::Authorized(response)),
        Err(OAuthError::TokenEndpoint(error)) if status == 400 => match error {
            ProviderTokenError::AuthorizationPending => {
                Ok(DevicePollResult::Pending(context.session))
            }
            ProviderTokenError::SlowDown => {
                let mut session = context.session;
                session.interval_seconds = session
                    .interval_seconds
                    .checked_add(SLOW_DOWN_INCREMENT_SECONDS)
                    .ok_or(OAuthError::InvalidConfiguration(
                        ConfigurationViolation::DeviceAuthorization,
                    ))?;
                Ok(DevicePollResult::SlowDown(session))
            }
            ProviderTokenError::AccessDenied => Ok(DevicePollResult::Denied),
            ProviderTokenError::ExpiredToken => Ok(DevicePollResult::ExpiredToken),
            other => Err(OAuthError::TokenEndpoint(other)),
        },
        Err(OAuthError::TokenEndpoint(
            ProviderTokenError::AuthorizationPending
            | ProviderTokenError::SlowDown
            | ProviderTokenError::AccessDenied
            | ProviderTokenError::ExpiredToken,
        )) => Err(OAuthError::InvalidTokenResponse(
            TokenResponseViolation::Status,
        )),
        Err(error) => Err(error),
    })();
    let outcome = match result.as_ref() {
        Ok(DevicePollResult::Denied) => OAuthAuditOutcome::Denied,
        Ok(_) => OAuthAuditOutcome::Succeeded,
        Err(error) => OAuthAuditOutcome::Failed(error.failure_class()),
    };
    audited_with_outcome(
        provider,
        trace,
        OAuthAuditAction::DeviceTokenPollResponseClassify,
        outcome,
        result,
    )
}

fn decode_response_inner(
    context: &DeviceAuthorizationResponseContext,
    status: u16,
    content_type: &str,
    body: &[u8],
) -> Result<DeviceAuthorization, OAuthError> {
    if status != 200 {
        return Err(invalid(DeviceAuthorizationResponseViolation::Status));
    }
    if content_type.is_empty()
        || content_type.len() > MAX_CONTENT_TYPE_BYTES
        || !content_type.is_ascii()
        || !is_json_content_type(content_type)
    {
        return Err(invalid(DeviceAuthorizationResponseViolation::MediaType));
    }
    if body.is_empty() || body.len() > MAX_DEVICE_RESPONSE_BYTES {
        return Err(invalid(DeviceAuthorizationResponseViolation::Encoding));
    }
    let text = std::str::from_utf8(body)
        .map_err(|_| invalid(DeviceAuthorizationResponseViolation::Encoding))?;
    if !json_nesting_within_limit(body) {
        return Err(invalid(DeviceAuthorizationResponseViolation::Encoding));
    }
    let mut root = coding_adventures_bounded_json::parse_with_depth_limit(text, MAX_JSON_NESTING)
        .map_err(|_| invalid(DeviceAuthorizationResponseViolation::Encoding))?;
    let outcome = parse_response_object(context, &root);
    zeroize_json(&mut root);
    outcome
}

fn parse_response_object(
    context: &DeviceAuthorizationResponseContext,
    root: &JsonValue,
) -> Result<DeviceAuthorization, OAuthError> {
    let JsonValue::Object(fields) = root else {
        return Err(invalid(DeviceAuthorizationResponseViolation::Shape));
    };
    if fields.len() > MAX_DEVICE_RESPONSE_FIELDS {
        return Err(invalid(DeviceAuthorizationResponseViolation::Shape));
    }
    let mut seen = BTreeSet::new();
    for (key, _) in fields {
        if !seen.insert(key.as_str()) {
            return Err(invalid(DeviceAuthorizationResponseViolation::Shape));
        }
    }
    if find_field(fields, "error").is_some() {
        return Err(invalid(DeviceAuthorizationResponseViolation::Shape));
    }

    let device_code = required_code(fields, "device_code", MAX_DEVICE_CODE_BYTES)?;
    let user_code = required_code(fields, "user_code", MAX_USER_CODE_BYTES)?;
    let verification_uri = required_string(fields, "verification_uri", MAX_ENDPOINT_BYTES)?;
    validate_verification_uri(&verification_uri, false)?;
    let verification_uri_complete =
        optional_string(fields, "verification_uri_complete", MAX_ENDPOINT_BYTES)?;
    if let Some(uri) = &verification_uri_complete {
        validate_verification_uri(uri, true)?;
    }
    let verification_uri_complete = verification_uri_complete.map(Zeroizing::new);
    let expires_in_seconds = required_unsigned(fields, "expires_in")?;
    let interval_seconds =
        optional_unsigned(fields, "interval")?.unwrap_or(DEFAULT_POLL_INTERVAL_SECONDS);
    if expires_in_seconds == 0
        || expires_in_seconds > MAX_DEVICE_LIFETIME_SECONDS
        || interval_seconds == 0
        || interval_seconds > MAX_POLL_INTERVAL_SECONDS
        || interval_seconds > expires_in_seconds
    {
        return Err(invalid(DeviceAuthorizationResponseViolation::Timing));
    }

    Ok(DeviceAuthorization {
        verification: DeviceVerification {
            user_code,
            verification_uri,
            verification_uri_complete,
            expires_in_seconds,
        },
        polling: DevicePollingSession {
            provider: context.provider.clone(),
            trace: context.trace,
            client_id: context.client_id.clone(),
            token_endpoint: context.token_endpoint.clone(),
            device_code,
            expires_in_seconds,
            interval_seconds,
        },
    })
}

fn find_field<'a>(fields: &'a [(String, JsonValue)], name: &str) -> Option<&'a JsonValue> {
    fields
        .iter()
        .find_map(|(key, value)| (key == name).then_some(value))
}

fn required_code(
    fields: &[(String, JsonValue)],
    name: &str,
    maximum: usize,
) -> Result<Zeroizing<String>, OAuthError> {
    let value = required_string(fields, name, maximum)?;
    Ok(Zeroizing::new(value))
}

fn required_string(
    fields: &[(String, JsonValue)],
    name: &str,
    maximum: usize,
) -> Result<String, OAuthError> {
    optional_string(fields, name, maximum)?.ok_or_else(|| {
        let violation = if matches!(name, "device_code" | "user_code") {
            DeviceAuthorizationResponseViolation::Code
        } else {
            DeviceAuthorizationResponseViolation::VerificationUri
        };
        invalid(violation)
    })
}

fn optional_string(
    fields: &[(String, JsonValue)],
    name: &str,
    maximum: usize,
) -> Result<Option<String>, OAuthError> {
    match find_field(fields, name) {
        None => Ok(None),
        Some(JsonValue::String(value))
            if !value.is_empty()
                && value.len() <= maximum
                && !value.chars().any(char::is_control) =>
        {
            Ok(Some(value.clone()))
        }
        Some(_) => {
            let violation = if matches!(name, "device_code" | "user_code") {
                DeviceAuthorizationResponseViolation::Code
            } else {
                DeviceAuthorizationResponseViolation::VerificationUri
            };
            Err(invalid(violation))
        }
    }
}

fn required_unsigned(fields: &[(String, JsonValue)], name: &str) -> Result<u64, OAuthError> {
    optional_unsigned(fields, name)?
        .ok_or_else(|| invalid(DeviceAuthorizationResponseViolation::Timing))
}

fn optional_unsigned(
    fields: &[(String, JsonValue)],
    name: &str,
) -> Result<Option<u64>, OAuthError> {
    match find_field(fields, name) {
        None => Ok(None),
        Some(JsonValue::Number(JsonNumber::Integer(value))) if *value >= 0 => {
            Ok(Some(*value as u64))
        }
        Some(_) => Err(invalid(DeviceAuthorizationResponseViolation::Timing)),
    }
}

fn validate_verification_uri(value: &str, allow_query: bool) -> Result<(), OAuthError> {
    if value.is_empty()
        || value.len() > MAX_ENDPOINT_BYTES
        || value.trim() != value
        || !valid_uri_text(value)
    {
        return Err(invalid(
            DeviceAuthorizationResponseViolation::VerificationUri,
        ));
    }
    let parsed = Url::parse(value)
        .map_err(|_| invalid(DeviceAuthorizationResponseViolation::VerificationUri))?;
    if parsed.scheme != "https"
        || parsed.host.is_none()
        || parsed.userinfo.is_some()
        || (!allow_query && parsed.query.is_some())
        || parsed.fragment.is_some()
    {
        return Err(invalid(
            DeviceAuthorizationResponseViolation::VerificationUri,
        ));
    }
    Ok(())
}

fn is_json_content_type(value: &str) -> bool {
    value
        .split(';')
        .next()
        .is_some_and(|media_type| media_type.trim().eq_ignore_ascii_case("application/json"))
}

fn invalid(reason: DeviceAuthorizationResponseViolation) -> OAuthError {
    OAuthError::InvalidDeviceAuthorizationResponse(reason)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        decode_authorization_server_metadata, prepare_authorization_server_metadata,
        OAuthAuditError, OAuthAuditEvent, OAuthAuditOutcome, OAuthAuditSink,
    };

    #[derive(Default)]
    struct Sink {
        events: Vec<OAuthAuditEvent>,
        fail: bool,
    }

    impl OAuthAuditSink for Sink {
        fn publish(&mut self, event: &OAuthAuditEvent) -> Result<(), OAuthAuditError> {
            if self.fail {
                return Err(OAuthAuditError);
            }
            self.events.push(event.clone());
            Ok(())
        }
    }

    fn trace() -> OAuthTraceId {
        OAuthTraceId::new([9; 16])
    }

    fn metadata_with(
        device_endpoint: Option<&str>,
        grant: bool,
        public_client: bool,
    ) -> AuthorizationServerMetadata {
        let request = prepare_authorization_server_metadata(
            ProviderId::new("fixture").unwrap(),
            "https://login.example/tenant",
            trace(),
        )
        .publish_then_release(&mut Sink::default())
        .unwrap();
        let device_field = device_endpoint
            .map(|endpoint| format!(r#","device_authorization_endpoint":"{endpoint}""#))
            .unwrap_or_default();
        let device_grant = if grant {
            format!(r#", "{DEVICE_GRANT_TYPE}""#)
        } else {
            String::new()
        };
        let auth = if public_client {
            r#"["none"]"#
        } else {
            r#"["client_secret_basic"]"#
        };
        let body = format!(
            r#"{{
                "issuer":"https://login.example/tenant",
                "authorization_endpoint":"https://login.example/auth",
                "token_endpoint":"https://login.example/token",
                "response_types_supported":["code"],
                "grant_types_supported":["authorization_code"{device_grant}],
                "token_endpoint_auth_methods_supported":{auth},
                "code_challenge_methods_supported":["S256"]
                {device_field}
            }}"#
        );
        decode_authorization_server_metadata(
            request.response_context(),
            200,
            "application/json",
            Zeroizing::new(body.into_bytes()),
        )
        .publish_then_release(&mut Sink::default())
        .unwrap()
    }

    fn profile() -> DeviceAuthorizationProfile {
        DeviceAuthorizationProfile::from_metadata(
            &metadata_with(Some("https://login.example/device"), true, true),
            "public/client",
        )
        .unwrap()
    }

    fn polling_session(interval_seconds: u64) -> DevicePollingSession {
        DevicePollingSession {
            provider: ProviderId::new("fixture").unwrap(),
            trace: trace(),
            client_id: "public/client".to_owned(),
            token_endpoint: "https://login.example/token".to_owned(),
            device_code: Zeroizing::new("device-secret".to_owned()),
            expires_in_seconds: 900,
            interval_seconds,
        }
    }

    fn poll_context(session: DevicePollingSession) -> DeviceTokenPollResponseContext {
        prepare_device_token_poll(session)
            .publish_then_release(&mut Sink::default())
            .unwrap()
            .response_context()
    }

    fn provider_config(provider: &str, client_id: &str, token_endpoint: &str) -> ProviderConfig {
        ProviderConfig::new(
            ProviderId::new(provider).unwrap(),
            "https://login.example/auth",
            token_endpoint,
            client_id,
            "http://127.0.0.1:49152/callback",
        )
        .unwrap()
        .with_distinct_redirect_uri()
    }

    #[test]
    fn metadata_capabilities_bind_request_before_transport_release() {
        let audited =
            prepare_device_authorization(&profile(), &["files.read", "files.write"], trace());
        assert_eq!(
            audited.audit().action(),
            OAuthAuditAction::DeviceAuthorizationPrepare
        );
        assert_eq!(audited.audit().outcome(), OAuthAuditOutcome::Succeeded);
        let request = audited.publish_then_release(&mut Sink::default()).unwrap();
        assert_eq!(request.provider().as_str(), "fixture");
        assert_eq!(request.trace(), trace());
        assert_eq!(request.client_id(), "public/client");
        assert_eq!(request.endpoint(), "https://login.example/device");
        assert_eq!(
            request.form_body(),
            "client_id=public%2Fclient&scope=files.read%20files.write"
        );
        assert!(!format!("{request:?}").contains("files.read"));
        let context = request.response_context();
        assert_eq!(context.provider().as_str(), "fixture");
        assert_eq!(context.trace(), trace());
    }

    #[test]
    fn opaque_polling_session_checks_every_provider_config_binding() {
        let session = polling_session(5);
        assert!(session.is_bound_to(&provider_config(
            "fixture",
            "public/client",
            "https://login.example/token",
        )));
        assert!(!session.is_bound_to(&provider_config(
            "other",
            "public/client",
            "https://login.example/token",
        )));
        assert!(!session.is_bound_to(&provider_config(
            "fixture",
            "other-client",
            "https://login.example/token",
        )));
        assert!(!session.is_bound_to(&provider_config(
            "fixture",
            "public/client",
            "https://other.example/token",
        )));
    }

    #[test]
    fn response_is_bounded_redacted_and_defaults_poll_interval() {
        let request = prepare_device_authorization(&profile(), &["files.read"], trace())
            .publish_then_release(&mut Sink::default())
            .unwrap();
        let body = br#"{
            "device_code":"device-secret",
            "user_code":"ABCD-EFGH",
            "verification_uri":"https://login.example/activate",
            "verification_uri_complete":"https://login.example/activate?user_code=ABCD-EFGH",
            "expires_in":900
        }"#
        .to_vec();
        let audited = decode_device_authorization_response(
            request.response_context(),
            200,
            "application/json; charset=utf-8",
            Zeroizing::new(body),
        );
        assert_eq!(
            audited.audit().action(),
            OAuthAuditAction::DeviceAuthorizationResponseDecode
        );
        let authorization = audited.publish_then_release(&mut Sink::default()).unwrap();
        assert_eq!(authorization.verification().user_code(), "ABCD-EFGH");
        assert_eq!(
            authorization.verification().verification_uri_complete(),
            Some("https://login.example/activate?user_code=ABCD-EFGH")
        );
        assert_eq!(authorization.polling().provider().as_str(), "fixture");
        assert_eq!(authorization.polling().trace(), trace());
        assert_eq!(authorization.polling().interval_seconds(), 5);
        assert_eq!(authorization.polling().expires_in_seconds(), 900);
        let debug = format!("{authorization:?}");
        assert!(!debug.contains("device-secret"));
        assert!(!debug.contains("ABCD-EFGH"));

        let (_, session) = authorization.into_parts();
        let poll = prepare_device_token_poll(session);
        assert_eq!(
            poll.audit().action(),
            OAuthAuditAction::DeviceTokenPollPrepare
        );
        let poll = poll.publish_then_release(&mut Sink::default()).unwrap();
        assert_eq!(poll.provider().as_str(), "fixture");
        assert_eq!(poll.trace(), trace());
        assert_eq!(poll.client_id(), "public/client");
        assert_eq!(poll.endpoint(), "https://login.example/token");
        assert_eq!(
            poll.form_body(),
            "grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Adevice_code&device_code=device-secret&client_id=public%2Fclient"
        );
        assert!(!format!("{poll:?}").contains("device-secret"));
    }

    #[test]
    fn poll_sequence_is_classified_with_caller_owned_scheduling() {
        let pending = decode_device_token_poll_response(
            poll_context(polling_session(7)),
            400,
            TokenResponseFormat::Json,
            Zeroizing::new(br#"{"error":"authorization_pending"}"#.to_vec()),
        );
        assert_eq!(
            pending.audit().action(),
            OAuthAuditAction::DeviceTokenPollResponseClassify
        );
        assert_eq!(pending.audit().provider().as_str(), "fixture");
        assert_eq!(pending.audit().trace(), trace());
        assert_eq!(pending.audit().outcome(), OAuthAuditOutcome::Succeeded);
        let pending = pending.publish_then_release(&mut Sink::default()).unwrap();
        assert_eq!(pending.retry_after_seconds(), Some(7));
        let DevicePollResult::Pending(session) = pending else {
            panic!("expected pending continuation");
        };

        let slow_down = decode_device_token_poll_response(
            poll_context(session),
            400,
            TokenResponseFormat::Json,
            Zeroizing::new(br#"{"error":"slow_down"}"#.to_vec()),
        )
        .publish_then_release(&mut Sink::default())
        .unwrap();
        assert_eq!(slow_down.retry_after_seconds(), Some(12));
        let DevicePollResult::SlowDown(session) = slow_down else {
            panic!("expected slow-down continuation");
        };

        let pending = decode_device_token_poll_response(
            poll_context(session),
            400,
            TokenResponseFormat::Json,
            Zeroizing::new(br#"{"error":"authorization_pending"}"#.to_vec()),
        )
        .publish_then_release(&mut Sink::default())
        .unwrap();
        assert_eq!(pending.retry_after_seconds(), Some(12));
        let DevicePollResult::Pending(session) = pending else {
            panic!("expected pending continuation");
        };

        let slow_down = decode_device_token_poll_response(
            poll_context(session),
            400,
            TokenResponseFormat::Json,
            Zeroizing::new(br#"{"error":"slow_down"}"#.to_vec()),
        )
        .publish_then_release(&mut Sink::default())
        .unwrap();
        assert_eq!(slow_down.retry_after_seconds(), Some(17));
        let DevicePollResult::SlowDown(session) = slow_down else {
            panic!("expected cumulative slow-down continuation");
        };

        let authorized = decode_device_token_poll_response(
            poll_context(session),
            200,
            TokenResponseFormat::Json,
            Zeroizing::new(
                br#"{"access_token":"access-secret","token_type":"Bearer","expires_in":3600,"refresh_token":"refresh-secret"}"#
                    .to_vec(),
            ),
        )
        .publish_then_release(&mut Sink::default())
        .unwrap();
        assert_eq!(authorized.retry_after_seconds(), None);
        let DevicePollResult::Authorized(response) = authorized else {
            panic!("expected authorized response");
        };
        assert_eq!(response.provider().as_str(), "fixture");
        assert_eq!(response.trace(), trace());
        let credentials = response
            .release_credentials()
            .publish_then_release(&mut Sink::default())
            .unwrap();
        assert_eq!(credentials.access_token(), "access-secret");
    }

    #[test]
    fn terminal_device_errors_are_closed_and_audited() {
        let denied = decode_device_token_poll_response(
            poll_context(polling_session(5)),
            400,
            TokenResponseFormat::Json,
            Zeroizing::new(
                br#"{"error":"access_denied","error_description":"attacker text"}"#.to_vec(),
            ),
        );
        assert_eq!(denied.audit().outcome(), OAuthAuditOutcome::Denied);
        assert!(matches!(
            denied.publish_then_release(&mut Sink::default()).unwrap(),
            DevicePollResult::Denied
        ));

        let expired = decode_device_token_poll_response(
            poll_context(polling_session(5)),
            400,
            TokenResponseFormat::FormEncoded,
            Zeroizing::new(b"error=expired_token&error_description=attacker+text".to_vec()),
        );
        assert_eq!(expired.audit().outcome(), OAuthAuditOutcome::Succeeded);
        assert!(matches!(
            expired.publish_then_release(&mut Sink::default()).unwrap(),
            DevicePollResult::ExpiredToken
        ));
    }

    #[test]
    fn poll_classifier_rejects_wrong_status_and_preserves_other_errors() {
        assert_eq!(
            decode_device_token_poll_response(
                poll_context(polling_session(5)),
                401,
                TokenResponseFormat::Json,
                Zeroizing::new(br#"{"error":"authorization_pending"}"#.to_vec()),
            )
            .publish_then_release(&mut Sink::default())
            .unwrap_err(),
            OAuthError::InvalidTokenResponse(TokenResponseViolation::Status)
        );
        assert_eq!(
            decode_device_token_poll_response(
                poll_context(polling_session(5)),
                400,
                TokenResponseFormat::Json,
                Zeroizing::new(br#"{"error":"invalid_client"}"#.to_vec()),
            )
            .publish_then_release(&mut Sink::default())
            .unwrap_err(),
            OAuthError::TokenEndpoint(ProviderTokenError::InvalidClient)
        );
    }

    #[test]
    fn poll_request_context_and_results_redact_all_sensitive_values() {
        let request = prepare_device_token_poll(polling_session(5))
            .publish_then_release(&mut Sink::default())
            .unwrap();
        assert!(!format!("{request:?}").contains("device-secret"));
        let context = request.response_context();
        assert_eq!(context.provider().as_str(), "fixture");
        assert_eq!(context.trace(), trace());
        assert!(!format!("{context:?}").contains("device-secret"));
        let result = decode_device_token_poll_response(
            context,
            400,
            TokenResponseFormat::Json,
            Zeroizing::new(br#"{"error":"authorization_pending"}"#.to_vec()),
        )
        .publish_then_release(&mut Sink::default())
        .unwrap();
        assert!(!format!("{result:?}").contains("device-secret"));
    }

    #[test]
    fn profile_requires_exact_endpoint_grant_and_public_authentication() {
        for metadata in [
            metadata_with(None, true, true),
            metadata_with(Some("https://login.example/device"), false, true),
            metadata_with(Some("https://login.example/device"), true, false),
        ] {
            assert_eq!(
                DeviceAuthorizationProfile::from_metadata(&metadata, "public-client").unwrap_err(),
                OAuthError::InvalidConfiguration(ConfigurationViolation::DeviceAuthorization)
            );
        }
    }

    #[test]
    fn response_rejects_ambiguous_unsafe_or_unbounded_fields() {
        let cases = [
            (
                br#"{"device_code":"one","device_code":"two","user_code":"ABCD","verification_uri":"https://login.example/activate","expires_in":900}"#.as_slice(),
                DeviceAuthorizationResponseViolation::Shape,
            ),
            (
                br#"{"device_code":"one","user_code":"ABCD","verification_uri":"http://login.example/activate","expires_in":900}"#.as_slice(),
                DeviceAuthorizationResponseViolation::VerificationUri,
            ),
            (
                br#"{"device_code":"one","user_code":"ABCD","verification_uri":"https://login.example/activate?code=ABCD","expires_in":900}"#.as_slice(),
                DeviceAuthorizationResponseViolation::VerificationUri,
            ),
            (
                br#"{"device_code":"one","user_code":"ABCD","verification_uri":"https://login.example/activate","expires_in":900,"interval":901}"#.as_slice(),
                DeviceAuthorizationResponseViolation::Timing,
            ),
            (
                br#"{"device_code":"one","user_code":"ABCD","verification_uri":"https://login.example/activate","expires_in":900,"error":"authorization_pending"}"#.as_slice(),
                DeviceAuthorizationResponseViolation::Shape,
            ),
        ];
        for (body, expected) in cases {
            let request = prepare_device_authorization(&profile(), &["files.read"], trace())
                .publish_then_release(&mut Sink::default())
                .unwrap();
            assert_eq!(
                decode_device_authorization_response(
                    request.response_context(),
                    200,
                    "application/json",
                    Zeroizing::new(body.to_vec()),
                )
                .publish_then_release(&mut Sink::default())
                .unwrap_err(),
                invalid(expected)
            );
        }
    }

    #[test]
    fn status_media_encoding_and_code_bounds_fail_closed() {
        let valid = br#"{"device_code":"one","user_code":"ABCD","verification_uri":"https://login.example/activate","expires_in":900}"#;
        for (status, content_type, body, expected) in [
            (
                201,
                "application/json",
                valid.as_slice(),
                DeviceAuthorizationResponseViolation::Status,
            ),
            (
                200,
                "text/plain",
                valid.as_slice(),
                DeviceAuthorizationResponseViolation::MediaType,
            ),
            (
                200,
                "application/json",
                b"{".as_slice(),
                DeviceAuthorizationResponseViolation::Encoding,
            ),
            (
                200,
                "application/json",
                br#"{"device_code":"one","user_code":"bad\ncode","verification_uri":"https://login.example/activate","expires_in":900}"#.as_slice(),
                DeviceAuthorizationResponseViolation::Code,
            ),
        ] {
            let request = prepare_device_authorization(&profile(), &["files.read"], trace())
                .publish_then_release(&mut Sink::default())
                .unwrap();
            assert_eq!(
                decode_device_authorization_response(
                    request.response_context(),
                    status,
                    content_type,
                    Zeroizing::new(body.to_vec()),
                )
                .publish_then_release(&mut Sink::default())
                .unwrap_err(),
                invalid(expected)
            );
        }

        let request = prepare_device_authorization(&profile(), &["files.read"], trace())
            .publish_then_release(&mut Sink::default())
            .unwrap();
        assert_eq!(
            decode_device_authorization_response(
                request.response_context(),
                200,
                "application/json",
                Zeroizing::new(vec![b'x'; MAX_DEVICE_RESPONSE_BYTES + 1]),
            )
            .publish_then_release(&mut Sink::default())
            .unwrap_err(),
            invalid(DeviceAuthorizationResponseViolation::Encoding)
        );
    }

    #[test]
    fn audit_failure_withholds_request_and_response() {
        let mut failing = Sink {
            events: Vec::new(),
            fail: true,
        };
        assert_eq!(
            prepare_device_authorization(&profile(), &["files.read"], trace())
                .publish_then_release(&mut failing)
                .unwrap_err(),
            OAuthError::Audit
        );

        let request = prepare_device_authorization(&profile(), &["files.read"], trace())
            .publish_then_release(&mut Sink::default())
            .unwrap();
        let body = br#"{"device_code":"device-secret","user_code":"ABCD","verification_uri":"https://login.example/activate","expires_in":900}"#.to_vec();
        assert_eq!(
            decode_device_authorization_response(
                request.response_context(),
                200,
                "application/json",
                Zeroizing::new(body),
            )
            .publish_then_release(&mut failing)
            .unwrap_err(),
            OAuthError::Audit
        );

        let request = prepare_device_authorization(&profile(), &["files.read"], trace())
            .publish_then_release(&mut Sink::default())
            .unwrap();
        let authorization = decode_device_authorization_response(
            request.response_context(),
            200,
            "application/json",
            Zeroizing::new(
                br#"{"device_code":"device-secret","user_code":"ABCD","verification_uri":"https://login.example/activate","expires_in":900}"#.to_vec(),
            ),
        )
        .publish_then_release(&mut Sink::default())
        .unwrap();
        let (_, session) = authorization.into_parts();
        assert_eq!(
            prepare_device_token_poll(session)
                .publish_then_release(&mut failing)
                .unwrap_err(),
            OAuthError::Audit
        );

        let response = decode_device_token_poll_response(
            poll_context(polling_session(5)),
            400,
            TokenResponseFormat::Json,
            Zeroizing::new(br#"{"error":"authorization_pending"}"#.to_vec()),
        );
        assert_eq!(
            response.publish_then_release(&mut failing).unwrap_err(),
            OAuthError::Audit
        );
    }
}
