use coding_adventures_vault_pm_format::{AnnouncementV1, CommitV1, ObjectFrameV1, ObjectId};
use coding_adventures_vault_pm_repository::{
    CommitSummary, OpenReport, PinnedHeads, Publication, PublicationReceipt, Repository,
    RepositoryAddress, RepositoryError, RepositoryVerifier, VerificationError, VerifiedObject,
};
use coding_adventures_vault_pm_storage::{
    BackendCapabilities, BucketId, ChangeCursor, ChangePage, DeleteOutcome, ListCursor,
    ObjectBytes, ObjectId as StorageObjectId, ObjectPage, ObjectStat, PutImmutableOutcome,
    StoreError, VaultLocator, VaultObjectStore,
};
use core::fmt::{self, Debug, Display, Formatter};
use std::sync::Arc;

/// Closed, payload-free failure from the erased application repository seam.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ApplicationRepositoryError {
    /// The repository has not been bound to its opaque storage address.
    NotInitialized,
    /// A caller-supplied publication, pin, ID, or limit is invalid.
    InvalidInput,
    /// A fixed repository or application bound would be exceeded.
    BoundExceeded,
    /// The injected provider could not complete an operation.
    StorageUnavailable,
    /// Verification, graph, pin, or immutable-value integrity failed closed.
    IntegrityFailure,
}

impl ApplicationRepositoryError {
    fn label(self) -> &'static str {
        match self {
            Self::NotInitialized => "NotInitialized",
            Self::InvalidInput => "InvalidInput",
            Self::BoundExceeded => "BoundExceeded",
            Self::StorageUnavailable => "StorageUnavailable",
            Self::IntegrityFailure => "IntegrityFailure",
        }
    }
}

impl Debug for ApplicationRepositoryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

impl Display for ApplicationRepositoryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("vault-pm-application repository: ")?;
        formatter.write_str(self.label())
    }
}

impl std::error::Error for ApplicationRepositoryError {}

/// Object-safe application view of the verified VLT-PM04 repository.
pub trait ApplicationRepository: Send + Sync {
    /// Idempotently bind the injected provider to the derived repository address.
    fn initialize(&self) -> Result<(), ApplicationRepositoryError>;

    /// Discover and verify the complete repository view relative to local pins.
    fn open(&self, pins: &PinnedHeads) -> Result<OpenReport, ApplicationRepositoryError>;

    /// Consume and publish one exact already-randomized recovery-journal batch.
    fn publish(
        &self,
        publication: Publication,
        current_heads: &PinnedHeads,
    ) -> Result<PublicationReceipt, ApplicationRepositoryError>;

    /// Read one hash-verified encrypted application object.
    fn read_object(&self, id: ObjectId) -> Result<VerifiedObject, ApplicationRepositoryError>;

    /// Read and verify one commit summary.
    fn read_commit(&self, id: ObjectId) -> Result<CommitSummary, ApplicationRepositoryError>;

    /// Walk bounded verified ancestry from one commit.
    fn history(
        &self,
        start: ObjectId,
        limit: usize,
    ) -> Result<Vec<CommitSummary>, ApplicationRepositoryError>;
}

/// Host-injected constructor used only after unlock derives address and verifier.
pub trait ApplicationRepositoryFactory: Send + Sync {
    /// Connect one mandatory verified repository at its opaque derived address.
    fn connect(
        &self,
        address: RepositoryAddress,
        verifier: Box<dyn RepositoryVerifier>,
    ) -> Result<Box<dyn ApplicationRepository>, ApplicationRepositoryError>;
}

/// Production factory that shares one injected VLT-PM02 object store.
pub struct V1ApplicationRepositoryFactory<S: VaultObjectStore + 'static> {
    store: Arc<S>,
}

impl<S: VaultObjectStore + 'static> V1ApplicationRepositoryFactory<S> {
    /// Own and share one provider-neutral object store across unlocked sessions.
    pub fn new(store: S) -> Self {
        Self {
            store: Arc::new(store),
        }
    }

    /// Reuse an already-shared provider-neutral object store.
    pub fn from_shared(store: Arc<S>) -> Self {
        Self { store }
    }
}

impl<S: VaultObjectStore + 'static> ApplicationRepositoryFactory
    for V1ApplicationRepositoryFactory<S>
{
    fn connect(
        &self,
        address: RepositoryAddress,
        verifier: Box<dyn RepositoryVerifier>,
    ) -> Result<Box<dyn ApplicationRepository>, ApplicationRepositoryError> {
        Ok(Box::new(V1ApplicationRepository {
            repository: Repository::new(
                SharedObjectStore(Arc::clone(&self.store)),
                ErasedVerifier(verifier),
                address,
            ),
        }))
    }
}

impl<S: VaultObjectStore + 'static> Debug for V1ApplicationRepositoryFactory<S> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("V1ApplicationRepositoryFactory(<redacted>)")
    }
}

struct V1ApplicationRepository<S: VaultObjectStore> {
    repository: Repository<SharedObjectStore<S>, ErasedVerifier>,
}

impl<S: VaultObjectStore + 'static> ApplicationRepository for V1ApplicationRepository<S> {
    fn initialize(&self) -> Result<(), ApplicationRepositoryError> {
        self.repository.initialize().map_err(map_repository)
    }

    fn open(&self, pins: &PinnedHeads) -> Result<OpenReport, ApplicationRepositoryError> {
        self.repository.open(pins).map_err(map_repository)
    }

    fn publish(
        &self,
        publication: Publication,
        current_heads: &PinnedHeads,
    ) -> Result<PublicationReceipt, ApplicationRepositoryError> {
        self.repository
            .publish(publication, current_heads)
            .map_err(map_repository)
    }

    fn read_object(&self, id: ObjectId) -> Result<VerifiedObject, ApplicationRepositoryError> {
        self.repository.read_object(id).map_err(map_repository)
    }

    fn read_commit(&self, id: ObjectId) -> Result<CommitSummary, ApplicationRepositoryError> {
        self.repository.read_commit(id).map_err(map_repository)
    }

    fn history(
        &self,
        start: ObjectId,
        limit: usize,
    ) -> Result<Vec<CommitSummary>, ApplicationRepositoryError> {
        self.repository
            .history(start, limit)
            .map_err(map_repository)
    }
}

struct ErasedVerifier(Box<dyn RepositoryVerifier>);

impl RepositoryVerifier for ErasedVerifier {
    fn verify_commit(
        &self,
        expected: &ObjectId,
        frame: &ObjectFrameV1,
    ) -> Result<CommitV1, VerificationError> {
        self.0.verify_commit(expected, frame)
    }

    fn verify_announcement(&self, bytes: &[u8]) -> Result<AnnouncementV1, VerificationError> {
        self.0.verify_announcement(bytes)
    }
}

struct SharedObjectStore<S: VaultObjectStore>(Arc<S>);

impl<S: VaultObjectStore> VaultObjectStore for SharedObjectStore<S> {
    fn initialize(&self, locator: &VaultLocator) -> Result<(), StoreError> {
        self.0.initialize(locator)
    }

    fn capabilities(&self) -> BackendCapabilities {
        self.0.capabilities()
    }

    fn get(
        &self,
        bucket: &BucketId,
        object: &StorageObjectId,
    ) -> Result<Option<ObjectBytes>, StoreError> {
        self.0.get(bucket, object)
    }

    fn stat(
        &self,
        bucket: &BucketId,
        object: &StorageObjectId,
    ) -> Result<Option<ObjectStat>, StoreError> {
        self.0.stat(bucket, object)
    }

    fn put_immutable(
        &self,
        bucket: &BucketId,
        object: &StorageObjectId,
        bytes: &ObjectBytes,
    ) -> Result<PutImmutableOutcome, StoreError> {
        self.0.put_immutable(bucket, object, bytes)
    }

    fn list(
        &self,
        bucket: &BucketId,
        cursor: Option<&ListCursor>,
        limit: usize,
    ) -> Result<ObjectPage, StoreError> {
        self.0.list(bucket, cursor, limit)
    }

    fn delete_unreferenced(
        &self,
        bucket: &BucketId,
        object: &StorageObjectId,
    ) -> Result<DeleteOutcome, StoreError> {
        self.0.delete_unreferenced(bucket, object)
    }

    fn changes(&self, cursor: Option<&ChangeCursor>) -> Result<Option<ChangePage>, StoreError> {
        self.0.changes(cursor)
    }
}

fn map_repository(error: RepositoryError) -> ApplicationRepositoryError {
    match error {
        RepositoryError::NotInitialized => ApplicationRepositoryError::NotInitialized,
        RepositoryError::InvalidInput => ApplicationRepositoryError::InvalidInput,
        RepositoryError::BoundExceeded => ApplicationRepositoryError::BoundExceeded,
        RepositoryError::Storage => ApplicationRepositoryError::StorageUnavailable,
        RepositoryError::Verification
        | RepositoryError::Corruption
        | RepositoryError::ProviderWithholding
        | RepositoryError::DeviceEquivocation
        | RepositoryError::GraphCycle
        | RepositoryError::PinConflict => ApplicationRepositoryError::IntegrityFailure,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_vault_pm_format::{DeviceId, Signature, VaultId, CRYPTO_SUITE_V1};
    use coding_adventures_vault_pm_storage::{InMemoryObjectStore, PutImmutableOutcome};

    const VALID_SIGNATURE: Signature = Signature::new([9; 64]);

    struct FixtureVerifier;

    impl RepositoryVerifier for FixtureVerifier {
        fn verify_commit(
            &self,
            expected: &ObjectId,
            frame: &ObjectFrameV1,
        ) -> Result<CommitV1, VerificationError> {
            if frame.id().ok().as_ref() != Some(expected) {
                return Err(VerificationError);
            }
            let commit = CommitV1::decode(&frame.ciphertext).map_err(|_| VerificationError)?;
            if commit.signature != VALID_SIGNATURE {
                return Err(VerificationError);
            }
            Ok(commit)
        }

        fn verify_announcement(&self, bytes: &[u8]) -> Result<AnnouncementV1, VerificationError> {
            let announcement = AnnouncementV1::decode(bytes).map_err(|_| VerificationError)?;
            if announcement.signature != VALID_SIGNATURE {
                return Err(VerificationError);
            }
            Ok(announcement)
        }
    }

    fn frame(seed: u8, ciphertext: Vec<u8>) -> ObjectFrameV1 {
        ObjectFrameV1 {
            suite: CRYPTO_SUITE_V1,
            wrap_nonce: [seed; 24],
            wrapped_dek: [seed.wrapping_add(1); 32],
            wrap_tag: [seed.wrapping_add(2); 16],
            payload_nonce: [seed.wrapping_add(3); 24],
            ciphertext,
            payload_tag: [seed.wrapping_add(4); 16],
        }
    }

    fn publication() -> (Publication, ObjectId, ObjectId) {
        let certificate = frame(1, vec![1]);
        let certificate_id = certificate.id().unwrap();
        let catalog = frame(2, vec![2]);
        let catalog_id = catalog.id().unwrap();
        let mut added_objects = vec![certificate_id, catalog_id];
        added_objects.sort_unstable();
        let commit = CommitV1 {
            vault_id: VaultId::new([3; 16]),
            device_id: DeviceId::new([4; 16]),
            device_counter: 1,
            parents: Vec::new(),
            catalog_root: catalog_id,
            added_objects,
            tombstone_root: None,
            wall_time_ms: 5,
            device_certificate: certificate_id,
            signature: VALID_SIGNATURE,
        };
        let commit_frame = frame(3, commit.encode().unwrap());
        let commit_id = commit_frame.id().unwrap();
        let announcement = AnnouncementV1 {
            vault_id: commit.vault_id,
            device_id: commit.device_id,
            device_counter: commit.device_counter,
            commit_id,
            device_certificate: certificate_id,
            signature: VALID_SIGNATURE,
        }
        .encode()
        .unwrap();
        (
            Publication::new(vec![certificate, catalog], commit_frame, announcement),
            commit_id,
            catalog_id,
        )
    }

    #[test]
    fn factory_forwards_the_complete_verified_repository_contract() {
        let shared = Arc::new(InMemoryObjectStore::new());
        let factory = V1ApplicationRepositoryFactory::from_shared(Arc::clone(&shared));
        assert_eq!(
            format!("{factory:?}"),
            "V1ApplicationRepositoryFactory(<redacted>)"
        );
        let repository = factory
            .connect(
                RepositoryAddress::derive(&[7; 32]),
                Box::new(FixtureVerifier),
            )
            .unwrap();
        assert_eq!(
            repository.open(&PinnedHeads::empty()).err(),
            Some(ApplicationRepositoryError::NotInitialized)
        );
        repository.initialize().unwrap();

        let (publication, commit_id, catalog_id) = publication();
        let receipt = repository
            .publish(publication, &PinnedHeads::empty())
            .unwrap();
        assert_eq!(receipt.commit_id(), commit_id);
        assert_eq!(receipt.heads().len(), 1);

        let report = repository.open(receipt.heads()).unwrap();
        assert_eq!(report.heads(), receipt.heads());
        assert!(!report.fresh_device_unanchored());
        assert_eq!(repository.read_object(catalog_id).unwrap().id(), catalog_id);
        assert_eq!(repository.read_commit(commit_id).unwrap().id(), commit_id);
        assert_eq!(repository.history(commit_id, 1).unwrap().len(), 1);
        assert_eq!(
            repository.read_object(ObjectId::new([0xff; 32])).err(),
            Some(ApplicationRepositoryError::IntegrityFailure)
        );
    }

    #[test]
    fn shared_store_delegates_every_provider_operation() {
        let store = SharedObjectStore(Arc::new(InMemoryObjectStore::new()));
        let locator = VaultLocator::new([1; 32]);
        let bucket = BucketId::new([2; 32]);
        let object = StorageObjectId::new([3; 32]);
        let bytes = ObjectBytes::new(vec![4, 5, 6]).unwrap();
        store.initialize(&locator).unwrap();
        assert!(store.capabilities().conditional_create);
        assert_eq!(
            store.put_immutable(&bucket, &object, &bytes).unwrap(),
            PutImmutableOutcome::Created
        );
        assert_eq!(store.get(&bucket, &object).unwrap(), Some(bytes));
        assert!(store.stat(&bucket, &object).unwrap().is_some());
        assert_eq!(store.list(&bucket, None, 10).unwrap().entries.len(), 1);
        assert!(store.changes(None).unwrap().is_some());
        assert_eq!(
            store.delete_unreferenced(&bucket, &object).unwrap(),
            DeleteOutcome::Deleted
        );
    }

    #[test]
    fn factory_constructor_and_error_mapping_are_closed() {
        let factory = V1ApplicationRepositoryFactory::new(InMemoryObjectStore::new());
        let _repository = factory
            .connect(
                RepositoryAddress::derive(&[8; 32]),
                Box::new(FixtureVerifier),
            )
            .unwrap();
        for (source, expected) in [
            (
                RepositoryError::NotInitialized,
                ApplicationRepositoryError::NotInitialized,
            ),
            (
                RepositoryError::InvalidInput,
                ApplicationRepositoryError::InvalidInput,
            ),
            (
                RepositoryError::BoundExceeded,
                ApplicationRepositoryError::BoundExceeded,
            ),
            (
                RepositoryError::Storage,
                ApplicationRepositoryError::StorageUnavailable,
            ),
            (
                RepositoryError::Verification,
                ApplicationRepositoryError::IntegrityFailure,
            ),
            (
                RepositoryError::Corruption,
                ApplicationRepositoryError::IntegrityFailure,
            ),
            (
                RepositoryError::ProviderWithholding,
                ApplicationRepositoryError::IntegrityFailure,
            ),
            (
                RepositoryError::DeviceEquivocation,
                ApplicationRepositoryError::IntegrityFailure,
            ),
            (
                RepositoryError::GraphCycle,
                ApplicationRepositoryError::IntegrityFailure,
            ),
            (
                RepositoryError::PinConflict,
                ApplicationRepositoryError::IntegrityFailure,
            ),
        ] {
            assert_eq!(map_repository(source), expected);
        }
        for (error, label) in [
            (ApplicationRepositoryError::NotInitialized, "NotInitialized"),
            (ApplicationRepositoryError::InvalidInput, "InvalidInput"),
            (ApplicationRepositoryError::BoundExceeded, "BoundExceeded"),
            (
                ApplicationRepositoryError::StorageUnavailable,
                "StorageUnavailable",
            ),
            (
                ApplicationRepositoryError::IntegrityFailure,
                "IntegrityFailure",
            ),
        ] {
            assert_eq!(format!("{error:?}"), label);
            assert_eq!(
                error.to_string(),
                format!("vault-pm-application repository: {label}")
            );
        }
    }
}
