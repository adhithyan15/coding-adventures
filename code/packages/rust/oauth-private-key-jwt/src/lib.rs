//! Bounded, provider-driven OAuth `private_key_jwt` assertions.
//!
//! This pure construction layer owns neither entropy, time, keys, nor network
//! authority. It validates provider capabilities, builds RFC 7523 claims, and
//! delegates the only signing effect to the audited non-exporting signer.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_base64::{encode_into as encode_base64_into, URL_SAFE_NO_PAD};
use coding_adventures_bounded_json::{serialize, JsonNumber, JsonValue};
use coding_adventures_oauth::{AuthorizationServerMetadata, OAuthTraceId, ProviderId};
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
        mut replay_entropy: [u8; 32],
        trace: OAuthTraceId,
        audit: &mut A,
    ) -> Result<PrivateKeyJwtAssertion, PrivateKeyJwtError> {
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
        encode_base64_into(&replay_entropy, &URL_SAFE_NO_PAD, &mut jti);
        replay_entropy.zeroize();
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
    /// The audited non-exporting signer rejected or withheld the operation.
    Signing(PrivateKeySignerError),
}

impl Debug for PrivateKeyJwtError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidConfiguration => "InvalidConfiguration",
            Self::InvalidTime => "InvalidTime",
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
}
