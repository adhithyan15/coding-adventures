use crate::{encode_item_revision, ApplicationError};
use coding_adventures_ct_compare::ct_eq_fixed;
use coding_adventures_sha256::sha256;
use coding_adventures_vault_pm_domain::{
    ItemCandidate, ItemDocument, ItemId, ItemState, ObservedSet, RevisionId, Tombstone,
};
use coding_adventures_vault_pm_format::VaultId;
use coding_adventures_zeroize::{Zeroize, Zeroizing};
use core::fmt::{self, Debug, Formatter};
use std::collections::{BTreeMap, BTreeSet};

const CANDIDATE_DOMAIN: &[u8] = b"VPM-PORTABLE-RESTORE-CANDIDATE-v1";
const GROUP_DOMAIN: &[u8] = b"VPM-PORTABLE-RESTORE-GROUP-v1";
const ROOT_DOMAIN: &[u8] = b"VPM-PORTABLE-RESTORE-ROOT-v1";

/// Opaque source semantics retained across one cross-vault import.
///
/// The token binds every current candidate group after normalizing only the
/// item identity that import is required to replace. It exposes no source
/// identity, schema, timestamp, record value, deletion time, or digest.
pub struct PortableRestoreExpectationV1 {
    source_vault_id: VaultId,
    source_item_ids: BTreeSet<ItemId>,
    source_revision_ids: BTreeSet<RevisionId>,
    semantic_root: [u8; 32],
    item_count: usize,
    candidate_count: usize,
    conflicted_item_count: usize,
}

impl PortableRestoreExpectationV1 {
    pub(crate) fn from_source(
        source_vault_id: VaultId,
        source_items: &BTreeMap<ItemId, Vec<ItemCandidate>>,
    ) -> Result<Self, ApplicationError> {
        let source_item_ids = source_items.keys().copied().collect();
        let source_revision_ids = source_items
            .values()
            .flatten()
            .map(ItemCandidate::revision_id)
            .collect();
        let (semantic_root, item_count, candidate_count, conflicted_item_count) =
            portable_semantic_root(source_items)?;
        Ok(Self {
            source_vault_id,
            source_item_ids,
            source_revision_ids,
            semantic_root,
            item_count,
            candidate_count,
            conflicted_item_count,
        })
    }

    pub(crate) fn verify_target(
        &self,
        target_vault_id: VaultId,
        target_items: &BTreeMap<ItemId, Vec<ItemCandidate>>,
    ) -> Result<PortableRestoreVerificationV1, ApplicationError> {
        if target_vault_id == self.source_vault_id
            || target_items
                .keys()
                .any(|item_id| self.source_item_ids.contains(item_id))
            || target_items.values().flatten().any(|candidate| {
                self.source_revision_ids.contains(&candidate.revision_id())
                    || !candidate.causal_parents().is_empty()
            })
        {
            return Err(ApplicationError::IntegrityFailure);
        }
        let (semantic_root, item_count, candidate_count, conflicted_item_count) =
            portable_semantic_root(target_items)?;
        if item_count != self.item_count
            || candidate_count != self.candidate_count
            || conflicted_item_count != self.conflicted_item_count
            || !ct_eq_fixed(&semantic_root, &self.semantic_root)
        {
            return Err(ApplicationError::IntegrityFailure);
        }
        Ok(PortableRestoreVerificationV1 {
            item_count,
            candidate_count,
            conflicted_item_count,
        })
    }
}

impl Drop for PortableRestoreExpectationV1 {
    fn drop(&mut self) {
        self.semantic_root.zeroize();
        self.source_item_ids.clear();
        self.source_revision_ids.clear();
    }
}

impl Debug for PortableRestoreExpectationV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("PortableRestoreExpectationV1(<redacted>)")
    }
}

/// Aggregate proof released only after semantic comparison and audit publication.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PortableRestoreVerificationV1 {
    item_count: usize,
    candidate_count: usize,
    conflicted_item_count: usize,
}

impl PortableRestoreVerificationV1 {
    /// Return the independently matched item-group count.
    pub const fn item_count(&self) -> usize {
        self.item_count
    }

    /// Return the independently matched current-candidate count.
    pub const fn candidate_count(&self) -> usize {
        self.candidate_count
    }

    /// Return the independently matched conflicted-item count.
    pub const fn conflicted_item_count(&self) -> usize {
        self.conflicted_item_count
    }
}

impl Debug for PortableRestoreVerificationV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PortableRestoreVerificationV1")
            .field("item_count", &self.item_count)
            .field("candidate_count", &self.candidate_count)
            .field("conflicted_item_count", &self.conflicted_item_count)
            .finish()
    }
}

/// Rewrite one item state for a cross-vault import, or for the normalization
/// that makes source and target comparable.
///
/// # Attachments do not cross a portable boundary
///
/// The attachment membership and its manifest references are both dropped,
/// because VLT-PM17's snapshot carries records and not blobs: the chunk and
/// manifest objects an attachment id names exist only in the source vault's
/// repository, so carrying the references across would produce an item that
/// claims attachments no `attachment export` in the target could ever find.
/// Dropping is the honest projection of what actually travelled, and it is
/// consistent with the identities this function already replaces.
///
/// Because `portable_semantic_root` normalizes the source *and* the target
/// through this same function, dropping here does not weaken VLT-PM19/VLT-PM20
/// restore verification — it removes attachments from the compared closure on
/// both sides, which is exactly the claim the comparison should be making.
/// VLT-PM47 §8.3 records what carrying them would require.
pub(crate) fn remap_imported_item_state(
    state: &ItemState,
    item_id: ItemId,
) -> Result<ItemState, ApplicationError> {
    match state {
        ItemState::Live(document) => ItemDocument::new(
            item_id,
            document.schema().clone(),
            document.created_at_ms(),
            document.updated_at_ms(),
            document.favorite().clone(),
            document.collection_ids().clone(),
            document.tags().clone(),
            document.payload().clone(),
            ObservedSet::new(),
            BTreeMap::new(),
        )
        .map(|document| ItemState::Live(Box::new(document)))
        .map_err(|_| ApplicationError::IntegrityFailure),
        ItemState::Tombstone(tombstone) => Ok(ItemState::Tombstone(Tombstone {
            item_id,
            deleted_at_ms: tombstone.deleted_at_ms,
        })),
    }
}

fn portable_semantic_root(
    items: &BTreeMap<ItemId, Vec<ItemCandidate>>,
) -> Result<([u8; 32], usize, usize, usize), ApplicationError> {
    let mut group_hashes = Vec::with_capacity(items.len());
    let mut candidate_count = 0usize;
    let mut conflicted_item_count = 0usize;
    for (item_id, candidates) in items {
        if candidates.is_empty()
            || candidates
                .iter()
                .any(|candidate| candidate.item_id() != *item_id)
        {
            return Err(ApplicationError::IntegrityFailure);
        }
        candidate_count = candidate_count
            .checked_add(candidates.len())
            .ok_or(ApplicationError::BoundExceeded)?;
        conflicted_item_count += usize::from(candidates.len() > 1);
        let mut candidate_hashes = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            let normalized = remap_imported_item_state(candidate.state(), ItemId::new([0; 16]))?;
            let encoded = Zeroizing::new(encode_item_revision(&BTreeSet::new(), &normalized)?);
            let mut preimage = Zeroizing::new(Vec::with_capacity(
                CANDIDATE_DOMAIN.len() + 8 + encoded.len(),
            ));
            preimage.extend_from_slice(CANDIDATE_DOMAIN);
            append_count(&mut preimage, encoded.len())?;
            preimage.extend_from_slice(&encoded);
            candidate_hashes.push(sha256(&preimage));
        }
        candidate_hashes.sort_unstable();
        let mut group_preimage = Zeroizing::new(Vec::with_capacity(
            GROUP_DOMAIN.len() + 8 + 32 * candidates.len(),
        ));
        group_preimage.extend_from_slice(GROUP_DOMAIN);
        append_count(&mut group_preimage, candidates.len())?;
        for candidate_hash in candidate_hashes {
            group_preimage.extend_from_slice(&candidate_hash);
        }
        group_hashes.push(sha256(&group_preimage));
    }
    group_hashes.sort_unstable();
    let mut root_preimage = Zeroizing::new(Vec::with_capacity(
        ROOT_DOMAIN.len() + 24 + 32 * items.len(),
    ));
    root_preimage.extend_from_slice(ROOT_DOMAIN);
    append_count(&mut root_preimage, items.len())?;
    append_count(&mut root_preimage, candidate_count)?;
    append_count(&mut root_preimage, conflicted_item_count)?;
    for group_hash in group_hashes {
        root_preimage.extend_from_slice(&group_hash);
    }
    Ok((
        sha256(&root_preimage),
        items.len(),
        candidate_count,
        conflicted_item_count,
    ))
}

fn append_count(output: &mut Vec<u8>, count: usize) -> Result<(), ApplicationError> {
    output.extend_from_slice(
        &u64::try_from(count)
            .map_err(|_| ApplicationError::BoundExceeded)?
            .to_be_bytes(),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tombstone_candidate(
        item: u8,
        revision: u8,
        deleted_at_ms: u64,
        parents: impl IntoIterator<Item = u8>,
    ) -> ItemCandidate {
        ItemCandidate::new(
            RevisionId::new([revision; 32]),
            parents
                .into_iter()
                .map(|parent| RevisionId::new([parent; 32])),
            ItemState::Tombstone(Tombstone {
                item_id: ItemId::new([item; 16]),
                deleted_at_ms,
            }),
        )
        .unwrap()
    }

    fn one_item(candidate: ItemCandidate) -> BTreeMap<ItemId, Vec<ItemCandidate>> {
        BTreeMap::from([(candidate.item_id(), vec![candidate])])
    }

    #[test]
    fn semantic_verification_normalizes_only_cross_vault_identities() {
        let source = one_item(tombstone_candidate(1, 2, 73, [9]));
        let expectation =
            PortableRestoreExpectationV1::from_source(VaultId::new([3; 16]), &source).unwrap();

        let exact_target = one_item(tombstone_candidate(4, 5, 73, []));
        let verified = expectation
            .verify_target(VaultId::new([6; 16]), &exact_target)
            .unwrap();
        assert_eq!(verified.item_count(), 1);
        assert_eq!(verified.candidate_count(), 1);
        assert_eq!(verified.conflicted_item_count(), 0);

        let changed_deletion = one_item(tombstone_candidate(4, 5, 74, []));
        assert_eq!(
            expectation.verify_target(VaultId::new([6; 16]), &changed_deletion),
            Err(ApplicationError::IntegrityFailure)
        );
        let retained_parent = one_item(tombstone_candidate(4, 5, 73, [7]));
        assert_eq!(
            expectation.verify_target(VaultId::new([6; 16]), &retained_parent),
            Err(ApplicationError::IntegrityFailure)
        );
        let reused_item = one_item(tombstone_candidate(1, 5, 73, []));
        assert_eq!(
            expectation.verify_target(VaultId::new([6; 16]), &reused_item),
            Err(ApplicationError::IntegrityFailure)
        );
        let reused_revision = one_item(tombstone_candidate(4, 2, 73, []));
        assert_eq!(
            expectation.verify_target(VaultId::new([6; 16]), &reused_revision),
            Err(ApplicationError::IntegrityFailure)
        );
        assert_eq!(
            expectation.verify_target(VaultId::new([3; 16]), &exact_target),
            Err(ApplicationError::IntegrityFailure)
        );
    }
}
