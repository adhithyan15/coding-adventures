//! Encrypted OAuth credential storage over `vault-sealed-store`.
//!
//! Provider behavior remains data. This adapter maps a validated provider and
//! opaque account key to a sealed record, while `CredentialCustody` remains the
//! mandatory audit-before-access/edit boundary.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_oauth_credential_custody::{
    CredentialKey, CredentialMetadata, CredentialRecord, CredentialRevision, CredentialStore,
    CredentialStoreError, StoredCredential,
};
use coding_adventures_sha256::sha256;
use coding_adventures_vault_sealed_store::{SealedStore, SealedStoreError};
use coding_adventures_zeroize::Zeroizing;
use std::fmt::{self, Debug, Formatter};
use storage_core::{Revision, StorageError};

const NAMESPACE: &str = "oauth-credentials";
const MAGIC: &[u8; 8] = b"OAUTHCRD";
const VERSION: u8 = 1;
const MAX_TOKEN_BYTES: usize = 64 * 1024;
const MAX_TOKEN_TYPE_BYTES: usize = 64;
const MAX_SCOPES: usize = 64;
const MAX_SCOPE_BYTES: usize = 256;
const MAX_ENVELOPE_BYTES: usize = MAGIC.len()
    + 1
    + (4 + MAX_TOKEN_BYTES) * 3
    + 2
    + 4
    + MAX_TOKEN_TYPE_BYTES
    + 1
    + 8
    + 2
    + MAX_SCOPES * (4 + MAX_SCOPE_BYTES);

/// Concrete encrypted implementation of the OAuth `CredentialStore` contract.
pub struct SealedCredentialStore {
    sealed: SealedStore,
}

impl SealedCredentialStore {
    /// Bind the adapter to an initialized, unsealed store.
    pub const fn new(sealed: SealedStore) -> Self {
        Self { sealed }
    }

    fn current_revision(
        &self,
        storage_key: &str,
        expected: CredentialRevision,
    ) -> Result<Revision, CredentialStoreError> {
        let current = self
            .sealed
            .get(NAMESPACE, storage_key)
            .map_err(map_read_error)?
            .ok_or(CredentialStoreError::Conflict)?;
        if credential_revision(&current.revision) != expected {
            return Err(CredentialStoreError::Conflict);
        }
        Ok(current.revision)
    }
}

impl Debug for SealedCredentialStore {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("SealedCredentialStore(<redacted>)")
    }
}

impl CredentialStore for SealedCredentialStore {
    fn create(
        &self,
        key: &CredentialKey,
        record: CredentialRecord,
    ) -> Result<CredentialRevision, CredentialStoreError> {
        let storage_key = storage_key(key);
        let envelope = encode(&record)?;
        self.sealed
            .put_if_absent(NAMESPACE, &storage_key, &envelope)
            .map(|revision| credential_revision(&revision))
            .map_err(map_write_error)
    }

    fn load(&self, key: &CredentialKey) -> Result<Option<StoredCredential>, CredentialStoreError> {
        let storage_key = storage_key(key);
        let Some(sealed) = self
            .sealed
            .get(NAMESPACE, &storage_key)
            .map_err(map_read_error)?
        else {
            return Ok(None);
        };
        let revision = credential_revision(&sealed.revision);
        let record = decode(&sealed.plaintext)?;
        Ok(Some(StoredCredential::new(revision, record)))
    }

    fn compare_and_swap(
        &self,
        key: &CredentialKey,
        expected: CredentialRevision,
        replacement: CredentialRecord,
    ) -> Result<CredentialRevision, CredentialStoreError> {
        let storage_key = storage_key(key);
        let current = self.current_revision(&storage_key, expected)?;
        let envelope = encode(&replacement)?;
        self.sealed
            .put(NAMESPACE, &storage_key, &envelope, Some(current))
            .map(|revision| credential_revision(&revision))
            .map_err(map_write_error)
    }

    fn delete(
        &self,
        key: &CredentialKey,
        expected: CredentialRevision,
    ) -> Result<(), CredentialStoreError> {
        let storage_key = storage_key(key);
        let current = self.current_revision(&storage_key, expected)?;
        self.sealed
            .delete(NAMESPACE, &storage_key, Some(current))
            .map_err(map_write_error)
    }
}

fn storage_key(key: &CredentialKey) -> String {
    let mut encoded = String::with_capacity(key.provider().as_str().len() + 1 + 64);
    encoded.push_str(key.provider().as_str());
    encoded.push('/');
    for byte in key.account().as_bytes() {
        use std::fmt::Write;
        write!(encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    encoded
}

fn credential_revision(revision: &Revision) -> CredentialRevision {
    CredentialRevision::new(sha256(revision.as_str().as_bytes()))
}

fn encode(record: &CredentialRecord) -> Result<Zeroizing<Vec<u8>>, CredentialStoreError> {
    // Reserve the full accepted envelope up front. A growing `Vec` may leave
    // secret-bearing bytes in a freed allocation when it reallocates; the
    // final zeroizing drop cannot reach such an old allocation.
    let mut output = Zeroizing::new(Vec::with_capacity(MAX_ENVELOPE_BYTES));
    output.extend_from_slice(MAGIC);
    output.push(VERSION);
    write_string(
        &mut output,
        record.access_token_for_storage(),
        MAX_TOKEN_BYTES,
    )?;
    write_optional_string(
        &mut output,
        record.refresh_token_for_storage(),
        MAX_TOKEN_BYTES,
    )?;
    write_optional_string(&mut output, record.id_token_for_storage(), MAX_TOKEN_BYTES)?;
    write_string(
        &mut output,
        record.metadata().token_type(),
        MAX_TOKEN_TYPE_BYTES,
    )?;
    match record.metadata().expires_at_unix_seconds() {
        Some(expiry) => {
            output.push(1);
            output.extend_from_slice(&expiry.to_be_bytes());
        }
        None => output.push(0),
    }
    let scopes = record.metadata().scopes();
    let count = u16::try_from(scopes.len()).map_err(|_| CredentialStoreError::Corruption)?;
    output.extend_from_slice(&count.to_be_bytes());
    for scope in scopes {
        write_string(&mut output, scope, MAX_SCOPE_BYTES)?;
    }
    if output.len() > MAX_ENVELOPE_BYTES {
        return Err(CredentialStoreError::Corruption);
    }
    Ok(output)
}

fn write_optional_string(
    output: &mut Vec<u8>,
    value: Option<&str>,
    limit: usize,
) -> Result<(), CredentialStoreError> {
    match value {
        Some(value) => {
            output.push(1);
            write_string(output, value, limit)
        }
        None => {
            output.push(0);
            Ok(())
        }
    }
}

fn write_string(
    output: &mut Vec<u8>,
    value: &str,
    limit: usize,
) -> Result<(), CredentialStoreError> {
    if value.len() > limit {
        return Err(CredentialStoreError::Corruption);
    }
    let length = u32::try_from(value.len()).map_err(|_| CredentialStoreError::Corruption)?;
    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(value.as_bytes());
    Ok(())
}

fn decode(input: &[u8]) -> Result<CredentialRecord, CredentialStoreError> {
    if input.len() > MAX_ENVELOPE_BYTES {
        return Err(CredentialStoreError::Corruption);
    }
    let mut reader = Reader::new(input);
    if reader.take(MAGIC.len())? != MAGIC || reader.byte()? != VERSION {
        return Err(CredentialStoreError::Corruption);
    }
    let access_token = Zeroizing::new(reader.string(MAX_TOKEN_BYTES)?);
    let refresh_token = reader.optional_string(MAX_TOKEN_BYTES)?.map(Zeroizing::new);
    let id_token = reader.optional_string(MAX_TOKEN_BYTES)?.map(Zeroizing::new);
    let token_type = reader.string(MAX_TOKEN_TYPE_BYTES)?;
    let expires_at_unix_seconds = match reader.byte()? {
        0 => None,
        1 => Some(reader.u64()?),
        _ => return Err(CredentialStoreError::Corruption),
    };
    let scope_count = usize::from(reader.u16()?);
    if scope_count > MAX_SCOPES {
        return Err(CredentialStoreError::Corruption);
    }
    let mut scopes = Vec::with_capacity(scope_count);
    for _ in 0..scope_count {
        scopes.push(reader.string(MAX_SCOPE_BYTES)?);
    }
    if !reader.is_empty() {
        return Err(CredentialStoreError::Corruption);
    }
    let metadata = CredentialMetadata::new(token_type, expires_at_unix_seconds, scopes)
        .map_err(|_| CredentialStoreError::Corruption)?;
    CredentialRecord::restore(access_token, refresh_token, id_token, metadata)
        .map_err(|_| CredentialStoreError::Corruption)
}

struct Reader<'a> {
    remaining: &'a [u8],
}

impl<'a> Reader<'a> {
    const fn new(input: &'a [u8]) -> Self {
        Self { remaining: input }
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], CredentialStoreError> {
        if length > self.remaining.len() {
            return Err(CredentialStoreError::Corruption);
        }
        let (value, remaining) = self.remaining.split_at(length);
        self.remaining = remaining;
        Ok(value)
    }

    fn byte(&mut self) -> Result<u8, CredentialStoreError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, CredentialStoreError> {
        let bytes: [u8; 2] = self
            .take(2)?
            .try_into()
            .map_err(|_| CredentialStoreError::Corruption)?;
        Ok(u16::from_be_bytes(bytes))
    }

    fn u32(&mut self) -> Result<u32, CredentialStoreError> {
        let bytes: [u8; 4] = self
            .take(4)?
            .try_into()
            .map_err(|_| CredentialStoreError::Corruption)?;
        Ok(u32::from_be_bytes(bytes))
    }

    fn u64(&mut self) -> Result<u64, CredentialStoreError> {
        let bytes: [u8; 8] = self
            .take(8)?
            .try_into()
            .map_err(|_| CredentialStoreError::Corruption)?;
        Ok(u64::from_be_bytes(bytes))
    }

    fn string(&mut self, limit: usize) -> Result<String, CredentialStoreError> {
        let length = usize::try_from(self.u32()?).map_err(|_| CredentialStoreError::Corruption)?;
        if length > limit {
            return Err(CredentialStoreError::Corruption);
        }
        String::from_utf8(self.take(length)?.to_vec()).map_err(|_| CredentialStoreError::Corruption)
    }

    fn optional_string(&mut self, limit: usize) -> Result<Option<String>, CredentialStoreError> {
        match self.byte()? {
            0 => Ok(None),
            1 => self.string(limit).map(Some),
            _ => Err(CredentialStoreError::Corruption),
        }
    }

    const fn is_empty(&self) -> bool {
        self.remaining.is_empty()
    }
}

fn map_read_error(error: SealedStoreError) -> CredentialStoreError {
    match error {
        SealedStoreError::Tamper { .. }
        | SealedStoreError::Validation { .. }
        | SealedStoreError::Crypto(_) => CredentialStoreError::Corruption,
        _ => CredentialStoreError::Backend,
    }
}

fn map_write_error(error: SealedStoreError) -> CredentialStoreError {
    match error {
        SealedStoreError::Storage(StorageError::Conflict { .. }) => CredentialStoreError::Conflict,
        _ => CredentialStoreError::Backend,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_oauth::{OAuthTraceId, ProviderId};
    use coding_adventures_oauth_credential_custody::{
        AccountId, CredentialAuditError, CredentialAuditEvent, CredentialAuditOutcome,
        CredentialAuditSink, CredentialCustody,
    };
    use std::sync::Arc;
    use storage_core::InMemoryStorageBackend;

    fn key() -> CredentialKey {
        CredentialKey::new(
            ProviderId::new("example-provider").expect("valid provider"),
            AccountId::new([0x5a; 32]),
        )
    }

    fn record(access: &str, refresh: Option<&str>) -> CredentialRecord {
        CredentialRecord::restore(
            Zeroizing::new(access.to_string()),
            refresh.map(|token| Zeroizing::new(token.to_string())),
            Some(Zeroizing::new("opaque-id-token".to_string())),
            CredentialMetadata::new(
                "Bearer",
                Some(1_900_000_000),
                vec!["mail.read".to_string(), "mail.send".to_string()],
            )
            .expect("valid metadata"),
        )
        .expect("valid credential")
    }

    fn stores() -> (SealedStore, SealedCredentialStore) {
        let backend = Arc::new(InMemoryStorageBackend::new());
        let sealed = SealedStore::new(backend.clone());
        sealed.init_with_kek(&[0x42; 32]).expect("initialize");
        let adapter_sealed = SealedStore::new(backend);
        adapter_sealed
            .unseal_with_kek(&[0x42; 32])
            .expect("unseal adapter");
        (sealed, SealedCredentialStore::new(adapter_sealed))
    }

    #[derive(Default)]
    struct Audit(Vec<CredentialAuditEvent>);

    impl CredentialAuditSink for Audit {
        fn publish(&mut self, event: &CredentialAuditEvent) -> Result<(), CredentialAuditError> {
            self.0.push(event.clone());
            Ok(())
        }
    }

    #[test]
    fn encrypted_round_trip_is_released_only_after_custody_audit() {
        let (_, adapter) = stores();
        let key = key();
        let revision = adapter
            .create(&key, record("access-secret", Some("refresh-secret")))
            .expect("create");
        let custody = CredentialCustody::new(adapter);
        let trace = OAuthTraceId::new([0x11; 16]);
        let mut audit = Audit::default();

        let access = custody
            .with_access_token(&key, trace, &mut audit, str::to_owned)
            .expect("read access token");
        let refresh = custody
            .with_refresh_token(&key, trace, &mut audit, |token, loaded_revision| {
                (token.to_owned(), loaded_revision)
            })
            .expect("read refresh token");

        assert_eq!(access, "access-secret");
        assert_eq!(refresh.0, "refresh-secret");
        assert_eq!(refresh.1, revision);
        assert_eq!(audit.0.len(), 4);
        assert!(audit.0.iter().all(
            |event| event.key().provider().as_str() == "example-provider" && event.trace() == trace
        ));
        assert_eq!(audit.0[0].outcome(), CredentialAuditOutcome::Attempted);
        assert_eq!(audit.0[1].outcome(), CredentialAuditOutcome::Succeeded);
    }

    #[test]
    fn create_and_compare_and_swap_preserve_atomic_revision_semantics() {
        let (_, adapter) = stores();
        let key = key();
        let first = adapter
            .create(&key, record("first", Some("refresh")))
            .expect("create");
        assert_eq!(
            adapter.create(&key, record("duplicate", None)),
            Err(CredentialStoreError::Conflict)
        );

        let stale = CredentialRevision::new([0; 32]);
        assert_eq!(
            adapter.compare_and_swap(&key, stale, record("stale", None)),
            Err(CredentialStoreError::Conflict)
        );
        let second = adapter
            .compare_and_swap(&key, first, record("second", Some("rotated")))
            .expect("replace");
        assert_ne!(first, second);
        assert_eq!(
            adapter.load(&key).expect("load").unwrap().revision(),
            second
        );
    }

    #[test]
    fn delete_requires_the_current_revision() {
        let (_, adapter) = stores();
        let key = key();
        let revision = adapter
            .create(&key, record("access", None))
            .expect("create");
        assert_eq!(
            adapter.delete(&key, CredentialRevision::new([0; 32])),
            Err(CredentialStoreError::Conflict)
        );
        adapter.delete(&key, revision).expect("delete");
        assert!(adapter.load(&key).expect("load").is_none());
        assert_eq!(
            adapter.delete(&key, revision),
            Err(CredentialStoreError::Conflict)
        );
    }

    #[test]
    fn malformed_decrypted_envelope_is_closed_corruption() {
        let (sealed, adapter) = stores();
        let key = key();
        sealed
            .put(
                NAMESPACE,
                &storage_key(&key),
                b"not-an-oauth-envelope",
                None,
            )
            .expect("write malformed encrypted record");
        assert!(matches!(
            adapter.load(&key),
            Err(CredentialStoreError::Corruption)
        ));
    }

    #[test]
    fn key_uses_only_provider_and_opaque_account_bytes() {
        assert_eq!(
            storage_key(&key()),
            format!("example-provider/{}", "5a".repeat(32))
        );
    }

    #[test]
    fn maximum_valid_envelope_fits_the_single_zeroizing_allocation() {
        let token = "x".repeat(MAX_TOKEN_BYTES);
        let scopes = (0..MAX_SCOPES)
            .map(|index| format!("scope-{index:02}{}", "x".repeat(MAX_SCOPE_BYTES - 8)))
            .collect();
        let record = CredentialRecord::restore(
            Zeroizing::new(token.clone()),
            Some(Zeroizing::new(token.clone())),
            Some(Zeroizing::new(token)),
            CredentialMetadata::new("T".repeat(MAX_TOKEN_TYPE_BYTES), Some(u64::MAX), scopes)
                .expect("maximum metadata is valid"),
        )
        .expect("maximum credential is valid");

        let envelope = encode(&record).expect("encode maximum credential");
        assert_eq!(envelope.len(), MAX_ENVELOPE_BYTES);
        assert_eq!(envelope.capacity(), MAX_ENVELOPE_BYTES);
        assert!(decode(&envelope).is_ok());
    }
}
