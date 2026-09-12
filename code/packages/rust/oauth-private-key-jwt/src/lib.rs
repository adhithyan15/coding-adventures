//! Bounded, provider-driven OAuth `private_key_jwt` assertions.
//!
//! This pure construction layer owns neither entropy, time, keys, nor network
//! authority. It validates provider capabilities, builds RFC 7523 claims, and
//! delegates the only signing effect to the audited non-exporting signer.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_base64::{encode_into as encode_base64_into, URL_SAFE_NO_PAD};
use coding_adventures_bounded_json::{serialize, JsonNumber, JsonValue};
use coding_adventures_oauth::{
    AuthorizationServerMetadata, OAuthTraceId, ProviderId, TokenExchangeRequest,
    TokenRefreshRequest, TokenResponseContext, TokenRevocationRequest,
};
use coding_adventures_oauth_private_key_signer::{
    AuditedPrivateKeySigner, PrivateKeyAuditSink, PrivateKeyId, PrivateKeyJwtAlgorithm,
    PrivateKeySignerError, SigningAuthority,
};
use coding_adventures_zeroize::{Zeroize, Zeroizing};
use std::fmt::{self, Debug, Display, Formatter};
use url_parser::Url;

const MAX_CLIENT_ID_BYTES: usize = 1_024;
const MAX_AUDIENCE_BYTES: usize = 2_048;
const MAX_KEY_ID_BYTES: usize = 1_024;
const MAX_ASSERTION_LIFETIME_SECONDS: u64 = 300;
const CLIENT_ASSERTION_TYPE: &str = "urn:ietf:params:oauth:client-assertion-type:jwt-bearer";

/// Validated provider data needed to construct one class of client assertion.
pub struct PrivateKeyJwtProfile {
    provider: ProviderId,
    client_id: String,
    audience: String,
    key: PrivateKeyId,
    algorithm: PrivateKeyJwtAlgorithm,
    key_id: Option<String>,
    lifetime_seconds: u64,
}

impl PrivateKeyJwtProfile {
    /// Validate exact provider data without inventing authentication defaults.
    ///
    /// The authentication method and algorithm lists are matched exactly and
    /// case-sensitively. `audience` must be a bounded HTTPS token endpoint.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        provider: ProviderId,
        client_id: impl Into<String>,
        audience: impl Into<String>,
        token_endpoint_auth_methods_supported: &[String],
        token_endpoint_auth_signing_alg_values_supported: &[String],
        key: PrivateKeyId,
        algorithm: PrivateKeyJwtAlgorithm,
        key_id: Option<String>,
        lifetime_seconds: u64,
    ) -> Result<Self, PrivateKeyJwtError> {
        let client_id = client_id.into();
        let audience = audience.into();
        if key.provider() != &provider
            || !valid_client_id(&client_id)
            || !valid_audience(&audience)
            || !token_endpoint_auth_methods_supported
                .iter()
                .any(|method| method == "private_key_jwt")
            || !token_endpoint_auth_signing_alg_values_supported
                .iter()
                .any(|candidate| candidate == algorithm.as_str())
            || !valid_key_id(key_id.as_deref())
            || !(1..=MAX_ASSERTION_LIFETIME_SECONDS).contains(&lifetime_seconds)
        {
            return Err(PrivateKeyJwtError::InvalidConfiguration);
        }
        Ok(Self {
            provider,
            client_id,
            audience,
            key,
            algorithm,
            key_id,
            lifetime_seconds,
        })
    }

    /// Derive assertion data from already-validated RFC 8414 metadata.
    ///
    /// The token endpoint becomes the audience and the exact retained method
    /// and algorithm lists remain authoritative.
    pub fn from_metadata(
        metadata: &AuthorizationServerMetadata,
        client_id: impl Into<String>,
        key: PrivateKeyId,
        algorithm: PrivateKeyJwtAlgorithm,
        key_id: Option<String>,
        lifetime_seconds: u64,
    ) -> Result<Self, PrivateKeyJwtError> {
        Self::new(
            metadata.provider().clone(),
            client_id,
            metadata.token_endpoint(),
            metadata.token_endpoint_auth_methods_supported(),
            metadata.token_endpoint_auth_signing_alg_values_supported(),
            key,
            algorithm,
            key_id,
            lifetime_seconds,
        )
    }

    /// Return the provider identity bound to every assertion and audit event.
    pub const fn provider(&self) -> &ProviderId {
        &self.provider
    }

    /// Return the exact provider-selected signing algorithm.
    pub const fn algorithm(&self) -> &PrivateKeyJwtAlgorithm {
        &self.algorithm
    }

    /// Build and audit-sign one short-lived RFC 7523 client assertion.
    ///
    /// `replay_entropy` must come from a caller-owned cryptographically secure
    /// random source and is encoded losslessly as the `jti`. `issued_at` comes
    /// from a caller-owned trusted clock.
    pub fn sign_assertion<S: SigningAuthority, A: PrivateKeyAuditSink>(
        &self,
        signer: &AuditedPrivateKeySigner<S>,
        issued_at: u64,
        replay_entropy: [u8; 32],
        trace: OAuthTraceId,
        audit: &mut A,
    ) -> Result<PrivateKeyJwtAssertion, PrivateKeyJwtError> {
        let replay_entropy = Zeroizing::new(replay_entropy);
        let expires_at = issued_at
            .checked_add(self.lifetime_seconds)
            .filter(|value| *value <= i64::MAX as u64)
            .ok_or(PrivateKeyJwtError::InvalidTime)?;
        let issued_at = i64::try_from(issued_at).map_err(|_| PrivateKeyJwtError::InvalidTime)?;

        let mut header_members = vec![(
            "alg".to_owned(),
            JsonValue::String(self.algorithm.as_str().to_owned()),
        )];
        if let Some(key_id) = &self.key_id {
            header_members.push(("kid".to_owned(), JsonValue::String(key_id.clone())));
        }

        let mut jti = Zeroizing::new(String::new());
        encode_base64_into(replay_entropy.as_slice(), &URL_SAFE_NO_PAD, &mut jti);
        let claims = JsonValue::Object(vec![
            ("iss".to_owned(), JsonValue::String(self.client_id.clone())),
            ("sub".to_owned(), JsonValue::String(self.client_id.clone())),
            ("aud".to_owned(), JsonValue::String(self.audience.clone())),
            (
                "exp".to_owned(),
                JsonValue::Number(JsonNumber::Integer(expires_at as i64)),
            ),
            (
                "iat".to_owned(),
                JsonValue::Number(JsonNumber::Integer(issued_at)),
            ),
            ("jti".to_owned(), JsonValue::String(jti.as_str().to_owned())),
        ]);

        let header = serialize_and_scrub(JsonValue::Object(header_members))?;
        let claims = serialize_and_scrub(claims)?;
        let mut signing_input = Zeroizing::new(String::new());
        encode_base64_into(header.as_bytes(), &URL_SAFE_NO_PAD, &mut signing_input);
        signing_input.push('.');
        encode_base64_into(claims.as_bytes(), &URL_SAFE_NO_PAD, &mut signing_input);

        let signature = signer.sign(
            &self.key,
            &self.algorithm,
            signing_input.as_bytes(),
            trace,
            audit,
        )?;
        let mut assertion = signing_input;
        assertion.push('.');
        encode_base64_into(signature.as_bytes(), &URL_SAFE_NO_PAD, &mut assertion);

        Ok(PrivateKeyJwtAssertion {
            provider: self.provider.clone(),
            trace,
            client_id: self.client_id.clone(),
            audience: self.audience.clone(),
            value: assertion,
        })
    }

    /// Sign and bind client authentication to an authorization-code exchange.
    ///
    /// Provider, client, token endpoint, and trace are inherited from the
    /// already audit-released request. Any identity mismatch fails before the
    /// signing effect.
    pub fn authenticate_token_exchange<S: SigningAuthority, A: PrivateKeyAuditSink>(
        &self,
        signer: &AuditedPrivateKeySigner<S>,
        request: TokenExchangeRequest,
        issued_at: u64,
        replay_entropy: [u8; 32],
        audit: &mut A,
    ) -> Result<PrivateKeyJwtAuthenticatedTokenExchange, PrivateKeyJwtError> {
        let response_context = request.response_context();
        let request = self.authenticate_request(
            signer,
            RequestBinding {
                provider: request.provider(),
                trace: request.trace(),
                client_id: request.client_id(),
                endpoint: request.endpoint(),
                form_body: request.form_body(),
            },
            issued_at,
            replay_entropy,
            audit,
        )?;
        Ok(PrivateKeyJwtAuthenticatedTokenExchange {
            request,
            response_context,
        })
    }

    /// Sign and bind client authentication to a refresh-token grant.
    ///
    /// Provider, client, token endpoint, and trace are inherited from the
    /// already audit-released request. Any identity mismatch fails before the
    /// signing effect.
    pub fn authenticate_token_refresh<S: SigningAuthority, A: PrivateKeyAuditSink>(
        &self,
        signer: &AuditedPrivateKeySigner<S>,
        request: TokenRefreshRequest,
        issued_at: u64,
        replay_entropy: [u8; 32],
        audit: &mut A,
    ) -> Result<PrivateKeyJwtAuthenticatedTokenRefresh, PrivateKeyJwtError> {
        let response_context = request.response_context();
        let request = self.authenticate_request(
            signer,
            RequestBinding {
                provider: request.provider(),
                trace: request.trace(),
                client_id: request.client_id(),
                endpoint: request.endpoint(),
                form_body: request.form_body(),
            },
            issued_at,
            replay_entropy,
            audit,
        )?;
        Ok(PrivateKeyJwtAuthenticatedTokenRefresh {
            request,
            response_context,
        })
    }

    /// Sign and bind client authentication to an RFC 7009 revocation request.
    ///
    /// The profile audience must exactly match the revocation endpoint. This
    /// avoids inventing whether a provider accepts its token endpoint as the
    /// audience for a different authenticated endpoint.
    pub fn authenticate_token_revocation<S: SigningAuthority, A: PrivateKeyAuditSink>(
        &self,
        signer: &AuditedPrivateKeySigner<S>,
        request: TokenRevocationRequest,
        issued_at: u64,
        replay_entropy: [u8; 32],
        audit: &mut A,
    ) -> Result<PrivateKeyJwtAuthenticatedTokenRevocation, PrivateKeyJwtError> {
        let request = self.authenticate_request(
            signer,
            RequestBinding {
                provider: request.provider(),
                trace: request.trace(),
                client_id: request.client_id(),
                endpoint: request.endpoint(),
                form_body: request.form_body(),
            },
            issued_at,
            replay_entropy,
            audit,
        )?;
        Ok(PrivateKeyJwtAuthenticatedTokenRevocation { request })
    }

    fn authenticate_request<S: SigningAuthority, A: PrivateKeyAuditSink>(
        &self,
        signer: &AuditedPrivateKeySigner<S>,
        request: RequestBinding<'_>,
        issued_at: u64,
        replay_entropy: [u8; 32],
        audit: &mut A,
    ) -> Result<PrivateKeyJwtAuthenticatedRequest, PrivateKeyJwtError> {
        let replay_entropy = Zeroizing::new(replay_entropy);
        if request.provider != &self.provider
            || request.client_id != self.client_id
            || request.endpoint != self.audience
        {
            return Err(PrivateKeyJwtError::RequestBinding);
        }
        let assertion =
            self.sign_assertion(signer, issued_at, *replay_entropy, request.trace, audit)?;
        Ok(build_authenticated_request(request, assertion))
    }
}

impl Debug for PrivateKeyJwtProfile {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PrivateKeyJwtProfile")
            .field("provider", &self.provider)
            .field("client_id", &"<redacted>")
            .field("audience", &"<redacted>")
            .field("key", &self.key)
            .field("algorithm", &self.algorithm)
            .field("has_key_id", &self.key_id.is_some())
            .field("lifetime_seconds", &self.lifetime_seconds)
            .finish()
    }
}

struct RequestBinding<'a> {
    provider: &'a ProviderId,
    trace: OAuthTraceId,
    client_id: &'a str,
    endpoint: &'a str,
    form_body: &'a str,
}

/// Zeroizing private-key-authenticated HTTP request material.
pub struct PrivateKeyJwtAuthenticatedRequest {
    provider: ProviderId,
    trace: OAuthTraceId,
    client_id: String,
    endpoint: String,
    form_body: Zeroizing<String>,
}

impl PrivateKeyJwtAuthenticatedRequest {
    /// Return the provider identity used for routing and external-effect audit.
    pub const fn provider(&self) -> &ProviderId {
        &self.provider
    }

    /// Return the correlation identity used for signing and transport audit.
    pub const fn trace(&self) -> OAuthTraceId {
        self.trace
    }

    /// Return the exact client identity bound before signing.
    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    /// Borrow the exact validated HTTPS endpoint.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Borrow the wipe-on-drop authenticated form body for transport.
    pub fn form_body(&self) -> &str {
        self.form_body.as_str()
    }

    /// Return the exact request media type.
    pub const fn content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }
}

impl Debug for PrivateKeyJwtAuthenticatedRequest {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PrivateKeyJwtAuthenticatedRequest")
            .field("provider", &self.provider)
            .field("trace", &self.trace)
            .field("client_id", &"<redacted>")
            .field("endpoint", &"<redacted>")
            .field("form_body", &"<redacted>")
            .finish()
    }
}

/// Private-key-authenticated authorization-code exchange and response binding.
pub struct PrivateKeyJwtAuthenticatedTokenExchange {
    request: PrivateKeyJwtAuthenticatedRequest,
    response_context: TokenResponseContext,
}

impl PrivateKeyJwtAuthenticatedTokenExchange {
    /// Borrow the authenticated wire request.
    pub const fn request(&self) -> &PrivateKeyJwtAuthenticatedRequest {
        &self.request
    }

    /// Borrow the exact provider/trace response binding.
    pub const fn response_context(&self) -> &TokenResponseContext {
        &self.response_context
    }
}

impl Debug for PrivateKeyJwtAuthenticatedTokenExchange {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PrivateKeyJwtAuthenticatedTokenExchange")
            .field("request", &self.request)
            .field("response_context", &self.response_context)
            .finish()
    }
}

/// Private-key-authenticated refresh grant and response binding.
pub struct PrivateKeyJwtAuthenticatedTokenRefresh {
    request: PrivateKeyJwtAuthenticatedRequest,
    response_context: TokenResponseContext,
}

impl PrivateKeyJwtAuthenticatedTokenRefresh {
    /// Borrow the authenticated wire request.
    pub const fn request(&self) -> &PrivateKeyJwtAuthenticatedRequest {
        &self.request
    }

    /// Borrow the exact provider/trace response binding.
    pub const fn response_context(&self) -> &TokenResponseContext {
        &self.response_context
    }
}

impl Debug for PrivateKeyJwtAuthenticatedTokenRefresh {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PrivateKeyJwtAuthenticatedTokenRefresh")
            .field("request", &self.request)
            .field("response_context", &self.response_context)
            .finish()
    }
}

/// Private-key-authenticated RFC 7009 revocation request.
pub struct PrivateKeyJwtAuthenticatedTokenRevocation {
    request: PrivateKeyJwtAuthenticatedRequest,
}

impl PrivateKeyJwtAuthenticatedTokenRevocation {
    /// Borrow the authenticated wire request.
    pub const fn request(&self) -> &PrivateKeyJwtAuthenticatedRequest {
        &self.request
    }
}

impl Debug for PrivateKeyJwtAuthenticatedTokenRevocation {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PrivateKeyJwtAuthenticatedTokenRevocation")
            .field("request", &self.request)
            .finish()
    }
}

/// Wipe-on-drop signed client assertion with its request-binding identities.
pub struct PrivateKeyJwtAssertion {
    provider: ProviderId,
    trace: OAuthTraceId,
    client_id: String,
    audience: String,
    value: Zeroizing<String>,
}

impl PrivateKeyJwtAssertion {
    /// Return the provider identity that a request and transport must match.
    pub const fn provider(&self) -> &ProviderId {
        &self.provider
    }

    /// Return the trace that a request and external-effect audit must match.
    pub const fn trace(&self) -> OAuthTraceId {
        self.trace
    }

    /// Return the client identity that an authenticated request must match.
    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    /// Return the token-endpoint audience that a request must match exactly.
    pub fn audience(&self) -> &str {
        &self.audience
    }

    /// Borrow the assertion only for immediate form encoding and transport.
    pub fn as_str(&self) -> &str {
        self.value.as_str()
    }

    /// Return the required OAuth client assertion type.
    pub const fn assertion_type(&self) -> &'static str {
        CLIENT_ASSERTION_TYPE
    }
}

impl Debug for PrivateKeyJwtAssertion {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PrivateKeyJwtAssertion")
            .field("provider", &self.provider)
            .field("trace", &self.trace)
            .field("client_id", &"<redacted>")
            .field("audience", &"<redacted>")
            .field("value", &"<redacted>")
            .finish()
    }
}

/// Closed assertion failure without claim, assertion, or signer diagnostics.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PrivateKeyJwtError {
    /// Provider data, capability binding, or a bounded field was invalid.
    InvalidConfiguration,
    /// Issued-at or expiration time could not be represented safely.
    InvalidTime,
    /// A prepared request did not match the assertion profile exactly.
    RequestBinding,
    /// The audited non-exporting signer rejected or withheld the operation.
    Signing(PrivateKeySignerError),
}

impl Debug for PrivateKeyJwtError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidConfiguration => "InvalidConfiguration",
            Self::InvalidTime => "InvalidTime",
            Self::RequestBinding => "RequestBinding",
            Self::Signing(_) => "Signing",
        })
    }
}

impl Display for PrivateKeyJwtError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("oauth private-key JWT: ")?;
        Debug::fmt(self, formatter)
    }
}

impl std::error::Error for PrivateKeyJwtError {}

impl From<PrivateKeySignerError> for PrivateKeyJwtError {
    fn from(error: PrivateKeySignerError) -> Self {
        Self::Signing(error)
    }
}

fn valid_client_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_CLIENT_ID_BYTES && !value.chars().any(char::is_control)
}

fn valid_audience(value: &str) -> bool {
    if value.is_empty()
        || value.len() > MAX_AUDIENCE_BYTES
        || value.trim() != value
        || !value.bytes().all(|byte| matches!(byte, 0x21..=0x7e))
    {
        return false;
    }
    let Ok(parsed) = Url::parse(value) else {
        return false;
    };
    parsed.scheme == "https"
        && parsed.host.is_some()
        && parsed.userinfo.is_none()
        && parsed.query.is_none()
        && parsed.fragment.is_none()
}

fn valid_key_id(value: Option<&str>) -> bool {
    value.is_none_or(|value| {
        !value.is_empty() && value.len() <= MAX_KEY_ID_BYTES && !value.chars().any(char::is_control)
    })
}

fn build_authenticated_request(
    request: RequestBinding<'_>,
    assertion: PrivateKeyJwtAssertion,
) -> PrivateKeyJwtAuthenticatedRequest {
    let mut form_body = Zeroizing::new(request.form_body.to_owned());
    form_body.push_str("&client_assertion_type=");
    append_form_encoded(&mut form_body, assertion.assertion_type());
    form_body.push_str("&client_assertion=");
    append_form_encoded(&mut form_body, assertion.as_str());
    PrivateKeyJwtAuthenticatedRequest {
        provider: request.provider.clone(),
        trace: request.trace,
        client_id: request.client_id.to_owned(),
        endpoint: request.endpoint.to_owned(),
        form_body,
    }
}

fn append_form_encoded(output: &mut String, value: &str) {
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            output.push(char::from(byte));
        } else if byte == b' ' {
            output.push('+');
        } else {
            const HEX: &[u8; 16] = b"0123456789ABCDEF";
            output.push('%');
            output.push(char::from(HEX[usize::from(byte >> 4)]));
            output.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
    }
}

fn serialize_and_scrub(mut value: JsonValue) -> Result<Zeroizing<String>, PrivateKeyJwtError> {
    let result = serialize(&value)
        .map(Zeroizing::new)
        .map_err(|_| PrivateKeyJwtError::InvalidConfiguration);
    scrub_json(&mut value);
    result
}

fn scrub_json(value: &mut JsonValue) {
    match value {
        JsonValue::Object(members) => {
            for (name, member) in members {
                name.zeroize();
                scrub_json(member);
            }
        }
        JsonValue::Array(members) => {
            for member in members {
                scrub_json(member);
            }
        }
        JsonValue::String(value) => value.zeroize(),
        JsonValue::Number(_) | JsonValue::Bool(_) | JsonValue::Null => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_base64::decode;
    use coding_adventures_oauth::{
        begin_authorization, complete_authorization, prepare_token_refresh,
        prepare_token_revocation, Audited, EntropySource, OAuthAuditError, OAuthAuditEvent,
        OAuthAuditSink, OAuthError, ProviderConfig, RevocationTokenHint,
    };
    use coding_adventures_oauth_private_key_signer::{
        PrivateKeyAuditError, PrivateKeyAuditEvent, PrivateKeyAuditOutcome, PrivateKeyReference,
        PrivateKeySignature, SigningAuthorityError,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct RecordingAuthority {
        inputs: Arc<Mutex<Vec<Vec<u8>>>>,
        calls: Arc<AtomicUsize>,
    }

    impl SigningAuthority for RecordingAuthority {
        fn sign(
            &self,
            _key: &PrivateKeyId,
            _algorithm: &PrivateKeyJwtAlgorithm,
            signing_input: &[u8],
        ) -> Result<PrivateKeySignature, SigningAuthorityError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.inputs.lock().unwrap().push(signing_input.to_vec());
            PrivateKeySignature::new(vec![0xfb, 0xff, 0x00])
                .map_err(|_| SigningAuthorityError::Backend)
        }
    }

    #[derive(Default)]
    struct Audit {
        events: Vec<PrivateKeyAuditEvent>,
        fail_at: Option<usize>,
    }

    #[derive(Default)]
    struct OAuthAudit;

    impl OAuthAuditSink for OAuthAudit {
        fn publish(&mut self, _event: &OAuthAuditEvent) -> Result<(), OAuthAuditError> {
            Ok(())
        }
    }

    struct FixedEntropy([u8; 64]);

    impl EntropySource for FixedEntropy {
        fn fill(&mut self, destination: &mut [u8]) -> Result<(), OAuthError> {
            destination.copy_from_slice(&self.0[..destination.len()]);
            Ok(())
        }
    }

    impl PrivateKeyAuditSink for Audit {
        fn publish(&mut self, event: &PrivateKeyAuditEvent) -> Result<(), PrivateKeyAuditError> {
            if self.fail_at == Some(self.events.len()) {
                return Err(PrivateKeyAuditError);
            }
            self.events.push(event.clone());
            Ok(())
        }
    }

    fn provider(value: &str) -> ProviderId {
        ProviderId::new(value).unwrap()
    }

    fn key(provider: ProviderId) -> PrivateKeyId {
        PrivateKeyId::new(provider, PrivateKeyReference::new([9; 32]))
    }

    fn algorithm(value: &str) -> PrivateKeyJwtAlgorithm {
        PrivateKeyJwtAlgorithm::new(value).unwrap()
    }

    fn profile() -> PrivateKeyJwtProfile {
        let provider = provider("fixture");
        PrivateKeyJwtProfile::new(
            provider.clone(),
            "client\"id",
            "https://token.example/oauth2/token",
            &["none".to_owned(), "private_key_jwt".to_owned()],
            &["EdDSA".to_owned(), "RS256".to_owned()],
            key(provider),
            algorithm("EdDSA"),
            Some("key-1".to_owned()),
            120,
        )
        .unwrap()
    }

    fn trace() -> OAuthTraceId {
        OAuthTraceId::new([7; 16])
    }

    fn secret(value: &str) -> Zeroizing<String> {
        Zeroizing::new(value.to_owned())
    }

    fn config() -> ProviderConfig {
        config_with(
            "fixture",
            "client id/plus+",
            "https://token.example/oauth2/token",
        )
    }

    fn config_with(provider_name: &str, client_id: &str, token_endpoint: &str) -> ProviderConfig {
        ProviderConfig::new(
            provider(provider_name),
            "https://authorize.example/oauth2/auth",
            token_endpoint,
            client_id,
            "http://127.0.0.1:53682/callback",
        )
        .unwrap()
        .with_distinct_redirect_uri()
        .with_revocation_endpoint("https://token.example/oauth2/revoke")
        .unwrap()
    }

    fn request_profile(audience: &str) -> PrivateKeyJwtProfile {
        let provider = provider("fixture");
        PrivateKeyJwtProfile::new(
            provider.clone(),
            "client id/plus+",
            audience,
            &["none".to_owned(), "private_key_jwt".to_owned()],
            &["EdDSA".to_owned()],
            key(provider),
            algorithm("EdDSA"),
            Some("key-1".to_owned()),
            120,
        )
        .unwrap()
    }

    fn release<T>(audited: Audited<T>) -> T {
        audited.publish_then_release(&mut OAuthAudit).unwrap()
    }

    fn exchange_request(config: &ProviderConfig) -> TokenExchangeRequest {
        let begin = release(begin_authorization(
            config,
            &["files.read"],
            trace(),
            &mut FixedEntropy([0x29; 64]),
        ));
        let state = begin
            .url()
            .as_str()
            .split('&')
            .find_map(|parameter| parameter.strip_prefix("state="))
            .unwrap()
            .to_owned();
        let (_, transaction) = begin.into_parts();
        release(complete_authorization(
            transaction,
            &format!("http://127.0.0.1:53682/callback?code=code-value&state={state}"),
        ))
    }

    #[test]
    fn builds_exact_bounded_claims_and_audited_signing_input() {
        let authority = RecordingAuthority::default();
        let inspection = authority.clone();
        let signer = AuditedPrivateKeySigner::from_audited_authority(authority);
        let mut audit = Audit::default();
        let assertion = profile()
            .sign_assertion(&signer, 1_700_000_000, [3; 32], trace(), &mut audit)
            .unwrap();

        let parts: Vec<&str> = assertion.as_str().split('.').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(
            String::from_utf8(decode(parts[0], &URL_SAFE_NO_PAD).unwrap()).unwrap(),
            r#"{"alg":"EdDSA","kid":"key-1"}"#
        );
        assert_eq!(
            String::from_utf8(decode(parts[1], &URL_SAFE_NO_PAD).unwrap()).unwrap(),
            r#"{"iss":"client\"id","sub":"client\"id","aud":"https://token.example/oauth2/token","exp":1700000120,"iat":1700000000,"jti":"AwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwM"}"#
        );
        assert_eq!(parts[2], "-_8A");
        assert_eq!(inspection.calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            inspection.inputs.lock().unwrap()[0],
            format!("{}.{}", parts[0], parts[1]).as_bytes()
        );
        assert_eq!(audit.events.len(), 2);
        for event in &audit.events {
            assert_eq!(event.key().provider().as_str(), "fixture");
            assert_eq!(event.algorithm().as_str(), "EdDSA");
            assert_eq!(event.trace(), trace());
        }
        assert_eq!(audit.events[0].outcome(), PrivateKeyAuditOutcome::Attempted);
        assert_eq!(audit.events[1].outcome(), PrivateKeyAuditOutcome::Succeeded);
        assert_eq!(assertion.provider().as_str(), "fixture");
        assert_eq!(assertion.client_id(), "client\"id");
        assert_eq!(assertion.audience(), "https://token.example/oauth2/token");
        assert_eq!(assertion.trace(), trace());
        assert_eq!(assertion.assertion_type(), CLIENT_ASSERTION_TYPE);
    }

    #[test]
    fn binds_exchange_and_response_context_to_the_signed_request() {
        let authority = RecordingAuthority::default();
        let inspection = authority.clone();
        let signer = AuditedPrivateKeySigner::from_audited_authority(authority);
        let mut audit = Audit::default();
        let authenticated = request_profile("https://token.example/oauth2/token")
            .authenticate_token_exchange(
                &signer,
                exchange_request(&config()),
                1_700_000_000,
                [0x31; 32],
                &mut audit,
            )
            .unwrap();

        let request = authenticated.request();
        assert_eq!(request.provider().as_str(), "fixture");
        assert_eq!(request.trace(), trace());
        assert_eq!(request.client_id(), "client id/plus+");
        assert_eq!(request.endpoint(), "https://token.example/oauth2/token");
        assert_eq!(request.content_type(), "application/x-www-form-urlencoded");
        assert!(request
            .form_body()
            .contains("grant_type=authorization_code"));
        assert!(request.form_body().contains("code=code-value"));
        assert!(request
            .form_body()
            .contains("client_id=client%20id%2Fplus%2B"));
        assert!(request.form_body().contains(concat!(
            "&client_assertion_type=urn%3Aietf%3Aparams%3Aoauth%3A",
            "client-assertion-type%3Ajwt-bearer&client_assertion="
        )));
        let assertion = request
            .form_body()
            .split("&client_assertion=")
            .nth(1)
            .unwrap();
        assert_eq!(assertion.split('.').count(), 3);
        assert_eq!(
            authenticated.response_context().provider().as_str(),
            "fixture"
        );
        assert_eq!(authenticated.response_context().trace(), trace());
        assert_eq!(inspection.calls.load(Ordering::SeqCst), 1);
        assert_eq!(audit.events.len(), 2);
        assert!(audit.events.iter().all(|event| event.trace() == trace()));
    }

    #[test]
    fn binds_refresh_and_revocation_to_their_exact_audiences() {
        let authority = RecordingAuthority::default();
        let inspection = authority.clone();
        let signer = AuditedPrivateKeySigner::from_audited_authority(authority);
        let mut audit = Audit::default();
        let config = config();

        let refresh = release(prepare_token_refresh(
            &config,
            secret("refresh-token"),
            &["files.read"],
            trace(),
        ));
        let refresh = request_profile("https://token.example/oauth2/token")
            .authenticate_token_refresh(&signer, refresh, 9, [0x41; 32], &mut audit)
            .unwrap();
        assert!(refresh
            .request()
            .form_body()
            .contains("refresh_token=refresh-token"));
        assert_eq!(refresh.response_context().provider().as_str(), "fixture");
        assert_eq!(refresh.response_context().trace(), trace());

        let revocation = release(prepare_token_revocation(
            &config,
            secret("access-token"),
            RevocationTokenHint::AccessToken,
            trace(),
        ));
        let revocation = request_profile("https://token.example/oauth2/revoke")
            .authenticate_token_revocation(&signer, revocation, 10, [0x42; 32], &mut audit)
            .unwrap();
        assert_eq!(
            revocation.request().endpoint(),
            "https://token.example/oauth2/revoke"
        );
        assert!(revocation
            .request()
            .form_body()
            .contains("token=access-token"));
        assert!(revocation
            .request()
            .form_body()
            .contains("client_assertion_type="));
        assert_eq!(inspection.calls.load(Ordering::SeqCst), 2);
        assert_eq!(audit.events.len(), 4);
        assert!(audit.events.iter().all(|event| {
            event.key().provider().as_str() == "fixture" && event.trace() == trace()
        }));
    }

    #[test]
    fn request_identity_mismatches_fail_before_signing_or_audit() {
        let authority = RecordingAuthority::default();
        let inspection = authority.clone();
        let signer = AuditedPrivateKeySigner::from_audited_authority(authority);
        let mut audit = Audit::default();
        let profile = request_profile("https://token.example/oauth2/token");

        let mismatches = [
            config_with(
                "other",
                "client id/plus+",
                "https://token.example/oauth2/token",
            ),
            config_with(
                "fixture",
                "other-client",
                "https://token.example/oauth2/token",
            ),
            config_with(
                "fixture",
                "client id/plus+",
                "https://other.example/oauth2/token",
            ),
        ];
        for (index, config) in mismatches.iter().enumerate() {
            let request = release(prepare_token_refresh(
                config,
                secret("refresh-token"),
                &[],
                trace(),
            ));
            assert_eq!(
                profile
                    .authenticate_token_refresh(&signer, request, 1, [index as u8; 32], &mut audit,)
                    .unwrap_err(),
                PrivateKeyJwtError::RequestBinding
            );
        }
        assert_eq!(inspection.calls.load(Ordering::SeqCst), 0);
        assert!(audit.events.is_empty());
    }

    #[test]
    fn rejects_unbound_or_unadvertised_provider_data() {
        let fixture = provider("fixture");
        let other = provider("other");
        let methods = vec!["private_key_jwt".to_owned()];
        let algorithms = vec!["EdDSA".to_owned()];
        let cases = [
            PrivateKeyJwtProfile::new(
                fixture.clone(),
                "client",
                "https://token.example/token",
                &methods,
                &algorithms,
                key(other),
                algorithm("EdDSA"),
                None,
                60,
            ),
            PrivateKeyJwtProfile::new(
                fixture.clone(),
                "client",
                "https://token.example/token",
                &["none".to_owned()],
                &algorithms,
                key(fixture.clone()),
                algorithm("EdDSA"),
                None,
                60,
            ),
            PrivateKeyJwtProfile::new(
                fixture.clone(),
                "client",
                "https://token.example/token",
                &methods,
                &["RS256".to_owned()],
                key(fixture.clone()),
                algorithm("EdDSA"),
                None,
                60,
            ),
            PrivateKeyJwtProfile::new(
                fixture.clone(),
                "client",
                "http://token.example/token",
                &methods,
                &algorithms,
                key(fixture.clone()),
                algorithm("EdDSA"),
                None,
                60,
            ),
            PrivateKeyJwtProfile::new(
                fixture,
                "client",
                "https://token.example/token",
                &methods,
                &algorithms,
                key(provider("fixture")),
                algorithm("EdDSA"),
                Some(String::new()),
                0,
            ),
        ];
        assert!(cases
            .into_iter()
            .all(|result| result.unwrap_err() == PrivateKeyJwtError::InvalidConfiguration));
    }

    #[test]
    fn invalid_time_fails_before_signing_or_audit() {
        let authority = RecordingAuthority::default();
        let inspection = authority.clone();
        let signer = AuditedPrivateKeySigner::from_audited_authority(authority);
        let mut audit = Audit::default();
        assert_eq!(
            profile()
                .sign_assertion(&signer, i64::MAX as u64, [4; 32], trace(), &mut audit)
                .unwrap_err(),
            PrivateKeyJwtError::InvalidTime
        );
        assert_eq!(inspection.calls.load(Ordering::SeqCst), 0);
        assert!(audit.events.is_empty());
    }

    #[test]
    fn audit_failure_after_signing_withholds_the_assertion() {
        let authority = RecordingAuthority::default();
        let inspection = authority.clone();
        let signer = AuditedPrivateKeySigner::from_audited_authority(authority);
        let mut audit = Audit {
            events: Vec::new(),
            fail_at: Some(1),
        };
        assert_eq!(
            profile()
                .sign_assertion(&signer, 1, [5; 32], trace(), &mut audit)
                .unwrap_err(),
            PrivateKeyJwtError::Signing(PrivateKeySignerError::Audit)
        );
        assert_eq!(inspection.calls.load(Ordering::SeqCst), 1);
        assert_eq!(audit.events.len(), 1);
        assert_eq!(audit.events[0].outcome(), PrivateKeyAuditOutcome::Attempted);
    }

    #[test]
    fn audit_failure_before_signing_prevents_the_effect() {
        let authority = RecordingAuthority::default();
        let inspection = authority.clone();
        let signer = AuditedPrivateKeySigner::from_audited_authority(authority);
        let mut audit = Audit {
            events: Vec::new(),
            fail_at: Some(0),
        };
        assert_eq!(
            profile()
                .sign_assertion(&signer, 1, [5; 32], trace(), &mut audit)
                .unwrap_err(),
            PrivateKeyJwtError::Signing(PrivateKeySignerError::Audit)
        );
        assert_eq!(inspection.calls.load(Ordering::SeqCst), 0);
        assert!(audit.events.is_empty());
    }

    #[test]
    fn debug_output_redacts_assertion_and_identity_fields() {
        let authority = RecordingAuthority::default();
        let signer = AuditedPrivateKeySigner::from_audited_authority(authority);
        let mut audit = Audit::default();
        let profile = profile();
        let assertion = profile
            .sign_assertion(&signer, 1, [6; 32], trace(), &mut audit)
            .unwrap();
        let profile_debug = format!("{profile:?}");
        let assertion_debug = format!("{assertion:?}");
        assert!(!profile_debug.contains("client\"id"));
        assert!(!profile_debug.contains("token.example"));
        assert!(!profile_debug.contains("key-1"));
        assert!(!assertion_debug.contains(assertion.as_str()));
        assert!(!assertion_debug.contains("client\"id"));
        assert!(!assertion_debug.contains("token.example"));
    }

    #[test]
    fn authenticated_request_debug_redacts_endpoint_body_and_assertion() {
        let signer = AuditedPrivateKeySigner::from_audited_authority(RecordingAuthority::default());
        let mut audit = Audit::default();
        let authenticated = request_profile("https://token.example/oauth2/token")
            .authenticate_token_refresh(
                &signer,
                release(prepare_token_refresh(
                    &config(),
                    secret("refresh-secret"),
                    &[],
                    trace(),
                )),
                1,
                [0x51; 32],
                &mut audit,
            )
            .unwrap();
        let debug = format!("{authenticated:?}");
        assert!(!debug.contains("token.example"));
        assert!(!debug.contains("refresh-secret"));
        assert!(!debug.contains("client_assertion"));
        assert!(!debug.contains("client id"));
    }
}
