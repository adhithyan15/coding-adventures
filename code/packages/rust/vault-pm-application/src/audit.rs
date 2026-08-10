use crate::{
    open_object, ActiveStateV1, ApplicationError, ApplicationRepository,
    ApplicationRepositoryError, CatalogV1, ObjectKind, V1Keys,
};
use coding_adventures_vault_pm_domain::{ItemId, RevisionId};
use core::fmt::{self, Debug, Formatter};
use std::collections::BTreeSet;

/// Secret-free result of a complete unlocked vault integrity audit.
///
/// A report exists only after every check succeeds. Failures return the closed
/// [`ApplicationError`] taxonomy instead of a partial or falsely clean report.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AuditVerificationV1 {
    announcement_count: usize,
    commit_count: usize,
    catalog_count: usize,
    revision_count: usize,
    item_count: usize,
}

impl AuditVerificationV1 {
    /// Return true for a completely verified report.
    pub const fn integrity_verified(&self) -> bool {
        true
    }

    /// Return the number of unique signed announcements verified.
    pub const fn announcement_count(&self) -> usize {
        self.announcement_count
    }

    /// Return the number of reachable signed commits verified.
    pub const fn commit_count(&self) -> usize {
        self.commit_count
    }

    /// Return the number of distinct reachable catalogs decrypted.
    pub const fn catalog_count(&self) -> usize {
        self.catalog_count
    }

    /// Return the number of distinct catalog-referenced revisions decrypted.
    pub const fn revision_count(&self) -> usize {
        self.revision_count
    }

    /// Return the number of distinct item identities found across those catalogs.
    pub const fn item_count(&self) -> usize {
        self.item_count
    }
}

impl Debug for AuditVerificationV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuditVerificationV1")
            .field("integrity_verified", &true)
            .field("announcement_count", &self.announcement_count)
            .field("commit_count", &self.commit_count)
            .field("catalog_count", &self.catalog_count)
            .field("revision_count", &self.revision_count)
            .field("item_count", &self.item_count)
            .finish()
    }
}

pub(crate) fn audit_verify(
    active: &ActiveStateV1,
    keys: &V1Keys,
    repository: &dyn ApplicationRepository,
) -> Result<AuditVerificationV1, ApplicationError> {
    let report = repository
        .open(active.pinned_heads())
        .map_err(map_repository)?;
    if report.fresh_device_unanchored() || report.heads().is_empty() {
        return Err(ApplicationError::IntegrityFailure);
    }

    let mut commits = BTreeSet::new();
    let mut catalogs = BTreeSet::new();
    let mut revisions = BTreeSet::<RevisionId>::new();
    let mut items = BTreeSet::<ItemId>::new();
    let mut local_anchor_verified = false;

    for head_id in report.heads().iter().copied() {
        let history = repository
            .complete_history(head_id)
            .map_err(map_repository)?;
        if history.first().map(|commit| commit.id()) != Some(head_id) {
            return Err(ApplicationError::IntegrityFailure);
        }
        for commit in history {
            if !commits.insert(commit.id()) {
                continue;
            }
            if commit.vault_id() != active.vault_id() {
                return Err(ApplicationError::IntegrityFailure);
            }
            if active.pinned_heads().iter().any(|pin| *pin == commit.id())
                && commit.device_id() == active.device_id()
                && commit.device_counter() == active.last_device_counter()
                && commit.catalog_root() == active.catalog_root()
                && commit.device_certificate() == active.device_certificate_id()
            {
                local_anchor_verified = true;
            }

            let catalog_id = commit.catalog_root();
            if !catalogs.insert(catalog_id) {
                continue;
            }
            let catalog_object = repository.read_object(catalog_id).map_err(map_repository)?;
            if catalog_object.id() != catalog_id {
                return Err(ApplicationError::IntegrityFailure);
            }
            let plaintext = open_object(keys, ObjectKind::Catalog, catalog_object.frame())?;
            let catalog = CatalogV1::decode(&plaintext)?;
            for (item_id, revision_ids) in catalog.entries() {
                items.insert(*item_id);
                for revision_id in revision_ids {
                    if !revisions.insert(*revision_id) {
                        continue;
                    }
                    let candidate = crate::open::read_candidate(keys, repository, *revision_id)?;
                    if candidate.item_id() != *item_id {
                        return Err(ApplicationError::IntegrityFailure);
                    }
                    for parent_id in candidate.causal_parents() {
                        let parent = crate::open::read_candidate(keys, repository, *parent_id)?;
                        if parent.item_id() != *item_id {
                            return Err(ApplicationError::IntegrityFailure);
                        }
                    }
                }
            }
        }
    }

    if !local_anchor_verified || commits.len() != report.commit_count() {
        return Err(ApplicationError::IntegrityFailure);
    }
    Ok(AuditVerificationV1 {
        announcement_count: report.announcement_count(),
        commit_count: commits.len(),
        catalog_count: catalogs.len(),
        revision_count: revisions.len(),
        item_count: items.len(),
    })
}

fn map_repository(error: ApplicationRepositoryError) -> ApplicationError {
    match error {
        ApplicationRepositoryError::NotInitialized => ApplicationError::NotInitialized,
        ApplicationRepositoryError::InvalidInput => ApplicationError::InvalidInput,
        ApplicationRepositoryError::BoundExceeded => ApplicationError::BoundExceeded,
        ApplicationRepositoryError::StorageUnavailable => ApplicationError::StorageUnavailable,
        ApplicationRepositoryError::IntegrityFailure => ApplicationError::IntegrityFailure,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_repository_errors_use_the_closed_application_taxonomy() {
        let cases = [
            (
                ApplicationRepositoryError::NotInitialized,
                ApplicationError::NotInitialized,
            ),
            (
                ApplicationRepositoryError::InvalidInput,
                ApplicationError::InvalidInput,
            ),
            (
                ApplicationRepositoryError::BoundExceeded,
                ApplicationError::BoundExceeded,
            ),
            (
                ApplicationRepositoryError::StorageUnavailable,
                ApplicationError::StorageUnavailable,
            ),
            (
                ApplicationRepositoryError::IntegrityFailure,
                ApplicationError::IntegrityFailure,
            ),
        ];
        for (source, expected) in cases {
            assert_eq!(map_repository(source), expected);
        }
    }
}
