//! Audit-first OAuth account identity proof and opaque-key selection.
//!
//! OAuth token responses may carry an OpenID Connect ID token, but its payload
//! is untrusted until a separate authority verifies the signature and bound
//! claims. This crate defines that narrow authority boundary. It owns neither
//! JWT/JOSE implementation, JWKS retrieval, discovery, time, storage, nor
//! network access.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_oauth::{OAuthTraceId, ProviderId};
use coding_adventures_oauth_credential_custody::{AccountId, CredentialKey};
use coding_adventures_zeroize::Zeroizing;
use std::collections::BTreeSet;
use std::fmt::{self, Debug, Display, Formatter};
use url_parser::Url;

const MAX_ID_TOKEN_BYTES: usize = 64 * 1024;
const MAX_CLIENT_ID_BYTES: usize = 1_024;
const MAX_ISSUER_BYTES: usize = 2_048;
const MAX_NONCE_BYTES: usize = 1_024;
const MAX_ALGORITHMS: usize = 16;
const MAX_ALGORITHM_BYTES: usize = 64;

/// Opaque reference to trusted verification state such as a reviewed JWKS set.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IdentityVerificationReference([u8; 32]);

impl IdentityVerificationReference {
    /// Construct an opaque reference from exact caller-owned bytes.
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Borrow exact bytes for a trusted authority's lossless lookup.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl Debug for IdentityVerificationReference {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("IdentityVerificationReference(<redacted>)")
    }
}

/// Provider and opaque verification-context tuple used for every proof.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct IdentityVerificationId {
    provider: ProviderId,
    reference: IdentityVerificationReference,
}

impl IdentityVerificationId {
    /// Bind one validated provider to one opaque verification reference.
    pub const fn new(provider: ProviderId, reference: IdentityVerificationReference) -> Self {
        Self {
            provider,
            reference,
        }
    }

    /// Return the exact provider identity.
    pub const fn provider(&self) -> &ProviderId {
        &self.provider
    }

    /// Return the opaque verification reference.
    pub const fn reference(&self) -> IdentityVerificationReference {
        self.reference
    }
}

impl Debug for IdentityVerificationId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("IdentityVerificationId")
            .field("provider", &self.provider)
            .field("reference", &self.reference)
            .finish()
    }
}

/// Validated case-sensitive JOSE algorithm retained as provider policy data.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct IdTokenSigningAlgorithm(String);

impl IdTokenSigningAlgorithm {
    /// Validate one bounded JOSE `alg` identifier without enabling it.
    ///
    /// `none` is forbidden. The injected authority must still support the
    /// exact algorithm and verify that the ID token selects an allowed value.
    pub fn new(value: impl Into<String>) -> Result<Self, AccountIdentityError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.len() <= MAX_ALGORITHM_BYTES
            && value != "none"
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'));
        if !valid {
            return Err(AccountIdentityError::InvalidInput);
        }
        Ok(Self(value))
    }

    /// Return the exact case-sensitive JOSE `alg` identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Immutable provider data required for one class of ID-token proof.
pub struct IdTokenIdentityProfile {
    provider: ProviderId,
    client_id: String,
    issuer: String,
    allowed_algorithms: Vec<IdTokenSigningAlgorithm>,
    context: IdentityVerificationId,
}

impl IdTokenIdentityProfile {
    /// Validate exact provider, audience, issuer, algorithm, and context data.
    ///
    /// The issuer must be a bounded HTTPS URL without userinfo, query, or
    /// fragment. Algorithms are unique and case-sensitive; no default is
    /// invented. Constructing this profile enables no verification algorithm.
    pub fn new(
        provider: ProviderId,
        client_id: impl Into<String>,
        issuer: impl Into<String>,
        allowed_algorithms: Vec<IdTokenSigningAlgorithm>,
        context: IdentityVerificationId,
    ) -> Result<Self, AccountIdentityError> {
        let client_id = client_id.into();
        let issuer = issuer.into();
        let algorithms_are_valid = !allowed_algorithms.is_empty()
            && allowed_algorithms.len() <= MAX_ALGORITHMS
            && allowed_algorithms.iter().collect::<BTreeSet<_>>().len() == allowed_algorithms.len();
        if context.provider() != &provider
            || !valid_client_id(&client_id)
            || !valid_issuer(&issuer)
            || !algorithms_are_valid
        {
            return Err(AccountIdentityError::InvalidInput);
        }
        Ok(Self {
            provider,
            client_id,
            issuer,
            allowed_algorithms,
            context,
        })
    }

    /// Return the exact provider identity.
    pub const fn provider(&self) -> &ProviderId {
        &self.provider
    }

    /// Return the exact OAuth client ID used as the ID-token audience.
    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    /// Return the exact expected HTTPS issuer.
    pub fn issuer(&self) -> &str {
        &self.issuer
    }

    /// Return the exact allowed case-sensitive JOSE algorithms.
    pub fn allowed_algorithms(&self) -> &[IdTokenSigningAlgorithm] {
        &self.allowed_algorithms
    }

    /// Return the provider-bound opaque verification context.
    pub const fn context(&self) -> &IdentityVerificationId {
        &self.context
    }
}

impl Debug for IdTokenIdentityProfile {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("IdTokenIdentityProfile")
            .field("provider", &self.provider)
            .field("client_id", &"<redacted>")
            .field("issuer", &"<redacted>")
            .field("allowed_algorithms", &self.allowed_algorithms)
            .field("context", &self.context)
            .finish()
    }
}

/// Borrowed, bounded proof input visible only to the trusted authority call.
pub struct IdTokenVerificationRequest<'a> {
    profile: &'a IdTokenIdentityProfile,
    id_token: &'a str,
    expected_nonce: &'a str,
    observed_at_unix_seconds: u64,
}

impl IdTokenVerificationRequest<'_> {
    /// Return the exact provider identity.
    pub const fn provider(&self) -> &ProviderId {
        self.profile.provider()
    }

    /// Return the exact OAuth client ID that the `aud` claim must identify.
    pub fn client_id(&self) -> &str {
        self.profile.client_id()
    }

    /// Return the exact expected `iss` claim.
    pub fn issuer(&self) -> &str {
        self.profile.issuer()
    }

    /// Return the exact allowed case-sensitive JOSE algorithms.
    pub fn allowed_algorithms(&self) -> &[IdTokenSigningAlgorithm] {
        self.profile.allowed_algorithms()
    }

    /// Return the provider-bound opaque verification context.
    pub const fn context(&self) -> &IdentityVerificationId {
        self.profile.context()
    }

    /// Borrow the zeroizing token bytes for this authority call only.
    pub const fn id_token(&self) -> &str {
        self.id_token
    }

    /// Borrow the zeroizing expected transaction nonce for this call only.
    pub const fn expected_nonce(&self) -> &str {
        self.expected_nonce
    }

    /// Return the caller-owned Unix time used for expiry validation.
    pub const fn observed_at_unix_seconds(&self) -> u64 {
        self.observed_at_unix_seconds
    }
}

impl Debug for IdTokenVerificationRequest<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("IdTokenVerificationRequest")
            .field("provider", &self.provider())
            .field("client_id", &"<redacted>")
            .field("issuer", &"<redacted>")
            .field("allowed_algorithms", &self.allowed_algorithms())
            .field("context", &self.context())
            .field("id_token", &"<redacted>")
            .field("expected_nonce", &"<redacted>")
            .field("observed_at_unix_seconds", &self.observed_at_unix_seconds)
            .finish()
    }
}

/// Closed authority failure without token, claim, key, or backend diagnostics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdentityAuthorityError {
    /// The opaque verification context was absent.
    ContextNotFound,
    /// The token selected an unsupported or unadvertised algorithm.
    AlgorithmMismatch,
    /// Signature verification failed.
    SignatureInvalid,
    /// An issuer, audience, expiry, nonce, or subject requirement failed.
    ClaimsInvalid,
    /// The trusted authority failed internally.
    Backend,
}

/// Trusted ID-token verifier and provider-scoped opaque identity derivation.
pub trait AccountIdentityAuthority: Send + Sync {
    /// Verify one exact request and return only an opaque stable account ID.
    ///
    /// Implementations must select the token algorithm only from
    /// `request.allowed_algorithms()`, verify its signature using the exact
    /// provider-bound context, validate `iss`, `aud`, `exp`, and `nonce`,
    /// require a valid subject, and derive a provider-scoped opaque ID. Raw
    /// subject, claim, token, and verification-key material must not escape.
    fn verify_id_token(
        &self,
        request: &IdTokenVerificationRequest<'_>,
    ) -> Result<AccountId, IdentityAuthorityError>;
}

/// Identity operation recorded before every verification effect and result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccountIdentityAuditAction {
    /// Verify one ID token and derive an opaque credential key.
    VerifyIdToken,
}

/// Closed privacy-safe audit outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccountIdentityAuditOutcome {
    /// Durable intent before verification.
    Attempted,
    /// Durable success before the opaque key is released.
    Succeeded,
    /// Closed failure classification.
    Failed(AccountIdentityFailureClass),
}

/// Closed failure class safe for durable audit storage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccountIdentityFailureClass {
    /// Caller input or provider binding was invalid.
    InvalidInput,
    /// The opaque verification context was absent.
    ContextNotFound,
    /// The token selected an unsupported or unadvertised algorithm.
    AlgorithmMismatch,
    /// Signature verification failed.
    SignatureInvalid,
    /// Bound claim validation failed.
    ClaimsInvalid,
    /// The injected authority failed internally.
    Backend,
}

/// Privacy-safe event containing no client ID, issuer, nonce, token, or claim.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountIdentityAuditEvent {
    context: IdentityVerificationId,
    allowed_algorithms: Vec<IdTokenSigningAlgorithm>,
    trace: OAuthTraceId,
    action: AccountIdentityAuditAction,
    outcome: AccountIdentityAuditOutcome,
}

impl AccountIdentityAuditEvent {
    /// Return the exact provider and opaque verification context.
    pub const fn context(&self) -> &IdentityVerificationId {
        &self.context
    }

    /// Return the exact allowed algorithm policy used for the attempt.
    pub fn allowed_algorithms(&self) -> &[IdTokenSigningAlgorithm] {
        &self.allowed_algorithms
    }

    /// Return the caller-owned correlation trace.
    pub const fn trace(&self) -> OAuthTraceId {
        self.trace
    }

    /// Return the closed operation.
    pub const fn action(&self) -> AccountIdentityAuditAction {
        self.action
    }

    /// Return the closed outcome.
    pub const fn outcome(&self) -> AccountIdentityAuditOutcome {
        self.outcome
    }
}

/// Durable audit publication required before every effect and release.
pub trait AccountIdentityAuditSink {
    /// Persist `event` durably or fail closed.
    fn publish(
        &mut self,
        event: &AccountIdentityAuditEvent,
    ) -> Result<(), AccountIdentityAuditError>;
}

/// Closed durable-audit failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AccountIdentityAuditError;

/// Opaque credential key proven from one exact client transaction.
pub struct VerifiedCredentialKey {
    key: CredentialKey,
    client_id: String,
    trace: OAuthTraceId,
}

impl VerifiedCredentialKey {
    /// Borrow the provider-scoped opaque credential key.
    pub const fn credential_key(&self) -> &CredentialKey {
        &self.key
    }

    /// Return the exact client identity whose token established this key.
    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    /// Return the authorization transaction trace bound to the proof.
    pub const fn trace(&self) -> OAuthTraceId {
        self.trace
    }

    /// Consume the proof wrapper and release only its opaque credential key.
    pub fn into_credential_key(self) -> CredentialKey {
        self.key
    }
}

impl Debug for VerifiedCredentialKey {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedCredentialKey")
            .field("key", &self.key)
            .field("client_id", &"<redacted>")
            .field("trace", &self.trace)
            .finish()
    }
}

/// Closed account-identity failure without evidence or authority diagnostics.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AccountIdentityError {
    /// Caller input or provider/context binding was invalid.
    InvalidInput,
    /// The opaque verification context was absent.
    ContextNotFound,
    /// The token selected an unsupported or unadvertised algorithm.
    AlgorithmMismatch,
    /// Signature verification failed.
    SignatureInvalid,
    /// Bound claim validation failed.
    ClaimsInvalid,
    /// The trusted authority failed internally.
    Backend,
    /// Durable audit publication failed and the result was withheld.
    Audit,
}

impl Debug for AccountIdentityError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidInput => "InvalidInput",
            Self::ContextNotFound => "ContextNotFound",
            Self::AlgorithmMismatch => "AlgorithmMismatch",
            Self::SignatureInvalid => "SignatureInvalid",
            Self::ClaimsInvalid => "ClaimsInvalid",
            Self::Backend => "Backend",
            Self::Audit => "Audit",
        })
    }
}

impl Display for AccountIdentityError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("oauth account identity: ")?;
        Debug::fmt(self, formatter)
    }
}

impl std::error::Error for AccountIdentityError {}

/// Audit-gated account identity proof over one injected trusted authority.
pub struct AuditedAccountIdentityAuthority<V: AccountIdentityAuthority> {
    authority: V,
}

impl<V: AccountIdentityAuthority> AuditedAccountIdentityAuthority<V> {
    /// Wrap one authority whose verification-state provisioning is separately audited.
    pub const fn from_audited_authority(authority: V) -> Self {
        Self { authority }
    }

    /// Audit and verify one ID token, returning only an opaque credential key.
    ///
    /// Token and expected-nonce ownership is wipe-on-drop. `observed_at` must
    /// come from a caller-owned trusted clock. The authority effect occurs
    /// only after durable intent; the key is withheld unless its result event
    /// is also durable.
    pub fn verify_id_token<A: AccountIdentityAuditSink>(
        &self,
        profile: &IdTokenIdentityProfile,
        id_token: Zeroizing<String>,
        expected_nonce: Zeroizing<String>,
        observed_at_unix_seconds: u64,
        trace: OAuthTraceId,
        audit: &mut A,
    ) -> Result<VerifiedCredentialKey, AccountIdentityError> {
        attempt(audit, profile, trace)?;
        let result = if !valid_evidence(
            id_token.as_str(),
            expected_nonce.as_str(),
            observed_at_unix_seconds,
        ) {
            Err(AccountIdentityError::InvalidInput)
        } else {
            let request = IdTokenVerificationRequest {
                profile,
                id_token: id_token.as_str(),
                expected_nonce: expected_nonce.as_str(),
                observed_at_unix_seconds,
            };
            self.authority
                .verify_id_token(&request)
                .map(|account| VerifiedCredentialKey {
                    key: CredentialKey::new(profile.provider().clone(), account),
                    client_id: profile.client_id().to_owned(),
                    trace,
                })
                .map_err(map_authority_error)
        };
        finish(audit, profile, trace, result)
    }
}

impl<V: AccountIdentityAuthority> Debug for AuditedAccountIdentityAuthority<V> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("AuditedAccountIdentityAuthority(<redacted>)")
    }
}

fn valid_client_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_CLIENT_ID_BYTES && !value.chars().any(char::is_control)
}

fn valid_issuer(value: &str) -> bool {
    if value.is_empty()
        || value.len() > MAX_ISSUER_BYTES
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

fn valid_evidence(id_token: &str, expected_nonce: &str, observed_at_unix_seconds: u64) -> bool {
    !id_token.is_empty()
        && id_token.len() <= MAX_ID_TOKEN_BYTES
        && !id_token.chars().any(char::is_control)
        && !expected_nonce.is_empty()
        && expected_nonce.len() <= MAX_NONCE_BYTES
        && !expected_nonce.chars().any(char::is_control)
        && observed_at_unix_seconds <= i64::MAX as u64
}

fn attempt<A: AccountIdentityAuditSink>(
    audit: &mut A,
    profile: &IdTokenIdentityProfile,
    trace: OAuthTraceId,
) -> Result<(), AccountIdentityError> {
    publish(
        audit,
        profile,
        trace,
        AccountIdentityAuditOutcome::Attempted,
    )
}

fn finish<T, A: AccountIdentityAuditSink>(
    audit: &mut A,
    profile: &IdTokenIdentityProfile,
    trace: OAuthTraceId,
    result: Result<T, AccountIdentityError>,
) -> Result<T, AccountIdentityError> {
    let outcome = match &result {
        Ok(_) => AccountIdentityAuditOutcome::Succeeded,
        Err(error) => AccountIdentityAuditOutcome::Failed(error.failure_class()),
    };
    publish(audit, profile, trace, outcome)?;
    result
}

fn publish<A: AccountIdentityAuditSink>(
    audit: &mut A,
    profile: &IdTokenIdentityProfile,
    trace: OAuthTraceId,
    outcome: AccountIdentityAuditOutcome,
) -> Result<(), AccountIdentityError> {
    audit
        .publish(&AccountIdentityAuditEvent {
            context: profile.context().clone(),
            allowed_algorithms: profile.allowed_algorithms().to_vec(),
            trace,
            action: AccountIdentityAuditAction::VerifyIdToken,
            outcome,
        })
        .map_err(|_| AccountIdentityError::Audit)
}

fn map_authority_error(error: IdentityAuthorityError) -> AccountIdentityError {
    match error {
        IdentityAuthorityError::ContextNotFound => AccountIdentityError::ContextNotFound,
        IdentityAuthorityError::AlgorithmMismatch => AccountIdentityError::AlgorithmMismatch,
        IdentityAuthorityError::SignatureInvalid => AccountIdentityError::SignatureInvalid,
        IdentityAuthorityError::ClaimsInvalid => AccountIdentityError::ClaimsInvalid,
        IdentityAuthorityError::Backend => AccountIdentityError::Backend,
    }
}

impl AccountIdentityError {
    fn failure_class(self) -> AccountIdentityFailureClass {
        match self {
            Self::InvalidInput => AccountIdentityFailureClass::InvalidInput,
            Self::ContextNotFound => AccountIdentityFailureClass::ContextNotFound,
            Self::AlgorithmMismatch => AccountIdentityFailureClass::AlgorithmMismatch,
            Self::SignatureInvalid => AccountIdentityFailureClass::SignatureInvalid,
            Self::ClaimsInvalid => AccountIdentityFailureClass::ClaimsInvalid,
            Self::Backend => AccountIdentityFailureClass::Backend,
            Self::Audit => unreachable!("audit failures are never recursively audited"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct Audit {
        events: Vec<AccountIdentityAuditEvent>,
        fail_at: Option<usize>,
    }

    impl AccountIdentityAuditSink for Audit {
        fn publish(
            &mut self,
            event: &AccountIdentityAuditEvent,
        ) -> Result<(), AccountIdentityAuditError> {
            if self.fail_at == Some(self.events.len()) {
                return Err(AccountIdentityAuditError);
            }
            self.events.push(event.clone());
            Ok(())
        }
    }

    fn provider(name: &str) -> ProviderId {
        ProviderId::new(name).unwrap()
    }

    fn context(name: &str, marker: u8) -> IdentityVerificationId {
        IdentityVerificationId::new(
            provider(name),
            IdentityVerificationReference::new([marker; 32]),
        )
    }

    fn profile() -> IdTokenIdentityProfile {
        IdTokenIdentityProfile::new(
            provider("fixture"),
            "client-1",
            "https://issuer.example/tenant",
            vec![
                IdTokenSigningAlgorithm::new("EdDSA").unwrap(),
                IdTokenSigningAlgorithm::new("RS256").unwrap(),
            ],
            context("fixture", 3),
        )
        .unwrap()
    }

    fn trace() -> OAuthTraceId {
        OAuthTraceId::new([9; 16])
    }

    #[derive(Clone, Default)]
    struct RecordingAuthority {
        calls: Arc<AtomicUsize>,
        seen: Arc<Mutex<Vec<String>>>,
    }

    impl AccountIdentityAuthority for RecordingAuthority {
        fn verify_id_token(
            &self,
            request: &IdTokenVerificationRequest<'_>,
        ) -> Result<AccountId, IdentityAuthorityError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.seen.lock().unwrap().push(format!(
                "{}|{}|{}|{}|{}|{}|{}",
                request.provider().as_str(),
                request.client_id(),
                request.issuer(),
                request
                    .allowed_algorithms()
                    .iter()
                    .map(IdTokenSigningAlgorithm::as_str)
                    .collect::<Vec<_>>()
                    .join(","),
                request.id_token(),
                request.expected_nonce(),
                request.observed_at_unix_seconds(),
            ));
            Ok(AccountId::new([0x42; 32]))
        }
    }

    #[test]
    fn profile_rejects_unsafe_or_ambiguous_policy() {
        assert!(matches!(
            IdTokenSigningAlgorithm::new("none"),
            Err(AccountIdentityError::InvalidInput)
        ));
        assert!(matches!(
            IdTokenSigningAlgorithm::new("RS 256"),
            Err(AccountIdentityError::InvalidInput)
        ));
        let algorithm = IdTokenSigningAlgorithm::new("RS256").unwrap();

        for result in [
            IdTokenIdentityProfile::new(
                provider("fixture"),
                "",
                "https://issuer.example",
                vec![algorithm.clone()],
                context("fixture", 1),
            ),
            IdTokenIdentityProfile::new(
                provider("fixture"),
                "client",
                "http://issuer.example",
                vec![algorithm.clone()],
                context("fixture", 1),
            ),
            IdTokenIdentityProfile::new(
                provider("fixture"),
                "client",
                "https://issuer.example?query=yes",
                vec![algorithm.clone()],
                context("fixture", 1),
            ),
            IdTokenIdentityProfile::new(
                provider("fixture"),
                "client",
                "https://issuer.example",
                Vec::new(),
                context("fixture", 1),
            ),
            IdTokenIdentityProfile::new(
                provider("fixture"),
                "client",
                "https://issuer.example",
                vec![algorithm.clone(), algorithm.clone()],
                context("fixture", 1),
            ),
            IdTokenIdentityProfile::new(
                provider("fixture"),
                "client",
                "https://issuer.example",
                vec![algorithm],
                context("other", 1),
            ),
        ] {
            assert!(matches!(result, Err(AccountIdentityError::InvalidInput)));
        }
        assert!(matches!(
            IdTokenIdentityProfile::new(
                provider("fixture"),
                "client",
                "https://issuer.example",
                vec![IdTokenSigningAlgorithm::new("RS256").unwrap(); MAX_ALGORITHMS + 1],
                context("fixture", 1),
            ),
            Err(AccountIdentityError::InvalidInput)
        ));
    }

    #[test]
    fn verifies_exact_bindings_and_releases_only_an_opaque_key() {
        let authority = RecordingAuthority::default();
        let inspection = authority.clone();
        let verifier = AuditedAccountIdentityAuthority::from_audited_authority(authority);
        let mut audit = Audit::default();

        let verified = verifier
            .verify_id_token(
                &profile(),
                Zeroizing::new("header.payload.signature".to_owned()),
                Zeroizing::new("nonce-value".to_owned()),
                1_700_000_000,
                trace(),
                &mut audit,
            )
            .unwrap();

        assert_eq!(inspection.calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            inspection.seen.lock().unwrap().as_slice(),
            ["fixture|client-1|https://issuer.example/tenant|EdDSA,RS256|header.payload.signature|nonce-value|1700000000"]
        );
        assert_eq!(verified.credential_key().provider().as_str(), "fixture");
        assert_eq!(
            verified.credential_key().account(),
            AccountId::new([0x42; 32])
        );
        assert_eq!(verified.client_id(), "client-1");
        assert_eq!(verified.trace(), trace());
        assert_eq!(audit.events.len(), 2);
        for event in &audit.events {
            assert_eq!(event.context(), profile().context());
            assert_eq!(event.trace(), trace());
            assert_eq!(event.action(), AccountIdentityAuditAction::VerifyIdToken);
            assert_eq!(
                event
                    .allowed_algorithms()
                    .iter()
                    .map(IdTokenSigningAlgorithm::as_str)
                    .collect::<Vec<_>>(),
                ["EdDSA", "RS256"]
            );
        }
        assert_eq!(
            audit.events[0].outcome(),
            AccountIdentityAuditOutcome::Attempted
        );
        assert_eq!(
            audit.events[1].outcome(),
            AccountIdentityAuditOutcome::Succeeded
        );
    }

    #[test]
    fn first_audit_failure_prevents_authority_access() {
        let authority = RecordingAuthority::default();
        let inspection = authority.clone();
        let verifier = AuditedAccountIdentityAuthority::from_audited_authority(authority);
        let mut audit = Audit {
            events: Vec::new(),
            fail_at: Some(0),
        };

        assert!(matches!(
            verifier.verify_id_token(
                &profile(),
                Zeroizing::new("token".to_owned()),
                Zeroizing::new("nonce".to_owned()),
                10,
                trace(),
                &mut audit,
            ),
            Err(AccountIdentityError::Audit)
        ));
        assert_eq!(inspection.calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn final_audit_failure_withholds_the_verified_key() {
        let authority = RecordingAuthority::default();
        let inspection = authority.clone();
        let verifier = AuditedAccountIdentityAuthority::from_audited_authority(authority);
        let mut audit = Audit {
            events: Vec::new(),
            fail_at: Some(1),
        };

        assert!(matches!(
            verifier.verify_id_token(
                &profile(),
                Zeroizing::new("token".to_owned()),
                Zeroizing::new("nonce".to_owned()),
                10,
                trace(),
                &mut audit,
            ),
            Err(AccountIdentityError::Audit)
        ));
        assert_eq!(inspection.calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn invalid_evidence_is_audited_without_authority_access() {
        let authority = RecordingAuthority::default();
        let inspection = authority.clone();
        let verifier = AuditedAccountIdentityAuthority::from_audited_authority(authority);
        let cases = [
            ("", "nonce", 10),
            ("token\n", "nonce", 10),
            ("token", "", 10),
            ("token", "nonce\n", 10),
            ("token", "nonce", i64::MAX as u64 + 1),
        ];

        for (token, nonce, observed_at) in cases {
            let mut audit = Audit::default();
            assert!(matches!(
                verifier.verify_id_token(
                    &profile(),
                    Zeroizing::new(token.to_owned()),
                    Zeroizing::new(nonce.to_owned()),
                    observed_at,
                    trace(),
                    &mut audit,
                ),
                Err(AccountIdentityError::InvalidInput)
            ));
            assert_eq!(audit.events.len(), 2);
            assert_eq!(
                audit.events[1].outcome(),
                AccountIdentityAuditOutcome::Failed(AccountIdentityFailureClass::InvalidInput)
            );
        }
        for (token, nonce) in [
            ("t".repeat(MAX_ID_TOKEN_BYTES + 1), "nonce".to_owned()),
            ("token".to_owned(), "n".repeat(MAX_NONCE_BYTES + 1)),
        ] {
            let mut audit = Audit::default();
            assert!(matches!(
                verifier.verify_id_token(
                    &profile(),
                    Zeroizing::new(token),
                    Zeroizing::new(nonce),
                    10,
                    trace(),
                    &mut audit,
                ),
                Err(AccountIdentityError::InvalidInput)
            ));
            assert_eq!(audit.events.len(), 2);
        }
        assert_eq!(inspection.calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn authority_failures_are_closed_and_audited() {
        struct FailingAuthority(IdentityAuthorityError);
        impl AccountIdentityAuthority for FailingAuthority {
            fn verify_id_token(
                &self,
                _request: &IdTokenVerificationRequest<'_>,
            ) -> Result<AccountId, IdentityAuthorityError> {
                Err(self.0)
            }
        }

        let cases = [
            (
                IdentityAuthorityError::ContextNotFound,
                AccountIdentityError::ContextNotFound,
                AccountIdentityFailureClass::ContextNotFound,
            ),
            (
                IdentityAuthorityError::AlgorithmMismatch,
                AccountIdentityError::AlgorithmMismatch,
                AccountIdentityFailureClass::AlgorithmMismatch,
            ),
            (
                IdentityAuthorityError::SignatureInvalid,
                AccountIdentityError::SignatureInvalid,
                AccountIdentityFailureClass::SignatureInvalid,
            ),
            (
                IdentityAuthorityError::ClaimsInvalid,
                AccountIdentityError::ClaimsInvalid,
                AccountIdentityFailureClass::ClaimsInvalid,
            ),
            (
                IdentityAuthorityError::Backend,
                AccountIdentityError::Backend,
                AccountIdentityFailureClass::Backend,
            ),
        ];

        for (authority_error, expected_error, expected_class) in cases {
            let verifier = AuditedAccountIdentityAuthority::from_audited_authority(
                FailingAuthority(authority_error),
            );
            let mut audit = Audit::default();
            let error = verifier
                .verify_id_token(
                    &profile(),
                    Zeroizing::new("token".to_owned()),
                    Zeroizing::new("nonce".to_owned()),
                    10,
                    trace(),
                    &mut audit,
                )
                .unwrap_err();
            assert_eq!(error, expected_error);
            assert_eq!(
                audit.events[1].outcome(),
                AccountIdentityAuditOutcome::Failed(expected_class)
            );
        }
    }

    #[test]
    fn debug_and_errors_do_not_reveal_identity_evidence() {
        let profile = profile();
        let request = IdTokenVerificationRequest {
            profile: &profile,
            id_token: "secret-token",
            expected_nonce: "secret-nonce",
            observed_at_unix_seconds: 10,
        };
        let request_debug = format!("{request:?}");
        assert!(!request_debug.contains("secret-token"));
        assert!(!request_debug.contains("secret-nonce"));
        assert!(!format!("{profile:?}").contains("client-1"));
        assert_eq!(
            format!("{:?}", IdentityVerificationReference::new([1; 32])),
            "IdentityVerificationReference(<redacted>)"
        );
        assert_eq!(
            AccountIdentityError::SignatureInvalid.to_string(),
            "oauth account identity: SignatureInvalid"
        );
    }
}
