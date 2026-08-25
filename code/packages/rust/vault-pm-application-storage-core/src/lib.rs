//! VLT-PM05 bootstrap and owner-state stores over an injected `storage-core` backend.
//!
//! The adapters retain the application's exact byte contracts while leaving
//! filesystem paths, provider SDKs, and platform permission policy to the host.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_json_value::JsonValue;
use coding_adventures_sha256::sha256;
use coding_adventures_vault_pm_application::{
    BootstrapLocator, BootstrapStore, BootstrapStoreError, LocalStateStore, LocalStateStoreError,
    LocalVaultStateV1,
};
use coding_adventures_vault_pm_format::{BootstrapId, BootstrapV1};
use core::fmt::{self, Debug, Formatter};
use std::collections::BTreeSet;
use std::sync::Mutex;
use storage_core::{
    StorageBackend, StorageError, StorageListOptions, StoragePutInput, StorageRecord,
};

const BOOTSTRAP_LATEST_NAMESPACE_DOMAIN: &[u8] =
    b"vault-pm/application-storage-core/bootstrap-latest/v1";
const BOOTSTRAP_GENERATION_NAMESPACE_DOMAIN: &[u8] =
    b"vault-pm/application-storage-core/bootstrap-generation/v1";
const LOCAL_STATE_NAMESPACE_DOMAIN: &[u8] = b"vault-pm/application-storage-core/local-state/v1";
const OPAQUE_CONTENT_TYPE: &str = "application/octet-stream";
const MAX_BOOTSTRAP_BYTES: usize = 1024 * 1024;
const MAX_LOCAL_STATE_BYTES: usize = 32 * 1024 * 1024;

/// VLT-PM05 bootstrap and owner-private local-state stores over one backend.
///
/// A host should use a backend instance dedicated to this adapter. The local
/// CLI will use a separately permission-checked root from the immutable object
/// repository so application state remains private and independently movable.
pub struct StorageCoreApplicationStore<B: StorageBackend> {
    backend: B,
    write_lock: Mutex<()>,
}

impl<B: StorageBackend> StorageCoreApplicationStore<B> {
    /// Construct both application stores over one injected backend.
    pub const fn new(backend: B) -> Self {
        Self {
            backend,
            write_lock: Mutex::new(()),
        }
    }

    /// Borrow the injected backend for host composition and diagnostics.
    pub const fn backend(&self) -> &B {
        &self.backend
    }

    /// Consume the adapter and return its injected backend.
    pub fn into_backend(self) -> B {
        self.backend
    }

    fn initialize_bootstrap(&self) -> Result<(), BootstrapStoreError> {
        self.backend.initialize().map_err(map_bootstrap_read)
    }

    fn initialize_local(&self) -> Result<(), LocalStateStoreError> {
        self.backend.initialize().map_err(map_local_read)
    }

    fn read_bootstrap_record(
        &self,
        namespace: &str,
        key: &str,
        maximum_body_bytes: usize,
    ) -> Result<Option<StorageRecord>, BootstrapStoreError> {
        let record = self
            .backend
            .get(namespace, key)
            .map_err(map_bootstrap_read)?;
        record
            .map(|record| {
                validate_record(&record, namespace, key, maximum_body_bytes)
                    .map_err(|()| BootstrapStoreError::Corruption)?;
                Ok(record)
            })
            .transpose()
    }

    fn read_generation(
        &self,
        locator: BootstrapLocator,
        id: BootstrapId,
    ) -> Result<(BootstrapV1, Vec<u8>), BootstrapStoreError> {
        let namespace = generation_namespace(locator);
        let key = hex_bytes(id.as_bytes());
        let record = self
            .read_bootstrap_record(&namespace, &key, MAX_BOOTSTRAP_BYTES)?
            .ok_or(BootstrapStoreError::Corruption)?;
        let bootstrap = decode_bootstrap(&record.body)?;
        if bootstrap
            .id()
            .map_err(|_| BootstrapStoreError::Corruption)?
            != id
        {
            return Err(BootstrapStoreError::Corruption);
        }
        Ok((bootstrap, record.body))
    }

    fn read_latest(
        &self,
        locator: BootstrapLocator,
    ) -> Result<Option<LatestBootstrap>, BootstrapStoreError> {
        let namespace = latest_namespace();
        let key = locator_key(locator);
        let Some(pointer) = self.read_bootstrap_record(&namespace, &key, 32)? else {
            return Ok(None);
        };
        let id = BootstrapId::new(
            pointer
                .body
                .as_slice()
                .try_into()
                .map_err(|_| BootstrapStoreError::Corruption)?,
        );
        let (bootstrap, bytes) = self.read_generation(locator, id)?;
        Ok(Some(LatestBootstrap {
            pointer,
            id,
            bootstrap,
            bytes,
        }))
    }

    fn install_generation(
        &self,
        locator: BootstrapLocator,
        id: BootstrapId,
        exact_bootstrap: &[u8],
    ) -> Result<(), BootstrapStoreError> {
        let namespace = generation_namespace(locator);
        let key = hex_bytes(id.as_bytes());
        let input = put_input(&namespace, &key, exact_bootstrap.to_vec())
            .map_err(|_| BootstrapStoreError::Corruption)?
            .with_if_absent();
        match self.backend.put(input) {
            Ok(record) => validate_record(&record, &namespace, &key, MAX_BOOTSTRAP_BYTES)
                .and_then(|()| (record.body == exact_bootstrap).then_some(()).ok_or(()))
                .map_err(|()| BootstrapStoreError::Corruption),
            Err(StorageError::Conflict { .. }) => {
                let existing = self
                    .read_bootstrap_record(&namespace, &key, MAX_BOOTSTRAP_BYTES)?
                    .ok_or(BootstrapStoreError::Corruption)?;
                if existing.body == exact_bootstrap {
                    Ok(())
                } else {
                    Err(BootstrapStoreError::Corruption)
                }
            }
            Err(error) => Err(map_bootstrap_write(error)),
        }
    }

    fn advance_latest(
        &self,
        locator: BootstrapLocator,
        current: Option<&StorageRecord>,
        intended: BootstrapId,
        exact_bootstrap: &[u8],
    ) -> Result<(), BootstrapStoreError> {
        let namespace = latest_namespace();
        let key = locator_key(locator);
        let input = put_input(&namespace, &key, intended.as_bytes().to_vec())
            .map_err(|_| BootstrapStoreError::Corruption)?;
        let input = match current {
            Some(record) => input.with_if_revision(Some(record.revision.clone())),
            None => input.with_if_absent(),
        };
        match self.backend.put(input) {
            Ok(record) => {
                validate_record(&record, &namespace, &key, 32)
                    .map_err(|()| BootstrapStoreError::Corruption)?;
                if record.body.as_slice() != intended.as_bytes() {
                    return Err(BootstrapStoreError::Corruption);
                }
            }
            Err(StorageError::Conflict { .. }) => {
                let winner = self
                    .read_latest(locator)?
                    .ok_or(BootstrapStoreError::Conflict)?;
                if winner.id == intended && winner.bytes == exact_bootstrap {
                    return Ok(());
                }
                return Err(BootstrapStoreError::Conflict);
            }
            Err(error) => return Err(map_bootstrap_write(error)),
        }
        let readback = self
            .read_latest(locator)?
            .ok_or(BootstrapStoreError::Corruption)?;
        if readback.id == intended && readback.bytes == exact_bootstrap {
            Ok(())
        } else {
            Err(BootstrapStoreError::Corruption)
        }
    }

    fn read_local_record(
        &self,
        locator: BootstrapLocator,
    ) -> Result<Option<StorageRecord>, LocalStateStoreError> {
        let namespace = local_namespace();
        let key = locator_key(locator);
        let record = self.backend.get(&namespace, &key).map_err(map_local_read)?;
        record
            .map(|record| {
                validate_record(&record, &namespace, &key, MAX_LOCAL_STATE_BYTES)
                    .and_then(|()| (!record.body.is_empty()).then_some(()).ok_or(()))
                    .map_err(|()| LocalStateStoreError::Corruption)?;
                Ok(record)
            })
            .transpose()
    }

    /// Reclaim `PreparedInit` journals stranded by a generation zero that
    /// crashed before its locator ever became discoverable in configuration.
    ///
    /// # The leak this closes
    ///
    /// `init` and `vault create` both draw a fresh random [`BootstrapLocator`]
    /// and durably install its `PreparedInit` journal — via
    /// [`LocalStateStore::compare_exchange`] — strictly *before* writing the
    /// configuration record that makes the locator discoverable again. That
    /// ordering is deliberate: VLT-PM41 §4.2 and §8 require that once
    /// configuration names a locator, a crash-resumable journal is already
    /// there to be found. The cost of choosing that ordering is the mirror
    /// case — a crash strictly between the two writes leaves the journal
    /// durable under a locator nothing will ever name again, since the value
    /// that would have named it lived only in the crashed process's memory.
    /// VLT-PM41 §8 records the finding; this closes it for the local,
    /// single-device case documented there.
    ///
    /// # Why `PreparedInit` is the only state ever touched
    ///
    /// A locator's owner-state record can only ever be found in a state other
    /// than `PreparedInit` — `Active`, `PendingPublication`, or
    /// `PendingRotation` — after the configuration write that names it has
    /// already durably succeeded, because every caller performs that
    /// configuration write before ever driving the journal past `PreparedInit`
    /// (see `complete_generation_zero`, called only after the configuration
    /// write returns). So a record found in one of those later states is
    /// never an orphan of *this* leak, no matter what `live_locators` says,
    /// and this method leaves it untouched unconditionally. That is the
    /// entire safety argument: the state check alone is sufficient, without
    /// needing `live_locators` to be perfectly complete.
    ///
    /// `live_locators` is nonetheless required and consulted first, as
    /// defense in depth and to make the call idempotent-safe against a
    /// `PreparedInit` a caller is *itself* still in the middle of installing
    /// via a locator it already generated but has not yet reached
    /// `compare_exchange` for — the caller's own resume/probe paths always
    /// pass the locator's owner-state or its containing configuration exactly
    /// as they are already reading it for other reasons, so they always name
    /// a locator whose `PreparedInit` is legitimately live before the check
    /// on state above would matter.
    ///
    /// # Concurrency
    ///
    /// Holds the same write lock every mutating method on this store holds.
    /// Every deletion is a compare-and-delete against the exact revision this
    /// call observed, so a record that changes between the list and the
    /// delete — which the write lock already forbids from this store, but
    /// costs nothing to guard against structurally — is left alone rather
    /// than torn out from under a concurrent write.
    ///
    /// Returns the number of reclaimed journals.
    pub fn reclaim_orphaned_preparations(
        &self,
        live_locators: &BTreeSet<BootstrapLocator>,
    ) -> Result<usize, LocalStateStoreError> {
        let _write = self
            .write_lock
            .lock()
            .map_err(|_| LocalStateStoreError::Unavailable)?;
        self.initialize_local()?;
        let namespace = local_namespace();
        let mut reclaimed = 0usize;
        let mut cursor = None;
        loop {
            let page = self
                .backend
                .list(
                    &namespace,
                    StorageListOptions {
                        prefix: None,
                        recursive: false,
                        page_size: None,
                        cursor: cursor.clone(),
                    },
                )
                .map_err(map_local_read)?;
            for record in &page.records {
                let Some(locator) = parse_locator_key(&record.key) else {
                    continue;
                };
                if live_locators.contains(&locator) {
                    continue;
                }
                if validate_record(record, &namespace, &record.key, MAX_LOCAL_STATE_BYTES).is_err()
                {
                    continue;
                }
                let Ok(LocalVaultStateV1::PreparedInit(_)) =
                    LocalVaultStateV1::decode(&record.body)
                else {
                    continue;
                };
                match self
                    .backend
                    .delete(&namespace, &record.key, Some(&record.revision))
                {
                    Ok(()) => reclaimed += 1,
                    Err(StorageError::Conflict { .. }) => {}
                    Err(error) => return Err(map_local_write(error)),
                }
            }
            cursor = page.next_cursor;
            if cursor.is_none() {
                break;
            }
        }
        Ok(reclaimed)
    }
}

impl<B: StorageBackend> Debug for StorageCoreApplicationStore<B> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StorageCoreApplicationStore")
            .field("backend", &"<redacted>")
            .finish()
    }
}

impl<B: StorageBackend> BootstrapStore for StorageCoreApplicationStore<B> {
    fn load_latest(
        &self,
        locator: BootstrapLocator,
    ) -> Result<Option<Vec<u8>>, BootstrapStoreError> {
        self.initialize_bootstrap()?;
        Ok(self.read_latest(locator)?.map(|latest| latest.bytes))
    }

    fn put_generation(
        &self,
        locator: BootstrapLocator,
        expected_previous: Option<BootstrapId>,
        exact_bootstrap: &[u8],
    ) -> Result<(), BootstrapStoreError> {
        let _write = self
            .write_lock
            .lock()
            .map_err(|_| BootstrapStoreError::Unavailable)?;
        self.initialize_bootstrap()?;
        let intended = decode_bootstrap(exact_bootstrap)?;
        let intended_id = intended.id().map_err(|_| BootstrapStoreError::Corruption)?;
        if intended.previous_bootstrap != expected_previous {
            return Err(BootstrapStoreError::Conflict);
        }

        let current = self.read_latest(locator)?;
        if let Some(current) = &current {
            if current.id == intended_id {
                return if current.bytes == exact_bootstrap {
                    Ok(())
                } else {
                    Err(BootstrapStoreError::Corruption)
                };
            }
            if expected_previous != Some(current.id)
                || intended.generation
                    != current
                        .bootstrap
                        .generation
                        .checked_add(1)
                        .ok_or(BootstrapStoreError::Corruption)?
                || intended.vault_id != current.bootstrap.vault_id
            {
                return Err(BootstrapStoreError::Conflict);
            }
        } else if expected_previous.is_some()
            || intended.generation != 0
            || intended.previous_bootstrap.is_some()
        {
            return Err(BootstrapStoreError::Conflict);
        }

        self.install_generation(locator, intended_id, exact_bootstrap)?;
        self.advance_latest(
            locator,
            current.as_ref().map(|latest| &latest.pointer),
            intended_id,
            exact_bootstrap,
        )
    }

    /// Remove one retired generation record, refusing to remove the live one.
    ///
    /// VLT-PM43 §5.4. Advancing the latest pointer is not enough to retire a
    /// passphrase: the superseded generation still holds a wrapping of the
    /// *same, unchanged* vault root key under the *old* passphrase-derived key,
    /// so anyone who later obtained a copy of this directory and the retired
    /// passphrase could open the vault through it. The delete is the part of a
    /// rotation that makes the rotation mean something.
    ///
    /// Two behaviours are load-bearing rather than incidental:
    ///
    /// - The latest generation is refused outright, with `Conflict`. That
    ///   record is the only way into the vault, and a guard is worth more than
    ///   a convention that every caller passes the right identifier.
    /// - An already-absent record is success, because the rotation's recovery
    ///   replays this call after a crash and must be able to reach the end.
    ///
    /// # Why the wrap is overwritten before it is unlinked
    ///
    /// Every other durable step in a rotation is a write, and a lost write is
    /// merely lost work: the journal replays it. This step is a *removal*, and
    /// a lost removal is the opposite — it resurrects key material that a
    /// successful rotation has already declared gone, into a vault whose owner
    /// state has moved on and will therefore never revisit it.
    ///
    /// Unlinks are weaker than they look. `fs::remove_file` returning `Ok` and
    /// a follow-up `get` returning `None` prove only that the removal is
    /// visible through the page cache; on a journalling filesystem the entry's
    /// disappearance can still be uncommitted while a later `fsync`ed write in
    /// another directory lands ahead of it. A power cut in that window would
    /// leave the retired `passphrase_root_wrap` — a wrapping of the *same,
    /// unchanged* vault root key under the *old* passphrase — readable forever.
    ///
    /// So the record's body is first replaced with nothing, through the very
    /// `put` path — write, `fsync`, `rename` — that makes every other step of
    /// the ceremony durable, and only then unlinked. After that write returns,
    /// the wrap is gone from the file whether or not the unlink survives.
    fn supersede_generation(
        &self,
        locator: BootstrapLocator,
        superseded: BootstrapId,
    ) -> Result<(), BootstrapStoreError> {
        let _write = self
            .write_lock
            .lock()
            .map_err(|_| BootstrapStoreError::Unavailable)?;
        self.initialize_bootstrap()?;
        if self
            .read_latest(locator)?
            .is_some_and(|latest| latest.id == superseded)
        {
            return Err(BootstrapStoreError::Conflict);
        }
        let namespace = generation_namespace(locator);
        let key = hex_bytes(superseded.as_bytes());

        if let Some(existing) = self
            .backend
            .get(&namespace, &key)
            .map_err(map_bootstrap_read)?
        {
            if !existing.body.is_empty() {
                let input = put_input(&namespace, &key, Vec::new())
                    .map_err(|_| BootstrapStoreError::Corruption)?
                    .with_if_revision(Some(existing.revision));
                self.backend.put(input).map_err(map_bootstrap_write)?;
            }
        }

        match self.backend.delete(&namespace, &key, None) {
            Ok(()) | Err(StorageError::NotFound { .. }) => {}
            Err(error) => return Err(map_bootstrap_write(error)),
        }
        match self.backend.get(&namespace, &key) {
            Ok(None) => Ok(()),
            Ok(Some(_)) => Err(BootstrapStoreError::Corruption),
            Err(error) => Err(map_bootstrap_read(error)),
        }
    }
}

impl<B: StorageBackend> LocalStateStore for StorageCoreApplicationStore<B> {
    fn load(&self, locator: BootstrapLocator) -> Result<Option<Vec<u8>>, LocalStateStoreError> {
        self.initialize_local()?;
        Ok(self.read_local_record(locator)?.map(|record| record.body))
    }

    fn compare_exchange(
        &self,
        locator: BootstrapLocator,
        expected: Option<&[u8]>,
        replacement: &[u8],
    ) -> Result<(), LocalStateStoreError> {
        let _write = self
            .write_lock
            .lock()
            .map_err(|_| LocalStateStoreError::Unavailable)?;
        self.initialize_local()?;
        validate_local_bytes(replacement)?;
        if let Some(expected) = expected {
            validate_local_bytes(expected)?;
        }
        let current = self.read_local_record(locator)?;
        match (expected, current.as_ref()) {
            (None, None) => {}
            (Some(expected), Some(record)) if record.body == expected => {}
            _ => return Err(LocalStateStoreError::ConcurrentHost),
        }

        let namespace = local_namespace();
        let key = locator_key(locator);
        let input = put_input(&namespace, &key, replacement.to_vec())
            .map_err(|_| LocalStateStoreError::Corruption)?;
        let input = match current {
            Some(record) => input.with_if_revision(Some(record.revision)),
            None => input.with_if_absent(),
        };
        let written = match self.backend.put(input) {
            Ok(record) => record,
            Err(StorageError::Conflict { .. }) => return Err(LocalStateStoreError::ConcurrentHost),
            Err(error) => return Err(map_local_write(error)),
        };
        validate_record(&written, &namespace, &key, MAX_LOCAL_STATE_BYTES)
            .map_err(|()| LocalStateStoreError::Corruption)?;
        if written.body != replacement {
            return Err(LocalStateStoreError::Corruption);
        }
        let readback = self
            .read_local_record(locator)?
            .ok_or(LocalStateStoreError::Corruption)?;
        if readback.body == replacement {
            Ok(())
        } else {
            Err(LocalStateStoreError::Corruption)
        }
    }
}

struct LatestBootstrap {
    pointer: StorageRecord,
    id: BootstrapId,
    bootstrap: BootstrapV1,
    bytes: Vec<u8>,
}

fn decode_bootstrap(bytes: &[u8]) -> Result<BootstrapV1, BootstrapStoreError> {
    if bytes.is_empty() || bytes.len() > MAX_BOOTSTRAP_BYTES {
        return Err(BootstrapStoreError::Corruption);
    }
    BootstrapV1::decode(bytes).map_err(|_| BootstrapStoreError::Corruption)
}

fn validate_local_bytes(bytes: &[u8]) -> Result<(), LocalStateStoreError> {
    if bytes.is_empty() || bytes.len() > MAX_LOCAL_STATE_BYTES {
        Err(LocalStateStoreError::Corruption)
    } else {
        Ok(())
    }
}

fn validate_record(
    record: &StorageRecord,
    namespace: &str,
    key: &str,
    maximum_body_bytes: usize,
) -> Result<(), ()> {
    if record.namespace != namespace
        || record.key != key
        || record.content_type != OPAQUE_CONTENT_TYPE
        || record.metadata != empty_metadata()
        || record.body.len() > maximum_body_bytes
    {
        Err(())
    } else {
        Ok(())
    }
}

fn put_input(namespace: &str, key: &str, body: Vec<u8>) -> Result<StoragePutInput, StorageError> {
    StoragePutInput::new(namespace, key, OPAQUE_CONTENT_TYPE, empty_metadata(), body)
}

fn latest_namespace() -> String {
    hex_bytes(&sha256(BOOTSTRAP_LATEST_NAMESPACE_DOMAIN))
}

fn local_namespace() -> String {
    hex_bytes(&sha256(LOCAL_STATE_NAMESPACE_DOMAIN))
}

fn generation_namespace(locator: BootstrapLocator) -> String {
    let mut preimage = Vec::with_capacity(BOOTSTRAP_GENERATION_NAMESPACE_DOMAIN.len() + 32);
    preimage.extend_from_slice(BOOTSTRAP_GENERATION_NAMESPACE_DOMAIN);
    preimage.extend_from_slice(locator.as_bytes());
    hex_bytes(&sha256(&preimage))
}

fn locator_key(locator: BootstrapLocator) -> String {
    hex_bytes(locator.as_bytes())
}

/// Recover the exact locator a `local_namespace()` key was written under.
///
/// Returns `None` for anything that is not a lower-case hex encoding of
/// exactly 32 bytes, which cannot be a key this store ever wrote — a
/// `list()` result should never contain one, but a foreign or corrupted
/// entry is skipped rather than trusted.
fn parse_locator_key(key: &str) -> Option<BootstrapLocator> {
    let key = key.as_bytes();
    if key.len() != 64 {
        return None;
    }
    let mut bytes = [0_u8; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        let high = hex_nibble(key[index * 2])?;
        let low = hex_nibble(key[index * 2 + 1])?;
        *byte = (high << 4) | low;
    }
    Some(BootstrapLocator::new(bytes))
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn empty_metadata() -> JsonValue {
    JsonValue::Object(Vec::new())
}

fn map_bootstrap_read(error: StorageError) -> BootstrapStoreError {
    match error {
        StorageError::Unavailable { .. } => BootstrapStoreError::Unavailable,
        StorageError::Conflict { .. }
        | StorageError::NotFound { .. }
        | StorageError::LeaseDenied { .. }
        | StorageError::Validation { .. }
        | StorageError::Backend { .. } => BootstrapStoreError::Corruption,
    }
}

fn map_bootstrap_write(error: StorageError) -> BootstrapStoreError {
    match error {
        StorageError::Unavailable { .. } => BootstrapStoreError::Unavailable,
        StorageError::Conflict { .. } => BootstrapStoreError::Conflict,
        StorageError::NotFound { .. }
        | StorageError::LeaseDenied { .. }
        | StorageError::Validation { .. }
        | StorageError::Backend { .. } => BootstrapStoreError::Corruption,
    }
}

fn map_local_read(error: StorageError) -> LocalStateStoreError {
    match error {
        StorageError::Unavailable { .. } => LocalStateStoreError::Unavailable,
        StorageError::Conflict { .. }
        | StorageError::NotFound { .. }
        | StorageError::LeaseDenied { .. }
        | StorageError::Validation { .. }
        | StorageError::Backend { .. } => LocalStateStoreError::Corruption,
    }
}

fn map_local_write(error: StorageError) -> LocalStateStoreError {
    match error {
        StorageError::Unavailable { .. } => LocalStateStoreError::Unavailable,
        StorageError::Conflict { .. } => LocalStateStoreError::ConcurrentHost,
        StorageError::NotFound { .. }
        | StorageError::LeaseDenied { .. }
        | StorageError::Validation { .. }
        | StorageError::Backend { .. } => LocalStateStoreError::Corruption,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_storage_fs::FsStorageBackend;
    use coding_adventures_vault_pm_application::{
        prepare_audited_generation_zero, ActiveStateV1, AuditedGenerationZeroRandomness,
        GenerationZeroPolicyV1, PreparedGenerationZero, AUDITED_GENERATION_ZERO_RANDOM_BYTES,
    };
    use coding_adventures_vault_pm_format::{
        AeadEnvelopeV1, Argon2idParametersV1, PublicKey, Signature, VaultId, CRYPTO_SUITE_V1,
    };
    use coding_adventures_zeroize::Zeroizing;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::{Arc, Barrier};
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};
    use storage_core::{InMemoryStorageBackend, Revision, StorageBackend, StorageError};

    fn locator(seed: u8) -> BootstrapLocator {
        BootstrapLocator::new([seed; 32])
    }

    fn bootstrap(
        generation: u64,
        previous_bootstrap: Option<BootstrapId>,
        vault_seed: u8,
        signature_seed: u8,
    ) -> Vec<u8> {
        BootstrapV1 {
            vault_id: VaultId::new([vault_seed; 16]),
            generation,
            previous_bootstrap,
            crypto_suite: CRYPTO_SUITE_V1,
            kdf: Argon2idParametersV1 {
                memory_kib: 8 * 1024,
                iterations: 1,
                lanes: 1,
                salt: [3; 16],
            },
            passphrase_root_wrap: AeadEnvelopeV1 {
                suite: CRYPTO_SUITE_V1,
                nonce: [4; 24],
                ciphertext: vec![5; 32],
                tag: [6; 16],
            },
            authority_public_key: PublicKey::new([7; 32]),
            recovery_wraps: Vec::new(),
            signature: Signature::new([signature_seed; 64]),
        }
        .encode()
        .unwrap()
    }

    fn bootstrap_id(bytes: &[u8]) -> BootstrapId {
        BootstrapV1::decode(bytes).unwrap().id().unwrap()
    }

    #[test]
    fn generation_zero_and_rotation_round_trip_with_exact_idempotent_retries() {
        let store = StorageCoreApplicationStore::new(InMemoryStorageBackend::new());
        let locator = locator(1);
        let first = bootstrap(0, None, 9, 10);
        assert_eq!(store.load_latest(locator).unwrap(), None);
        store.put_generation(locator, None, &first).unwrap();
        store.put_generation(locator, None, &first).unwrap();
        assert_eq!(store.load_latest(locator).unwrap(), Some(first.clone()));

        let first_id = bootstrap_id(&first);
        let second = bootstrap(1, Some(first_id), 9, 11);
        store
            .put_generation(locator, Some(first_id), &second)
            .unwrap();
        store
            .put_generation(locator, Some(first_id), &second)
            .unwrap();
        assert_eq!(store.load_latest(locator).unwrap(), Some(second));
    }

    /// VLT-PM43 §5.4. The delete is what makes a rotation mean anything, so it
    /// is checked the only way that proves it: the retired record's own bytes
    /// must be unreadable from the backend afterwards, not merely unreferenced.
    #[test]
    fn a_superseded_generation_is_really_gone_and_the_live_one_is_refused() {
        let store = StorageCoreApplicationStore::new(InMemoryStorageBackend::new());
        let locator = locator(7);
        let first = bootstrap(0, None, 9, 20);
        let first_id = bootstrap_id(&first);
        store.put_generation(locator, None, &first).unwrap();
        let second = bootstrap(1, Some(first_id), 9, 21);
        let second_id = bootstrap_id(&second);
        store
            .put_generation(locator, Some(first_id), &second)
            .unwrap();

        let namespace = generation_namespace(locator);
        let retired_key = hex_bytes(first_id.as_bytes());
        assert!(store
            .backend()
            .get(&namespace, &retired_key)
            .unwrap()
            .is_some());

        // The live generation is the only way into the vault, so removing it
        // is refused outright rather than left to caller discipline.
        assert_eq!(
            store.supersede_generation(locator, second_id),
            Err(BootstrapStoreError::Conflict)
        );

        store.supersede_generation(locator, first_id).unwrap();
        assert!(store
            .backend()
            .get(&namespace, &retired_key)
            .unwrap()
            .is_none());
        // Recovery replays this call, so a second attempt must reach the end.
        store.supersede_generation(locator, first_id).unwrap();
        assert_eq!(store.load_latest(locator).unwrap(), Some(second));
    }

    /// A backend whose `delete` silently does nothing.
    ///
    /// This is not a fanciful fault. `fs::remove_file` returning `Ok` proves
    /// the entry is gone from the page cache, not that its removal is
    /// committed, so a power cut can undo an unlink that every subsequent read
    /// in the same process agreed had happened. Simulating a delete that never
    /// takes is therefore simulating a real crash outcome, and it is the one
    /// outcome in which the defence below is all that stands between a retired
    /// passphrase and the unchanged vault root key.
    struct DeafDeleteBackend(InMemoryStorageBackend);

    impl StorageBackend for DeafDeleteBackend {
        fn initialize(&self) -> Result<(), StorageError> {
            self.0.initialize()
        }
        fn get(&self, namespace: &str, key: &str) -> Result<Option<StorageRecord>, StorageError> {
            self.0.get(namespace, key)
        }
        fn put(&self, input: StoragePutInput) -> Result<StorageRecord, StorageError> {
            self.0.put(input)
        }
        fn delete(
            &self,
            _namespace: &str,
            _key: &str,
            _if_revision: Option<&Revision>,
        ) -> Result<(), StorageError> {
            Ok(())
        }
        fn list(
            &self,
            namespace: &str,
            options: storage_core::StorageListOptions,
        ) -> Result<storage_core::StoragePage, StorageError> {
            self.0.list(namespace, options)
        }
        fn stat(
            &self,
            namespace: &str,
            key: &str,
        ) -> Result<Option<storage_core::StorageStat>, StorageError> {
            self.0.stat(namespace, key)
        }
        fn get_summary(
            &self,
            namespace: &str,
            key: &str,
        ) -> Result<Option<storage_core::StorageRecordSummary>, StorageError> {
            self.0.get_summary(namespace, key)
        }
        fn list_summaries(
            &self,
            namespace: &str,
            options: storage_core::StorageListOptions,
        ) -> Result<storage_core::StorageSummaryPage, StorageError> {
            self.0.list_summaries(namespace, options)
        }
        fn acquire_lease(
            &self,
            name: &str,
            ttl_ms: u64,
        ) -> Result<Option<storage_core::StorageLease>, StorageError> {
            self.0.acquire_lease(name, ttl_ms)
        }
    }

    /// VLT-PM43 §5.4.1. The wrap is destroyed by a durable *write*, not only by
    /// the unlink that follows it.
    ///
    /// Every other durable step in a rotation is a write, and a lost write is
    /// merely lost work the journal replays. A lost *removal* is the opposite:
    /// it resurrects key material into a vault whose owner state has already
    /// moved on and will therefore never revisit it. So the retired record's
    /// body is emptied through the fsync-durable `put` path first. Here the
    /// delete is made deaf, and the record still holds no wrap.
    #[test]
    fn a_retired_wrap_is_destroyed_even_if_the_unlink_never_takes() {
        let store =
            StorageCoreApplicationStore::new(DeafDeleteBackend(InMemoryStorageBackend::new()));
        let locator = locator(8);
        let first = bootstrap(0, None, 9, 30);
        let first_id = bootstrap_id(&first);
        store.put_generation(locator, None, &first).unwrap();
        let second = bootstrap(1, Some(first_id), 9, 31);
        store
            .put_generation(locator, Some(first_id), &second)
            .unwrap();

        let namespace = generation_namespace(locator);
        let retired_key = hex_bytes(first_id.as_bytes());
        let wrap = BootstrapV1::decode(&first)
            .unwrap()
            .passphrase_root_wrap
            .ciphertext;
        assert!(store
            .backend()
            .get(&namespace, &retired_key)
            .unwrap()
            .unwrap()
            .body
            .windows(wrap.len())
            .any(|window| window == wrap));

        // The read-back still reports the record as present, so the call fails
        // closed and the rotation's journal stays for another attempt...
        assert_eq!(
            store.supersede_generation(locator, first_id),
            Err(BootstrapStoreError::Corruption)
        );
        // ...but the wrap is already gone, which is the property that matters
        // when the machine never gets another attempt.
        let residue = store
            .backend()
            .get(&namespace, &retired_key)
            .unwrap()
            .unwrap()
            .body;
        assert!(residue.is_empty());
        assert!(!residue.windows(wrap.len()).any(|window| window == wrap));
    }

    #[test]
    fn bootstrap_rejects_stale_wrong_vault_skipped_and_malformed_successors() {
        let store = StorageCoreApplicationStore::new(InMemoryStorageBackend::new());
        let selected = locator(2);
        let first = bootstrap(0, None, 9, 12);
        let first_id = bootstrap_id(&first);
        store.put_generation(selected, None, &first).unwrap();

        let wrong_previous = bootstrap(1, Some(BootstrapId::new([99; 32])), 9, 13);
        assert_eq!(
            store.put_generation(selected, Some(first_id), &wrong_previous),
            Err(BootstrapStoreError::Conflict)
        );
        let wrong_vault = bootstrap(1, Some(first_id), 8, 14);
        assert_eq!(
            store.put_generation(selected, Some(first_id), &wrong_vault),
            Err(BootstrapStoreError::Conflict)
        );
        let skipped = bootstrap(2, Some(first_id), 9, 15);
        assert_eq!(
            store.put_generation(selected, Some(first_id), &skipped),
            Err(BootstrapStoreError::Conflict)
        );
        assert_eq!(
            store.put_generation(selected, Some(first_id), b"not canonical bootstrap"),
            Err(BootstrapStoreError::Corruption)
        );
        assert_eq!(store.load_latest(selected).unwrap(), Some(first));

        let empty_store = StorageCoreApplicationStore::new(InMemoryStorageBackend::new());
        assert_eq!(
            empty_store.put_generation(locator(22), Some(first_id), &skipped),
            Err(BootstrapStoreError::Conflict)
        );
        assert_eq!(
            empty_store.put_generation(locator(22), None, b""),
            Err(BootstrapStoreError::Corruption)
        );
    }

    #[test]
    fn immutable_generation_and_pointer_races_are_exact_and_idempotent() {
        let store = StorageCoreApplicationStore::new(InMemoryStorageBackend::new());
        let selected = locator(23);
        let first = bootstrap(0, None, 9, 23);
        let first_id = bootstrap_id(&first);
        store.initialize_bootstrap().unwrap();
        store
            .install_generation(selected, first_id, &first)
            .unwrap();
        store
            .install_generation(selected, first_id, &first)
            .unwrap();
        store
            .advance_latest(selected, None, first_id, &first)
            .unwrap();
        store
            .advance_latest(selected, None, first_id, &first)
            .unwrap();

        let competitor = bootstrap(0, None, 8, 24);
        let competitor_id = bootstrap_id(&competitor);
        store
            .install_generation(selected, competitor_id, &competitor)
            .unwrap();
        assert_eq!(
            store.advance_latest(selected, None, competitor_id, &competitor),
            Err(BootstrapStoreError::Conflict)
        );

        let corrupt_locator = locator(24);
        let namespace = generation_namespace(corrupt_locator);
        let key = hex_bytes(first_id.as_bytes());
        store
            .backend()
            .put(
                put_input(&namespace, &key, b"different generation bytes".to_vec())
                    .unwrap()
                    .with_if_absent(),
            )
            .unwrap();
        assert_eq!(
            store.install_generation(corrupt_locator, first_id, &first),
            Err(BootstrapStoreError::Corruption)
        );
    }

    #[test]
    fn malformed_pointer_generation_and_record_envelopes_fail_closed() {
        let store = StorageCoreApplicationStore::new(InMemoryStorageBackend::new());
        store.initialize_bootstrap().unwrap();
        let selected = locator(25);
        let first = bootstrap(0, None, 9, 25);
        let first_id = bootstrap_id(&first);
        let different = bootstrap(0, None, 8, 26);
        let generation_namespace = generation_namespace(selected);
        store
            .backend()
            .put(
                put_input(
                    &generation_namespace,
                    &hex_bytes(first_id.as_bytes()),
                    different,
                )
                .unwrap()
                .with_if_absent(),
            )
            .unwrap();
        store
            .backend()
            .put(
                put_input(
                    &latest_namespace(),
                    &locator_key(selected),
                    first_id.as_bytes().to_vec(),
                )
                .unwrap()
                .with_if_absent(),
            )
            .unwrap();
        assert_eq!(
            store.load_latest(selected),
            Err(BootstrapStoreError::Corruption)
        );

        let malformed_locator = locator(26);
        store
            .backend()
            .put(
                StoragePutInput::new(
                    latest_namespace(),
                    locator_key(malformed_locator),
                    "text/plain",
                    empty_metadata(),
                    vec![0; 32],
                )
                .unwrap()
                .with_if_absent(),
            )
            .unwrap();
        assert_eq!(
            store.load_latest(malformed_locator),
            Err(BootstrapStoreError::Corruption)
        );

        let empty_local_locator = locator(27);
        store
            .backend()
            .put(
                put_input(
                    &local_namespace(),
                    &locator_key(empty_local_locator),
                    Vec::new(),
                )
                .unwrap()
                .with_if_absent(),
            )
            .unwrap();
        assert_eq!(
            store.load(empty_local_locator),
            Err(LocalStateStoreError::Corruption)
        );
    }

    #[test]
    fn competing_bootstrap_successors_leave_exactly_one_latest_winner() {
        let store = Arc::new(StorageCoreApplicationStore::new(
            InMemoryStorageBackend::new(),
        ));
        let locator = locator(3);
        let first = bootstrap(0, None, 9, 16);
        let first_id = bootstrap_id(&first);
        store.put_generation(locator, None, &first).unwrap();
        let barrier = Arc::new(Barrier::new(2));
        let candidates = [
            bootstrap(1, Some(first_id), 9, 17),
            bootstrap(1, Some(first_id), 9, 18),
        ];
        let workers: Vec<_> = candidates
            .iter()
            .cloned()
            .map(|candidate| {
                let store = Arc::clone(&store);
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    barrier.wait();
                    (
                        candidate.clone(),
                        store.put_generation(locator, Some(first_id), &candidate),
                    )
                })
            })
            .collect();
        let results: Vec<_> = workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect();
        assert_eq!(
            results.iter().filter(|(_, result)| result.is_ok()).count(),
            1
        );
        assert_eq!(
            results
                .iter()
                .filter(|(_, result)| *result == Err(BootstrapStoreError::Conflict))
                .count(),
            1
        );
        let latest = store.load_latest(locator).unwrap().unwrap();
        assert!(results
            .iter()
            .any(|(candidate, result)| result.is_ok() && candidate == &latest));
    }

    #[test]
    fn local_state_create_replace_and_stale_compare_exchange_are_exact() {
        let store = StorageCoreApplicationStore::new(InMemoryStorageBackend::new());
        let locator = locator(4);
        let first = b"canonical-owner-state-a";
        let second = b"canonical-owner-state-b";
        assert_eq!(store.load(locator).unwrap(), None);
        store.compare_exchange(locator, None, first).unwrap();
        assert_eq!(store.load(locator).unwrap(), Some(first.to_vec()));
        assert_eq!(
            store.compare_exchange(locator, None, second),
            Err(LocalStateStoreError::ConcurrentHost)
        );
        assert_eq!(
            store.compare_exchange(locator, Some(b"stale"), second),
            Err(LocalStateStoreError::ConcurrentHost)
        );
        store
            .compare_exchange(locator, Some(first), second)
            .unwrap();
        assert_eq!(store.load(locator).unwrap(), Some(second.to_vec()));
    }

    #[test]
    fn concurrent_local_state_writers_have_one_winner() {
        let store = Arc::new(StorageCoreApplicationStore::new(
            InMemoryStorageBackend::new(),
        ));
        let locator = locator(5);
        let initial = b"initial-owner-state";
        store.compare_exchange(locator, None, initial).unwrap();
        let barrier = Arc::new(Barrier::new(8));
        let workers: Vec<_> = (0..8)
            .map(|index| {
                let store = Arc::clone(&store);
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    let replacement = format!("replacement-owner-state-{index}").into_bytes();
                    barrier.wait();
                    let result = store.compare_exchange(locator, Some(initial), &replacement);
                    (replacement, result)
                })
            })
            .collect();
        let results: Vec<_> = workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect();
        assert_eq!(
            results.iter().filter(|(_, result)| result.is_ok()).count(),
            1
        );
        assert_eq!(
            results
                .iter()
                .filter(|(_, result)| *result == Err(LocalStateStoreError::ConcurrentHost))
                .count(),
            7
        );
        let winner = store.load(locator).unwrap().unwrap();
        assert!(results
            .iter()
            .any(|(replacement, result)| result.is_ok() && replacement == &winner));
    }

    #[test]
    fn local_state_rejects_empty_and_oversized_values_without_writing() {
        let store = StorageCoreApplicationStore::new(InMemoryStorageBackend::new());
        let locator = locator(6);
        assert_eq!(
            store.compare_exchange(locator, None, b""),
            Err(LocalStateStoreError::Corruption)
        );
        assert_eq!(
            store.compare_exchange(locator, None, &vec![0; MAX_LOCAL_STATE_BYTES + 1]),
            Err(LocalStateStoreError::Corruption)
        );
        assert_eq!(store.load(locator).unwrap(), None);
    }

    #[test]
    fn filesystem_reconstruction_preserves_bootstrap_history_and_owner_state() {
        let root = temporary_root("restart");
        let locator = locator(7);
        let first = bootstrap(0, None, 9, 19);
        let first_id = bootstrap_id(&first);
        let second = bootstrap(1, Some(first_id), 9, 20);
        {
            let store = StorageCoreApplicationStore::new(FsStorageBackend::new(&root));
            store.put_generation(locator, None, &first).unwrap();
            store
                .put_generation(locator, Some(first_id), &second)
                .unwrap();
            store
                .compare_exchange(locator, None, b"durable-owner-state")
                .unwrap();
        }
        {
            let reopened = StorageCoreApplicationStore::new(FsStorageBackend::new(&root));
            assert_eq!(reopened.load_latest(locator).unwrap(), Some(second));
            assert_eq!(
                reopened.load(locator).unwrap(),
                Some(b"durable-owner-state".to_vec())
            );
            reopened
                .compare_exchange(
                    locator,
                    Some(b"durable-owner-state"),
                    b"first-state-after-restart",
                )
                .unwrap();
            assert_eq!(
                reopened.compare_exchange(
                    locator,
                    Some(b"durable-owner-state"),
                    b"stale-state-after-restart",
                ),
                Err(LocalStateStoreError::ConcurrentHost)
            );
            assert_eq!(
                reopened.load(locator).unwrap(),
                Some(b"first-state-after-restart".to_vec())
            );
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn diagnostics_and_names_do_not_expose_payloads_or_backend_details() {
        let store = StorageCoreApplicationStore::new(InMemoryStorageBackend::new());
        assert_eq!(
            format!("{store:?}"),
            "StorageCoreApplicationStore { backend: \"<redacted>\" }"
        );
        let secret = "never-log-this-owner-state";
        for error in [
            BootstrapStoreError::Unavailable,
            BootstrapStoreError::Conflict,
            BootstrapStoreError::Corruption,
        ] {
            assert!(!format!("{error:?} {error}").contains(secret));
        }
        for error in [
            LocalStateStoreError::Unavailable,
            LocalStateStoreError::ConcurrentHost,
            LocalStateStoreError::Corruption,
        ] {
            assert!(!format!("{error:?} {error}").contains(secret));
        }
        assert_eq!(latest_namespace().len(), 64);
        assert_eq!(local_namespace().len(), 64);
        assert_eq!(generation_namespace(locator(8)).len(), 64);
        assert_eq!(locator_key(locator(8)).len(), 64);

        let backend = StorageCoreApplicationStore::new(InMemoryStorageBackend::new());
        let _ = backend.backend();
        let _ = backend.into_backend();
    }

    #[test]
    fn storage_error_translation_is_closed_and_payload_blind() {
        let not_found = StorageError::NotFound {
            namespace: "secret namespace".to_string(),
            key: "secret key".to_string(),
        };
        let conflict = StorageError::Conflict {
            namespace: "secret namespace".to_string(),
            key: "secret key".to_string(),
            expected_revision: Some("secret expected".to_string()),
            actual_revision: Some("secret actual".to_string()),
        };
        let unavailable = StorageError::Unavailable {
            message: "secret unavailable".to_string(),
        };
        let lease = StorageError::LeaseDenied {
            name: "secret lease".to_string(),
        };
        let validation = StorageError::Validation {
            field: "secret field".to_string(),
            message: "secret validation".to_string(),
        };
        let backend = StorageError::Backend {
            message: "secret backend".to_string(),
        };

        assert_eq!(
            map_bootstrap_read(unavailable.clone()),
            BootstrapStoreError::Unavailable
        );
        for error in [
            not_found.clone(),
            conflict.clone(),
            lease.clone(),
            validation.clone(),
            backend.clone(),
        ] {
            assert_eq!(map_bootstrap_read(error), BootstrapStoreError::Corruption);
        }
        assert_eq!(
            map_bootstrap_write(unavailable.clone()),
            BootstrapStoreError::Unavailable
        );
        assert_eq!(
            map_bootstrap_write(conflict.clone()),
            BootstrapStoreError::Conflict
        );
        for error in [
            not_found.clone(),
            lease.clone(),
            validation.clone(),
            backend.clone(),
        ] {
            assert_eq!(map_bootstrap_write(error), BootstrapStoreError::Corruption);
        }
        assert_eq!(
            map_local_read(unavailable.clone()),
            LocalStateStoreError::Unavailable
        );
        for error in [
            not_found.clone(),
            conflict.clone(),
            lease.clone(),
            validation.clone(),
            backend.clone(),
        ] {
            assert_eq!(map_local_read(error), LocalStateStoreError::Corruption);
        }
        assert_eq!(
            map_local_write(unavailable),
            LocalStateStoreError::Unavailable
        );
        assert_eq!(
            map_local_write(conflict),
            LocalStateStoreError::ConcurrentHost
        );
        for error in [not_found, lease, validation, backend] {
            assert_eq!(map_local_write(error), LocalStateStoreError::Corruption);
        }

        let malformed = StorageRecord::new(
            "wrong-namespace",
            "wrong-key",
            Revision::new("r1").unwrap(),
            OPAQUE_CONTENT_TYPE,
            empty_metadata(),
            Vec::new(),
            1,
            1,
        )
        .unwrap();
        assert_eq!(
            validate_record(&malformed, "expected", "expected", 1),
            Err(())
        );
    }

    // --- VLT-PM41 §8: reclaiming a generation zero stranded before its
    // locator ever reached configuration ------------------------------------

    fn generation_zero_policy() -> GenerationZeroPolicyV1 {
        GenerationZeroPolicyV1::new(8 * 1024, 1, 1, 1_700_000_000_000).unwrap()
    }

    /// A distinct, deterministic `AUDITED_GENERATION_ZERO_RANDOM_BYTES` block
    /// per `seed`, so repeated calls draw distinct locators the same way
    /// distinct real `init` attempts would.
    fn fixture_audited_randomness(seed: u8) -> AuditedGenerationZeroRandomness {
        let mut bytes = [0_u8; AUDITED_GENERATION_ZERO_RANDOM_BYTES];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = (index as u8).wrapping_mul(17).wrapping_add(seed);
        }
        AuditedGenerationZeroRandomness::new(bytes)
    }

    /// Exactly what `vault-pm-cli::begin_init` has in hand right before its
    /// `compare_exchange(locator, None, &exact_prepared)` call.
    fn crashed_preparation(seed: u8) -> PreparedGenerationZero {
        prepare_audited_generation_zero(
            Zeroizing::new(b"correct horse battery staple".to_vec()),
            generation_zero_policy(),
            fixture_audited_randomness(seed),
        )
        .unwrap()
    }

    fn intended_active(prepared: &PreparedGenerationZero) -> ActiveStateV1 {
        let LocalVaultStateV1::PreparedInit(journal) = prepared.owner_state() else {
            panic!("generation zero must be prepared")
        };
        journal.intended_active().clone()
    }

    /// This is the leak VLT-PM41 §8 names, reproduced exactly: install a
    /// `PreparedInit` journal under a random locator the way `begin_init`
    /// does, then stop — as a crash strictly before the configuration write
    /// would. Nothing durable anywhere records that this locator was ever
    /// drawn, so without a sweep the bytes are gone from every code path that
    /// could reach them, yet still occupy the backend forever. `reclaim_...`
    /// is the only thing that ever revisits them.
    #[test]
    fn a_generation_zero_crashed_before_configuration_leaks_until_reclaimed() {
        let store = StorageCoreApplicationStore::new(InMemoryStorageBackend::new());
        let prepared = crashed_preparation(40);
        let locator = prepared.bootstrap_locator();
        let exact_prepared = prepared.owner_state().encode().unwrap();

        // The crashed process's one durable write.
        store
            .compare_exchange(locator, None, &exact_prepared)
            .unwrap();

        // The leak: the bytes are durable, and no configuration anywhere
        // names this locator, yet ordinary use of the store — a second,
        // unrelated `load` — does not make them disappear on its own.
        assert_eq!(store.load(locator).unwrap(), Some(exact_prepared.clone()));
        assert_eq!(store.load(locator).unwrap(), Some(exact_prepared.clone()));

        // The fix: nothing currently configured names any locator, so the
        // whole namespace is fair game, and the orphan is gone.
        let reclaimed = store
            .reclaim_orphaned_preparations(&BTreeSet::new())
            .unwrap();
        assert_eq!(reclaimed, 1);
        assert_eq!(store.load(locator).unwrap(), None);

        // Idempotent: nothing left to reclaim a second time.
        assert_eq!(
            store
                .reclaim_orphaned_preparations(&BTreeSet::new())
                .unwrap(),
            0
        );
    }

    /// A `PreparedInit` a caller still considers live — its locator is in
    /// `live_locators` — is exactly the ordinary crash-resumable case
    /// (VLT-PM42): configuration already names it, so `init`'s resume path
    /// will find and finish it. Reclaiming it would turn a recoverable crash
    /// into the exact unrecoverable leak this method exists to prevent.
    #[test]
    fn reclaim_never_touches_a_locator_the_caller_still_considers_live() {
        let store = StorageCoreApplicationStore::new(InMemoryStorageBackend::new());
        let prepared = crashed_preparation(41);
        let locator = prepared.bootstrap_locator();
        let exact_prepared = prepared.owner_state().encode().unwrap();
        store
            .compare_exchange(locator, None, &exact_prepared)
            .unwrap();

        let mut live = BTreeSet::new();
        live.insert(locator);
        assert_eq!(store.reclaim_orphaned_preparations(&live).unwrap(), 0);
        assert_eq!(store.load(locator).unwrap(), Some(exact_prepared));
    }

    /// `Active` can only exist because the configuration write that names its
    /// locator already succeeded (`complete_generation_zero` is only ever
    /// invoked after it), so it is never this leak's orphan no matter what
    /// `live_locators` says. This is the load-bearing safety property: the
    /// state check alone protects a real vault from ever being swept.
    #[test]
    fn reclaim_never_touches_an_active_record_even_if_unlisted() {
        let store = StorageCoreApplicationStore::new(InMemoryStorageBackend::new());
        let prepared = crashed_preparation(42);
        let locator = prepared.bootstrap_locator();
        let active = intended_active(&prepared);
        let exact_active = LocalVaultStateV1::Active(active).encode().unwrap();
        store
            .compare_exchange(locator, None, &exact_active)
            .unwrap();

        assert_eq!(
            store
                .reclaim_orphaned_preparations(&BTreeSet::new())
                .unwrap(),
            0
        );
        assert_eq!(store.load(locator).unwrap(), Some(exact_active));
    }

    /// A record this store did not write — wrong content type, foreign
    /// namespace shape, or bytes that do not decode as `LocalVaultStateV1` at
    /// all — is left alone rather than trusted. Reclaiming is opt-in per
    /// record, never a default.
    #[test]
    fn reclaim_skips_records_it_cannot_positively_identify_as_a_stranded_preparation() {
        let store = StorageCoreApplicationStore::new(InMemoryStorageBackend::new());
        let namespace = local_namespace();
        let key = locator_key(locator(44));
        store
            .backend()
            .put(
                put_input(&namespace, &key, b"not a canonical owner state".to_vec())
                    .unwrap()
                    .with_if_absent(),
            )
            .unwrap();

        assert_eq!(
            store
                .reclaim_orphaned_preparations(&BTreeSet::new())
                .unwrap(),
            0
        );
        assert!(store.backend().get(&namespace, &key).unwrap().is_some());
    }

    /// Reproduces the leak over a real filesystem backend, matching how
    /// `vault-pm-cli` actually reaches this store, and proves reclaim sees
    /// past a process restart exactly as `init`'s own resume path does for
    /// the locators it keeps.
    #[test]
    fn reclaim_over_a_real_filesystem_backend_survives_reopening_the_store() {
        let root = temporary_root("reclaim");
        let prepared = crashed_preparation(45);
        let locator = prepared.bootstrap_locator();
        let exact_prepared = prepared.owner_state().encode().unwrap();
        {
            let store = StorageCoreApplicationStore::new(FsStorageBackend::new(&root));
            store
                .compare_exchange(locator, None, &exact_prepared)
                .unwrap();
        }
        {
            // A fresh process, exactly as a restarted `vault-pm init` would
            // open it, finds the orphan still durable on disk.
            let reopened = StorageCoreApplicationStore::new(FsStorageBackend::new(&root));
            assert_eq!(reopened.load(locator).unwrap(), Some(exact_prepared));
            assert_eq!(
                reopened
                    .reclaim_orphaned_preparations(&BTreeSet::new())
                    .unwrap(),
                1
            );
            assert_eq!(reopened.load(locator).unwrap(), None);
        }
        fs::remove_dir_all(root).unwrap();
    }

    fn temporary_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "vault-pm-application-storage-core-{label}-{}-{nanos}",
            std::process::id()
        ))
    }
}
