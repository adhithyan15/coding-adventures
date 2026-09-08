//! Audit-first, storage-agnostic OAuth client-secret custody.
//!
//! Provider configuration retains only opaque references. Secret bytes are
//! zeroizing and cross the boundary only into one audited closure.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_base64::{encode_into as encode_base64_into, STANDARD};
use coding_adventures_oauth::{
    OAuthTraceId, ProviderConfig, ProviderId, TokenExchangeRequest, TokenRefreshRequest,
    TokenResponseContext, TokenRevocationRequest,
};
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

/// Password-based client authentication selected entirely by provider data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClientSecretAuthenticationMethod {
    /// RFC 6749 HTTP Basic authentication (the preferred password method).
    ClientSecretBasic,
    /// RFC 6749 `client_id` and `client_secret` request-body parameters.
    ClientSecretPost,
}

/// Provider-bound client authentication without raw secret material.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientSecretAuthentication {
    key: ClientSecretKey,
    method: ClientSecretAuthenticationMethod,
}

impl ClientSecretAuthentication {
    /// Bind a provider/opaque-reference key to one wire authentication method.
    pub const fn new(key: ClientSecretKey, method: ClientSecretAuthenticationMethod) -> Self {
        Self { key, method }
    }

    /// Return the provider and opaque reference used for custody access.
    pub const fn key(&self) -> &ClientSecretKey {
        &self.key
    }

    /// Return the provider-selected wire authentication method.
    pub const fn method(&self) -> ClientSecretAuthenticationMethod {
        self.method
    }
}

/// Secret-bearing HTTP request material released only after custody audit.
pub struct ClientSecretAuthenticatedRequest {
    provider: ProviderId,
    trace: OAuthTraceId,
    endpoint: String,
    form_body: Zeroizing<String>,
    authorization_header: Option<Zeroizing<String>>,
}

impl ClientSecretAuthenticatedRequest {
    /// Return the provider identity used for routing and external-effect audit.
    pub const fn provider(&self) -> &ProviderId {
        &self.provider
    }

    /// Return the correlation identity used for custody and transport audit.
    pub const fn trace(&self) -> OAuthTraceId {
        self.trace
    }

    /// Borrow the validated HTTPS endpoint.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Borrow the wipe-on-drop form body for an authorized transport.
    pub fn form_body(&self) -> &str {
        self.form_body.as_str()
    }

    /// Borrow the wipe-on-drop Authorization value when Basic was selected.
    pub fn authorization_header(&self) -> Option<&str> {
        self.authorization_header
            .as_ref()
            .map(|header| header.as_str())
    }

    /// Return the exact request media type.
    pub const fn content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }
}

impl Debug for ClientSecretAuthenticatedRequest {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ClientSecretAuthenticatedRequest")
            .field("provider", &self.provider)
            .field("trace", &self.trace)
            .field("endpoint", &"<redacted>")
            .field("form_body", &"<redacted>")
            .field(
                "has_authorization_header",
                &self.authorization_header.is_some(),
            )
            .finish()
    }
}

/// Authenticated authorization-code exchange plus its response binding.
pub struct ClientSecretAuthenticatedTokenExchange {
    request: ClientSecretAuthenticatedRequest,
    response_context: TokenResponseContext,
}

impl ClientSecretAuthenticatedTokenExchange {
    /// Borrow the authenticated wire request.
    pub const fn request(&self) -> &ClientSecretAuthenticatedRequest {
        &self.request
    }

    /// Borrow the exact provider/trace response binding.
    pub const fn response_context(&self) -> &TokenResponseContext {
        &self.response_context
    }
}

impl Debug for ClientSecretAuthenticatedTokenExchange {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ClientSecretAuthenticatedTokenExchange")
            .field("request", &self.request)
            .field("response_context", &self.response_context)
            .finish()
    }
}

/// Authenticated refresh grant plus its response binding.
pub struct ClientSecretAuthenticatedTokenRefresh {
    request: ClientSecretAuthenticatedRequest,
    response_context: TokenResponseContext,
}

impl ClientSecretAuthenticatedTokenRefresh {
    /// Borrow the authenticated wire request.
    pub const fn request(&self) -> &ClientSecretAuthenticatedRequest {
        &self.request
    }

    /// Borrow the exact provider/trace response binding.
    pub const fn response_context(&self) -> &TokenResponseContext {
        &self.response_context
    }
}

impl Debug for ClientSecretAuthenticatedTokenRefresh {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ClientSecretAuthenticatedTokenRefresh")
            .field("request", &self.request)
            .field("response_context", &self.response_context)
            .finish()
    }
}

/// Authenticated RFC 7009 revocation request.
pub struct ClientSecretAuthenticatedTokenRevocation {
    request: ClientSecretAuthenticatedRequest,
}

impl ClientSecretAuthenticatedTokenRevocation {
    /// Borrow the authenticated wire request.
    pub const fn request(&self) -> &ClientSecretAuthenticatedRequest {
        &self.request
    }
}

impl Debug for ClientSecretAuthenticatedTokenRevocation {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ClientSecretAuthenticatedTokenRevocation")
            .field("request", &self.request)
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

    /// Authenticate an audited authorization-code exchange through custody.
    pub fn authenticate_token_exchange<A: ClientSecretAuditSink>(
        &self,
        config: &ProviderConfig,
        authentication: &ClientSecretAuthentication,
        request: TokenExchangeRequest,
        audit: &mut A,
    ) -> Result<ClientSecretAuthenticatedTokenExchange, ClientSecretCustodyError> {
        let response_context = request.response_context();
        let authenticated = self.authenticate_request(
            config,
            authentication,
            request.provider(),
            request.trace(),
            request.client_id(),
            request.endpoint(),
            request.form_body(),
            audit,
        )?;
        Ok(ClientSecretAuthenticatedTokenExchange {
            request: authenticated,
            response_context,
        })
    }

    /// Authenticate an audited refresh grant through custody.
    pub fn authenticate_token_refresh<A: ClientSecretAuditSink>(
        &self,
        config: &ProviderConfig,
        authentication: &ClientSecretAuthentication,
        request: TokenRefreshRequest,
        audit: &mut A,
    ) -> Result<ClientSecretAuthenticatedTokenRefresh, ClientSecretCustodyError> {
        let response_context = request.response_context();
        let authenticated = self.authenticate_request(
            config,
            authentication,
            request.provider(),
            request.trace(),
            request.client_id(),
            request.endpoint(),
            request.form_body(),
            audit,
        )?;
        Ok(ClientSecretAuthenticatedTokenRefresh {
            request: authenticated,
            response_context,
        })
    }

    /// Authenticate an audited RFC 7009 revocation through custody.
    pub fn authenticate_token_revocation<A: ClientSecretAuditSink>(
        &self,
        config: &ProviderConfig,
        authentication: &ClientSecretAuthentication,
        request: TokenRevocationRequest,
        audit: &mut A,
    ) -> Result<ClientSecretAuthenticatedTokenRevocation, ClientSecretCustodyError> {
        let authenticated = self.authenticate_request(
            config,
            authentication,
            request.provider(),
            request.trace(),
            request.client_id(),
            request.endpoint(),
            request.form_body(),
            audit,
        )?;
        Ok(ClientSecretAuthenticatedTokenRevocation {
            request: authenticated,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn authenticate_request<A: ClientSecretAuditSink>(
        &self,
        config: &ProviderConfig,
        authentication: &ClientSecretAuthentication,
        provider: &ProviderId,
        trace: OAuthTraceId,
        request_client_id: &str,
        endpoint: &str,
        form_body: &str,
        audit: &mut A,
    ) -> Result<ClientSecretAuthenticatedRequest, ClientSecretCustodyError> {
        if config.provider() != provider
            || config.client_id() != request_client_id
            || authentication.key().provider() != provider
        {
            return Err(ClientSecretCustodyError::InvalidInput);
        }
        self.with_secret(authentication.key(), trace, audit, |secret, _revision| {
            build_authenticated_request(
                provider.clone(),
                trace,
                endpoint.to_owned(),
                form_body,
                request_client_id,
                secret,
                authentication.method(),
            )
        })
    }
}

impl<S: ClientSecretStore> Debug for ClientSecretCustody<S> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("ClientSecretCustody(<redacted>)")
    }
}

fn build_authenticated_request(
    provider: ProviderId,
    trace: OAuthTraceId,
    endpoint: String,
    form_body: &str,
    client_id: &str,
    client_secret: &str,
    method: ClientSecretAuthenticationMethod,
) -> ClientSecretAuthenticatedRequest {
    let (form_body, authorization_header) = match method {
        ClientSecretAuthenticationMethod::ClientSecretBasic => {
            let mut credentials = Zeroizing::new(form_encode(client_id));
            credentials.push(':');
            append_form_encoded(&mut credentials, client_secret);
            let mut header = Zeroizing::new(String::from("Basic "));
            encode_base64_into(credentials.as_bytes(), &STANDARD, &mut header);
            (form_without_client_id(form_body), Some(header))
        }
        ClientSecretAuthenticationMethod::ClientSecretPost => {
            let mut body = Zeroizing::new(form_body.to_owned());
            body.push_str("&client_secret=");
            append_form_encoded(&mut body, client_secret);
            (body, None)
        }
    };
    ClientSecretAuthenticatedRequest {
        provider,
        trace,
        endpoint,
        form_body,
        authorization_header,
    }
}

fn form_without_client_id(form_body: &str) -> Zeroizing<String> {
    let mut output = Zeroizing::new(String::with_capacity(form_body.len()));
    for parameter in form_body
        .split('&')
        .filter(|parameter| !parameter.starts_with("client_id="))
    {
        if !output.is_empty() {
            output.push('&');
        }
        output.push_str(parameter);
    }
    output
}

fn form_encode(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    append_form_encoded(&mut output, value);
    output
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
    use coding_adventures_oauth::{
        begin_authorization, complete_authorization, prepare_token_refresh,
        prepare_token_revocation, Audited, EntropySource, OAuthAuditError, OAuthAuditEvent,
        OAuthAuditSink, OAuthError, RevocationTokenHint,
    };

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

    fn config() -> ProviderConfig {
        ProviderConfig::new(
            ProviderId::new("provider-a").expect("valid provider"),
            "https://authorize.example/oauth2/auth",
            "https://token.example/oauth2/token",
            "client id/plus+",
            "http://127.0.0.1:53682/callback",
        )
        .expect("valid config")
        .with_distinct_redirect_uri()
        .with_revocation_endpoint("https://token.example/oauth2/revoke")
        .expect("valid revocation endpoint")
    }

    fn release<T>(audited: Audited<T>) -> T {
        audited
            .publish_then_release(&mut OAuthAudit)
            .expect("oauth audit and preparation")
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

    #[test]
    fn basic_authenticates_exchange_without_duplicate_body_identity() {
        let custody = ClientSecretCustody::new(InMemoryClientSecretStore::new());
        let key = key();
        let mut audit = Audit::default();
        custody
            .create(&key, secret("s e:c/r+et"), trace(), &mut audit)
            .expect("create");
        audit.events.clear();

        let config = config();
        let begin = release(begin_authorization(
            &config,
            &["files.read"],
            trace(),
            &mut FixedEntropy([0x29; 64]),
        ));
        let state = begin
            .url()
            .as_str()
            .split('&')
            .find_map(|parameter| parameter.strip_prefix("state="))
            .expect("state")
            .to_owned();
        let (_, transaction) = begin.into_parts();
        let exchange = release(complete_authorization(
            transaction,
            &format!("http://127.0.0.1:53682/callback?code=code-value&state={state}"),
        ));
        let authentication = ClientSecretAuthentication::new(
            key,
            ClientSecretAuthenticationMethod::ClientSecretBasic,
        );
        let authenticated = custody
            .authenticate_token_exchange(&config, &authentication, exchange, &mut audit)
            .expect("authenticate exchange");

        assert_eq!(authenticated.request().provider().as_str(), "provider-a");
        assert_eq!(authenticated.request().trace(), trace());
        assert_eq!(
            authenticated.request().authorization_header(),
            Some("Basic Y2xpZW50K2lkJTJGcGx1cyUyQjpzK2UlM0FjJTJGciUyQmV0")
        );
        assert!(!authenticated.request().form_body().contains("client_id="));
        assert!(!authenticated
            .request()
            .form_body()
            .contains("client_secret="));
        assert_eq!(authenticated.response_context().trace(), trace());
        assert_eq!(audit.events.len(), 2);
        assert_eq!(
            audit.events[0].outcome(),
            ClientSecretAuditOutcome::Attempted
        );
        assert_eq!(
            audit.events[1].outcome(),
            ClientSecretAuditOutcome::Succeeded
        );
    }

    #[test]
    fn post_authenticates_refresh_and_revocation_in_the_form_body() {
        let custody = ClientSecretCustody::new(InMemoryClientSecretStore::new());
        let key = key();
        let mut audit = Audit::default();
        custody
            .create(&key, secret("s e:c/r+et"), trace(), &mut audit)
            .expect("create");
        audit.events.clear();
        let config = config();
        let authentication = ClientSecretAuthentication::new(
            key,
            ClientSecretAuthenticationMethod::ClientSecretPost,
        );

        let refresh = release(prepare_token_refresh(
            &config,
            secret("refresh-token"),
            &["files.read"],
            trace(),
        ));
        let refresh = custody
            .authenticate_token_refresh(&config, &authentication, refresh, &mut audit)
            .expect("authenticate refresh");
        assert_eq!(refresh.request().authorization_header(), None);
        assert!(refresh
            .request()
            .form_body()
            .contains("client_id=client%20id%2Fplus%2B"));
        assert!(refresh
            .request()
            .form_body()
            .ends_with("client_secret=s+e%3Ac%2Fr%2Bet"));
        assert_eq!(refresh.response_context().trace(), trace());

        let revocation = release(prepare_token_revocation(
            &config,
            secret("access-token"),
            RevocationTokenHint::AccessToken,
            trace(),
        ));
        let revocation = custody
            .authenticate_token_revocation(&config, &authentication, revocation, &mut audit)
            .expect("authenticate revocation");
        assert_eq!(revocation.request().authorization_header(), None);
        assert!(revocation
            .request()
            .form_body()
            .ends_with("client_secret=s+e%3Ac%2Fr%2Bet"));
        assert_eq!(audit.events.len(), 4);
        assert!(audit.events.iter().all(|event| event.trace() == trace()));
    }

    #[test]
    fn provider_or_client_mismatch_fails_before_secret_access() {
        let custody = ClientSecretCustody::new(InMemoryClientSecretStore::new());
        let config = config();
        let other_key = ClientSecretKey::new(
            ProviderId::new("provider-b").expect("provider"),
            ClientSecretReference::new([0x72; 32]),
        );
        let authentication = ClientSecretAuthentication::new(
            other_key,
            ClientSecretAuthenticationMethod::ClientSecretPost,
        );
        let refresh = release(prepare_token_refresh(
            &config,
            secret("refresh-token"),
            &[],
            trace(),
        ));
        let mut audit = Audit::default();

        let result =
            custody.authenticate_token_refresh(&config, &authentication, refresh, &mut audit);
        assert!(matches!(
            result,
            Err(ClientSecretCustodyError::InvalidInput)
        ));
        assert!(audit.events.is_empty());

        let authentication = ClientSecretAuthentication::new(
            key(),
            ClientSecretAuthenticationMethod::ClientSecretPost,
        );
        let wrong_client_config = ProviderConfig::new(
            ProviderId::new("provider-a").expect("provider"),
            "https://authorize.example/oauth2/auth",
            "https://token.example/oauth2/token",
            "another-client",
            "http://127.0.0.1:53682/callback",
        )
        .expect("config");
        let refresh = release(prepare_token_refresh(
            &config,
            secret("refresh-token"),
            &[],
            trace(),
        ));
        let result = custody.authenticate_token_refresh(
            &wrong_client_config,
            &authentication,
            refresh,
            &mut audit,
        );
        assert!(matches!(
            result,
            Err(ClientSecretCustodyError::InvalidInput)
        ));
        assert!(audit.events.is_empty());
    }

    #[test]
    fn authenticated_request_debug_redacts_all_wire_secrets() {
        let request = build_authenticated_request(
            ProviderId::new("provider-a").expect("provider"),
            trace(),
            "https://token.example/oauth2/token".to_owned(),
            "grant_type=refresh_token&refresh_token=raw-token&client_id=client",
            "client",
            "raw-secret",
            ClientSecretAuthenticationMethod::ClientSecretBasic,
        );
        let debug = format!("{request:?}");
        assert!(!debug.contains("raw-token"));
        assert!(!debug.contains("raw-secret"));
        assert!(!debug.contains("Basic "));
        assert!(debug.contains("has_authorization_header: true"));
    }
}
