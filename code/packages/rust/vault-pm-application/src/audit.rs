use crate::{
    decode_device_certificate, decode_signed_audit_event, open_object, ActiveStateV1,
    ApplicationError, ApplicationRepository, ApplicationRepositoryError, CatalogV1, ObjectKind,
    V1Keys,
};
use coding_adventures_vault_pm_audit::{AuditActionV1, AuditEventV1, AuditOutcomeV1};
use coding_adventures_vault_pm_domain::{ItemId, OperationId, RevisionId};
use coding_adventures_vault_pm_format::ObjectId;
use coding_adventures_vault_pm_repository::CommitSummary;
use core::fmt::{self, Debug, Formatter};
use std::collections::{BTreeMap, BTreeSet};

/// Default number of newest audit events returned by a history list.
pub const DEFAULT_AUDIT_HISTORY_LIMIT: usize = 100;
/// Hard maximum number of audit events returned by one history list.
pub const MAX_AUDIT_HISTORY_LIMIT: usize = 4_096;

/// Verified secret-free projection of one signed operation-audit event.
#[derive(Clone, PartialEq, Eq)]
pub struct AuditEventViewV1 {
    trace_id: OperationId,
    device_counter: u64,
    action: AuditActionV1,
    outcome: AuditOutcomeV1,
    item_id: Option<ItemId>,
    selected_revision: Option<RevisionId>,
    result_revision: Option<RevisionId>,
    timestamp_ms: u64,
}

impl AuditEventViewV1 {
    fn from_event(event: &AuditEventV1) -> Self {
        Self {
            trace_id: event.trace_id(),
            device_counter: event.device_counter(),
            action: event.action(),
            outcome: event.outcome(),
            item_id: event.item_id(),
            selected_revision: event.selected_revision(),
            result_revision: event.result_revision(),
            timestamp_ms: event.timestamp_ms(),
        }
    }

    /// Return the random correlation identity for this operation.
    pub const fn trace_id(&self) -> OperationId {
        self.trace_id
    }

    /// Return the acting device's monotonic event/commit counter.
    pub const fn device_counter(&self) -> u64 {
        self.device_counter
    }

    /// Return the closed operation action.
    pub const fn action(&self) -> AuditActionV1 {
        self.action
    }

    /// Return the closed operation outcome.
    pub const fn outcome(&self) -> AuditOutcomeV1 {
        self.outcome
    }

    /// Return the stable item identity when this action is item-scoped.
    pub const fn item_id(&self) -> Option<ItemId> {
        self.item_id
    }

    /// Return the exact selected revision when the event shape permits it.
    pub const fn selected_revision(&self) -> Option<RevisionId> {
        self.selected_revision
    }

    /// Return the exact result revision for a successful item mutation.
    pub const fn result_revision(&self) -> Option<RevisionId> {
        self.result_revision
    }

    /// Return the caller-supplied advisory wall time.
    pub const fn timestamp_ms(&self) -> u64 {
        self.timestamp_ms
    }
}

impl Debug for AuditEventViewV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuditEventViewV1")
            .field("device_counter", &self.device_counter)
            .field("action", &self.action)
            .field("outcome", &self.outcome)
            .field("item_scoped", &self.item_id.is_some())
            .field("selected_revision", &self.selected_revision.is_some())
            .field("result_revision", &self.result_revision.is_some())
            .field("timestamp_ms", &self.timestamp_ms)
            .finish()
    }
}

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
    audit_verify_with_events(active, keys, repository).map(|(report, _)| report)
}

pub(crate) fn audit_history(
    active: &ActiveStateV1,
    keys: &V1Keys,
    repository: &dyn ApplicationRepository,
    limit: usize,
) -> Result<Vec<AuditEventViewV1>, ApplicationError> {
    if limit == 0 || limit > MAX_AUDIT_HISTORY_LIMIT {
        return Err(ApplicationError::BoundExceeded);
    }
    let (_, mut events) = audit_verify_with_events(active, keys, repository)?;
    events.truncate(limit);
    Ok(events)
}

pub(crate) fn audit_event(
    active: &ActiveStateV1,
    keys: &V1Keys,
    repository: &dyn ApplicationRepository,
    trace_id: OperationId,
) -> Result<Option<AuditEventViewV1>, ApplicationError> {
    let (_, events) = audit_verify_with_events(active, keys, repository)?;
    Ok(events.into_iter().find(|event| event.trace_id == trace_id))
}

fn audit_verify_with_events(
    active: &ActiveStateV1,
    keys: &V1Keys,
    repository: &dyn ApplicationRepository,
) -> Result<(AuditVerificationV1, Vec<AuditEventViewV1>), ApplicationError> {
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
    let audit_events = verify_audit_chain(active, keys, repository, &commits_by_counter)?;
    let report = AuditVerificationV1 {
        announcement_count: report.announcement_count(),
        commit_count: commits.len(),
        catalog_count: catalogs.len(),
        revision_count: revisions.len(),
        item_count: items.len(),
        audit_event_count: audit_events.len(),
    };
    Ok((report, audit_events))
}

fn verify_audit_chain(
    active: &ActiveStateV1,
    keys: &V1Keys,
    repository: &dyn ApplicationRepository,
    commits_by_counter: &BTreeMap<u64, CommitSummary>,
) -> Result<Vec<AuditEventViewV1>, ApplicationError> {
    let Some(mut event_id) = active.audit_event_head() else {
        return Ok(Vec::new());
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
    let mut event_views = Vec::new();
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
        event_views.push(AuditEventViewV1::from_event(event));

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
    Ok(event_views)
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
    use coding_adventures_vault_pm_format::{DeviceId, VaultId};

    fn redacted_view() -> AuditEventViewV1 {
        AuditEventViewV1::from_event(
            &AuditEventV1::new(
                VaultId::new([1; 16]),
                DeviceId::new([2; 16]),
                7,
                OperationId::new([3; 32]),
                AuditActionV1::ItemUpdate,
                AuditOutcomeV1::Succeeded,
                Some(ItemId::new([4; 16])),
                Some(RevisionId::new([5; 32])),
                Some(RevisionId::new([6; 32])),
                Some(ObjectId::new([7; 32])),
                vec![ObjectId::new([8; 32])],
                1_700_000_000_000,
            )
            .unwrap(),
        )
    }

    #[test]
    fn event_view_is_useful_but_debug_redacts_stable_identities() {
        let view = redacted_view();
        assert_eq!(view.action().label(), "item_update");
        assert_eq!(view.outcome().label(), "succeeded");
        assert_eq!(view.device_counter(), 7);
        assert_eq!(view.timestamp_ms(), 1_700_000_000_000);
        assert!(view.item_id().is_some());
        assert!(view.selected_revision().is_some());
        assert!(view.result_revision().is_some());

        let debug = format!("{view:?}");
        for forbidden in [
            view.trace_id().to_user_string(),
            view.item_id().unwrap().to_user_string(),
            view.selected_revision().unwrap().to_user_string(),
            view.result_revision().unwrap().to_user_string(),
        ] {
            assert!(!debug.contains(&forbidden));
        }
    }

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
