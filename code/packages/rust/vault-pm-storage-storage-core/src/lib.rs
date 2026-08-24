//! VLT-PM02 bridge from an injected `storage-core` backend to immutable vault storage.
//!
//! The adapter gives the local password-manager repository one persistence
//! contract without coupling it to filesystem paths or provider SDKs. Bucket
//! and object names are opaque lowercase hex, and every write is immutable.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_json_value::JsonValue;
use coding_adventures_sha256::sha256;
use coding_adventures_vault_pm_storage::{
    BackendCapabilities, BucketId, ChangeCursor, ChangePage, DeleteOutcome, InputViolation,
    ListCursor, ObjectBytes, ObjectEntry, ObjectId, ObjectPage, ObjectStat, ProviderRevision,
    PutImmutableOutcome, StoreError, VaultLocator, VaultObjectStore, MAX_LIST_LIMIT,
    MAX_OBJECT_BYTES,
};
use std::fmt::{self, Debug, Formatter};
use std::sync::Mutex;
use storage_core::{
    StorageBackend, StorageError, StorageListOptions, StoragePutInput, StorageRecord, StorageStat,
};

const BINDING_NAMESPACE_LABEL: &[u8] = b"vault-pm/storage-core/binding/namespace/v1";
const BINDING_KEY_LABEL: &[u8] = b"vault-pm/storage-core/binding/key/v1";
const OPAQUE_CONTENT_TYPE: &str = "application/octet-stream";
const CURSOR_BYTES: usize = 64;

/// Initial provider-efficient immutable pack target for local storage.
pub const PREFERRED_PACK_BYTES: u64 = 8 * 1024 * 1024;

/// VLT-PM02 immutable object storage over one injected `storage-core` backend.
pub struct StorageCoreObjectStore<B: StorageBackend> {
    backend: B,
    locator: Mutex<Option<VaultLocator>>,
}

impl<B: StorageBackend> StorageCoreObjectStore<B> {
    /// Construct an uninitialized adapter over `backend`.
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            locator: Mutex::new(None),
        }
    }

    fn ensure_initialized(&self) -> Result<(), StoreError> {
        let locator = self.locator.lock().map_err(|_| StoreError::Provider)?;
        if locator.is_some() {
            Ok(())
        } else {
            Err(StoreError::NotInitialized)
        }
    }

    fn read_binding(&self) -> Result<Option<StorageRecord>, StoreError> {
        self.backend
            .get(&binding_namespace(), &binding_key())
            .map_err(map_storage_error)
    }

    fn binding_matches(record: &StorageRecord, locator: &VaultLocator) -> bool {
        record.namespace == binding_namespace()
            && record.key == binding_key()
            && record.body.as_slice() == locator.as_bytes()
    }

    fn initialize_binding(&self, locator: &VaultLocator) -> Result<(), StoreError> {
        if let Some(record) = self.read_binding()? {
            return if Self::binding_matches(&record, locator) {
                Ok(())
            } else {
                Err(StoreError::Conflict)
            };
        }

        let input = StoragePutInput::new(
            binding_namespace(),
            binding_key(),
            OPAQUE_CONTENT_TYPE,
            empty_metadata(),
            locator.as_bytes().to_vec(),
        )
        .map_err(map_storage_error)?
        .with_if_absent();

        match self.backend.put(input) {
            Ok(record) if Self::binding_matches(&record, locator) => Ok(()),
            Ok(_) => Err(StoreError::Conflict),
            Err(StorageError::Conflict { .. }) => match self.read_binding()? {
                Some(record) if Self::binding_matches(&record, locator) => Ok(()),
                _ => Err(StoreError::Conflict),
            },
            Err(error) => Err(map_storage_error(error)),
        }
    }

    fn existing_outcome(
        record: Option<StorageRecord>,
        bytes: &ObjectBytes,
    ) -> Result<PutImmutableOutcome, StoreError> {
        let Some(record) = record else {
            return Err(StoreError::Conflict);
        };
        if record.body.len() > MAX_OBJECT_BYTES {
            return Err(StoreError::Corruption);
        }
        if record.body.as_slice() == bytes.as_slice() {
            Ok(PutImmutableOutcome::AlreadyPresent)
        } else {
            Err(StoreError::Corruption)
        }
    }
}

impl<B: StorageBackend> Debug for StorageCoreObjectStore<B> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let initialized = self
            .locator
            .lock()
            .map(|locator| locator.is_some())
            .unwrap_or(false);
        formatter
            .debug_struct("StorageCoreObjectStore")
            .field("backend", &"<redacted>")
            .field("initialized", &initialized)
            .finish()
    }
}

impl<B: StorageBackend> VaultObjectStore for StorageCoreObjectStore<B> {
    fn initialize(&self, locator: &VaultLocator) -> Result<(), StoreError> {
        self.backend.initialize().map_err(map_storage_error)?;
        let mut bound = self.locator.lock().map_err(|_| StoreError::Provider)?;
        if let Some(existing) = *bound {
            return if existing == *locator {
                Ok(())
            } else {
                Err(StoreError::Conflict)
            };
        }
        self.initialize_binding(locator)?;
        *bound = Some(*locator);
        Ok(())
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            strong_read_after_write: true,
            strong_list_after_write: true,
            conditional_create: true,
            conditional_replace: false,
            change_feed: false,
            push_notifications: false,
            resumable_upload: false,
            range_read: false,
            server_checksum: false,
            physical_delete: true,
            shareable_container: false,
            max_object_size: Some(MAX_OBJECT_BYTES as u64),
            preferred_pack_size: PREFERRED_PACK_BYTES,
        }
    }

    fn get(&self, bucket: &BucketId, object: &ObjectId) -> Result<Option<ObjectBytes>, StoreError> {
        self.ensure_initialized()?;
        let record = self
            .backend
            .get(&bucket_namespace(bucket), &object_key(object))
            .map_err(map_storage_error)?;
        record.map(record_body).transpose()
    }

    fn stat(&self, bucket: &BucketId, object: &ObjectId) -> Result<Option<ObjectStat>, StoreError> {
        self.ensure_initialized()?;
        let stat = self
            .backend
            .stat(&bucket_namespace(bucket), &object_key(object))
            .map_err(map_storage_error)?;
        stat.as_ref().map(object_stat).transpose()
    }

    fn put_immutable(
        &self,
        bucket: &BucketId,
        object: &ObjectId,
        bytes: &ObjectBytes,
    ) -> Result<PutImmutableOutcome, StoreError> {
        self.ensure_initialized()?;
        let namespace = bucket_namespace(bucket);
        let key = object_key(object);
        if let Some(record) = self
            .backend
            .get(&namespace, &key)
            .map_err(map_storage_error)?
        {
            return Self::existing_outcome(Some(record), bytes);
        }

        let input = StoragePutInput::new(
            namespace.clone(),
            key.clone(),
            OPAQUE_CONTENT_TYPE,
            empty_metadata(),
            bytes.as_slice().to_vec(),
        )
        .map_err(map_storage_error)?
        .with_if_absent();
        match self.backend.put(input) {
            Ok(record)
                if record.namespace == namespace
                    && record.key == key
                    && record.body.as_slice() == bytes.as_slice() =>
            {
                Ok(PutImmutableOutcome::Created)
            }
            Ok(_) => Err(StoreError::Corruption),
            Err(StorageError::Conflict { .. }) => Self::existing_outcome(
                self.backend
                    .get(&namespace, &key)
                    .map_err(map_storage_error)?,
                bytes,
            ),
            Err(error) => Err(map_storage_error(error)),
        }
    }

    fn list(
        &self,
        bucket: &BucketId,
        cursor: Option<&ListCursor>,
        limit: usize,
    ) -> Result<ObjectPage, StoreError> {
        self.ensure_initialized()?;
        if !(1..=MAX_LIST_LIMIT).contains(&limit) {
            return Err(StoreError::InvalidInput(InputViolation::ListLimit));
        }
        let namespace = bucket_namespace(bucket);
        let storage_cursor = cursor
            .map(|cursor| decode_cursor(bucket, cursor).map(|object| object_key(&object)))
            .transpose()?;
        let page = self
            .backend
            .list(
                &namespace,
                StorageListOptions {
                    prefix: None,
                    recursive: true,
                    page_size: Some(limit),
                    cursor: storage_cursor,
                },
            )
            .map_err(map_storage_error)?;
        if page.records.len() > limit {
            return Err(StoreError::Corruption);
        }

        let mut entries = Vec::with_capacity(page.records.len());
        let mut previous = None;
        for record in page.records {
            if record.namespace != namespace || record.body.len() > MAX_OBJECT_BYTES {
                return Err(StoreError::Corruption);
            }
            let object = parse_object_key(&record.key)?;
            if previous.is_some_and(|candidate| candidate >= object) {
                return Err(StoreError::Corruption);
            }
            let stat = object_stat_from_record(&record)?;
            entries.push(ObjectEntry { object, stat });
            previous = Some(object);
        }

        let next_cursor = match page.next_cursor {
            None => None,
            Some(storage_cursor) => {
                let object = parse_object_key(&storage_cursor)?;
                if entries.last().map(|entry| entry.object) != Some(object) {
                    return Err(StoreError::Corruption);
                }
                Some(encode_cursor(bucket, &object)?)
            }
        };
        Ok(ObjectPage {
            entries,
            next_cursor,
        })
    }

    fn delete_unreferenced(
        &self,
        bucket: &BucketId,
        object: &ObjectId,
    ) -> Result<DeleteOutcome, StoreError> {
        self.ensure_initialized()?;
        let namespace = bucket_namespace(bucket);
        let key = object_key(object);
        let Some(stat) = self
            .backend
            .stat(&namespace, &key)
            .map_err(map_storage_error)?
        else {
            return Ok(DeleteOutcome::Missing);
        };
        if stat.body_len > MAX_OBJECT_BYTES {
            return Err(StoreError::Corruption);
        }
        self.backend
            .delete(&namespace, &key, Some(&stat.revision))
            .map_err(map_storage_error)?;
        Ok(DeleteOutcome::Deleted)
    }

    fn changes(&self, _cursor: Option<&ChangeCursor>) -> Result<Option<ChangePage>, StoreError> {
        self.ensure_initialized()?;
        Err(StoreError::Unsupported)
    }
}

fn empty_metadata() -> JsonValue {
    JsonValue::Object(Vec::new())
}

fn binding_namespace() -> String {
    hex_bytes(&sha256(BINDING_NAMESPACE_LABEL))
}

fn binding_key() -> String {
    hex_bytes(&sha256(BINDING_KEY_LABEL))
}

fn bucket_namespace(bucket: &BucketId) -> String {
    hex_bytes(bucket.as_bytes())
}

fn object_key(object: &ObjectId) -> String {
    hex_bytes(object.as_bytes())
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

fn parse_object_key(key: &str) -> Result<ObjectId, StoreError> {
    if key.len() != 64 {
        return Err(StoreError::Corruption);
    }
    let mut bytes = [0u8; 32];
    for (index, pair) in key.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        let high = decode_hex(pair[0]).ok_or(StoreError::Corruption)?;
        let low = decode_hex(pair[1]).ok_or(StoreError::Corruption)?;
        bytes[index] = (high << 4) | low;
    }
    Ok(ObjectId::new(bytes))
}

fn decode_hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

fn encode_cursor(bucket: &BucketId, object: &ObjectId) -> Result<ListCursor, StoreError> {
    let mut bytes = Vec::with_capacity(CURSOR_BYTES);
    bytes.extend_from_slice(bucket.as_bytes());
    bytes.extend_from_slice(object.as_bytes());
    ListCursor::new(bytes)
}

fn decode_cursor(bucket: &BucketId, cursor: &ListCursor) -> Result<ObjectId, StoreError> {
    let bytes = cursor.as_bytes();
    if bytes.len() != CURSOR_BYTES {
        return Err(StoreError::InvalidInput(InputViolation::CursorMalformed));
    }
    if &bytes[..32] != bucket.as_bytes() {
        return Err(StoreError::InvalidInput(InputViolation::CursorScope));
    }
    let mut object = [0u8; 32];
    object.copy_from_slice(&bytes[32..]);
    Ok(ObjectId::new(object))
}

fn record_body(record: StorageRecord) -> Result<ObjectBytes, StoreError> {
    if record.body.len() > MAX_OBJECT_BYTES {
        return Err(StoreError::Corruption);
    }
    ObjectBytes::new(record.body).map_err(|_| StoreError::Corruption)
}

fn object_stat(stat: &StorageStat) -> Result<ObjectStat, StoreError> {
    if stat.body_len > MAX_OBJECT_BYTES {
        return Err(StoreError::Corruption);
    }
    let revision = ProviderRevision::new(stat.revision.as_str().to_string())
        .map_err(|_| StoreError::Corruption)?;
    Ok(ObjectStat {
        body_len: stat.body_len as u64,
        revision: Some(revision),
        server_checksum: None,
    })
}

fn object_stat_from_record(record: &StorageRecord) -> Result<ObjectStat, StoreError> {
    let revision = ProviderRevision::new(record.revision.as_str().to_string())
        .map_err(|_| StoreError::Corruption)?;
    Ok(ObjectStat {
        body_len: record.body.len() as u64,
        revision: Some(revision),
        server_checksum: None,
    })
}

fn map_storage_error(error: StorageError) -> StoreError {
    match error {
        StorageError::Conflict { .. } => StoreError::Conflict,
        StorageError::Unavailable { .. } => StoreError::Network,
        StorageError::NotFound { .. }
        | StorageError::LeaseDenied { .. }
        | StorageError::Validation { .. }
        | StorageError::Backend { .. } => StoreError::Provider,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_storage_fs::FsStorageBackend;
    use coding_adventures_vault_pm_storage::run_conformance_suite;
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    use storage_core::{
        InMemoryStorageBackend, Revision, StorageLease, StoragePage, StorageSummaryPage,
    };

    static TEMP_ROOT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    struct TempRoot(PathBuf);

    impl TempRoot {
        fn new() -> Self {
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let sequence = TEMP_ROOT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let mut path = env::temp_dir();
            path.push(format!(
                "vault-pm-storage-core-test-{}-{stamp}-{sequence}",
                std::process::id()
            ));
            Self(path)
        }

        fn path(&self) -> PathBuf {
            self.0.clone()
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn memory_backend_passes_shared_conformance() {
        let report =
            run_conformance_suite(
                || StorageCoreObjectStore::new(InMemoryStorageBackend::default()),
            )
            .unwrap();
        assert_eq!(report.checks, 24);
        assert!(report.capabilities.conditional_create);
        assert!(report.capabilities.physical_delete);
        assert!(!report.capabilities.change_feed);
    }

    #[test]
    fn filesystem_backend_passes_shared_conformance() {
        let root = TempRoot::new();
        let path = root.path();
        let report =
            run_conformance_suite(|| StorageCoreObjectStore::new(FsStorageBackend::new(path)))
                .unwrap();
        assert_eq!(report.checks, 24);
    }

    #[test]
    fn filesystem_locator_binding_survives_backend_reconstruction() {
        let root = TempRoot::new();
        let locator = VaultLocator::new([7; 32]);
        let first = StorageCoreObjectStore::new(FsStorageBackend::new(root.path()));
        first.initialize(&locator).unwrap();
        drop(first);

        let reopened = StorageCoreObjectStore::new(FsStorageBackend::new(root.path()));
        reopened.initialize(&locator).unwrap();
        drop(reopened);

        let wrong = StorageCoreObjectStore::new(FsStorageBackend::new(root.path()));
        assert_eq!(
            wrong.initialize(&VaultLocator::new([8; 32])),
            Err(StoreError::Conflict)
        );
    }

    #[test]
    fn capabilities_cursors_and_debug_are_closed() {
        let store = StorageCoreObjectStore::new(InMemoryStorageBackend::default());
        let bucket = BucketId::new([2; 32]);
        let object = ObjectId::new([3; 32]);
        assert_eq!(store.get(&bucket, &object), Err(StoreError::NotInitialized));
        assert_eq!(store.changes(None), Err(StoreError::NotInitialized));
        assert!(!format!("{store:?}").contains("InMemoryStorageBackend"));
        assert!(format!("{store:?}").contains("initialized: false"));

        store.initialize(&VaultLocator::new([1; 32])).unwrap();
        let capabilities = store.capabilities();
        assert!(capabilities.strong_read_after_write);
        assert!(capabilities.strong_list_after_write);
        assert_eq!(capabilities.max_object_size, Some(MAX_OBJECT_BYTES as u64));
        assert_eq!(capabilities.preferred_pack_size, PREFERRED_PACK_BYTES);
        assert!(!capabilities.server_checksum);
        assert!(format!("{store:?}").contains("initialized: true"));

        let malformed = ListCursor::new(vec![0; 1]).unwrap();
        assert_eq!(
            store.list(&bucket, Some(&malformed), 1),
            Err(StoreError::InvalidInput(InputViolation::CursorMalformed))
        );
    }

    #[test]
    fn cursor_is_fixed_width_and_bucket_bound() {
        let store = StorageCoreObjectStore::new(InMemoryStorageBackend::default());
        let locator = VaultLocator::new([1; 32]);
        let bucket = BucketId::new([2; 32]);
        store.initialize(&locator).unwrap();
        for byte in [3, 4] {
            store
                .put_immutable(
                    &bucket,
                    &ObjectId::new([byte; 32]),
                    &ObjectBytes::new(vec![byte]).unwrap(),
                )
                .unwrap();
        }
        let first = store.list(&bucket, None, 1).unwrap();
        let cursor = first.next_cursor.unwrap();
        assert_eq!(cursor.as_bytes().len(), CURSOR_BYTES);
        assert_eq!(&cursor.as_bytes()[..32], bucket.as_bytes());
        assert_eq!(
            store.list(&BucketId::new([9; 32]), Some(&cursor), 1),
            Err(StoreError::InvalidInput(InputViolation::CursorScope))
        );
    }

    #[test]
    fn malformed_storage_key_is_corruption() {
        let backend = InMemoryStorageBackend::default();
        backend.initialize().unwrap();
        let bucket = BucketId::new([2; 32]);
        backend
            .put(
                StoragePutInput::new(
                    bucket_namespace(&bucket),
                    "not-hex",
                    OPAQUE_CONTENT_TYPE,
                    empty_metadata(),
                    vec![1],
                )
                .unwrap(),
            )
            .unwrap();
        let store = StorageCoreObjectStore::new(backend);
        store.initialize(&VaultLocator::new([1; 32])).unwrap();
        assert_eq!(store.list(&bucket, None, 10), Err(StoreError::Corruption));
    }

    enum AdversarialBehavior {
        BindingRace(Vec<u8>),
        List(StoragePage),
        MalformedPut,
        ObjectRace(Vec<u8>),
    }

    struct AdversarialBackend {
        inner: InMemoryStorageBackend,
        behavior: Mutex<Option<AdversarialBehavior>>,
    }

    impl AdversarialBackend {
        fn new(behavior: AdversarialBehavior) -> Self {
            Self {
                inner: InMemoryStorageBackend::default(),
                behavior: Mutex::new(Some(behavior)),
            }
        }
    }

    impl StorageBackend for AdversarialBackend {
        fn initialize(&self) -> Result<(), StorageError> {
            self.inner.initialize()
        }

        fn get(&self, namespace: &str, key: &str) -> Result<Option<StorageRecord>, StorageError> {
            self.inner.get(namespace, key)
        }

        fn put(&self, input: StoragePutInput) -> Result<StorageRecord, StorageError> {
            let mut behavior = self.behavior.lock().unwrap();
            let pending = behavior.take();
            match pending {
                Some(AdversarialBehavior::BindingRace(body))
                    if input.namespace == binding_namespace() =>
                {
                    let competing = StoragePutInput {
                        body,
                        ..input.clone()
                    };
                    self.inner.put(competing)?;
                    return Err(StorageError::Conflict {
                        namespace: "do-not-leak-namespace".into(),
                        key: "do-not-leak-key".into(),
                        expected_revision: None,
                        actual_revision: Some("do-not-leak-revision".into()),
                    });
                }
                Some(AdversarialBehavior::ObjectRace(body))
                    if input.namespace != binding_namespace() =>
                {
                    let competing = StoragePutInput {
                        body,
                        ..input.clone()
                    };
                    self.inner.put(competing)?;
                    return Err(StorageError::Conflict {
                        namespace: "do-not-leak-namespace".into(),
                        key: "do-not-leak-key".into(),
                        expected_revision: None,
                        actual_revision: Some("do-not-leak-revision".into()),
                    });
                }
                Some(AdversarialBehavior::MalformedPut)
                    if input.namespace != binding_namespace() =>
                {
                    let mut record = self.inner.put(input)?;
                    record.key = "malformed-return-key".into();
                    return Ok(record);
                }
                other => *behavior = other,
            }
            drop(behavior);
            self.inner.put(input)
        }

        fn delete(
            &self,
            namespace: &str,
            key: &str,
            if_revision: Option<&Revision>,
        ) -> Result<(), StorageError> {
            self.inner.delete(namespace, key, if_revision)
        }

        fn list(
            &self,
            namespace: &str,
            options: StorageListOptions,
        ) -> Result<StoragePage, StorageError> {
            let mut behavior = self.behavior.lock().unwrap();
            let pending = behavior.take();
            if let Some(AdversarialBehavior::List(page)) = pending {
                return Ok(page);
            }
            *behavior = pending;
            drop(behavior);
            self.inner.list(namespace, options)
        }

        fn stat(&self, namespace: &str, key: &str) -> Result<Option<StorageStat>, StorageError> {
            self.inner.stat(namespace, key)
        }

        fn list_summaries(
            &self,
            namespace: &str,
            options: StorageListOptions,
        ) -> Result<StorageSummaryPage, StorageError> {
            self.inner.list_summaries(namespace, options)
        }

        fn acquire_lease(
            &self,
            name: &str,
            ttl_ms: u64,
        ) -> Result<Option<StorageLease>, StorageError> {
            self.inner.acquire_lease(name, ttl_ms)
        }
    }

    #[test]
    fn conditional_create_race_is_replayed_or_reported_as_corruption() {
        let bucket = BucketId::new([2; 32]);
        let object = ObjectId::new([3; 32]);
        let body = ObjectBytes::new(b"same".to_vec()).unwrap();

        let replay = StorageCoreObjectStore::new(AdversarialBackend::new(
            AdversarialBehavior::ObjectRace(b"same".to_vec()),
        ));
        replay.initialize(&VaultLocator::new([1; 32])).unwrap();
        assert_eq!(
            replay.put_immutable(&bucket, &object, &body),
            Ok(PutImmutableOutcome::AlreadyPresent)
        );

        let conflict = StorageCoreObjectStore::new(AdversarialBackend::new(
            AdversarialBehavior::ObjectRace(b"different".to_vec()),
        ));
        conflict.initialize(&VaultLocator::new([1; 32])).unwrap();
        assert_eq!(
            conflict.put_immutable(&bucket, &object, &body),
            Err(StoreError::Corruption)
        );
    }

    #[test]
    fn binding_create_race_accepts_only_the_same_locator() {
        let locator = VaultLocator::new([1; 32]);
        let matching = StorageCoreObjectStore::new(AdversarialBackend::new(
            AdversarialBehavior::BindingRace(locator.as_bytes().to_vec()),
        ));
        assert_eq!(matching.initialize(&locator), Ok(()));

        let mismatched = StorageCoreObjectStore::new(AdversarialBackend::new(
            AdversarialBehavior::BindingRace(vec![2; 32]),
        ));
        assert_eq!(mismatched.initialize(&locator), Err(StoreError::Conflict));
    }

    #[test]
    fn malformed_successful_put_is_corruption() {
        let store =
            StorageCoreObjectStore::new(AdversarialBackend::new(AdversarialBehavior::MalformedPut));
        store.initialize(&VaultLocator::new([1; 32])).unwrap();
        assert_eq!(
            store.put_immutable(
                &BucketId::new([2; 32]),
                &ObjectId::new([3; 32]),
                &ObjectBytes::new(vec![4]).unwrap(),
            ),
            Err(StoreError::Corruption)
        );
    }

    fn record(bucket: &BucketId, object: &ObjectId) -> StorageRecord {
        StorageRecord::new(
            bucket_namespace(bucket),
            object_key(object),
            Revision::new("revision").unwrap(),
            OPAQUE_CONTENT_TYPE,
            empty_metadata(),
            vec![1],
            1,
            1,
        )
        .unwrap()
    }

    fn list_with_page(page: StoragePage, limit: usize) -> Result<ObjectPage, StoreError> {
        let store =
            StorageCoreObjectStore::new(AdversarialBackend::new(AdversarialBehavior::List(page)));
        store.initialize(&VaultLocator::new([1; 32])).unwrap();
        store.list(&BucketId::new([2; 32]), None, limit)
    }

    #[test]
    fn malformed_backend_pages_are_corruption() {
        let bucket = BucketId::new([2; 32]);
        let first = ObjectId::new([3; 32]);
        let second = ObjectId::new([4; 32]);

        assert_eq!(
            list_with_page(
                StoragePage {
                    records: vec![record(&bucket, &first), record(&bucket, &second)],
                    next_cursor: None,
                },
                1,
            ),
            Err(StoreError::Corruption)
        );

        let mut wrong_namespace = record(&bucket, &first);
        wrong_namespace.namespace = "wrong-namespace".into();
        assert_eq!(
            list_with_page(
                StoragePage {
                    records: vec![wrong_namespace],
                    next_cursor: None,
                },
                1,
            ),
            Err(StoreError::Corruption)
        );

        assert_eq!(
            list_with_page(
                StoragePage {
                    records: vec![record(&bucket, &second), record(&bucket, &first)],
                    next_cursor: None,
                },
                2,
            ),
            Err(StoreError::Corruption)
        );

        assert_eq!(
            list_with_page(
                StoragePage {
                    records: vec![record(&bucket, &first)],
                    next_cursor: Some(object_key(&second)),
                },
                1,
            ),
            Err(StoreError::Corruption)
        );
    }

    #[test]
    fn defensive_helpers_reject_malformed_backend_values() {
        let body = ObjectBytes::new(vec![1]).unwrap();
        assert_eq!(
            StorageCoreObjectStore::<InMemoryStorageBackend>::existing_outcome(None, &body),
            Err(StoreError::Conflict)
        );

        let mut invalid_key = "0".repeat(64);
        invalid_key.replace_range(0..1, "g");
        assert_eq!(parse_object_key(&invalid_key), Err(StoreError::Corruption));

        let oversized = StorageStat {
            namespace: "namespace".into(),
            key: "key".into(),
            revision: Revision::new("revision").unwrap(),
            content_type: OPAQUE_CONTENT_TYPE.into(),
            metadata: empty_metadata(),
            body_len: MAX_OBJECT_BYTES + 1,
            created_at: 1,
            updated_at: 1,
            content_hash: [0; 32],
        };
        assert_eq!(object_stat(&oversized), Err(StoreError::Corruption));
    }

    #[test]
    fn storage_errors_map_without_backend_controlled_text() {
        let errors = [
            map_storage_error(StorageError::Conflict {
                namespace: "needle-namespace".into(),
                key: "needle-key".into(),
                expected_revision: Some("needle-expected".into()),
                actual_revision: Some("needle-actual".into()),
            }),
            map_storage_error(StorageError::Unavailable {
                message: "needle-unavailable".into(),
            }),
            map_storage_error(StorageError::NotFound {
                namespace: "needle-namespace".into(),
                key: "needle-key".into(),
            }),
            map_storage_error(StorageError::LeaseDenied {
                name: "needle-lease".into(),
            }),
            map_storage_error(StorageError::Validation {
                field: "needle-field".into(),
                message: "needle-validation".into(),
            }),
            map_storage_error(StorageError::Backend {
                message: "needle-backend".into(),
            }),
        ];
        assert_eq!(errors[0], StoreError::Conflict);
        assert_eq!(errors[1], StoreError::Network);
        assert!(errors[2..]
            .iter()
            .all(|error| *error == StoreError::Provider));
        assert!(!format!("{errors:?}").contains("needle"));
    }
}
