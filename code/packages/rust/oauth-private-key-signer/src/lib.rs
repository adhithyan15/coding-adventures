//! Audit-first, non-exporting OAuth private-key signing.
//!
//! Provider configuration retains only opaque key references. Private-key
//! material is owned by an injected authority and the public API exposes only
//! a bounded sign operation. Every signature is durably audited before the
//! effect and before its result is released.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_oauth::{OAuthTraceId, ProviderId};
use coding_adventures_zeroize::Zeroizing;
use std::fmt::{self, Debug, Display, Formatter};

const MAX_SIGNING_INPUT_BYTES: usize = 64 * 1024;
const MAX_ALGORITHM_BYTES: usize = 64;

/// Opaque stable reference stored in provider data instead of a private key.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PrivateKeyReference([u8; 32]);

impl PrivateKeyReference {
    /// Construct an opaque reference from exact caller-owned bytes.
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Borrow exact bytes for a trusted authority's lossless key lookup.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl Debug for PrivateKeyReference {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("PrivateKeyReference(<redacted>)")
    }
}

/// Provider and opaque-reference tuple used for every signing operation.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PrivateKeyId {
    provider: ProviderId,
    reference: PrivateKeyReference,
}

impl PrivateKeyId {
    /// Bind one validated provider to one opaque private-key reference.
    pub const fn new(provider: ProviderId, reference: PrivateKeyReference) -> Self {
        Self {
            provider,
            reference,
        }
    }

    /// Return the stable provider identity.
    pub const fn provider(&self) -> &ProviderId {
        &self.provider
    }

    /// Return the opaque private-key reference.
    pub const fn reference(&self) -> PrivateKeyReference {
        self.reference
    }
}

impl Debug for PrivateKeyId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PrivateKeyId")
            .field("provider", &self.provider)
            .field("reference", &self.reference)
            .finish()
    }
}

/// Validated JOSE signing algorithm selected entirely by provider data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrivateKeyJwtAlgorithm(String);

impl PrivateKeyJwtAlgorithm {
    /// Validate a bounded case-sensitive JOSE `alg` identifier.
    ///
    /// `none` is forbidden for OAuth client authentication. Capability
    /// negotiation and the injected authority must additionally accept the
    /// exact value; this type does not imply algorithm support.
    pub fn new(value: impl Into<String>) -> Result<Self, PrivateKeySignerError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.len() <= MAX_ALGORITHM_BYTES
            && value != "none"
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'));
        if !valid {
            return Err(PrivateKeySignerError::InvalidInput);
        }
        Ok(Self(value))
    }

    /// Return the exact case-sensitive JOSE `alg` identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A signature returned without exposing its private key.
pub struct PrivateKeySignature(Zeroizing<Vec<u8>>);

impl PrivateKeySignature {
    /// Construct a signature inside trusted signer code.
    pub fn new(bytes: Vec<u8>) -> Result<Self, PrivateKeySignerError> {
        if bytes.is_empty() || bytes.len() > MAX_SIGNING_INPUT_BYTES {
            return Err(PrivateKeySignerError::Backend);
        }
        Ok(Self(Zeroizing::new(bytes)))
    }

    /// Borrow signature bytes for immediate JOSE encoding.
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl Debug for PrivateKeySignature {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("PrivateKeySignature(<redacted>)")
    }
}

/// Closed authority failure without key material or backend diagnostics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SigningAuthorityError {
    /// The opaque key was not present in the authority.
    NotFound,
    /// The requested algorithm does not match the stored key.
    AlgorithmMismatch,
    /// The signing authority failed.
    Backend,
}

/// Minimal non-exporting signing contract for HSMs and storage adapters.
pub trait SigningAuthority: Send + Sync {
    /// Sign exact bounded bytes; this API deliberately has no key-read method.
    fn sign(
        &self,
        key: &PrivateKeyId,
        algorithm: &PrivateKeyJwtAlgorithm,
        signing_input: &[u8],
    ) -> Result<PrivateKeySignature, SigningAuthorityError>;
}

/// Signing operation recorded before every private-key effect and result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrivateKeyAuditAction {
    /// Use a private key to produce a signature.
    Sign,
}

/// Closed privacy-safe audit outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrivateKeyAuditOutcome {
    /// Durable intent before signing.
    Attempted,
    /// Durable success before the authority or signature is released.
    Succeeded,
    /// Closed failure classification.
    Failed(PrivateKeyFailureClass),
}

/// Closed failure class safe for durable audit storage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrivateKeyFailureClass {
    /// Caller input or binding was invalid.
    InvalidInput,
    /// The opaque key was absent.
    NotFound,
    /// The configured algorithm did not match the key.
    AlgorithmMismatch,
    /// The injected authority failed.
    Backend,
}

/// Privacy-safe event containing no client ID, private key, message, or signature.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrivateKeyAuditEvent {
    key: PrivateKeyId,
    algorithm: PrivateKeyJwtAlgorithm,
    trace: OAuthTraceId,
    action: PrivateKeyAuditAction,
    outcome: PrivateKeyAuditOutcome,
}

impl PrivateKeyAuditEvent {
    /// Return the exact provider and opaque key reference.
    pub const fn key(&self) -> &PrivateKeyId {
        &self.key
    }

    /// Return the exact provider-selected JOSE signing algorithm.
    pub const fn algorithm(&self) -> &PrivateKeyJwtAlgorithm {
        &self.algorithm
    }

    /// Return the caller-owned correlation trace.
    pub const fn trace(&self) -> OAuthTraceId {
        self.trace
    }

    /// Return the closed operation.
    pub const fn action(&self) -> PrivateKeyAuditAction {
        self.action
    }

    /// Return the closed outcome.
    pub const fn outcome(&self) -> PrivateKeyAuditOutcome {
        self.outcome
    }
}

/// Durable audit publication required before every effect and release.
pub trait PrivateKeyAuditSink {
    /// Persist `event` durably or fail closed.
    fn publish(&mut self, event: &PrivateKeyAuditEvent) -> Result<(), PrivateKeyAuditError>;
}

/// Closed durable-audit failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrivateKeyAuditError;

/// Closed signing error with no private material or backend diagnostics.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PrivateKeySignerError {
    /// Caller input or provider/key binding was invalid.
    InvalidInput,
    /// The requested opaque key does not exist.
    NotFound,
    /// The requested algorithm does not match the stored key.
    AlgorithmMismatch,
    /// The injected signing authority failed.
    Backend,
    /// Durable audit publication failed and the result was withheld.
    Audit,
}

impl Debug for PrivateKeySignerError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidInput => "InvalidInput",
            Self::NotFound => "NotFound",
            Self::AlgorithmMismatch => "AlgorithmMismatch",
            Self::Backend => "Backend",
            Self::Audit => "Audit",
        })
    }
}

impl Display for PrivateKeySignerError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("oauth private-key signer: ")?;
        Debug::fmt(self, formatter)
    }
}

impl std::error::Error for PrivateKeySignerError {}

/// Audit-gated non-exporting signing over one injected authority.
pub struct AuditedPrivateKeySigner<S: SigningAuthority> {
    authority: S,
}

impl<S: SigningAuthority> AuditedPrivateKeySigner<S> {
    /// Wrap one authority. Key provisioning must already have been audited.
    pub const fn from_audited_authority(authority: S) -> Self {
        Self { authority }
    }

    /// Audit and sign exact bytes without exposing private-key material.
    pub fn sign<A: PrivateKeyAuditSink>(
        &self,
        key: &PrivateKeyId,
        algorithm: &PrivateKeyJwtAlgorithm,
        signing_input: &[u8],
        trace: OAuthTraceId,
        audit: &mut A,
    ) -> Result<PrivateKeySignature, PrivateKeySignerError> {
        attempt(audit, key, algorithm, trace, PrivateKeyAuditAction::Sign)?;
        let result = if signing_input.is_empty() || signing_input.len() > MAX_SIGNING_INPUT_BYTES {
            Err(PrivateKeySignerError::InvalidInput)
        } else {
            self.authority
                .sign(key, algorithm, signing_input)
                .map_err(map_authority_error)
        };
        finish(
            audit,
            key,
            algorithm,
            trace,
            PrivateKeyAuditAction::Sign,
            result,
        )
    }
}

impl<S: SigningAuthority> Debug for AuditedPrivateKeySigner<S> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("AuditedPrivateKeySigner(<redacted>)")
    }
}

fn attempt<A: PrivateKeyAuditSink>(
    audit: &mut A,
    key: &PrivateKeyId,
    algorithm: &PrivateKeyJwtAlgorithm,
    trace: OAuthTraceId,
    action: PrivateKeyAuditAction,
) -> Result<(), PrivateKeySignerError> {
    publish(
        audit,
        key,
        algorithm,
        trace,
        action,
        PrivateKeyAuditOutcome::Attempted,
    )
}

fn finish<T, A: PrivateKeyAuditSink>(
    audit: &mut A,
    key: &PrivateKeyId,
    algorithm: &PrivateKeyJwtAlgorithm,
    trace: OAuthTraceId,
    action: PrivateKeyAuditAction,
    result: Result<T, PrivateKeySignerError>,
) -> Result<T, PrivateKeySignerError> {
    let outcome = match &result {
        Ok(_) => PrivateKeyAuditOutcome::Succeeded,
        Err(error) => PrivateKeyAuditOutcome::Failed(error.failure_class()),
    };
    publish(audit, key, algorithm, trace, action, outcome)?;
    result
}

fn publish<A: PrivateKeyAuditSink>(
    audit: &mut A,
    key: &PrivateKeyId,
    algorithm: &PrivateKeyJwtAlgorithm,
    trace: OAuthTraceId,
    action: PrivateKeyAuditAction,
    outcome: PrivateKeyAuditOutcome,
) -> Result<(), PrivateKeySignerError> {
    audit
        .publish(&PrivateKeyAuditEvent {
            key: key.clone(),
            algorithm: algorithm.clone(),
            trace,
            action,
            outcome,
        })
        .map_err(|_| PrivateKeySignerError::Audit)
}

fn map_authority_error(error: SigningAuthorityError) -> PrivateKeySignerError {
    match error {
        SigningAuthorityError::NotFound => PrivateKeySignerError::NotFound,
        SigningAuthorityError::AlgorithmMismatch => PrivateKeySignerError::AlgorithmMismatch,
        SigningAuthorityError::Backend => PrivateKeySignerError::Backend,
    }
}

impl PrivateKeySignerError {
    fn failure_class(self) -> PrivateKeyFailureClass {
        match self {
            Self::InvalidInput => PrivateKeyFailureClass::InvalidInput,
            Self::NotFound => PrivateKeyFailureClass::NotFound,
            Self::AlgorithmMismatch => PrivateKeyFailureClass::AlgorithmMismatch,
            Self::Backend => PrivateKeyFailureClass::Backend,
            Self::Audit => unreachable!("audit failures are never recursively audited"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn key(provider: &str, marker: u8) -> PrivateKeyId {
        PrivateKeyId::new(
            ProviderId::new(provider).unwrap(),
            PrivateKeyReference::new([marker; 32]),
        )
    }

    fn trace() -> OAuthTraceId {
        OAuthTraceId::new([7; 16])
    }

    #[test]
    fn audit_failure_before_sign_prevents_the_effect() {
        struct CountingAuthority(std::sync::atomic::AtomicUsize);
        impl SigningAuthority for CountingAuthority {
            fn sign(
                &self,
                _key: &PrivateKeyId,
                _algorithm: &PrivateKeyJwtAlgorithm,
                _signing_input: &[u8],
            ) -> Result<PrivateKeySignature, SigningAuthorityError> {
                self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                PrivateKeySignature::new(vec![1]).map_err(|_| SigningAuthorityError::Backend)
            }
        }

        let signer = AuditedPrivateKeySigner::from_audited_authority(CountingAuthority(
            std::sync::atomic::AtomicUsize::new(0),
        ));
        let mut audit = Audit {
            events: Vec::new(),
            fail_at: Some(0),
        };
        assert!(matches!(
            signer.sign(
                &key("example", 1),
                &PrivateKeyJwtAlgorithm::new("EdDSA").unwrap(),
                b"input",
                trace(),
                &mut audit,
            ),
            Err(PrivateKeySignerError::Audit)
        ));
        assert_eq!(
            signer.authority.0.load(std::sync::atomic::Ordering::SeqCst),
            0
        );
    }

    #[test]
    fn audit_failure_after_sign_withholds_the_signature() {
        let key = key("example", 2);
        let signer =
            AuditedPrivateKeySigner::from_audited_authority(FixedAuthority { key: key.clone() });
        let mut audit = Audit {
            events: Vec::new(),
            fail_at: Some(1),
        };
        assert!(matches!(
            signer.sign(
                &key,
                &PrivateKeyJwtAlgorithm::new("EdDSA").unwrap(),
                b"input",
                trace(),
                &mut audit,
            ),
            Err(PrivateKeySignerError::Audit)
        ));
    }

    #[test]
    fn wrong_key_and_invalid_input_fail_closed_with_audit() {
        let stored_key = key("example", 5);
        let mut audit = Audit::default();
        let signer =
            AuditedPrivateKeySigner::from_audited_authority(FixedAuthority { key: stored_key });

        assert!(matches!(
            signer.sign(
                &key("other", 5),
                &PrivateKeyJwtAlgorithm::new("EdDSA").unwrap(),
                b"input",
                trace(),
                &mut audit,
            ),
            Err(PrivateKeySignerError::NotFound)
        ));
        assert!(matches!(
            signer.sign(
                &key("example", 5),
                &PrivateKeyJwtAlgorithm::new("EdDSA").unwrap(),
                b"",
                trace(),
                &mut audit,
            ),
            Err(PrivateKeySignerError::InvalidInput)
        ));
        assert_eq!(
            audit.events.last().unwrap().outcome(),
            PrivateKeyAuditOutcome::Failed(PrivateKeyFailureClass::InvalidInput)
        );
    }

    #[test]
    fn debug_and_errors_never_reveal_key_or_wire_material() {
        let key = key("example", 8);
        assert_eq!(
            format!("{:?}", key.reference()),
            "PrivateKeyReference(<redacted>)"
        );
        assert_eq!(
            format!("{:?}", PrivateKeySignature::new(vec![42]).unwrap()),
            "PrivateKeySignature(<redacted>)"
        );
        assert_eq!(
            PrivateKeySignerError::NotFound.to_string(),
            "oauth private-key signer: NotFound"
        );
    }

    struct FixedAuthority {
        key: PrivateKeyId,
    }

    impl SigningAuthority for FixedAuthority {
        fn sign(
            &self,
            key: &PrivateKeyId,
            algorithm: &PrivateKeyJwtAlgorithm,
            signing_input: &[u8],
        ) -> Result<PrivateKeySignature, SigningAuthorityError> {
            if key != &self.key {
                return Err(SigningAuthorityError::NotFound);
            }
            if algorithm.as_str() != "EdDSA" {
                return Err(SigningAuthorityError::AlgorithmMismatch);
            }
            let mut output = signing_input.to_vec();
            output.reverse();
            PrivateKeySignature::new(output).map_err(|_| SigningAuthorityError::Backend)
        }
    }

    #[test]
    fn sign_is_audit_bracketed_and_algorithm_bound() {
        let key = key("example", 3);
        let signer =
            AuditedPrivateKeySigner::from_audited_authority(FixedAuthority { key: key.clone() });
        let algorithm = PrivateKeyJwtAlgorithm::new("EdDSA").unwrap();
        let mut audit = Audit::default();
        let signature = signer
            .sign(&key, &algorithm, b"header.payload", trace(), &mut audit)
            .unwrap();

        assert_eq!(signature.as_bytes(), b"daolyap.redaeh");
        assert_eq!(audit.events.len(), 2);
        assert_eq!(audit.events[0].action(), PrivateKeyAuditAction::Sign);
        assert_eq!(audit.events[0].outcome(), PrivateKeyAuditOutcome::Attempted);
        assert_eq!(audit.events[0].key(), &key);
        assert_eq!(audit.events[0].algorithm(), &algorithm);
        assert_eq!(audit.events[0].trace(), trace());
        assert_eq!(audit.events[1].outcome(), PrivateKeyAuditOutcome::Succeeded);
    }

    #[test]
    fn algorithms_are_bounded_and_none_is_forbidden() {
        assert_eq!(
            PrivateKeyJwtAlgorithm::new("EdDSA").unwrap().as_str(),
            "EdDSA"
        );
        assert_eq!(
            PrivateKeyJwtAlgorithm::new("RS256").unwrap().as_str(),
            "RS256"
        );
        assert_eq!(
            PrivateKeyJwtAlgorithm::new("none"),
            Err(PrivateKeySignerError::InvalidInput)
        );
        assert_eq!(
            PrivateKeyJwtAlgorithm::new("bad alg"),
            Err(PrivateKeySignerError::InvalidInput)
        );
    }
}
