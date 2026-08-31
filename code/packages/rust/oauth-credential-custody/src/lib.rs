//! Audit-first storage-agnostic OAuth credential custody.
//!
//! This crate is the secret-bearing boundary between the pure OAuth codecs and
//! a trusted encrypted store. It has no provider branches and owns no concrete
//! storage, network, browser, or clock authority.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_oauth::{OAuthTraceId, ProviderId, RefreshTokenUpdate, TokenCredentials};
use coding_adventures_zeroize::Zeroizing;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Debug, Display, Formatter};
use std::sync::Mutex;

const MAX_TOKEN_BYTES: usize = 64 * 1024;
const MAX_TOKEN_TYPE_BYTES: usize = 64;
const MAX_SCOPES: usize = 64;
const MAX_SCOPE_BYTES: usize = 256;

/// Opaque stable account identity derived only after trusted identity proof.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AccountId([u8; 32]);

impl AccountId {
    /// Construct an opaque account identity from exact caller-owned bytes.
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Borrow exact bytes for a trusted store's lossless key encoding.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl Debug for AccountId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("AccountId(<redacted>)")
    }
}

/// Provider and opaque account tuple used for every custody operation.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CredentialKey {
    provider: ProviderId,
    account: AccountId,
}

impl CredentialKey {
    /// Bind one validated provider to one opaque account.
    pub const fn new(provider: ProviderId, account: AccountId) -> Self {
        Self { provider, account }
    }

    /// Return the stable provider identity.
    pub const fn provider(&self) -> &ProviderId {
        &self.provider
    }

    /// Return the opaque account identity.
    pub const fn account(&self) -> AccountId {
        self.account
    }
}

impl Debug for CredentialKey {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CredentialKey")
            .field("provider", &self.provider)
            .field("account", &self.account)
            .finish()
    }
}

/// Opaque compare-and-swap revision issued by the injected store.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CredentialRevision([u8; 32]);

impl CredentialRevision {
    /// Restore an exact backend-issued revision.
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Borrow exact bytes for the trusted backend.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl Debug for CredentialRevision {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("CredentialRevision(<redacted>)")
    }
}

/// Bounded public metadata stored atomically beside credential bytes.
#[derive(Clone, PartialEq, Eq)]
pub struct CredentialMetadata {
    token_type: String,
    expires_at_unix_seconds: Option<u64>,
    scopes: Vec<String>,
}

impl CredentialMetadata {
    /// Validate provider response metadata after caller-owned clock conversion.
    pub fn new(
        token_type: impl Into<String>,
        expires_at_unix_seconds: Option<u64>,
        scopes: Vec<String>,
    ) -> Result<Self, CustodyError> {
        let token_type = token_type.into();
        if token_type.is_empty()
            || token_type.len() > MAX_TOKEN_TYPE_BYTES
            || !token_type.bytes().all(|byte| matches!(byte, 0x21..=0x7e))
            || scopes.len() > MAX_SCOPES
            || {
                let mut unique = BTreeSet::new();
                scopes.iter().any(|scope| {
                    scope.is_empty()
                        || scope.len() > MAX_SCOPE_BYTES
                        || !scope.bytes().all(valid_scope_byte)
                        || !unique.insert(scope.as_str())
                })
            }
        {
            return Err(CustodyError::InvalidInput);
        }
        Ok(Self {
            token_type,
            expires_at_unix_seconds,
            scopes,
        })
    }

    /// Return the declared token type.
    pub fn token_type(&self) -> &str {
        &self.token_type
    }

    /// Return the caller-converted absolute expiry, when supplied.
    pub const fn expires_at_unix_seconds(&self) -> Option<u64> {
        self.expires_at_unix_seconds
    }

    /// Return the exact bounded granted scopes.
    pub fn scopes(&self) -> &[String] {
        &self.scopes
    }
}

impl Debug for CredentialMetadata {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CredentialMetadata")
            .field("token_type", &self.token_type)
            .field("expires_at_unix_seconds", &self.expires_at_unix_seconds)
            .field("scope_count", &self.scopes.len())
            .finish()
    }
}

/// Complete zeroizing credential state passed only to a trusted store adapter.
pub struct CredentialRecord {
    access_token: Zeroizing<String>,
    refresh_token: Option<Zeroizing<String>>,
    id_token: Option<Zeroizing<String>>,
    metadata: CredentialMetadata,
}

impl CredentialRecord {
    /// Restore a decrypted record inside a trusted store adapter.
    pub fn restore(
        access_token: Zeroizing<String>,
        refresh_token: Option<Zeroizing<String>>,
        id_token: Option<Zeroizing<String>>,
        metadata: CredentialMetadata,
    ) -> Result<Self, CustodyError> {
        validate_token(&access_token)?;
        if let Some(token) = &refresh_token {
            validate_token(token)?;
        }
        if let Some(token) = &id_token {
            validate_token(token)?;
        }
        Ok(Self {
            access_token,
            refresh_token,
            id_token,
            metadata,
        })
    }

    /// Borrow the access token inside the trusted store boundary.
    pub fn access_token_for_storage(&self) -> &str {
        self.access_token.as_str()
    }

    /// Borrow the refresh token inside the trusted store boundary.
    pub fn refresh_token_for_storage(&self) -> Option<&str> {
        self.refresh_token.as_ref().map(|token| token.as_str())
    }

    /// Borrow the untrusted ID token inside the trusted store boundary.
    pub fn id_token_for_storage(&self) -> Option<&str> {
        self.id_token.as_ref().map(|token| token.as_str())
    }

    /// Borrow public metadata inside the trusted store boundary.
    pub const fn metadata(&self) -> &CredentialMetadata {
        &self.metadata
    }

    fn initial(
        credentials: TokenCredentials,
        metadata: CredentialMetadata,
    ) -> Result<Self, CustodyError> {
        let (access_token, refresh_update, id_token) = credentials.into_parts();
        let refresh_token = match refresh_update {
            RefreshTokenUpdate::Absent => None,
            RefreshTokenUpdate::Rotate(token) => Some(token),
            RefreshTokenUpdate::RetainExisting => return Err(CustodyError::InvalidInput),
        };
        Self::restore(access_token, refresh_token, id_token, metadata)
    }

    fn rotated(
        current: Self,
        credentials: TokenCredentials,
        metadata: CredentialMetadata,
    ) -> Result<Self, CustodyError> {
        let (access_token, refresh_update, id_update) = credentials.into_parts();
        let refresh_token = match refresh_update {
            RefreshTokenUpdate::RetainExisting => current.refresh_token,
            RefreshTokenUpdate::Rotate(token) => Some(token),
            RefreshTokenUpdate::Absent => return Err(CustodyError::InvalidInput),
        };
        let id_token = id_update.or(current.id_token);
        Self::restore(access_token, refresh_token, id_token, metadata)
    }

    fn duplicate_for_store_read(&self) -> Self {
        Self {
            access_token: Zeroizing::new(self.access_token.as_str().to_owned()),
            refresh_token: self
                .refresh_token
                .as_ref()
                .map(|token| Zeroizing::new(token.as_str().to_owned())),
            id_token: self
                .id_token
                .as_ref()
                .map(|token| Zeroizing::new(token.as_str().to_owned())),
            metadata: self.metadata.clone(),
        }
    }
}

impl Debug for CredentialRecord {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CredentialRecord")
            .field("access_token", &"<redacted>")
            .field(
                "refresh_token",
                &self.refresh_token.as_ref().map(|_| "<redacted>"),
            )
            .field("id_token", &self.id_token.as_ref().map(|_| "<redacted>"))
            .field("metadata", &self.metadata)
            .finish()
    }
}

/// One decrypted store read bound to its compare-and-swap revision.
pub struct StoredCredential {
    revision: CredentialRevision,
    record: CredentialRecord,
}

impl StoredCredential {
    /// Construct a validated store result inside an authorized adapter.
    pub const fn new(revision: CredentialRevision, record: CredentialRecord) -> Self {
        Self { revision, record }
    }

    /// Return the opaque backend revision.
    pub const fn revision(&self) -> CredentialRevision {
        self.revision
    }
}

impl Debug for StoredCredential {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StoredCredential")
            .field("revision", &self.revision)
            .field("record", &self.record)
            .finish()
    }
}

/// Closed trusted-store failure without backend diagnostics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CredentialStoreError {
    /// A create or compare-and-swap condition failed.
    Conflict,
    /// Encrypted or decoded provider state was corrupt.
    Corruption,
    /// The trusted backend failed or was unavailable.
    Backend,
}

/// Minimal atomic storage contract injected into credential custody.
pub trait CredentialStore: Send + Sync {
    /// Create a record only if its key is absent.
    fn create(
        &self,
        key: &CredentialKey,
        record: CredentialRecord,
    ) -> Result<CredentialRevision, CredentialStoreError>;

    /// Load and decrypt one exact record.
    fn load(&self, key: &CredentialKey) -> Result<Option<StoredCredential>, CredentialStoreError>;

    /// Atomically replace a record only at `expected`.
    fn compare_and_swap(
        &self,
        key: &CredentialKey,
        expected: CredentialRevision,
        replacement: CredentialRecord,
    ) -> Result<CredentialRevision, CredentialStoreError>;

    /// Atomically delete a record only at `expected`.
    fn delete(
        &self,
        key: &CredentialKey,
        expected: CredentialRevision,
    ) -> Result<(), CredentialStoreError>;
}

/// Credential operation recorded durably before any result or secret release.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CredentialAuditAction {
    /// Create initial credential state.
    Create,
    /// Disclose an access token to one authorized closure.
    AccessToken,
    /// Disclose a refresh token and revision to one authorized closure.
    RefreshToken,
    /// Atomically rotate credential state.
    Rotate,
    /// Conditionally delete credential state.
    Delete,
}

/// Closed privacy-safe audit result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CredentialAuditOutcome {
    /// Durable intent before storage access.
    Attempted,
    /// Durable success before result or secret release.
    Succeeded,
    /// Closed failure classification.
    Failed(CustodyFailureClass),
}

/// Closed failure class safe for durable audit storage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CustodyFailureClass {
    /// Caller data or credential shape was invalid.
    InvalidInput,
    /// The requested credential was absent.
    NotFound,
    /// A create or rotation lost an atomic race.
    Conflict,
    /// Stored credential material was corrupt.
    Corruption,
    /// The injected backend failed.
    Backend,
    /// The account has no refresh token.
    RefreshUnavailable,
}

/// Privacy-safe durable event containing no token or provider-controlled text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CredentialAuditEvent {
    key: CredentialKey,
    trace: OAuthTraceId,
    action: CredentialAuditAction,
    outcome: CredentialAuditOutcome,
}

impl CredentialAuditEvent {
    /// Return the exact provider/account key.
    pub const fn key(&self) -> &CredentialKey {
        &self.key
    }

    /// Return the caller-owned correlation trace.
    pub const fn trace(&self) -> OAuthTraceId {
        self.trace
    }

    /// Return the closed operation.
    pub const fn action(&self) -> CredentialAuditAction {
        self.action
    }

    /// Return the closed outcome.
    pub const fn outcome(&self) -> CredentialAuditOutcome {
        self.outcome
    }
}

/// Durable audit publication required before every custody effect and release.
pub trait CredentialAuditSink {
    /// Persist `event` durably or fail closed.
    fn publish(&mut self, event: &CredentialAuditEvent) -> Result<(), CredentialAuditError>;
}

/// Closed durable-audit failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CredentialAuditError;

/// Closed custody error with no credential or backend diagnostics.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CustodyError {
    /// Caller input or credential shape was invalid.
    InvalidInput,
    /// The requested account credential does not exist.
    NotFound,
    /// An atomic storage condition failed.
    Conflict,
    /// Stored credential state was corrupt.
    Corruption,
    /// The trusted backend failed.
    Backend,
    /// No refresh token is available for this account.
    RefreshUnavailable,
    /// Durable audit publication failed and the result was withheld.
    Audit,
}

impl CustodyError {
    fn failure_class(self) -> Option<CustodyFailureClass> {
        match self {
            Self::InvalidInput => Some(CustodyFailureClass::InvalidInput),
            Self::NotFound => Some(CustodyFailureClass::NotFound),
            Self::Conflict => Some(CustodyFailureClass::Conflict),
            Self::Corruption => Some(CustodyFailureClass::Corruption),
            Self::Backend => Some(CustodyFailureClass::Backend),
            Self::RefreshUnavailable => Some(CustodyFailureClass::RefreshUnavailable),
            Self::Audit => None,
        }
    }
}

impl Debug for CustodyError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidInput => "InvalidInput",
            Self::NotFound => "NotFound",
            Self::Conflict => "Conflict",
            Self::Corruption => "Corruption",
            Self::Backend => "Backend",
            Self::RefreshUnavailable => "RefreshUnavailable",
            Self::Audit => "Audit",
        })
    }
}

impl Display for CustodyError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("oauth credential custody: ")?;
        Debug::fmt(self, formatter)
    }
}

impl std::error::Error for CustodyError {}

/// Audit-gated credential operations over one injected trusted store.
pub struct CredentialCustody<S: CredentialStore> {
    store: S,
}

impl<S: CredentialStore> CredentialCustody<S> {
    /// Construct custody over one trusted store authority.
    pub const fn new(store: S) -> Self {
        Self { store }
    }

    /// Audit and create initial credential state if absent.
    pub fn create<A: CredentialAuditSink>(
        &self,
        key: &CredentialKey,
        credentials: TokenCredentials,
        metadata: CredentialMetadata,
        trace: OAuthTraceId,
        audit: &mut A,
    ) -> Result<CredentialRevision, CustodyError> {
        attempt(audit, key, trace, CredentialAuditAction::Create)?;
        let result = CredentialRecord::initial(credentials, metadata)
            .and_then(|record| self.store.create(key, record).map_err(map_store_error));
        finish(audit, key, trace, CredentialAuditAction::Create, result)
    }

    /// Audit, load, then disclose an access token to exactly one closure.
    pub fn with_access_token<R, A: CredentialAuditSink>(
        &self,
        key: &CredentialKey,
        trace: OAuthTraceId,
        audit: &mut A,
        use_token: impl FnOnce(&str) -> R,
    ) -> Result<R, CustodyError> {
        attempt(audit, key, trace, CredentialAuditAction::AccessToken)?;
        let loaded = match self.store.load(key).map_err(map_store_error) {
            Ok(Some(loaded)) => loaded,
            Ok(None) => {
                return finish(
                    audit,
                    key,
                    trace,
                    CredentialAuditAction::AccessToken,
                    Err(CustodyError::NotFound),
                )
            }
            Err(error) => {
                return finish(
                    audit,
                    key,
                    trace,
                    CredentialAuditAction::AccessToken,
                    Err(error),
                )
            }
        };
        publish(
            audit,
            key,
            trace,
            CredentialAuditAction::AccessToken,
            CredentialAuditOutcome::Succeeded,
        )?;
        Ok(use_token(loaded.record.access_token.as_str()))
    }

    /// Audit, load, then disclose a refresh token and exact revision once.
    pub fn with_refresh_token<R, A: CredentialAuditSink>(
        &self,
        key: &CredentialKey,
        trace: OAuthTraceId,
        audit: &mut A,
        use_token: impl FnOnce(&str, CredentialRevision) -> R,
    ) -> Result<R, CustodyError> {
        attempt(audit, key, trace, CredentialAuditAction::RefreshToken)?;
        let loaded = match self.store.load(key).map_err(map_store_error) {
            Ok(Some(loaded)) => loaded,
            Ok(None) => {
                return finish(
                    audit,
                    key,
                    trace,
                    CredentialAuditAction::RefreshToken,
                    Err(CustodyError::NotFound),
                )
            }
            Err(error) => {
                return finish(
                    audit,
                    key,
                    trace,
                    CredentialAuditAction::RefreshToken,
                    Err(error),
                )
            }
        };
        let Some(refresh_token) = loaded.record.refresh_token.as_ref() else {
            return finish(
                audit,
                key,
                trace,
                CredentialAuditAction::RefreshToken,
                Err(CustodyError::RefreshUnavailable),
            );
        };
        publish(
            audit,
            key,
            trace,
            CredentialAuditAction::RefreshToken,
            CredentialAuditOutcome::Succeeded,
        )?;
        Ok(use_token(refresh_token.as_str(), loaded.revision))
    }

    /// Audit and atomically rotate credential state at `expected`.
    pub fn rotate<A: CredentialAuditSink>(
        &self,
        key: &CredentialKey,
        expected: CredentialRevision,
        credentials: TokenCredentials,
        metadata: CredentialMetadata,
        trace: OAuthTraceId,
        audit: &mut A,
    ) -> Result<CredentialRevision, CustodyError> {
        attempt(audit, key, trace, CredentialAuditAction::Rotate)?;
        let result = (|| {
            let current = self
                .store
                .load(key)
                .map_err(map_store_error)?
                .ok_or(CustodyError::NotFound)?;
            if current.revision != expected {
                return Err(CustodyError::Conflict);
            }
            let replacement = CredentialRecord::rotated(current.record, credentials, metadata)?;
            self.store
                .compare_and_swap(key, expected, replacement)
                .map_err(map_store_error)
        })();
        finish(audit, key, trace, CredentialAuditAction::Rotate, result)
    }

    /// Audit and conditionally delete exact credential state.
    pub fn delete<A: CredentialAuditSink>(
        &self,
        key: &CredentialKey,
        expected: CredentialRevision,
        trace: OAuthTraceId,
        audit: &mut A,
    ) -> Result<(), CustodyError> {
        attempt(audit, key, trace, CredentialAuditAction::Delete)?;
        let result = self.store.delete(key, expected).map_err(map_store_error);
        finish(audit, key, trace, CredentialAuditAction::Delete, result)
    }
}

impl<S: CredentialStore> Debug for CredentialCustody<S> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CredentialCustody")
            .field("store", &"<redacted>")
            .finish()
    }
}

/// Deterministic thread-safe reference store for tests and local composition.
#[derive(Default)]
pub struct InMemoryCredentialStore {
    state: Mutex<MemoryState>,
}

#[derive(Default)]
struct MemoryState {
    next_revision: u64,
    records: BTreeMap<CredentialKey, (CredentialRevision, CredentialRecord)>,
}

impl InMemoryCredentialStore {
    /// Construct an empty in-memory store.
    pub const fn new() -> Self {
        Self {
            state: Mutex::new(MemoryState {
                next_revision: 0,
                records: BTreeMap::new(),
            }),
        }
    }
}

impl Debug for InMemoryCredentialStore {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("InMemoryCredentialStore(<redacted>)")
    }
}

impl CredentialStore for InMemoryCredentialStore {
    fn create(
        &self,
        key: &CredentialKey,
        record: CredentialRecord,
    ) -> Result<CredentialRevision, CredentialStoreError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| CredentialStoreError::Backend)?;
        if state.records.contains_key(key) {
            return Err(CredentialStoreError::Conflict);
        }
        let revision = next_revision(&mut state)?;
        state.records.insert(key.clone(), (revision, record));
        Ok(revision)
    }

    fn load(&self, key: &CredentialKey) -> Result<Option<StoredCredential>, CredentialStoreError> {
        let state = self
            .state
            .lock()
            .map_err(|_| CredentialStoreError::Backend)?;
        Ok(state.records.get(key).map(|(revision, record)| {
            StoredCredential::new(*revision, record.duplicate_for_store_read())
        }))
    }

    fn compare_and_swap(
        &self,
        key: &CredentialKey,
        expected: CredentialRevision,
        replacement: CredentialRecord,
    ) -> Result<CredentialRevision, CredentialStoreError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| CredentialStoreError::Backend)?;
        match state.records.get(key) {
            Some((revision, _)) if *revision == expected => {}
            _ => return Err(CredentialStoreError::Conflict),
        }
        let revision = next_revision(&mut state)?;
        state.records.insert(key.clone(), (revision, replacement));
        Ok(revision)
    }

    fn delete(
        &self,
        key: &CredentialKey,
        expected: CredentialRevision,
    ) -> Result<(), CredentialStoreError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| CredentialStoreError::Backend)?;
        match state.records.get(key) {
            Some((revision, _)) if *revision == expected => {
                state.records.remove(key);
                Ok(())
            }
            _ => Err(CredentialStoreError::Conflict),
        }
    }
}

fn next_revision(state: &mut MemoryState) -> Result<CredentialRevision, CredentialStoreError> {
    state.next_revision = state
        .next_revision
        .checked_add(1)
        .ok_or(CredentialStoreError::Backend)?;
    let mut bytes = [0_u8; 32];
    bytes[24..].copy_from_slice(&state.next_revision.to_be_bytes());
    Ok(CredentialRevision::new(bytes))
}

fn validate_token(token: &str) -> Result<(), CustodyError> {
    if token.is_empty() || token.len() > MAX_TOKEN_BYTES || token.chars().any(char::is_control) {
        Err(CustodyError::InvalidInput)
    } else {
        Ok(())
    }
}

fn valid_scope_byte(byte: u8) -> bool {
    matches!(byte, 0x21 | 0x23..=0x5b | 0x5d..=0x7e)
}

fn map_store_error(error: CredentialStoreError) -> CustodyError {
    match error {
        CredentialStoreError::Conflict => CustodyError::Conflict,
        CredentialStoreError::Corruption => CustodyError::Corruption,
        CredentialStoreError::Backend => CustodyError::Backend,
    }
}

fn attempt<A: CredentialAuditSink>(
    audit: &mut A,
    key: &CredentialKey,
    trace: OAuthTraceId,
    action: CredentialAuditAction,
) -> Result<(), CustodyError> {
    publish(audit, key, trace, action, CredentialAuditOutcome::Attempted)
}

fn finish<T, A: CredentialAuditSink>(
    audit: &mut A,
    key: &CredentialKey,
    trace: OAuthTraceId,
    action: CredentialAuditAction,
    result: Result<T, CustodyError>,
) -> Result<T, CustodyError> {
    let outcome = match result.as_ref() {
        Ok(_) => CredentialAuditOutcome::Succeeded,
        Err(error) => match error.failure_class() {
            Some(class) => CredentialAuditOutcome::Failed(class),
            None => return Err(CustodyError::Audit),
        },
    };
    publish(audit, key, trace, action, outcome)?;
    result
}

fn publish<A: CredentialAuditSink>(
    audit: &mut A,
    key: &CredentialKey,
    trace: OAuthTraceId,
    action: CredentialAuditAction,
    outcome: CredentialAuditOutcome,
) -> Result<(), CustodyError> {
    audit
        .publish(&CredentialAuditEvent {
            key: key.clone(),
            trace,
            action,
            outcome,
        })
        .map_err(|_| CustodyError::Audit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_oauth::{
        decode_token_response, prepare_token_refresh, OAuthAuditError, OAuthAuditEvent,
        OAuthAuditSink, ProviderConfig, TokenResponseFormat,
    };
    use std::sync::{Arc, Mutex as StdMutex};

    #[derive(Default)]
    struct CoreAudit;

    impl OAuthAuditSink for CoreAudit {
        fn publish(&mut self, _event: &OAuthAuditEvent) -> Result<(), OAuthAuditError> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct RecordingAudit {
        events: Vec<CredentialAuditEvent>,
        calls: usize,
        fail_on_call: Option<usize>,
        timeline: Option<Arc<StdMutex<Vec<&'static str>>>>,
    }

    impl CredentialAuditSink for RecordingAudit {
        fn publish(&mut self, event: &CredentialAuditEvent) -> Result<(), CredentialAuditError> {
            self.calls += 1;
            if self.fail_on_call == Some(self.calls) {
                return Err(CredentialAuditError);
            }
            if let Some(timeline) = &self.timeline {
                timeline.lock().unwrap().push(match event.outcome() {
                    CredentialAuditOutcome::Attempted => "audit-attempted",
                    CredentialAuditOutcome::Succeeded => "audit-succeeded",
                    CredentialAuditOutcome::Failed(_) => "audit-failed",
                });
            }
            self.events.push(event.clone());
            Ok(())
        }
    }

    fn provider() -> ProviderId {
        ProviderId::new("fixture").unwrap()
    }

    fn key() -> CredentialKey {
        CredentialKey::new(provider(), AccountId::new([0x31; 32]))
    }

    fn trace() -> OAuthTraceId {
        OAuthTraceId::new([0x42; 16])
    }

    fn config() -> ProviderConfig {
        ProviderConfig::new(
            provider(),
            "https://authorize.example/auth",
            "https://token.example/token",
            "public-client",
            "http://127.0.0.1:49152/callback",
        )
        .unwrap()
    }

    fn credentials(
        access: &str,
        refresh: Option<&str>,
        id_token: Option<&str>,
    ) -> TokenCredentials {
        let request = prepare_token_refresh(
            &config(),
            Zeroizing::new("request-refresh".to_owned()),
            &[],
            trace(),
        )
        .publish_then_release(&mut CoreAudit)
        .unwrap();
        let mut fields = vec![
            format!("\"access_token\":\"{access}\""),
            "\"token_type\":\"Bearer\"".to_owned(),
            "\"expires_in\":3600".to_owned(),
        ];
        if let Some(refresh) = refresh {
            fields.push(format!("\"refresh_token\":\"{refresh}\""));
        }
        if let Some(id_token) = id_token {
            fields.push(format!("\"id_token\":\"{id_token}\""));
        }
        decode_token_response(
            request.response_context(),
            200,
            TokenResponseFormat::Json,
            Zeroizing::new(format!("{{{}}}", fields.join(",")).into_bytes()),
        )
        .publish_then_release(&mut CoreAudit)
        .unwrap()
        .release_credentials()
        .publish_then_release(&mut CoreAudit)
        .unwrap()
    }

    fn metadata(expiry: u64) -> CredentialMetadata {
        CredentialMetadata::new(
            "Bearer",
            Some(expiry),
            vec!["vault.read".to_owned(), "vault.write".to_owned()],
        )
        .unwrap()
    }

    fn create(custody: &CredentialCustody<InMemoryCredentialStore>) -> CredentialRevision {
        custody
            .create(
                &key(),
                credentials("access-one", Some("refresh-one"), Some("id-one")),
                metadata(1_000),
                trace(),
                &mut RecordingAudit::default(),
            )
            .unwrap()
    }

    #[test]
    fn create_is_audited_and_duplicate_is_closed() {
        let custody = CredentialCustody::new(InMemoryCredentialStore::new());
        let mut audit = RecordingAudit::default();
        let revision = custody
            .create(
                &key(),
                credentials("access-one", Some("refresh-one"), Some("id-one")),
                metadata(1_000),
                trace(),
                &mut audit,
            )
            .unwrap();
        assert_ne!(revision.as_bytes(), &[0_u8; 32]);
        assert_eq!(audit.events.len(), 2);
        assert_eq!(audit.events[0].key(), &key());
        assert_eq!(audit.events[0].trace(), trace());
        assert_eq!(audit.events[0].action(), CredentialAuditAction::Create);
        assert_eq!(audit.events[0].outcome(), CredentialAuditOutcome::Attempted);
        assert_eq!(audit.events[1].outcome(), CredentialAuditOutcome::Succeeded);
        assert_eq!(
            custody.create(
                &key(),
                credentials("other", Some("other-refresh"), None),
                metadata(2_000),
                trace(),
                &mut audit,
            ),
            Err(CustodyError::Conflict)
        );
        assert_eq!(
            audit.events[3].outcome(),
            CredentialAuditOutcome::Failed(CustodyFailureClass::Conflict)
        );
    }

    #[test]
    fn pre_effect_audit_failure_prevents_create() {
        let custody = CredentialCustody::new(InMemoryCredentialStore::new());
        let mut failed = RecordingAudit {
            fail_on_call: Some(1),
            ..RecordingAudit::default()
        };
        assert_eq!(
            custody.create(
                &key(),
                credentials("access", Some("refresh"), None),
                metadata(1),
                trace(),
                &mut failed,
            ),
            Err(CustodyError::Audit)
        );
        assert!(custody
            .create(
                &key(),
                credentials("access", Some("refresh"), None),
                metadata(1),
                trace(),
                &mut RecordingAudit::default(),
            )
            .is_ok());
    }

    #[test]
    fn post_effect_audit_failure_withholds_revision_but_not_committed_state() {
        let custody = CredentialCustody::new(InMemoryCredentialStore::new());
        let mut failed = RecordingAudit {
            fail_on_call: Some(2),
            ..RecordingAudit::default()
        };
        assert_eq!(
            custody.create(
                &key(),
                credentials("access", Some("refresh"), None),
                metadata(1),
                trace(),
                &mut failed,
            ),
            Err(CustodyError::Audit)
        );
        assert_eq!(
            custody.create(
                &key(),
                credentials("other", Some("other-refresh"), None),
                metadata(2),
                trace(),
                &mut RecordingAudit::default(),
            ),
            Err(CustodyError::Conflict)
        );
    }

    #[test]
    fn access_token_is_disclosed_only_after_success_audit() {
        let custody = CredentialCustody::new(InMemoryCredentialStore::new());
        create(&custody);
        let timeline = Arc::new(StdMutex::new(Vec::new()));
        let mut audit = RecordingAudit {
            timeline: Some(Arc::clone(&timeline)),
            ..RecordingAudit::default()
        };
        let length = custody
            .with_access_token(&key(), trace(), &mut audit, |token| {
                timeline.lock().unwrap().push("token-use");
                assert_eq!(token, "access-one");
                token.len()
            })
            .unwrap();
        assert_eq!(length, 10);
        assert_eq!(
            *timeline.lock().unwrap(),
            ["audit-attempted", "audit-succeeded", "token-use"]
        );
    }

    #[test]
    fn result_audit_failure_prevents_token_disclosure() {
        let custody = CredentialCustody::new(InMemoryCredentialStore::new());
        create(&custody);
        let mut audit = RecordingAudit {
            fail_on_call: Some(2),
            ..RecordingAudit::default()
        };
        let called = Arc::new(StdMutex::new(false));
        let called_by_closure = Arc::clone(&called);
        assert_eq!(
            custody.with_access_token(&key(), trace(), &mut audit, move |_| {
                *called_by_closure.lock().unwrap() = true;
            }),
            Err(CustodyError::Audit)
        );
        assert!(!*called.lock().unwrap());
    }

    #[test]
    fn refresh_revision_drives_atomic_retain_and_rotation() {
        let custody = CredentialCustody::new(InMemoryCredentialStore::new());
        let initial = create(&custody);
        let observed = custody
            .with_refresh_token(
                &key(),
                trace(),
                &mut RecordingAudit::default(),
                |token, revision| {
                    assert_eq!(token, "refresh-one");
                    revision
                },
            )
            .unwrap();
        assert_eq!(observed, initial);

        let retained = custody
            .rotate(
                &key(),
                observed,
                credentials("access-two", None, None),
                metadata(2_000),
                trace(),
                &mut RecordingAudit::default(),
            )
            .unwrap();
        custody
            .with_access_token(&key(), trace(), &mut RecordingAudit::default(), |token| {
                assert_eq!(token, "access-two")
            })
            .unwrap();
        custody
            .with_refresh_token(
                &key(),
                trace(),
                &mut RecordingAudit::default(),
                |token, revision| {
                    assert_eq!(token, "refresh-one");
                    assert_eq!(revision, retained);
                },
            )
            .unwrap();

        assert_eq!(
            custody.rotate(
                &key(),
                observed,
                credentials("stale", Some("stale-refresh"), None),
                metadata(3_000),
                trace(),
                &mut RecordingAudit::default(),
            ),
            Err(CustodyError::Conflict)
        );
        let rotated = custody
            .rotate(
                &key(),
                retained,
                credentials("access-three", Some("refresh-three"), Some("id-three")),
                metadata(3_000),
                trace(),
                &mut RecordingAudit::default(),
            )
            .unwrap();
        custody
            .with_refresh_token(
                &key(),
                trace(),
                &mut RecordingAudit::default(),
                |token, revision| {
                    assert_eq!(token, "refresh-three");
                    assert_eq!(revision, rotated);
                },
            )
            .unwrap();
    }

    #[test]
    fn conditional_delete_is_audited_and_stale_revision_fails() {
        let custody = CredentialCustody::new(InMemoryCredentialStore::new());
        let revision = create(&custody);
        let stale = CredentialRevision::new([0x99; 32]);
        let mut audit = RecordingAudit::default();
        assert_eq!(
            custody.delete(&key(), stale, trace(), &mut audit),
            Err(CustodyError::Conflict)
        );
        custody
            .delete(&key(), revision, trace(), &mut audit)
            .unwrap();
        assert_eq!(
            custody.with_access_token(&key(), trace(), &mut audit, |_| ()),
            Err(CustodyError::NotFound)
        );
    }

    #[test]
    fn metadata_and_initial_rotation_shapes_fail_closed() {
        assert_eq!(
            CredentialMetadata::new("", None, Vec::new()),
            Err(CustodyError::InvalidInput)
        );
        assert_eq!(
            CredentialMetadata::new("Bearer", None, vec!["bad scope".to_owned()]),
            Err(CustodyError::InvalidInput)
        );
        assert_eq!(
            CredentialMetadata::new(
                "Bearer",
                None,
                vec!["duplicate".to_owned(), "duplicate".to_owned()]
            ),
            Err(CustodyError::InvalidInput)
        );
        let custody = CredentialCustody::new(InMemoryCredentialStore::new());
        assert_eq!(
            custody.create(
                &key(),
                credentials("access", None, None),
                metadata(1),
                trace(),
                &mut RecordingAudit::default(),
            ),
            Err(CustodyError::InvalidInput)
        );
    }

    #[test]
    fn debug_and_audit_surfaces_are_secret_free() {
        let record = CredentialRecord::restore(
            Zeroizing::new("top-secret-access".to_owned()),
            Some(Zeroizing::new("top-secret-refresh".to_owned())),
            Some(Zeroizing::new("top-secret-id".to_owned())),
            metadata(1),
        )
        .unwrap();
        let event = CredentialAuditEvent {
            key: key(),
            trace: trace(),
            action: CredentialAuditAction::AccessToken,
            outcome: CredentialAuditOutcome::Succeeded,
        };
        let debug = format!("{record:?} {event:?} {:?}", CustodyError::Backend);
        assert!(!debug.contains("top-secret"));
        assert!(!debug.contains("vault.read"));
        assert!(debug.contains("<redacted>"));
    }
}
