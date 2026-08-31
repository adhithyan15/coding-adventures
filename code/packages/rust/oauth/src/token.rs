//! Token endpoint, refresh, and revocation primitives.

use super::{
    audited, json_nesting_within_limit, render_secret_form, validate_scopes, Audited,
    OAuthAuditAction, OAuthError, OAuthTraceId, ProviderConfig, ProviderId, TokenExchangeRequest,
};
use coding_adventures_json_value::{JsonNumber, JsonValue};
use coding_adventures_zeroize::{Zeroize, Zeroizing};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Debug, Formatter};

const MAX_TOKEN_RESPONSE_BYTES: usize = 128 * 1024;
const MAX_TOKEN_RESPONSE_FIELDS: usize = 64;
const MAX_TOKEN_BYTES: usize = 64 * 1024;
const MAX_TOKEN_TYPE_BYTES: usize = 64;
const MAX_SCOPE_RESPONSE_BYTES: usize = 16 * 1024;

/// Wire representation selected by provider data, never provider-specific code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenResponseFormat {
    /// An RFC 8259 JSON object.
    Json,
    /// An `application/x-www-form-urlencoded` response body.
    FormEncoded,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TokenGrantKind {
    AuthorizationCode,
    RefreshToken,
}

/// Non-secret context that binds one response to its request and audit trace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenResponseContext {
    provider: ProviderId,
    trace: OAuthTraceId,
    grant: TokenGrantKind,
}

impl TokenResponseContext {
    /// Return the provider expected to own the response.
    pub fn provider(&self) -> &ProviderId {
        &self.provider
    }

    /// Return the correlation identity inherited from the request.
    pub const fn trace(&self) -> OAuthTraceId {
        self.trace
    }
}

impl TokenExchangeRequest {
    /// Bind a later response decoder to this exact exchange and trace.
    pub fn response_context(&self) -> TokenResponseContext {
        TokenResponseContext {
            provider: self.provider.clone(),
            trace: self.trace,
            grant: TokenGrantKind::AuthorizationCode,
        }
    }
}

/// A prepared refresh-token grant request with a wipe-on-drop body.
pub struct TokenRefreshRequest {
    provider: ProviderId,
    trace: OAuthTraceId,
    endpoint: String,
    form_body: Zeroizing<String>,
}

impl TokenRefreshRequest {
    /// Return the provider identifier for transport authorization.
    pub fn provider(&self) -> &ProviderId {
        &self.provider
    }

    /// Borrow the validated HTTPS token endpoint.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Borrow the secret-bearing body only for an authorized transport.
    pub fn form_body(&self) -> &str {
        self.form_body.as_str()
    }

    /// Return the exact request media type.
    pub const fn content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }

    /// Bind a later response decoder to this exact refresh and trace.
    pub fn response_context(&self) -> TokenResponseContext {
        TokenResponseContext {
            provider: self.provider.clone(),
            trace: self.trace,
            grant: TokenGrantKind::RefreshToken,
        }
    }
}

impl Debug for TokenRefreshRequest {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TokenRefreshRequest")
            .field("provider", &self.provider)
            .field("trace", &self.trace)
            .field("endpoint", &"<redacted>")
            .field("form_body", &"<redacted>")
            .finish()
    }
}

/// Prepare a public-client refresh grant and require audit before its release.
pub fn prepare_token_refresh(
    config: &ProviderConfig,
    refresh_token: Zeroizing<String>,
    requested_scopes: &[&str],
    trace: OAuthTraceId,
) -> Audited<TokenRefreshRequest> {
    let result = (|| {
        validate_token(&refresh_token)?;
        if !requested_scopes.is_empty() {
            validate_scopes(requested_scopes)?;
        }
        let scope = requested_scopes.join(" ");
        let mut parameters = vec![
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token.as_str()),
            ("client_id", config.client_id.as_str()),
        ];
        if !scope.is_empty() {
            parameters.push(("scope", scope.as_str()));
        }
        Ok(TokenRefreshRequest {
            provider: config.provider.clone(),
            trace,
            endpoint: config.token_endpoint.clone(),
            form_body: render_secret_form(parameters),
        })
    })();
    audited(
        config.provider.clone(),
        trace,
        OAuthAuditAction::TokenRefreshPrepare,
        result,
    )
}

/// RFC 7009 hint for the credential being revoked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RevocationTokenHint {
    /// The credential is an access token.
    AccessToken,
    /// The credential is a refresh token.
    RefreshToken,
}

impl RevocationTokenHint {
    const fn as_str(self) -> &'static str {
        match self {
            Self::AccessToken => "access_token",
            Self::RefreshToken => "refresh_token",
        }
    }
}

/// A prepared RFC 7009 revocation request with a wipe-on-drop body.
pub struct TokenRevocationRequest {
    provider: ProviderId,
    endpoint: String,
    form_body: Zeroizing<String>,
}

impl TokenRevocationRequest {
    /// Return the provider identifier for transport authorization.
    pub fn provider(&self) -> &ProviderId {
        &self.provider
    }

    /// Borrow the validated HTTPS revocation endpoint.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Borrow the secret-bearing body only for an authorized transport.
    pub fn form_body(&self) -> &str {
        self.form_body.as_str()
    }

    /// Return the exact request media type.
    pub const fn content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }
}

impl Debug for TokenRevocationRequest {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TokenRevocationRequest")
            .field("provider", &self.provider)
            .field("endpoint", &"<redacted>")
            .field("form_body", &"<redacted>")
            .finish()
    }
}

/// Prepare RFC 7009 revocation and require audit before transport can see it.
pub fn prepare_token_revocation(
    config: &ProviderConfig,
    token: Zeroizing<String>,
    hint: RevocationTokenHint,
    trace: OAuthTraceId,
) -> Audited<TokenRevocationRequest> {
    let result = (|| {
        validate_token(&token)?;
        let endpoint =
            config
                .revocation_endpoint
                .clone()
                .ok_or(OAuthError::InvalidConfiguration(
                    super::ConfigurationViolation::TokenInput,
                ))?;
        Ok(TokenRevocationRequest {
            provider: config.provider.clone(),
            endpoint,
            form_body: render_secret_form([
                ("token", token.as_str()),
                ("token_type_hint", hint.as_str()),
                ("client_id", config.client_id.as_str()),
            ]),
        })
    })();
    audited(
        config.provider.clone(),
        trace,
        OAuthAuditAction::TokenRevocationPrepare,
        result,
    )
}

/// Closed structural reason a token endpoint response was rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenResponseViolation {
    /// The body was empty, oversized, non-UTF-8, or syntactically invalid.
    Encoding,
    /// The top-level value or field set was invalid, duplicated, or ambiguous.
    Shape,
    /// A required token field was absent, wrong-typed, malformed, or oversized.
    Token,
    /// `expires_in` was not a non-negative integer.
    Expiry,
    /// The returned scope set violated OAuth scope-token grammar or bounds.
    Scope,
    /// HTTP success/error status and body semantics disagreed.
    Status,
}

/// Closed OAuth token-endpoint error classification without attacker text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderTokenError {
    /// `invalid_request`.
    InvalidRequest,
    /// `invalid_client`.
    InvalidClient,
    /// `invalid_grant`.
    InvalidGrant,
    /// `unauthorized_client`.
    UnauthorizedClient,
    /// `unsupported_grant_type`.
    UnsupportedGrantType,
    /// `invalid_scope`.
    InvalidScope,
    /// `temporarily_unavailable`.
    TemporarilyUnavailable,
    /// `server_error`.
    ServerError,
    /// A valid bounded extension error code.
    Other,
}

/// Provider response metadata plus opaque credential material.
pub struct TokenResponse {
    provider: ProviderId,
    trace: OAuthTraceId,
    token_type: String,
    expires_in_seconds: Option<u64>,
    scopes: Vec<String>,
    access_token: Zeroizing<String>,
    refresh_token: RefreshTokenUpdate,
    id_token: Option<Zeroizing<String>>,
}

impl TokenResponse {
    /// Return the declared token type, normally `Bearer`.
    pub fn token_type(&self) -> &str {
        &self.token_type
    }

    /// Return the relative lifetime; the broker owns clock conversion.
    pub const fn expires_in_seconds(&self) -> Option<u64> {
        self.expires_in_seconds
    }

    /// Return the bounded granted scopes, if the server supplied them.
    pub fn scopes(&self) -> &[String] {
        &self.scopes
    }

    /// Require a second durable event before credential bytes leave the codec.
    pub fn release_credentials(self) -> Audited<TokenCredentials> {
        let credentials = TokenCredentials {
            access_token: self.access_token,
            refresh_token: self.refresh_token,
            id_token: self.id_token,
        };
        audited(
            self.provider,
            self.trace,
            OAuthAuditAction::TokenCredentialRelease,
            Ok(credentials),
        )
    }
}

impl Debug for TokenResponse {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TokenResponse")
            .field("provider", &self.provider)
            .field("trace", &self.trace)
            .field("token_type", &self.token_type)
            .field("expires_in_seconds", &self.expires_in_seconds)
            .field("scope_count", &self.scopes.len())
            .field("access_token", &"<redacted>")
            .field("refresh_token", &self.refresh_token)
            .field("id_token", &self.id_token.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

/// Explicit refresh-token rotation result; absence on refresh means retain.
pub enum RefreshTokenUpdate {
    /// Authorization did not issue a refresh token.
    Absent,
    /// A refresh response omitted the field, so custody retains the old token.
    RetainExisting,
    /// The provider issued a new token that atomically replaces the old token.
    Rotate(Zeroizing<String>),
}

impl Debug for RefreshTokenUpdate {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Absent => "Absent",
            Self::RetainExisting => "RetainExisting",
            Self::Rotate(_) => "Rotate(<redacted>)",
        })
    }
}

/// Credential bytes released only after `TokenCredentialRelease` is durable.
pub struct TokenCredentials {
    access_token: Zeroizing<String>,
    refresh_token: RefreshTokenUpdate,
    id_token: Option<Zeroizing<String>>,
}

impl TokenCredentials {
    /// Borrow the access token for immediate handoff to an opaque custodian.
    pub fn access_token(&self) -> &str {
        self.access_token.as_str()
    }

    /// Borrow the explicit refresh-token update decision.
    pub const fn refresh_token(&self) -> &RefreshTokenUpdate {
        &self.refresh_token
    }

    /// Borrow an opaque ID token; no identity claim is trusted at this layer.
    pub fn id_token(&self) -> Option<&str> {
        self.id_token.as_ref().map(|token| token.as_str())
    }

    /// Transfer credential ownership to an opaque custodian without cloning.
    pub fn into_parts(
        self,
    ) -> (
        Zeroizing<String>,
        RefreshTokenUpdate,
        Option<Zeroizing<String>>,
    ) {
        (self.access_token, self.refresh_token, self.id_token)
    }
}

impl Debug for TokenCredentials {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TokenCredentials")
            .field("access_token", &"<redacted>")
            .field("refresh_token", &self.refresh_token)
            .field("id_token", &self.id_token.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

/// Decode a bounded token response and require audit before releasing its result.
pub fn decode_token_response(
    context: TokenResponseContext,
    status: u16,
    format: TokenResponseFormat,
    body: Zeroizing<Vec<u8>>,
) -> Audited<TokenResponse> {
    let result = decode_token_response_inner(&context, status, format, &body);
    audited(
        context.provider,
        context.trace,
        OAuthAuditAction::TokenResponseDecode,
        result,
    )
}

fn decode_token_response_inner(
    context: &TokenResponseContext,
    status: u16,
    format: TokenResponseFormat,
    body: &[u8],
) -> Result<TokenResponse, OAuthError> {
    if !(100..=599).contains(&status) {
        return Err(invalid(TokenResponseViolation::Status));
    }
    if body.is_empty() || body.len() > MAX_TOKEN_RESPONSE_BYTES {
        return Err(invalid(TokenResponseViolation::Encoding));
    }
    let text = std::str::from_utf8(body).map_err(|_| invalid(TokenResponseViolation::Encoding))?;
    let fields = match format {
        TokenResponseFormat::Json => parse_json_fields(text)?,
        TokenResponseFormat::FormEncoded => parse_form_fields(text)?,
    };
    let success = (200..300).contains(&status);
    let error = fields.string("error")?;
    if success {
        if error.is_some() {
            return Err(invalid(TokenResponseViolation::Status));
        }
    } else {
        let code = error.ok_or_else(|| invalid(TokenResponseViolation::Status))?;
        return Err(OAuthError::TokenEndpoint(classify_error(code)?));
    }

    let access_token = required_bounded_token(fields.string("access_token")?)?;
    let token_type = required_public_string(fields.string("token_type")?, MAX_TOKEN_TYPE_BYTES)?;
    let expires_in_seconds = fields.unsigned("expires_in")?;
    let scopes = match fields.string("scope")? {
        Some(scope) => decode_scopes(scope)?,
        None => Vec::new(),
    };
    let refresh_token = match fields.string("refresh_token")? {
        Some(token) => RefreshTokenUpdate::Rotate(required_bounded_token(Some(token))?),
        None if context.grant == TokenGrantKind::RefreshToken => RefreshTokenUpdate::RetainExisting,
        None => RefreshTokenUpdate::Absent,
    };
    let id_token = fields
        .string("id_token")?
        .map(|token| required_bounded_token(Some(token)))
        .transpose()?;

    Ok(TokenResponse {
        provider: context.provider.clone(),
        trace: context.trace,
        token_type,
        expires_in_seconds,
        scopes,
        access_token,
        refresh_token,
        id_token,
    })
}

enum FieldValue {
    String(Zeroizing<String>),
    Integer(i64),
    Other,
}

struct Fields(BTreeMap<String, FieldValue>);

impl Fields {
    fn string(&self, key: &str) -> Result<Option<&str>, OAuthError> {
        match self.0.get(key) {
            None => Ok(None),
            Some(FieldValue::String(value)) => Ok(Some(value.as_str())),
            Some(_) => Err(invalid(TokenResponseViolation::Shape)),
        }
    }

    fn unsigned(&self, key: &str) -> Result<Option<u64>, OAuthError> {
        match self.0.get(key) {
            None => Ok(None),
            Some(FieldValue::Integer(value)) if *value >= 0 => Ok(Some(*value as u64)),
            Some(FieldValue::String(value)) => value
                .parse::<u64>()
                .map(Some)
                .map_err(|_| invalid(TokenResponseViolation::Expiry)),
            Some(_) => Err(invalid(TokenResponseViolation::Expiry)),
        }
    }
}

fn parse_json_fields(text: &str) -> Result<Fields, OAuthError> {
    if !json_nesting_within_limit(text.as_bytes()) {
        return Err(invalid(TokenResponseViolation::Encoding));
    }
    let mut root = coding_adventures_json_value::parse(text)
        .map_err(|_| invalid(TokenResponseViolation::Encoding))?;
    let outcome = if let JsonValue::Object(pairs) = &root {
        if pairs.len() > MAX_TOKEN_RESPONSE_FIELDS {
            Err(invalid(TokenResponseViolation::Shape))
        } else {
            (|| {
                let mut seen = BTreeSet::new();
                let mut fields = BTreeMap::new();
                for (key, value) in pairs {
                    if !seen.insert(key.as_str()) {
                        return Err(invalid(TokenResponseViolation::Shape));
                    }
                    let field = match value {
                        JsonValue::String(value) => {
                            FieldValue::String(Zeroizing::new(value.clone()))
                        }
                        JsonValue::Number(JsonNumber::Integer(value)) => {
                            FieldValue::Integer(*value)
                        }
                        _ => FieldValue::Other,
                    };
                    fields.insert(key.clone(), field);
                }
                Ok(Fields(fields))
            })()
        }
    } else {
        Err(invalid(TokenResponseViolation::Shape))
    };
    zeroize_json(&mut root);
    outcome
}

fn parse_form_fields(text: &str) -> Result<Fields, OAuthError> {
    let mut fields = BTreeMap::new();
    for pair in text.split('&') {
        if fields.len() >= MAX_TOKEN_RESPONSE_FIELDS {
            return Err(invalid(TokenResponseViolation::Shape));
        }
        let (key, value) = pair
            .split_once('=')
            .ok_or_else(|| invalid(TokenResponseViolation::Encoding))?;
        let key = token_form_decode(key)?;
        let value = Zeroizing::new(token_form_decode(value)?);
        if key.is_empty()
            || key.len() > 256
            || value.len() > MAX_TOKEN_BYTES
            || fields.insert(key, FieldValue::String(value)).is_some()
        {
            return Err(invalid(TokenResponseViolation::Shape));
        }
    }
    Ok(Fields(fields))
}

fn token_form_decode(value: &str) -> Result<String, OAuthError> {
    let bytes = value.as_bytes();
    let mut output = Zeroizing::new(Vec::with_capacity(bytes.len()));
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                output.push(b' ');
                index += 1;
            }
            b'%' if index + 2 < bytes.len() => {
                let high = token_hex(bytes[index + 1])?;
                let low = token_hex(bytes[index + 2])?;
                output.push((high << 4) | low);
                index += 3;
            }
            b'%' => return Err(invalid(TokenResponseViolation::Encoding)),
            byte => {
                output.push(byte);
                index += 1;
            }
        }
    }
    let decoded = std::str::from_utf8(&output)
        .map_err(|_| invalid(TokenResponseViolation::Encoding))?
        .to_owned();
    Ok(decoded)
}

fn token_hex(byte: u8) -> Result<u8, OAuthError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(invalid(TokenResponseViolation::Encoding)),
    }
}

fn required_bounded_token(value: Option<&str>) -> Result<Zeroizing<String>, OAuthError> {
    let value = value.ok_or_else(|| invalid(TokenResponseViolation::Token))?;
    validate_token(value).map_err(|_| invalid(TokenResponseViolation::Token))?;
    Ok(Zeroizing::new(value.to_owned()))
}

fn required_public_string(value: Option<&str>, max: usize) -> Result<String, OAuthError> {
    let value = value.ok_or_else(|| invalid(TokenResponseViolation::Token))?;
    if value.is_empty() || value.len() > max || value.chars().any(char::is_control) {
        return Err(invalid(TokenResponseViolation::Token));
    }
    Ok(value.to_owned())
}

fn validate_token(value: &str) -> Result<(), OAuthError> {
    let valid = !value.is_empty()
        && value.len() <= MAX_TOKEN_BYTES
        && value
            .bytes()
            .all(|byte| matches!(byte, 0x20..=0x21 | 0x23..=0x5b | 0x5d..=0x7e));
    if !valid {
        return Err(OAuthError::InvalidConfiguration(
            super::ConfigurationViolation::TokenInput,
        ));
    }
    Ok(())
}

fn decode_scopes(value: &str) -> Result<Vec<String>, OAuthError> {
    if value.is_empty() || value.len() > MAX_SCOPE_RESPONSE_BYTES {
        return Err(invalid(TokenResponseViolation::Scope));
    }
    let scopes: Vec<&str> = value.split(' ').collect();
    validate_scopes(&scopes).map_err(|_| invalid(TokenResponseViolation::Scope))?;
    Ok(scopes.into_iter().map(str::to_owned).collect())
}

fn classify_error(value: &str) -> Result<ProviderTokenError, OAuthError> {
    let valid = !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'));
    if !valid {
        return Err(invalid(TokenResponseViolation::Token));
    }
    Ok(match value {
        "invalid_request" => ProviderTokenError::InvalidRequest,
        "invalid_client" => ProviderTokenError::InvalidClient,
        "invalid_grant" => ProviderTokenError::InvalidGrant,
        "unauthorized_client" => ProviderTokenError::UnauthorizedClient,
        "unsupported_grant_type" => ProviderTokenError::UnsupportedGrantType,
        "invalid_scope" => ProviderTokenError::InvalidScope,
        "temporarily_unavailable" => ProviderTokenError::TemporarilyUnavailable,
        "server_error" => ProviderTokenError::ServerError,
        _ => ProviderTokenError::Other,
    })
}

fn invalid(reason: TokenResponseViolation) -> OAuthError {
    OAuthError::InvalidTokenResponse(reason)
}

fn zeroize_object(pairs: &mut [(String, JsonValue)]) {
    for (key, value) in pairs {
        key.zeroize();
        zeroize_json(value);
    }
}

pub(crate) fn zeroize_json(value: &mut JsonValue) {
    match value {
        JsonValue::String(value) => value.zeroize(),
        JsonValue::Object(pairs) => zeroize_object(pairs),
        JsonValue::Array(values) => {
            for value in values {
                zeroize_json(value);
            }
        }
        JsonValue::Number(_) | JsonValue::Bool(_) | JsonValue::Null => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{OAuthAuditError, OAuthAuditEvent, OAuthAuditSink};

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
        OAuthTraceId::new([7; 16])
    }

    fn context(grant: TokenGrantKind) -> TokenResponseContext {
        TokenResponseContext {
            provider: ProviderId::new("fixture").unwrap(),
            trace: trace(),
            grant,
        }
    }

    fn config() -> ProviderConfig {
        ProviderConfig::new(
            ProviderId::new("fixture").unwrap(),
            "https://fixture.example/auth",
            "https://fixture.example/token",
            "public/client",
            "http://127.0.0.1:54321/callback",
        )
        .unwrap()
        .with_distinct_redirect_uri()
        .with_revocation_endpoint("https://fixture.example/revoke")
        .unwrap()
    }

    #[test]
    fn json_success_is_bounded_audited_and_credential_release_is_separate() {
        let body = br#"{"access_token":"access-secret","token_type":"Bearer","expires_in":3600,"refresh_token":"refresh-secret","id_token":"id-secret","scope":"files.read files.write"}"#.to_vec();
        let audited = decode_token_response(
            context(TokenGrantKind::AuthorizationCode),
            200,
            TokenResponseFormat::Json,
            Zeroizing::new(body),
        );
        assert_eq!(
            audited.audit().action(),
            OAuthAuditAction::TokenResponseDecode
        );
        let response = audited.publish_then_release(&mut Sink::default()).unwrap();
        assert_eq!(response.token_type(), "Bearer");
        assert_eq!(response.expires_in_seconds(), Some(3600));
        assert_eq!(response.scopes(), ["files.read", "files.write"]);
        let credentials = response
            .release_credentials()
            .publish_then_release(&mut Sink::default())
            .unwrap();
        assert_eq!(credentials.access_token(), "access-secret");
        assert_eq!(credentials.id_token(), Some("id-secret"));
        assert!(matches!(
            credentials.refresh_token(),
            RefreshTokenUpdate::Rotate(_)
        ));
    }

    #[test]
    fn form_refresh_omission_explicitly_retains_existing_token() {
        let audited = decode_token_response(
            context(TokenGrantKind::RefreshToken),
            200,
            TokenResponseFormat::FormEncoded,
            Zeroizing::new(b"access_token=new%2Faccess&token_type=bearer&expires_in=90".to_vec()),
        );
        let response = audited.publish_then_release(&mut Sink::default()).unwrap();
        let credentials = response
            .release_credentials()
            .publish_then_release(&mut Sink::default())
            .unwrap();
        assert_eq!(credentials.access_token(), "new/access");
        assert!(matches!(
            credentials.refresh_token(),
            RefreshTokenUpdate::RetainExisting
        ));
    }

    #[test]
    fn provider_errors_are_classified_without_description_or_uri() {
        let body = br#"{"error":"invalid_grant","error_description":"secret attacker text","error_uri":"https://evil.example/secret"}"#.to_vec();
        let audited = decode_token_response(
            context(TokenGrantKind::RefreshToken),
            400,
            TokenResponseFormat::Json,
            Zeroizing::new(body),
        );
        assert_eq!(
            audited
                .publish_then_release(&mut Sink::default())
                .unwrap_err(),
            OAuthError::TokenEndpoint(ProviderTokenError::InvalidGrant)
        );
    }

    #[test]
    fn duplicate_ambiguous_and_wrong_status_responses_fail_closed() {
        let duplicate =
            br#"{"access_token":"one","access_token":"two","token_type":"Bearer"}"#.to_vec();
        assert_eq!(
            decode_token_response(
                context(TokenGrantKind::AuthorizationCode),
                200,
                TokenResponseFormat::Json,
                Zeroizing::new(duplicate),
            )
            .publish_then_release(&mut Sink::default())
            .unwrap_err(),
            invalid(TokenResponseViolation::Shape)
        );
        let ambiguous =
            br#"{"access_token":"one","token_type":"Bearer","error":"invalid_grant"}"#.to_vec();
        assert_eq!(
            decode_token_response(
                context(TokenGrantKind::AuthorizationCode),
                200,
                TokenResponseFormat::Json,
                Zeroizing::new(ambiguous),
            )
            .publish_then_release(&mut Sink::default())
            .unwrap_err(),
            invalid(TokenResponseViolation::Status)
        );
    }

    #[test]
    fn refresh_and_revocation_requests_are_generic_redacted_and_audit_gated() {
        let refresh = prepare_token_refresh(
            &config(),
            Zeroizing::new("refresh/secret".to_owned()),
            &["files.read"],
            trace(),
        );
        assert_eq!(
            refresh.audit().action(),
            OAuthAuditAction::TokenRefreshPrepare
        );
        let request = refresh.publish_then_release(&mut Sink::default()).unwrap();
        assert!(request
            .form_body()
            .contains("refresh_token=refresh%2Fsecret"));
        assert!(!format!("{request:?}").contains("refresh/secret"));
        assert_eq!(request.response_context().trace(), trace());

        let revoke = prepare_token_revocation(
            &config(),
            Zeroizing::new("access-secret".to_owned()),
            RevocationTokenHint::AccessToken,
            trace(),
        );
        assert_eq!(
            revoke.audit().action(),
            OAuthAuditAction::TokenRevocationPrepare
        );
        let request = revoke.publish_then_release(&mut Sink::default()).unwrap();
        assert_eq!(request.endpoint(), "https://fixture.example/revoke");
        assert!(request.form_body().contains("token_type_hint=access_token"));
        assert!(!format!("{request:?}").contains("access-secret"));
    }

    #[test]
    fn audit_failure_withholds_secret_bearing_results() {
        let mut sink = Sink {
            events: Vec::new(),
            fail: true,
        };
        assert_eq!(
            prepare_token_refresh(
                &config(),
                Zeroizing::new("refresh-secret".to_owned()),
                &[],
                trace(),
            )
            .publish_then_release(&mut sink)
            .unwrap_err(),
            OAuthError::Audit
        );

        let response = decode_token_response(
            context(TokenGrantKind::AuthorizationCode),
            200,
            TokenResponseFormat::Json,
            Zeroizing::new(br#"{"access_token":"access-secret","token_type":"Bearer"}"#.to_vec()),
        )
        .publish_then_release(&mut Sink::default())
        .unwrap();
        assert_eq!(
            response
                .release_credentials()
                .publish_then_release(&mut sink)
                .unwrap_err(),
            OAuthError::Audit
        );
    }

    #[test]
    fn response_bounds_types_and_http_status_are_strict() {
        let oversized = vec![b'x'; MAX_TOKEN_RESPONSE_BYTES + 1];
        assert_eq!(
            decode_token_response(
                context(TokenGrantKind::AuthorizationCode),
                200,
                TokenResponseFormat::Json,
                Zeroizing::new(oversized),
            )
            .publish_then_release(&mut Sink::default())
            .unwrap_err(),
            invalid(TokenResponseViolation::Encoding)
        );
        assert_eq!(
            decode_token_response(
                context(TokenGrantKind::AuthorizationCode),
                700,
                TokenResponseFormat::Json,
                Zeroizing::new(br#"{"error":"invalid_grant"}"#.to_vec()),
            )
            .publish_then_release(&mut Sink::default())
            .unwrap_err(),
            invalid(TokenResponseViolation::Status)
        );
        assert_eq!(
            decode_token_response(
                context(TokenGrantKind::AuthorizationCode),
                200,
                TokenResponseFormat::Json,
                Zeroizing::new(
                    br#"{"access_token":"access-secret","token_type":"Bearer","expires_in":1.5}"#
                        .to_vec(),
                ),
            )
            .publish_then_release(&mut Sink::default())
            .unwrap_err(),
            invalid(TokenResponseViolation::Expiry)
        );

        let deeply_nested = format!("{}null{}", "[".repeat(65), "]".repeat(65)).into_bytes();
        assert_eq!(
            decode_token_response(
                context(TokenGrantKind::AuthorizationCode),
                200,
                TokenResponseFormat::Json,
                Zeroizing::new(deeply_nested),
            )
            .publish_then_release(&mut Sink::default())
            .unwrap_err(),
            invalid(TokenResponseViolation::Encoding)
        );
    }

    #[test]
    fn diagnostics_never_reveal_response_tokens() {
        let response = decode_token_response(
            context(TokenGrantKind::AuthorizationCode),
            200,
            TokenResponseFormat::Json,
            Zeroizing::new(
                br#"{"access_token":"access-secret","token_type":"Bearer","refresh_token":"refresh-secret"}"#
                    .to_vec(),
            ),
        )
        .publish_then_release(&mut Sink::default())
        .unwrap();
        let debug = format!("{response:?}");
        assert!(!debug.contains("access-secret"));
        assert!(!debug.contains("refresh-secret"));
    }
}
