//! RFC 8414 authorization-server metadata primitives.

use super::{
    audited, json_nesting_within_limit, validate_https_endpoint, validate_issuer, Audited,
    OAuthAuditAction, OAuthError, OAuthTraceId, ProviderConfig, ProviderId,
};
use crate::token::zeroize_json;
use coding_adventures_json_value::JsonValue;
use coding_adventures_zeroize::Zeroizing;
use std::collections::BTreeSet;
use std::fmt::{self, Debug, Formatter};

const WELL_KNOWN_PATH: &str = "/.well-known/oauth-authorization-server";
const MAX_METADATA_BYTES: usize = 128 * 1024;
const MAX_METADATA_FIELDS: usize = 128;
const MAX_METADATA_LIST_ENTRIES: usize = 128;
const MAX_METADATA_TOKEN_BYTES: usize = 256;
const MAX_CONTENT_TYPE_BYTES: usize = 256;

/// Closed reason that RFC 8414 metadata was rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetadataViolation {
    /// The HTTP response was not exactly a successful `200` response.
    Status,
    /// The response did not declare the JSON media type.
    MediaType,
    /// The body was empty, oversized, non-UTF-8, or invalid JSON.
    Encoding,
    /// The top-level object, duplicate fields, or a required field had an invalid shape.
    Shape,
    /// The returned issuer was absent or did not exactly match the configured issuer.
    Issuer,
    /// A required endpoint was absent or failed the client's strict HTTPS policy.
    Endpoint,
    /// Authorization Code was not advertised in `response_types_supported`.
    ResponseType,
    /// Authorization Code was not supported by the advertised grant set.
    GrantType,
    /// Public-client token endpoint authentication method `none` was not advertised.
    TokenAuthentication,
    /// PKCE method `S256` was not explicitly advertised.
    Pkce,
    /// RFC 9207 response-issuer mode was selected but not advertised.
    AuthorizationResponseIssuer,
}

/// An audited, provider-bound RFC 8414 metadata request.
pub struct AuthorizationServerMetadataRequest {
    provider: ProviderId,
    trace: OAuthTraceId,
    url: String,
    expected_issuer: String,
}

impl AuthorizationServerMetadataRequest {
    /// Return the stable provider identifier for transport authorization.
    pub fn provider(&self) -> &ProviderId {
        &self.provider
    }

    /// Borrow the validated metadata URL after the audit gate has released it.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Return the exact HTTP method required by RFC 8414.
    pub const fn method(&self) -> &'static str {
        "GET"
    }

    /// Bind a later response decoder to this exact provider, issuer, and trace.
    pub fn response_context(&self) -> AuthorizationServerMetadataContext {
        AuthorizationServerMetadataContext {
            provider: self.provider.clone(),
            trace: self.trace,
            expected_issuer: self.expected_issuer.clone(),
        }
    }
}

impl Debug for AuthorizationServerMetadataRequest {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthorizationServerMetadataRequest")
            .field("provider", &self.provider)
            .field("trace", &self.trace)
            .field("url", &"<redacted>")
            .field("expected_issuer", &"<redacted>")
            .finish()
    }
}

/// Non-secret binding between one metadata request and its response.
pub struct AuthorizationServerMetadataContext {
    provider: ProviderId,
    trace: OAuthTraceId,
    expected_issuer: String,
}

impl AuthorizationServerMetadataContext {
    /// Return the provider expected to own the response.
    pub fn provider(&self) -> &ProviderId {
        &self.provider
    }

    /// Return the caller-owned correlation identity.
    pub const fn trace(&self) -> OAuthTraceId {
        self.trace
    }
}

impl Debug for AuthorizationServerMetadataContext {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthorizationServerMetadataContext")
            .field("provider", &self.provider)
            .field("trace", &self.trace)
            .field("expected_issuer", &"<redacted>")
            .finish()
    }
}

/// Validated RFC 8414 metadata narrowed to this public installed-app profile.
///
/// Unknown metadata fields are deliberately discarded. The retained capability
/// sets are the exact values that justified accepting this record, so a cache
/// cannot silently widen provider behavior when it is restored.
pub struct AuthorizationServerMetadata {
    provider: ProviderId,
    issuer: String,
    authorization_endpoint: String,
    token_endpoint: String,
    revocation_endpoint: Option<String>,
    response_types_supported: Vec<String>,
    grant_types_supported: Vec<String>,
    token_endpoint_auth_methods_supported: Vec<String>,
    code_challenge_methods_supported: Vec<String>,
    authorization_response_iss_parameter_supported: bool,
}

impl AuthorizationServerMetadata {
    /// Return the provider identity bound to the metadata request.
    pub fn provider(&self) -> &ProviderId {
        &self.provider
    }

    /// Borrow the exact, non-normalized issuer string that was validated.
    pub fn issuer(&self) -> &str {
        &self.issuer
    }

    /// Borrow the validated authorization endpoint.
    pub fn authorization_endpoint(&self) -> &str {
        &self.authorization_endpoint
    }

    /// Borrow the validated token endpoint.
    pub fn token_endpoint(&self) -> &str {
        &self.token_endpoint
    }

    /// Borrow the optional validated RFC 7009 revocation endpoint.
    pub fn revocation_endpoint(&self) -> Option<&str> {
        self.revocation_endpoint.as_deref()
    }

    /// Borrow the exact response types retained in the cache record.
    pub fn response_types_supported(&self) -> &[String] {
        &self.response_types_supported
    }

    /// Borrow the exact grant types retained in the cache record.
    pub fn grant_types_supported(&self) -> &[String] {
        &self.grant_types_supported
    }

    /// Borrow the exact token authentication methods retained in the cache record.
    pub fn token_endpoint_auth_methods_supported(&self) -> &[String] {
        &self.token_endpoint_auth_methods_supported
    }

    /// Borrow the exact PKCE methods retained in the cache record.
    pub fn code_challenge_methods_supported(&self) -> &[String] {
        &self.code_challenge_methods_supported
    }

    /// Return whether RFC 9207 authorization responses explicitly include `iss`.
    pub const fn authorization_response_iss_parameter_supported(&self) -> bool {
        self.authorization_response_iss_parameter_supported
    }

    /// Derive a strict public installed-app configuration using RFC 9207.
    ///
    /// The validated issuer is installed as the RFC 9207 mix-up defense. A
    /// caller may add bounded authorization parameters afterward, but cannot
    /// replace any metadata-derived endpoint through this operation. This
    /// method fails closed unless the metadata explicitly advertised the
    /// authorization-response `iss` parameter.
    pub fn into_provider_config(
        self,
        client_id: impl Into<String>,
        redirect_uri: impl Into<String>,
    ) -> Result<ProviderConfig, OAuthError> {
        if !self.authorization_response_iss_parameter_supported {
            return Err(invalid(MetadataViolation::AuthorizationResponseIssuer));
        }
        let expected_issuer = self.issuer.clone();
        self.into_base_provider_config(client_id, redirect_uri)?
            .with_expected_issuer(expected_issuer)
    }

    /// Derive a configuration using a registry-owned distinct redirect URI.
    ///
    /// This supports providers that do not implement RFC 9207. As documented
    /// by [`ProviderConfig::with_distinct_redirect_uri`], the composition-root
    /// registry must reject any other provider that claims the same redirect.
    pub fn into_provider_config_with_distinct_redirect_uri(
        self,
        client_id: impl Into<String>,
        redirect_uri: impl Into<String>,
    ) -> Result<ProviderConfig, OAuthError> {
        Ok(self
            .into_base_provider_config(client_id, redirect_uri)?
            .with_distinct_redirect_uri())
    }

    fn into_base_provider_config(
        self,
        client_id: impl Into<String>,
        redirect_uri: impl Into<String>,
    ) -> Result<ProviderConfig, OAuthError> {
        let mut config = ProviderConfig::new(
            self.provider,
            self.authorization_endpoint,
            self.token_endpoint,
            client_id,
            redirect_uri,
        )?;
        if let Some(endpoint) = self.revocation_endpoint {
            config = config.with_revocation_endpoint(endpoint)?;
        }
        Ok(config)
    }
}

impl Debug for AuthorizationServerMetadata {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthorizationServerMetadata")
            .field("provider", &self.provider)
            .field("issuer", &"<redacted>")
            .field("authorization_endpoint", &"<redacted>")
            .field("token_endpoint", &"<redacted>")
            .field(
                "has_revocation_endpoint",
                &self.revocation_endpoint.is_some(),
            )
            .field("response_type_count", &self.response_types_supported.len())
            .field("grant_type_count", &self.grant_types_supported.len())
            .field(
                "token_auth_method_count",
                &self.token_endpoint_auth_methods_supported.len(),
            )
            .field(
                "pkce_method_count",
                &self.code_challenge_methods_supported.len(),
            )
            .field(
                "authorization_response_iss_parameter_supported",
                &self.authorization_response_iss_parameter_supported,
            )
            .finish()
    }
}

/// Prepare the RFC 8414 well-known request for an exact configured issuer.
pub fn prepare_authorization_server_metadata(
    provider: ProviderId,
    issuer: impl Into<String>,
    trace: OAuthTraceId,
) -> Audited<AuthorizationServerMetadataRequest> {
    let issuer = issuer.into();
    let result = (|| {
        validate_issuer(&issuer)?;
        let url = metadata_url(&issuer);
        if url.len() > super::MAX_ENDPOINT_BYTES {
            return Err(OAuthError::InvalidConfiguration(
                super::ConfigurationViolation::Issuer,
            ));
        }
        Ok(AuthorizationServerMetadataRequest {
            provider: provider.clone(),
            trace,
            url,
            expected_issuer: issuer,
        })
    })();
    audited(
        provider,
        trace,
        OAuthAuditAction::MetadataRequestPrepare,
        result,
    )
}

/// Decode and validate metadata for the public installed-app profile.
///
/// This function accepts ownership of a wipe-on-drop response body. The exact
/// issuer, Authorization Code support, public-client method `none`, and PKCE
/// `S256` are mandatory. The provider response is released only after the
/// returned audit descriptor is durably published.
pub fn decode_authorization_server_metadata(
    context: AuthorizationServerMetadataContext,
    status: u16,
    content_type: &str,
    body: Zeroizing<Vec<u8>>,
) -> Audited<AuthorizationServerMetadata> {
    let result = decode_metadata_inner(&context, status, content_type, &body);
    audited(
        context.provider,
        context.trace,
        OAuthAuditAction::MetadataResponseValidate,
        result,
    )
}

fn decode_metadata_inner(
    context: &AuthorizationServerMetadataContext,
    status: u16,
    content_type: &str,
    body: &[u8],
) -> Result<AuthorizationServerMetadata, OAuthError> {
    if status != 200 {
        return Err(invalid(MetadataViolation::Status));
    }
    if content_type.is_empty()
        || content_type.len() > MAX_CONTENT_TYPE_BYTES
        || !content_type.is_ascii()
        || !is_json_content_type(content_type)
    {
        return Err(invalid(MetadataViolation::MediaType));
    }
    if body.is_empty() || body.len() > MAX_METADATA_BYTES {
        return Err(invalid(MetadataViolation::Encoding));
    }
    let text = std::str::from_utf8(body).map_err(|_| invalid(MetadataViolation::Encoding))?;
    if !json_nesting_within_limit(body) {
        return Err(invalid(MetadataViolation::Encoding));
    }
    let mut root = coding_adventures_json_value::parse(text)
        .map_err(|_| invalid(MetadataViolation::Encoding))?;
    let outcome = parse_metadata_object(context, &root);
    zeroize_json(&mut root);
    outcome
}

fn parse_metadata_object(
    context: &AuthorizationServerMetadataContext,
    root: &JsonValue,
) -> Result<AuthorizationServerMetadata, OAuthError> {
    let JsonValue::Object(fields) = root else {
        return Err(invalid(MetadataViolation::Shape));
    };
    if fields.len() > MAX_METADATA_FIELDS {
        return Err(invalid(MetadataViolation::Shape));
    }
    let mut seen = BTreeSet::new();
    for (key, _) in fields {
        if !seen.insert(key.as_str()) {
            return Err(invalid(MetadataViolation::Shape));
        }
    }

    let issuer = required_string(fields, "issuer", super::MAX_ENDPOINT_BYTES)?;
    validate_issuer(&issuer).map_err(|_| invalid(MetadataViolation::Issuer))?;
    if issuer.as_bytes() != context.expected_issuer.as_bytes() {
        return Err(invalid(MetadataViolation::Issuer));
    }

    let authorization_endpoint =
        required_string(fields, "authorization_endpoint", super::MAX_ENDPOINT_BYTES)?;
    let token_endpoint = required_string(fields, "token_endpoint", super::MAX_ENDPOINT_BYTES)?;
    validate_https_endpoint(&authorization_endpoint)
        .map_err(|_| invalid(MetadataViolation::Endpoint))?;
    validate_https_endpoint(&token_endpoint).map_err(|_| invalid(MetadataViolation::Endpoint))?;

    let response_types_supported = required_string_array(fields, "response_types_supported")?;
    require_member(
        &response_types_supported,
        "code",
        MetadataViolation::ResponseType,
    )?;

    let grant_types_supported = match optional_string_array(fields, "grant_types_supported")? {
        Some(values) => {
            require_member(&values, "authorization_code", MetadataViolation::GrantType)?;
            values
        }
        None => vec!["authorization_code".to_owned(), "implicit".to_owned()],
    };

    let token_endpoint_auth_methods_supported =
        required_string_array(fields, "token_endpoint_auth_methods_supported")?;
    require_member(
        &token_endpoint_auth_methods_supported,
        "none",
        MetadataViolation::TokenAuthentication,
    )?;

    let code_challenge_methods_supported =
        required_string_array(fields, "code_challenge_methods_supported")?;
    require_member(
        &code_challenge_methods_supported,
        "S256",
        MetadataViolation::Pkce,
    )?;

    let authorization_response_iss_parameter_supported =
        optional_bool(fields, "authorization_response_iss_parameter_supported")?.unwrap_or(false);

    let revocation_endpoint =
        optional_string(fields, "revocation_endpoint", super::MAX_ENDPOINT_BYTES)?;
    if let Some(endpoint) = &revocation_endpoint {
        validate_https_endpoint(endpoint).map_err(|_| invalid(MetadataViolation::Endpoint))?;
    }

    Ok(AuthorizationServerMetadata {
        provider: context.provider.clone(),
        issuer,
        authorization_endpoint,
        token_endpoint,
        revocation_endpoint,
        response_types_supported,
        grant_types_supported,
        token_endpoint_auth_methods_supported,
        code_challenge_methods_supported,
        authorization_response_iss_parameter_supported,
    })
}

fn metadata_url(issuer: &str) -> String {
    let issuer = issuer.trim_end_matches('/');
    let authority_start = issuer
        .find("://")
        .map(|index| index + 3)
        .expect("validated issuer has an authority separator");
    let authority_end = issuer[authority_start..]
        .find('/')
        .map(|index| authority_start + index)
        .unwrap_or(issuer.len());
    let authority = &issuer[..authority_end];
    let issuer_path = issuer[authority_end..]
        .strip_prefix('/')
        .unwrap_or(&issuer[authority_end..]);
    if issuer_path.is_empty() {
        format!("{authority}{WELL_KNOWN_PATH}")
    } else {
        format!("{authority}{WELL_KNOWN_PATH}/{issuer_path}")
    }
}

fn is_json_content_type(value: &str) -> bool {
    value
        .split(';')
        .next()
        .is_some_and(|media_type| media_type.trim().eq_ignore_ascii_case("application/json"))
}

fn find_field<'a>(fields: &'a [(String, JsonValue)], name: &str) -> Option<&'a JsonValue> {
    fields
        .iter()
        .find_map(|(key, value)| (key == name).then_some(value))
}

fn required_string(
    fields: &[(String, JsonValue)],
    name: &str,
    maximum: usize,
) -> Result<String, OAuthError> {
    optional_string(fields, name, maximum)?.ok_or_else(|| invalid(MetadataViolation::Shape))
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
        Some(_) => Err(invalid(MetadataViolation::Shape)),
    }
}

fn required_string_array(
    fields: &[(String, JsonValue)],
    name: &str,
) -> Result<Vec<String>, OAuthError> {
    optional_string_array(fields, name)?.ok_or_else(|| invalid(MetadataViolation::Shape))
}

fn optional_string_array(
    fields: &[(String, JsonValue)],
    name: &str,
) -> Result<Option<Vec<String>>, OAuthError> {
    let Some(value) = find_field(fields, name) else {
        return Ok(None);
    };
    let JsonValue::Array(values) = value else {
        return Err(invalid(MetadataViolation::Shape));
    };
    if values.is_empty() || values.len() > MAX_METADATA_LIST_ENTRIES {
        return Err(invalid(MetadataViolation::Shape));
    }
    let mut seen = BTreeSet::new();
    let mut decoded = Vec::with_capacity(values.len());
    for value in values {
        let JsonValue::String(value) = value else {
            return Err(invalid(MetadataViolation::Shape));
        };
        if !valid_metadata_token(value) || !seen.insert(value.as_str()) {
            return Err(invalid(MetadataViolation::Shape));
        }
        decoded.push(value.clone());
    }
    Ok(Some(decoded))
}

fn optional_bool(fields: &[(String, JsonValue)], name: &str) -> Result<Option<bool>, OAuthError> {
    match find_field(fields, name) {
        None => Ok(None),
        Some(JsonValue::Bool(value)) => Ok(Some(*value)),
        Some(_) => Err(invalid(MetadataViolation::Shape)),
    }
}

fn valid_metadata_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_METADATA_TOKEN_BYTES
        && value.bytes().all(|byte| matches!(byte, 0x21..=0x7e))
}

fn require_member(
    values: &[String],
    required: &str,
    violation: MetadataViolation,
) -> Result<(), OAuthError> {
    if values.iter().any(|value| value == required) {
        Ok(())
    } else {
        Err(invalid(violation))
    }
}

fn invalid(reason: MetadataViolation) -> OAuthError {
    OAuthError::InvalidMetadata(reason)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ConfigurationViolation, OAuthAuditError, OAuthAuditEvent, OAuthAuditOutcome, OAuthAuditSink,
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

    fn provider() -> ProviderId {
        ProviderId::new("fixture").unwrap()
    }

    fn trace() -> OAuthTraceId {
        OAuthTraceId::new([8; 16])
    }

    fn request(issuer: &str) -> AuthorizationServerMetadataRequest {
        prepare_authorization_server_metadata(provider(), issuer, trace())
            .publish_then_release(&mut Sink::default())
            .unwrap()
    }

    fn valid_body(issuer: &str) -> Vec<u8> {
        format!(
            r#"{{
                "issuer":"{issuer}",
                "authorization_endpoint":"https://login.example/authorize",
                "token_endpoint":"https://login.example/token",
                "revocation_endpoint":"https://login.example/revoke",
                "response_types_supported":["code"],
                "grant_types_supported":["authorization_code","refresh_token"],
                "token_endpoint_auth_methods_supported":["none","client_secret_basic"],
                "code_challenge_methods_supported":["S256"],
                "authorization_response_iss_parameter_supported":true,
                "signed_metadata":"unverified-and-ignored",
                "provider_extension":{{"ignored":"text"}}
            }}"#
        )
        .into_bytes()
    }

    fn decode(issuer: &str, body: Vec<u8>) -> Audited<AuthorizationServerMetadata> {
        let context = request(issuer).response_context();
        decode_authorization_server_metadata(
            context,
            200,
            "application/json; charset=utf-8",
            Zeroizing::new(body),
        )
    }

    #[test]
    fn request_uses_rfc_8414_path_insertion_and_is_audit_gated() {
        let audited = prepare_authorization_server_metadata(
            provider(),
            "https://login.example/tenant/a",
            trace(),
        );
        assert!(audited.is_success());
        assert_eq!(
            audited.audit().action(),
            OAuthAuditAction::MetadataRequestPrepare
        );
        let mut sink = Sink::default();
        let prepared = audited.publish_then_release(&mut sink).unwrap();
        assert_eq!(
            prepared.url(),
            "https://login.example/.well-known/oauth-authorization-server/tenant/a"
        );
        assert_eq!(prepared.method(), "GET");
        assert_eq!(sink.events.len(), 1);

        let root = request("https://login.example/");
        assert_eq!(
            root.url(),
            "https://login.example/.well-known/oauth-authorization-server"
        );

        let double_slash = request("https://login.example//tenant");
        assert_eq!(
            double_slash.url(),
            "https://login.example/.well-known/oauth-authorization-server//tenant"
        );

        let trailing_slash = request("https://login.example/tenant/");
        assert_eq!(
            trailing_slash.url(),
            "https://login.example/.well-known/oauth-authorization-server/tenant"
        );
    }

    #[test]
    fn audit_failure_withholds_metadata_request_and_response() {
        let audited = prepare_authorization_server_metadata(
            provider(),
            "https://login.example/tenant",
            trace(),
        );
        let mut failing = Sink {
            fail: true,
            ..Sink::default()
        };
        assert_eq!(
            audited.publish_then_release(&mut failing).unwrap_err(),
            OAuthError::Audit
        );

        let response = decode(
            "https://login.example/tenant",
            valid_body("https://login.example/tenant"),
        );
        assert_eq!(
            response.publish_then_release(&mut failing).unwrap_err(),
            OAuthError::Audit
        );
    }

    #[test]
    fn valid_metadata_derives_strict_public_provider_config() {
        let issuer = "https://login.example/tenant";
        let audited = decode(issuer, valid_body(issuer));
        assert_eq!(
            audited.audit().action(),
            OAuthAuditAction::MetadataResponseValidate
        );
        let metadata = audited.publish_then_release(&mut Sink::default()).unwrap();
        assert_eq!(metadata.provider().as_str(), "fixture");
        assert_eq!(metadata.issuer(), issuer);
        assert_eq!(metadata.response_types_supported(), &["code"]);
        assert_eq!(
            metadata.token_endpoint_auth_methods_supported(),
            &["none", "client_secret_basic"]
        );
        assert!(metadata.authorization_response_iss_parameter_supported());
        assert!(!format!("{metadata:?}").contains(issuer));

        let config = metadata
            .into_provider_config("public-client", "http://127.0.0.1:43210/oauth/callback")
            .unwrap();
        assert_eq!(
            config.authorization_endpoint,
            "https://login.example/authorize"
        );
        assert_eq!(config.token_endpoint, "https://login.example/token");
        assert_eq!(
            config.revocation_endpoint.as_deref(),
            Some("https://login.example/revoke")
        );
        assert!(matches!(
            config.mix_up_defense,
            Some(super::super::MixUpDefense::AuthorizationResponseIssuer(ref value))
                if value == issuer
        ));
    }

    #[test]
    fn grant_types_uses_rfc_default_when_absent() {
        let issuer = "https://login.example";
        let body = String::from_utf8(valid_body(issuer))
            .unwrap()
            .replace(
                r#""grant_types_supported":["authorization_code","refresh_token"],"#,
                "",
            )
            .into_bytes();
        let metadata = decode(issuer, body)
            .publish_then_release(&mut Sink::default())
            .unwrap();
        assert_eq!(
            metadata.grant_types_supported(),
            &["authorization_code", "implicit"]
        );
    }

    #[test]
    fn exact_issuer_match_has_no_normalization() {
        let expected = "https://login.example/tenant";
        let audited = decode(expected, valid_body("https://LOGIN.example/tenant"));
        assert_eq!(
            audited
                .publish_then_release(&mut Sink::default())
                .unwrap_err(),
            invalid(MetadataViolation::Issuer)
        );
        assert_eq!(
            audited_outcome(expected, valid_body("https://login.example/other")),
            OAuthAuditOutcome::Failed(super::super::OAuthFailureClass::Provider)
        );
    }

    #[test]
    fn required_public_profile_capabilities_fail_closed() {
        let issuer = "https://login.example";
        for (from, to, violation) in [
            (
                r#"["code"]"#,
                r#"["token"]"#,
                MetadataViolation::ResponseType,
            ),
            (
                r#"["authorization_code","refresh_token"]"#,
                r#"["refresh_token"]"#,
                MetadataViolation::GrantType,
            ),
            (
                r#"["none","client_secret_basic"]"#,
                r#"["client_secret_basic"]"#,
                MetadataViolation::TokenAuthentication,
            ),
            (r#"["S256"]"#, r#"["plain"]"#, MetadataViolation::Pkce),
        ] {
            let body = String::from_utf8(valid_body(issuer))
                .unwrap()
                .replacen(from, to, 1)
                .into_bytes();
            assert_eq!(
                decode(issuer, body)
                    .publish_then_release(&mut Sink::default())
                    .unwrap_err(),
                invalid(violation)
            );
        }
    }

    #[test]
    fn missing_public_auth_or_pkce_advertisement_does_not_assume_support() {
        let issuer = "https://login.example";
        for field in [
            r#""token_endpoint_auth_methods_supported":["none","client_secret_basic"],"#,
            r#""code_challenge_methods_supported":["S256"],"#,
        ] {
            let body = String::from_utf8(valid_body(issuer))
                .unwrap()
                .replace(field, "")
                .into_bytes();
            assert!(!decode(issuer, body).is_success());
        }
    }

    #[test]
    fn providers_without_rfc_9207_require_distinct_redirect_mode() {
        let issuer = "https://login.example";
        let without_rfc_9207 = String::from_utf8(valid_body(issuer))
            .unwrap()
            .replace(
                r#""authorization_response_iss_parameter_supported":true"#,
                r#""authorization_response_iss_parameter_supported":false"#,
            )
            .into_bytes();
        let metadata = decode(issuer, without_rfc_9207)
            .publish_then_release(&mut Sink::default())
            .unwrap();
        assert_eq!(
            metadata
                .into_provider_config("public-client", "http://127.0.0.1:43000/callback")
                .unwrap_err(),
            invalid(MetadataViolation::AuthorizationResponseIssuer)
        );

        let omitted = String::from_utf8(valid_body(issuer))
            .unwrap()
            .replace(
                r#""authorization_response_iss_parameter_supported":true,"#,
                "",
            )
            .into_bytes();
        let metadata = decode(issuer, omitted)
            .publish_then_release(&mut Sink::default())
            .unwrap();
        assert!(!metadata.authorization_response_iss_parameter_supported());
        let config = metadata
            .into_provider_config_with_distinct_redirect_uri(
                "public-client",
                "http://127.0.0.1:43001/callback",
            )
            .unwrap();
        assert!(matches!(
            config.mix_up_defense,
            Some(super::super::MixUpDefense::DistinctRedirectUri)
        ));
    }

    #[test]
    fn status_media_type_shape_duplicates_and_bounds_are_rejected() {
        let issuer = "https://login.example";
        let context = request(issuer).response_context();
        assert_eq!(
            decode_authorization_server_metadata(
                context,
                404,
                "application/json",
                Zeroizing::new(valid_body(issuer)),
            )
            .publish_then_release(&mut Sink::default())
            .unwrap_err(),
            invalid(MetadataViolation::Status)
        );

        let context = request(issuer).response_context();
        assert_eq!(
            decode_authorization_server_metadata(
                context,
                200,
                "text/html",
                Zeroizing::new(valid_body(issuer)),
            )
            .publish_then_release(&mut Sink::default())
            .unwrap_err(),
            invalid(MetadataViolation::MediaType)
        );

        let duplicate = format!(r#"{{"issuer":"{issuer}","issuer":"{issuer}"}}"#).into_bytes();
        assert_eq!(
            decode(issuer, duplicate)
                .publish_then_release(&mut Sink::default())
                .unwrap_err(),
            invalid(MetadataViolation::Shape)
        );

        let oversized = vec![b' '; MAX_METADATA_BYTES + 1];
        assert_eq!(
            decode(issuer, oversized)
                .publish_then_release(&mut Sink::default())
                .unwrap_err(),
            invalid(MetadataViolation::Encoding)
        );

        let deeply_nested = format!("{}null{}", "[".repeat(65), "]".repeat(65)).into_bytes();
        assert_eq!(
            decode(issuer, deeply_nested)
                .publish_then_release(&mut Sink::default())
                .unwrap_err(),
            invalid(MetadataViolation::Encoding)
        );
    }

    #[test]
    fn endpoint_policy_rejects_credentials_query_fragment_and_plain_http() {
        let issuer = "https://login.example";
        for endpoint in [
            "https://user@login.example/authorize",
            "https://login.example/authorize?tenant=a",
            "https://login.example/authorize#fragment",
            "https://login.example/authorize path",
            "http://login.example/authorize",
        ] {
            let body = String::from_utf8(valid_body(issuer))
                .unwrap()
                .replace("https://login.example/authorize", endpoint)
                .into_bytes();
            assert_eq!(
                decode(issuer, body)
                    .publish_then_release(&mut Sink::default())
                    .unwrap_err(),
                invalid(MetadataViolation::Endpoint)
            );
        }
    }

    #[test]
    fn invalid_issuer_is_configuration_failure_and_debug_is_redacted() {
        let audited =
            prepare_authorization_server_metadata(provider(), "http://login.example", trace());
        assert_eq!(
            audited.audit().outcome(),
            OAuthAuditOutcome::Failed(super::super::OAuthFailureClass::InvalidInput)
        );
        assert_eq!(
            audited
                .publish_then_release(&mut Sink::default())
                .unwrap_err(),
            OAuthError::InvalidConfiguration(ConfigurationViolation::Issuer)
        );

        let whitespace =
            prepare_authorization_server_metadata(provider(), " https://login.example", trace());
        assert_eq!(
            whitespace
                .publish_then_release(&mut Sink::default())
                .unwrap_err(),
            OAuthError::InvalidConfiguration(ConfigurationViolation::Issuer)
        );

        let request = request("https://secret-host.example/tenant");
        let debug = format!("{request:?}");
        assert!(!debug.contains("secret-host"));
        assert!(!debug.contains("tenant"));
    }

    fn audited_outcome(issuer: &str, body: Vec<u8>) -> OAuthAuditOutcome {
        decode(issuer, body).audit().outcome()
    }
}
