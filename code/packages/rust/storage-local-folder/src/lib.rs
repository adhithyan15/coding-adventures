//! # storage-local-folder
//!
//! D18A names the first durable backend `storage-local-folder`: a portable
//! `StorageBackend` that persists Chief of Staff store records under a
//! caller-supplied local directory.
//!
//! The repository already has the lower-level STR-FILE implementation in
//! `coding_adventures_storage_fs`. This crate is the D18A-facing adapter over
//! that implementation, keeping one file format while exposing the package name
//! and type expected by the Chief of Staff store spec.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::path::PathBuf;

use coding_adventures_storage_fs::{
    fs_storage_backend_summary, FsStorageBackend, FsStorageBackendSummary,
};
use storage_core::{
    Revision, StorageBackend, StorageError, StorageLease, StorageListOptions, StoragePage,
    StoragePutInput, StorageRecord, StorageRecordSummary, StorageStat, StorageSummaryPage,
};

/// Payload-free summary of the local-folder backend's storage guarantees.
///
/// The summary intentionally mirrors the underlying STR-FILE surface while
/// omitting the root path and all record contents.
pub type LocalFolderStorageBackendSummary = FsStorageBackendSummary;

/// Return a payload-free description of the local-folder backend surface.
pub const fn local_folder_storage_backend_summary() -> LocalFolderStorageBackendSummary {
    fs_storage_backend_summary()
}

/// D18A local-folder backend for Chief of Staff stores.
///
/// This is a thin, spec-named adapter over `FsStorageBackend`. It is useful when
/// higher-level stores want to depend on the D18A backend vocabulary rather than
/// the lower-level STR-FILE package name.
pub struct LocalFolderStorageBackend {
    inner: FsStorageBackend,
}

impl LocalFolderStorageBackend {
    /// Create a backend rooted at the supplied local folder.
    ///
    /// The directory is created by `initialize()`, which is called by the
    /// higher-level stores before each operation.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            inner: FsStorageBackend::new(root),
        }
    }

    /// Wrap an existing STR-FILE backend.
    pub fn from_fs_backend(inner: FsStorageBackend) -> Self {
        Self { inner }
    }

    /// Borrow the wrapped STR-FILE backend.
    pub fn fs_backend(&self) -> &FsStorageBackend {
        &self.inner
    }

    /// Consume this adapter and return the wrapped STR-FILE backend.
    pub fn into_fs_backend(self) -> FsStorageBackend {
        self.inner
    }

    /// Describe storage guarantees without exposing the backend root path.
    pub fn surface_summary(&self) -> LocalFolderStorageBackendSummary {
        local_folder_storage_backend_summary()
    }
}

impl StorageBackend for LocalFolderStorageBackend {
    fn initialize(&self) -> Result<(), StorageError> {
        self.inner.initialize()
    }

    fn get(&self, namespace: &str, key: &str) -> Result<Option<StorageRecord>, StorageError> {
        self.inner.get(namespace, key)
    }

    fn put(&self, input: StoragePutInput) -> Result<StorageRecord, StorageError> {
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
        self.inner.list(namespace, options)
    }

    fn stat(&self, namespace: &str, key: &str) -> Result<Option<StorageStat>, StorageError> {
        self.inner.stat(namespace, key)
    }

    fn get_summary(
        &self,
        namespace: &str,
        key: &str,
    ) -> Result<Option<StorageRecordSummary>, StorageError> {
        self.inner.get_summary(namespace, key)
    }

    fn list_summaries(
        &self,
        namespace: &str,
        options: StorageListOptions,
    ) -> Result<StorageSummaryPage, StorageError> {
        self.inner.list_summaries(namespace, options)
    }

    fn acquire_lease(&self, name: &str, ttl_ms: u64) -> Result<Option<StorageLease>, StorageError> {
        self.inner.acquire_lease(name, ttl_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_json_value::JsonValue;
    use context_store::{AppendEntryInput, ContextEntryKind, ContextStore, CreateSessionInput};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};
    use storage_core::conformance;

    fn temp_root(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("test clock should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "storage-local-folder-{name}-{}-{nanos}",
            std::process::id()
        ))
    }

    fn with_backend<T>(name: &str, test: impl FnOnce(&LocalFolderStorageBackend) -> T) -> T {
        let root = temp_root(name);
        let backend = LocalFolderStorageBackend::new(&root);
        let result = test(&backend);
        let _ = fs::remove_dir_all(&root);
        result
    }

    #[test]
    fn surface_summary_matches_underlying_local_file_backend_without_root_path() {
        let root = temp_root("summary").join("private-chief-root");
        let backend = LocalFolderStorageBackend::new(&root);

        let summary = backend.surface_summary();
        assert_eq!(summary, local_folder_storage_backend_summary());
        assert_eq!(summary.record_magic, "STRF");
        assert!(summary.one_file_per_record);
        assert!(summary.atomic_write_rename);
        assert!(summary.tmp_files_cleaned_on_initialize);
        assert!(summary.content_opaque_to_backend);
        assert!(!format!("{summary:?}").contains("private-chief-root"));
    }

    #[test]
    fn conformance_initialize_twice_is_safe() {
        with_backend("initialize", |backend| {
            conformance::initialize_twice_is_safe(backend).unwrap();
        });
    }

    #[test]
    fn conformance_put_then_get_round_trips() {
        with_backend("round-trip", |backend| {
            conformance::put_then_get_round_trips(backend).unwrap();
        });
    }

    #[test]
    fn conformance_stale_revision_is_rejected() {
        with_backend("stale-revision", |backend| {
            conformance::stale_revision_is_rejected(backend).unwrap();
        });
    }

    #[test]
    fn conformance_create_if_absent_rejects_existing() {
        with_backend("create-if-absent", |backend| {
            conformance::create_if_absent_rejects_existing(backend).unwrap();
        });
    }

    #[test]
    fn conformance_concurrent_create_if_absent_has_one_winner() {
        let root = temp_root("concurrent-create-if-absent");
        let backend = std::sync::Arc::new(LocalFolderStorageBackend::new(&root));
        conformance::concurrent_create_if_absent_has_one_winner(backend).unwrap();
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn conformance_multiple_write_conditions_are_rejected() {
        with_backend("multiple-write-conditions", |backend| {
            conformance::multiple_write_conditions_are_rejected(backend).unwrap();
        });
    }

    #[test]
    fn conformance_delete_is_idempotent() {
        with_backend("delete", |backend| {
            conformance::delete_is_idempotent(backend).unwrap();
        });
    }

    #[test]
    fn conformance_prefix_listing_is_stable() {
        with_backend("listing", |backend| {
            conformance::prefix_listing_is_stable(backend).unwrap();
        });
    }

    #[test]
    fn conformance_advisory_lease_expires() {
        with_backend("lease", |backend| {
            conformance::advisory_lease_expires(backend).unwrap();
        });
    }

    #[test]
    fn context_store_persists_session_across_backend_rebuild() {
        let root = temp_root("context-store");

        {
            let store = ContextStore::new(LocalFolderStorageBackend::new(&root));
            let session = store
                .create_session(CreateSessionInput {
                    session_id: "umbrella-today".to_string(),
                    owner_id: "chief".to_string(),
                    title: "Umbrella Today".to_string(),
                })
                .unwrap();
            assert_eq!(session.session_id, "umbrella-today");

            let entry = store
                .append_entry(
                    "umbrella-today",
                    AppendEntryInput {
                        entry_id: "entry-1".to_string(),
                        kind: ContextEntryKind::Assistant,
                        timestamp: Some(1_778_900_000),
                        metadata: JsonValue::Object(vec![]),
                        body: JsonValue::String("Bring an umbrella.".to_string()),
                    },
                )
                .unwrap();
            assert_eq!(entry.entry_id, "entry-1");
        }

        {
            let reopened = ContextStore::new(LocalFolderStorageBackend::new(&root));
            let session = reopened
                .open_session("umbrella-today")
                .unwrap()
                .expect("session should survive backend rebuild");
            assert_eq!(session.title, "Umbrella Today");

            let entries = reopened.fetch_ordered_entries("umbrella-today").unwrap();
            assert_eq!(entries.len(), 1);
            assert_eq!(
                entries[0].body,
                JsonValue::String("Bring an umbrella.".to_string())
            );
        }

        let _ = fs::remove_dir_all(&root);
    }
}
