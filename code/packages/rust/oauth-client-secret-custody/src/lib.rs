//! Audit-first, storage-agnostic OAuth client-secret custody.
//!
//! Provider configuration retains only opaque references. Secret bytes are
//! zeroizing and cross the boundary only into one audited closure.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_oauth::{OAuthTraceId, ProviderId};
use coding_adventures_zeroize::Zeroizing;
use std::collections::BTreeMap;
use std::fmt::{self, Debug, Display, Formatter};
use std::sync::Mutex;

const MAX_SECRET_BYTES: usize = 64 * 1024;

/// Opaque stable reference stored in provider configuration instead of a secret.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClientSecretReference([u8; 32]);

impl ClientSecretReference {
    /// Construct an opaque reference from exact caller-owned bytes.
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Borrow exact bytes for a trusted store's lossless key encoding.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl Debug for ClientSecretReference {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("ClientSecretReference(<redacted>)")
    }
}

/// Provider and opaque reference tuple used for every custody operation.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ClientSecretKey {
    provider: ProviderId,
    reference: ClientSecretReference,
}

impl ClientSecretKey {
    /// Bind one validated provider to one opaque secret reference.
    pub const fn new(provider: ProviderId, reference: ClientSecretReference) -> Self {
        Self {
            provider,
            reference,
        }
    }

    /// Return the stable provider identity.
    pub const fn provider(&self) -> &ProviderId {
        &self.provider
    }

    /// Return the opaque secret reference.
    pub const fn reference(&self) -> ClientSecretReference {
        self.reference
    }
}

impl Debug for ClientSecretKey {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ClientSecretKey")
            .field("provider", &self.provider)
            .field("reference", &self.reference)
            .finish()
    }
}

/// Opaque compare-and-swap revision issued by the injected store.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ClientSecretRevision([u8; 32]);

impl ClientSecretRevision {
    /// Restore an exact backend-issued revision.
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Borrow exact bytes for a trusted backend.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl Debug for ClientSecretRevision {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("ClientSecretRevision(<redacted>)")
    }
}

/// Validated wipe-on-drop client-secret ownership for trusted adapters.
pub struct ClientSecret(Zeroizing<String>);

impl ClientSecret {
    /// Validate and take ownership of one client secret.
    pub fn new(secret: Zeroizing<String>) -> Result<Self, ClientSecretCustodyError> {
        if secret.is_empty()
            || secret.len() > MAX_SECRET_BYTES
            || secret.chars().any(char::is_control)
        {
            return Err(ClientSecretCustodyError::InvalidInput);
        }
        Ok(Self(secret))
    }

    /// Borrow the secret inside a trusted store adapter.
    pub fn expose_for_storage(&self) -> &str {
        self.0.as_str()
    }

    fn duplicate_for_store_read(&self) -> Self {
        Self(Zeroizing::new(self.0.as_str().to_owned()))
    }
}

impl Debug for ClientSecret {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("ClientSecret(<redacted>)")
    }
}

/// One decrypted store read bound to its exact revision.
pub struct StoredClientSecret {
    revision: ClientSecretRevision,
    secret: ClientSecret,
}

impl StoredClientSecret {
    /// Construct a trusted store result.
    pub const fn new(revision: ClientSecretRevision, secret: ClientSecret) -> Self {
        Self { revision, secret }
    }

    /// Return the opaque backend revision.
    pub const fn revision(&self) -> ClientSecretRevision {
        self.revision
    }
}

impl Debug for StoredClientSecret {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StoredClientSecret")
            .field("revision", &self.revision)
            .field("secret", &self.secret)
            .finish()
    }
}

/// Closed trusted-store failure without backend diagnostics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClientSecretStoreError {
    /// A create or compare-and-swap condition failed.
    Conflict,
    /// Encrypted or decoded secret state was corrupt.
    Corruption,
    /// The trusted backend failed or was unavailable.
    Backend,
}

/// Minimal atomic storage contract injected into client-secret custody.
pub trait ClientSecretStore: Send + Sync {
    /// Create a secret only when its key is absent.
    fn create(
        &self,
        key: &ClientSecretKey,
        secret: ClientSecret,
    ) -> Result<ClientSecretRevision, ClientSecretStoreError>;

    /// Load and decrypt one exact secret.
    fn load(
        &self,
        key: &ClientSecretKey,
    ) -> Result<Option<StoredClientSecret>, ClientSecretStoreError>;

    /// Atomically replace a secret only at `expected`.
    fn compare_and_swap(
        &self,
        key: &ClientSecretKey,
        expected: ClientSecretRevision,
        replacement: ClientSecret,
    ) -> Result<ClientSecretRevision, ClientSecretStoreError>;

    /// Atomically delete a secret only at `expected`.
    fn delete(
        &self,
        key: &ClientSecretKey,
        expected: ClientSecretRevision,
    ) -> Result<(), ClientSecretStoreError>;
}

/// Client-secret operation recorded before each access, edit, and result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClientSecretAuditAction {
    /// Create initial secret state.
    Create,
    /// Disclose a secret and revision to one authorized closure.
    Access,
    /// Atomically rotate secret state.
    Rotate,
    /// Conditionally delete secret state.
    Delete,
}

/// Closed privacy-safe audit result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClientSecretAuditOutcome {
    /// Durable intent before storage access or edit.
    Attempted,
    /// Durable success before result or secret release.
    Succeeded,
    /// Closed failure classification.
    Failed(ClientSecretFailureClass),
}

/// Closed failure class safe for durable audit storage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClientSecretFailureClass {
    /// Caller input or secret shape was invalid.
    InvalidInput,
    /// The requested secret was absent.
    NotFound,
    /// A conditional storage operation lost a race.
    Conflict,
    /// Stored client-secret material was corrupt.
    Corruption,
    /// The injected backend failed.
    Backend,
}

/// Privacy-safe event containing no client ID, secret, label, or backend text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientSecretAuditEvent {
    key: ClientSecretKey,
    trace: OAuthTraceId,
    action: ClientSecretAuditAction,
    outcome: ClientSecretAuditOutcome,
}

impl ClientSecretAuditEvent {
    /// Return the exact provider and opaque reference.
    pub const fn key(&self) -> &ClientSecretKey {
        &self.key
    }

    /// Return the caller-owned correlation trace.
    pub const fn trace(&self) -> OAuthTraceId {
        self.trace
    }

    /// Return the closed operation.
    pub const fn action(&self) -> ClientSecretAuditAction {
        self.action
    }

    /// Return the closed outcome.
    pub const fn outcome(&self) -> ClientSecretAuditOutcome {
        self.outcome
    }
}

/// Durable audit publication required before every effect and disclosure.
pub trait ClientSecretAuditSink {
    /// Persist `event` durably or fail closed.
    fn publish(&mut self, event: &ClientSecretAuditEvent) -> Result<(), ClientSecretAuditError>;
}

/// Closed durable-audit failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClientSecretAuditError;

/// Closed custody error with no secret or backend diagnostics.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ClientSecretCustodyError {
    /// Caller input or secret shape was invalid.
    InvalidInput,
    /// The requested secret does not exist.
    NotFound,
    /// An atomic storage condition failed.
    Conflict,
    /// Stored client-secret state was corrupt.
    Corruption,
    /// The trusted backend failed.
    Backend,
    /// Durable audit publication failed and the result was withheld.
    Audit,
}

impl Debug for ClientSecretCustodyError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidInput => "InvalidInput",
            Self::NotFound => "NotFound",
            Self::Conflict => "Conflict",
            Self::Corruption => "Corruption",
            Self::Backend => "Backend",
            Self::Audit => "Audit",
        })
    }
}

impl Display for ClientSecretCustodyError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("oauth client-secret custody: ")?;
        Debug::fmt(self, formatter)
    }
}

impl std::error::Error for ClientSecretCustodyError {}

/// Audit-gated client-secret operations over one injected trusted store.
pub struct ClientSecretCustody<S: ClientSecretStore> {
    store: S,
}

impl<S: ClientSecretStore> ClientSecretCustody<S> {
    /// Construct custody over one trusted storage authority.
    pub const fn new(store: S) -> Self {
        Self { store }
    }

    /// Audit and create initial client-secret state if absent.
    pub fn create<A: ClientSecretAuditSink>(
        &self,
        key: &ClientSecretKey,
        secret: Zeroizing<String>,
        trace: OAuthTraceId,
        audit: &mut A,
    ) -> Result<ClientSecretRevision, ClientSecretCustodyError> {
        attempt(audit, key, trace, ClientSecretAuditAction::Create)?;
        let result = ClientSecret::new(secret)
            .and_then(|secret| self.store.create(key, secret).map_err(map_store_error));
        finish(audit, key, trace, ClientSecretAuditAction::Create, result)
    }

    /// Audit, load, and disclose one secret and revision to exactly one closure.
    pub fn with_secret<R, A: ClientSecretAuditSink>(
        &self,
        key: &ClientSecretKey,
        trace: OAuthTraceId,
        audit: &mut A,
        use_secret: impl FnOnce(&str, ClientSecretRevision) -> R,
    ) -> Result<R, ClientSecretCustodyError> {
        attempt(audit, key, trace, ClientSecretAuditAction::Access)?;
        let loaded = match self.store.load(key).map_err(map_store_error) {
            Ok(Some(loaded)) => loaded,
            Ok(None) => {
                return finish(
                    audit,
                    key,
                    trace,
                    ClientSecretAuditAction::Access,
                    Err(ClientSecretCustodyError::NotFound),
                )
            }
            Err(error) => {
                return finish(
                    audit,
                    key,
                    trace,
                    ClientSecretAuditAction::Access,
                    Err(error),
                )
            }
        };
        publish(
            audit,
            key,
            trace,
            ClientSecretAuditAction::Access,
            ClientSecretAuditOutcome::Succeeded,
        )?;
        Ok(use_secret(loaded.secret.0.as_str(), loaded.revision))
    }

    /// Audit and atomically rotate client-secret state at `expected`.
    pub fn rotate<A: ClientSecretAuditSink>(
        &self,
        key: &ClientSecretKey,
        expected: ClientSecretRevision,
        replacement: Zeroizing<String>,
        trace: OAuthTraceId,
        audit: &mut A,
    ) -> Result<ClientSecretRevision, ClientSecretCustodyError> {
        attempt(audit, key, trace, ClientSecretAuditAction::Rotate)?;
        let result = ClientSecret::new(replacement).and_then(|secret| {
            self.store
                .compare_and_swap(key, expected, secret)
                .map_err(map_store_error)
        });
        finish(audit, key, trace, ClientSecretAuditAction::Rotate, result)
    }

    /// Audit and conditionally delete exact client-secret state.
    pub fn delete<A: ClientSecretAuditSink>(
        &self,
        key: &ClientSecretKey,
        expected: ClientSecretRevision,
        trace: OAuthTraceId,
        audit: &mut A,
    ) -> Result<(), ClientSecretCustodyError> {
        attempt(audit, key, trace, ClientSecretAuditAction::Delete)?;
        let result = self.store.delete(key, expected).map_err(map_store_error);
        finish(audit, key, trace, ClientSecretAuditAction::Delete, result)
    }
}

impl<S: ClientSecretStore> Debug for ClientSecretCustody<S> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("ClientSecretCustody(<redacted>)")
    }
}

/// Deterministic thread-safe reference store for tests and local composition.
#[derive(Default)]
pub struct InMemoryClientSecretStore {
    state: Mutex<MemoryState>,
}

#[derive(Default)]
struct MemoryState {
    next_revision: u64,
    secrets: BTreeMap<ClientSecretKey, (ClientSecretRevision, ClientSecret)>,
}

impl InMemoryClientSecretStore {
    /// Construct an empty in-memory store.
    pub const fn new() -> Self {
        Self {
            state: Mutex::new(MemoryState {
                next_revision: 0,
                secrets: BTreeMap::new(),
            }),
        }
    }
}

impl Debug for InMemoryClientSecretStore {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("InMemoryClientSecretStore(<redacted>)")
    }
}

impl ClientSecretStore for InMemoryClientSecretStore {
    fn create(
        &self,
        key: &ClientSecretKey,
        secret: ClientSecret,
    ) -> Result<ClientSecretRevision, ClientSecretStoreError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| ClientSecretStoreError::Backend)?;
        if state.secrets.contains_key(key) {
            return Err(ClientSecretStoreError::Conflict);
        }
        let revision = next_revision(&mut state)?;
        state.secrets.insert(key.clone(), (revision, secret));
        Ok(revision)
    }

    fn load(
        &self,
        key: &ClientSecretKey,
    ) -> Result<Option<StoredClientSecret>, ClientSecretStoreError> {
        let state = self
            .state
            .lock()
            .map_err(|_| ClientSecretStoreError::Backend)?;
        Ok(state.secrets.get(key).map(|(revision, secret)| {
            StoredClientSecret::new(*revision, secret.duplicate_for_store_read())
        }))
    }

    fn compare_and_swap(
        &self,
        key: &ClientSecretKey,
        expected: ClientSecretRevision,
        replacement: ClientSecret,
    ) -> Result<ClientSecretRevision, ClientSecretStoreError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| ClientSecretStoreError::Backend)?;
        match state.secrets.get(key) {
            Some((revision, _)) if *revision == expected => {}
            _ => return Err(ClientSecretStoreError::Conflict),
        }
        let revision = next_revision(&mut state)?;
        state.secrets.insert(key.clone(), (revision, replacement));
        Ok(revision)
    }

    fn delete(
        &self,
        key: &ClientSecretKey,
        expected: ClientSecretRevision,
    ) -> Result<(), ClientSecretStoreError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| ClientSecretStoreError::Backend)?;
        match state.secrets.get(key) {
            Some((revision, _)) if *revision == expected => {
                state.secrets.remove(key);
                Ok(())
            }
            _ => Err(ClientSecretStoreError::Conflict),
        }
    }
}

fn next_revision(state: &mut MemoryState) -> Result<ClientSecretRevision, ClientSecretStoreError> {
    state.next_revision = state
        .next_revision
        .checked_add(1)
        .ok_or(ClientSecretStoreError::Backend)?;
    let mut bytes = [0_u8; 32];
    bytes[24..].copy_from_slice(&state.next_revision.to_be_bytes());
    Ok(ClientSecretRevision::new(bytes))
}

fn attempt<A: ClientSecretAuditSink>(
    audit: &mut A,
    key: &ClientSecretKey,
    trace: OAuthTraceId,
    action: ClientSecretAuditAction,
) -> Result<(), ClientSecretCustodyError> {
    publish(
        audit,
        key,
        trace,
        action,
        ClientSecretAuditOutcome::Attempted,
    )
}

fn finish<T, A: ClientSecretAuditSink>(
    audit: &mut A,
    key: &ClientSecretKey,
    trace: OAuthTraceId,
    action: ClientSecretAuditAction,
    result: Result<T, ClientSecretCustodyError>,
) -> Result<T, ClientSecretCustodyError> {
    let outcome = match &result {
        Ok(_) => ClientSecretAuditOutcome::Succeeded,
        Err(error) => ClientSecretAuditOutcome::Failed((*error).failure_class()),
    };
    publish(audit, key, trace, action, outcome)?;
    result
}

fn publish<A: ClientSecretAuditSink>(
    audit: &mut A,
    key: &ClientSecretKey,
    trace: OAuthTraceId,
    action: ClientSecretAuditAction,
    outcome: ClientSecretAuditOutcome,
) -> Result<(), ClientSecretCustodyError> {
    audit
        .publish(&ClientSecretAuditEvent {
            key: key.clone(),
            trace,
            action,
            outcome,
        })
        .map_err(|_| ClientSecretCustodyError::Audit)
}

impl ClientSecretCustodyError {
    const fn failure_class(self) -> ClientSecretFailureClass {
        match self {
            Self::InvalidInput => ClientSecretFailureClass::InvalidInput,
            Self::NotFound => ClientSecretFailureClass::NotFound,
            Self::Conflict => ClientSecretFailureClass::Conflict,
            Self::Corruption => ClientSecretFailureClass::Corruption,
            Self::Backend => ClientSecretFailureClass::Backend,
            Self::Audit => unreachable!(),
        }
    }
}

fn map_store_error(error: ClientSecretStoreError) -> ClientSecretCustodyError {
    match error {
        ClientSecretStoreError::Conflict => ClientSecretCustodyError::Conflict,
        ClientSecretStoreError::Corruption => ClientSecretCustodyError::Corruption,
        ClientSecretStoreError::Backend => ClientSecretCustodyError::Backend,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Audit {
        events: Vec<ClientSecretAuditEvent>,
        fail_at: Option<usize>,
    }

    impl ClientSecretAuditSink for Audit {
        fn publish(
            &mut self,
            event: &ClientSecretAuditEvent,
        ) -> Result<(), ClientSecretAuditError> {
            if self.fail_at == Some(self.events.len()) {
                return Err(ClientSecretAuditError);
            }
            self.events.push(event.clone());
            Ok(())
        }
    }

    fn key() -> ClientSecretKey {
        ClientSecretKey::new(
            ProviderId::new("provider-a").expect("valid provider"),
            ClientSecretReference::new([0x51; 32]),
        )
    }

    fn trace() -> OAuthTraceId {
        OAuthTraceId::new([0x27; 16])
    }

    fn secret(value: &str) -> Zeroizing<String> {
        Zeroizing::new(value.to_string())
    }

    #[test]
    fn create_access_rotate_delete_are_audit_bracketed() {
        let custody = ClientSecretCustody::new(InMemoryClientSecretStore::new());
        let key = key();
        let mut audit = Audit::default();
        let first = custody
            .create(&key, secret("initial-secret"), trace(), &mut audit)
            .expect("create");
        let observed = custody
            .with_secret(&key, trace(), &mut audit, |value, revision| {
                (value.to_owned(), revision)
            })
            .expect("access");
        assert_eq!(observed.0, "initial-secret");
        assert_eq!(observed.1, first);
        let second = custody
            .rotate(&key, first, secret("rotated-secret"), trace(), &mut audit)
            .expect("rotate");
        custody
            .delete(&key, second, trace(), &mut audit)
            .expect("delete");

        assert_eq!(audit.events.len(), 8);
        for pair in audit.events.as_chunks::<2>().0 {
            assert_eq!(pair[0].outcome(), ClientSecretAuditOutcome::Attempted);
            assert_eq!(pair[1].outcome(), ClientSecretAuditOutcome::Succeeded);
            assert_eq!(pair[0].key().provider().as_str(), "provider-a");
            assert_eq!(pair[0].trace(), trace());
        }
    }

    #[test]
    fn pre_access_audit_failure_prevents_storage_access() {
        let custody = ClientSecretCustody::new(InMemoryClientSecretStore::new());
        let key = key();
        let mut create_audit = Audit::default();
        custody
            .create(&key, secret("secret"), trace(), &mut create_audit)
            .expect("create");
        let mut audit = Audit {
            events: Vec::new(),
            fail_at: Some(0),
        };
        let mut disclosed = false;
        let result = custody.with_secret(&key, trace(), &mut audit, |_, _| disclosed = true);
        assert_eq!(result, Err(ClientSecretCustodyError::Audit));
        assert!(!disclosed);
    }

    #[test]
    fn result_audit_failure_withholds_secret_disclosure() {
        let custody = ClientSecretCustody::new(InMemoryClientSecretStore::new());
        let key = key();
        let mut create_audit = Audit::default();
        custody
            .create(&key, secret("secret"), trace(), &mut create_audit)
            .expect("create");
        let mut audit = Audit {
            events: Vec::new(),
            fail_at: Some(1),
        };
        let mut disclosed = false;
        let result = custody.with_secret(&key, trace(), &mut audit, |_, _| disclosed = true);
        assert_eq!(result, Err(ClientSecretCustodyError::Audit));
        assert!(!disclosed);
    }

    #[test]
    fn invalid_missing_duplicate_and_stale_operations_fail_closed() {
        let custody = ClientSecretCustody::new(InMemoryClientSecretStore::new());
        let key = key();
        let mut audit = Audit::default();
        assert_eq!(
            custody.create(&key, secret(""), trace(), &mut audit),
            Err(ClientSecretCustodyError::InvalidInput)
        );
        assert_eq!(
            custody.with_secret(&key, trace(), &mut audit, |_, _| ()),
            Err(ClientSecretCustodyError::NotFound)
        );
        let revision = custody
            .create(&key, secret("secret"), trace(), &mut audit)
            .expect("create");
        assert_eq!(
            custody.create(&key, secret("duplicate"), trace(), &mut audit),
            Err(ClientSecretCustodyError::Conflict)
        );
        assert_eq!(
            custody.rotate(
                &key,
                ClientSecretRevision::new([0; 32]),
                secret("stale"),
                trace(),
                &mut audit,
            ),
            Err(ClientSecretCustodyError::Conflict)
        );
        assert_eq!(
            custody.delete(
                &key,
                ClientSecretRevision::new([0; 32]),
                trace(),
                &mut audit,
            ),
            Err(ClientSecretCustodyError::Conflict)
        );
        custody
            .delete(&key, revision, trace(), &mut audit)
            .expect("delete current");
    }

    #[test]
    fn debug_and_errors_never_include_secret_or_reference_bytes() {
        let value = ClientSecret::new(secret("super-secret")).expect("valid");
        assert_eq!(format!("{value:?}"), "ClientSecret(<redacted>)");
        assert_eq!(
            format!("{:?}", ClientSecretReference::new([0xff; 32])),
            "ClientSecretReference(<redacted>)"
        );
        assert_eq!(
            ClientSecretCustodyError::Backend.to_string(),
            "oauth client-secret custody: Backend"
        );
    }
}
