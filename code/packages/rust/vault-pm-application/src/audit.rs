use crate::{
    decode_device_certificate, decode_signed_audit_event, open_object, ActiveStateV1,
    ApplicationError, ApplicationRepository, ApplicationRepositoryError, CatalogV1, ObjectKind,
    V1Keys,
};
use coding_adventures_vault_pm_audit::AuditActionV1;
use coding_adventures_vault_pm_domain::{ItemId, RevisionId};
use coding_adventures_vault_pm_format::ObjectId;
use coding_adventures_vault_pm_repository::CommitSummary;
use core::fmt::{self, Debug, Formatter};
use std::collections::{BTreeMap, BTreeSet};

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
    audit_event_count: usize,
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

    /// Return the number of encrypted operation-audit events verified.
    ///
    /// Pre-audit vaults report zero until an explicit audit epoch is installed.
    pub const fn audit_event_count(&self) -> usize {
        self.audit_event_count
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
            .field("audit_event_count", &self.audit_event_count)
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
    let mut commits_by_counter = BTreeMap::<u64, CommitSummary>::new();
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
            if commit.device_id() != active.device_id()
                || commit.device_certificate() != active.device_certificate_id()
                || commits_by_counter
                    .insert(commit.device_counter(), commit.clone())
                    .is_some()
            {
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
    let audit_event_count = verify_audit_chain(active, keys, repository, &commits_by_counter)?;
    Ok(AuditVerificationV1 {
        announcement_count: report.announcement_count(),
        commit_count: commits.len(),
        catalog_count: catalogs.len(),
        revision_count: revisions.len(),
        item_count: items.len(),
        audit_event_count,
    })
}

fn verify_audit_chain(
    active: &ActiveStateV1,
    keys: &V1Keys,
    repository: &dyn ApplicationRepository,
    commits_by_counter: &BTreeMap<u64, CommitSummary>,
) -> Result<usize, ApplicationError> {
    let Some(mut event_id) = active.audit_event_head() else {
        return Ok(0);
    };

    let certificate_object = repository
        .read_object(active.device_certificate_id())
        .map_err(map_repository)?;
    if certificate_object.id() != active.device_certificate_id() {
        return Err(ApplicationError::IntegrityFailure);
    }
    let certificate_plaintext = open_object(
        keys,
        ObjectKind::DeviceCertificate,
        certificate_object.frame(),
    )?;
    let certificate = decode_device_certificate(&certificate_plaintext)?;
    if certificate.vault_id != active.vault_id() || certificate.device_id != active.device_id() {
        return Err(ApplicationError::IntegrityFailure);
    }
    let signing_public_key = certificate.signing_public_key;

    let mut seen_events = BTreeSet::new();
    let mut seen_counters = BTreeSet::new();
    let mut newer_counter = None;
    let root_counter = loop {
        if seen_events.len() >= commits_by_counter.len() || !seen_events.insert(event_id) {
            return Err(ApplicationError::IntegrityFailure);
        }
        let object = repository.read_object(event_id).map_err(map_repository)?;
        if object.id() != event_id {
            return Err(ApplicationError::IntegrityFailure);
        }
        let plaintext = open_object(keys, ObjectKind::AuditEvent, object.frame())?;
        let signed = decode_signed_audit_event(&plaintext)?;
        signed
            .verify(signing_public_key.as_bytes())
            .map_err(|_| ApplicationError::IntegrityFailure)?;
        let event = signed.event();
        if event.vault_id() != active.vault_id()
            || event.device_id() != active.device_id()
            || !seen_counters.insert(event.device_counter())
            || newer_counter
                .is_some_and(|counter| event.device_counter().checked_add(1) != Some(counter))
        {
            return Err(ApplicationError::IntegrityFailure);
        }
        let commit = commits_by_counter
            .get(&event.device_counter())
            .ok_or(ApplicationError::IntegrityFailure)?;
        verify_event_commit_binding(event_id, event, commit, keys, repository)?;

        match event.previous_event() {
            Some(previous) => {
                newer_counter = Some(event.device_counter());
                event_id = previous;
            }
            None => {
                if !matches!(
                    event.action(),
                    AuditActionV1::AuditEpochStart | AuditActionV1::VaultInitialize
                ) {
                    return Err(ApplicationError::IntegrityFailure);
                }
                break event.device_counter();
            }
        }
    };

    let expected_event_count = active
        .last_device_counter()
        .checked_sub(root_counter)
        .and_then(|distance| distance.checked_add(1))
        .and_then(|count| usize::try_from(count).ok())
        .ok_or(ApplicationError::IntegrityFailure)?;
    if seen_events.len() != expected_event_count
        || seen_counters
            .last()
            .copied()
            .is_none_or(|counter| counter != active.last_device_counter())
        || commits_by_counter
            .range(root_counter..=active.last_device_counter())
            .count()
            != expected_event_count
    {
        return Err(ApplicationError::IntegrityFailure);
    }
    Ok(seen_events.len())
}

fn verify_event_commit_binding(
    event_id: ObjectId,
    event: &coding_adventures_vault_pm_audit::AuditEventV1,
    commit: &CommitSummary,
    keys: &V1Keys,
    repository: &dyn ApplicationRepository,
) -> Result<(), ApplicationError> {
    if commit.parents() != event.basis_heads()
        || commit.wall_time_ms() != event.timestamp_ms()
        || !commit.added_objects().contains(&event_id)
    {
        return Err(ApplicationError::IntegrityFailure);
    }

    for revision_id in [event.selected_revision(), event.result_revision()]
        .into_iter()
        .flatten()
    {
        let object_id = ObjectId::new(*revision_id.as_bytes());
        if event.result_revision() == Some(revision_id)
            && !commit.added_objects().contains(&object_id)
        {
            return Err(ApplicationError::IntegrityFailure);
        }
        let candidate = crate::open::read_candidate(keys, repository, revision_id)?;
        if Some(candidate.item_id()) != event.item_id() {
            return Err(ApplicationError::IntegrityFailure);
        }
    }
    Ok(())
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
