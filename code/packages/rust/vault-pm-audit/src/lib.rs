//! Storage-neutral, signed operation auditing for the password manager.
//!
//! This crate owns the canonical event vocabulary and device signature. It
//! deliberately owns no persistence, encryption-at-rest, clock, entropy, or
//! host behavior. The application can therefore seal and publish an event in
//! the same immutable repository commit as the operation it describes.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_canonical_cbor::{decode, encode, CborValue};
use coding_adventures_ed25519::{generate_keypair, sign, verify};
use coding_adventures_vault_pm_domain::{ItemId, OperationId, RevisionId};
use coding_adventures_vault_pm_format::{
    DeviceId, ObjectId, Signature, VaultId, MAX_COMMIT_PARENTS,
};
use coding_adventures_zeroize::Zeroizing;
use core::fmt::{self, Debug, Display, Formatter};
use std::collections::BTreeMap;

const VERSION: u64 = 1;
const KIND_OPERATION_AUDIT: u64 = 1;
const SIGNING_DOMAIN: &[u8] = b"VPM-AUDIT-EVENT-v1";
const MAX_ENCODED_BYTES: usize = 16 * 1024;

/// Closed password-manager operation vocabulary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuditActionV1 {
    /// Start the first auditable epoch for a pre-audit vault.
    AuditEpochStart,
    /// Create generation zero for a new vault.
    VaultInitialize,
    /// Verify the complete authenticated repository.
    VaultVerify,
    /// Run an authenticated diagnostic operation.
    VaultDiagnose,
    /// Access the redacted operation-audit history itself.
    AuditRead,
    /// Re-wrap the vault root key under a newly collected master passphrase.
    ///
    /// The event is vault-scoped and deliberately carries nothing about the
    /// key material it concerns: no salt, no KDF parameters, no generation
    /// number, and no bootstrap identifier. An audit chain records that a
    /// rotation happened, not the shape of the credential it produced.
    PassphraseRotate,
    /// Create one item.
    ItemCreate,
    /// List redacted current items.
    ItemList,
    /// Access one current item projection or secret field.
    ItemRead,
    /// Replace one current item revision.
    ItemUpdate,
    /// Publish one item tombstone.
    ItemDelete,
    /// Restore one historical live item revision.
    ItemRestore,
    /// Access one item's redacted or revealed history.
    ItemHistoryRead,
    /// Search the unlocked local item projection.
    ItemSearch,
    /// Resolve one current item conflict.
    ItemConflictResolve,
    /// Merge multiple current item candidates into one authored revision.
    ItemConflictMerge,
    /// Import a portable snapshot.
    PortableImport,
    /// Export a portable snapshot.
    PortableExport,
    /// Independently verify one portable restore against its authenticated source snapshot.
    PortableRestoreVerify,
}

impl AuditActionV1 {
    /// Return the stable lowercase label used by redacted audit surfaces.
    pub const fn label(self) -> &'static str {
        match self {
            Self::AuditEpochStart => "audit_epoch_start",
            Self::VaultInitialize => "vault_initialize",
            Self::VaultVerify => "vault_verify",
            Self::VaultDiagnose => "vault_diagnose",
            Self::AuditRead => "audit_read",
            Self::PassphraseRotate => "passphrase_rotate",
            Self::ItemCreate => "item_create",
            Self::ItemList => "item_list",
            Self::ItemRead => "item_read",
            Self::ItemUpdate => "item_update",
            Self::ItemDelete => "item_delete",
            Self::ItemRestore => "item_restore",
            Self::ItemHistoryRead => "item_history_read",
            Self::ItemSearch => "item_search",
            Self::ItemConflictResolve => "item_conflict_resolve",
            Self::ItemConflictMerge => "item_conflict_merge",
            Self::PortableImport => "portable_import",
            Self::PortableExport => "portable_export",
            Self::PortableRestoreVerify => "portable_restore_verify",
        }
    }

    const fn code(self) -> u64 {
        match self {
            Self::AuditEpochStart => 1,
            Self::VaultInitialize => 2,
            Self::VaultVerify => 3,
            Self::VaultDiagnose => 4,
            Self::AuditRead => 5,
            Self::PassphraseRotate => 6,
            Self::ItemCreate => 10,
            Self::ItemList => 11,
            Self::ItemRead => 12,
            Self::ItemUpdate => 13,
            Self::ItemDelete => 14,
            Self::ItemRestore => 15,
            Self::ItemHistoryRead => 16,
            Self::ItemSearch => 17,
            Self::ItemConflictResolve => 18,
            Self::ItemConflictMerge => 19,
            Self::PortableImport => 20,
            Self::PortableExport => 21,
            Self::PortableRestoreVerify => 22,
        }
    }

    fn from_code(code: u64) -> Result<Self, AuditError> {
        match code {
            1 => Ok(Self::AuditEpochStart),
            2 => Ok(Self::VaultInitialize),
            3 => Ok(Self::VaultVerify),
            4 => Ok(Self::VaultDiagnose),
            5 => Ok(Self::AuditRead),
            6 => Ok(Self::PassphraseRotate),
            10 => Ok(Self::ItemCreate),
            11 => Ok(Self::ItemList),
            12 => Ok(Self::ItemRead),
            13 => Ok(Self::ItemUpdate),
            14 => Ok(Self::ItemDelete),
            15 => Ok(Self::ItemRestore),
            16 => Ok(Self::ItemHistoryRead),
            17 => Ok(Self::ItemSearch),
            18 => Ok(Self::ItemConflictResolve),
            19 => Ok(Self::ItemConflictMerge),
            20 => Ok(Self::PortableImport),
            21 => Ok(Self::PortableExport),
            22 => Ok(Self::PortableRestoreVerify),
            _ => Err(AuditError::Unsupported),
        }
    }

    /// Return whether the action changes the logical item collection.
    pub const fn is_item_mutation(self) -> bool {
        matches!(
            self,
            Self::ItemCreate
                | Self::ItemUpdate
                | Self::ItemDelete
                | Self::ItemRestore
                | Self::ItemConflictResolve
                | Self::ItemConflictMerge
        )
    }

    const fn is_item_scoped(self) -> bool {
        matches!(
            self,
            Self::ItemCreate
                | Self::ItemRead
                | Self::ItemUpdate
                | Self::ItemDelete
                | Self::ItemRestore
                | Self::ItemHistoryRead
                | Self::ItemConflictResolve
                | Self::ItemConflictMerge
        )
    }

    const fn selects_revision_on_success(self) -> bool {
        matches!(
            self,
            Self::ItemRead
                | Self::ItemUpdate
                | Self::ItemDelete
                | Self::ItemRestore
                | Self::ItemConflictResolve
        )
    }

    const fn permits_selected_revision(self) -> bool {
        self.selects_revision_on_success() || matches!(self, Self::ItemHistoryRead)
    }
}

/// Closed result class for one attempted authenticated operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuditOutcomeV1 {
    /// The operation completed and its effects, if any, became durable.
    Succeeded,
    /// Authenticated policy or user intent denied the operation.
    Denied,
    /// The operation failed after authentication without completing its effect.
    Failed,
}

impl AuditOutcomeV1 {
    /// Return the stable lowercase label used by redacted audit surfaces.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Denied => "denied",
            Self::Failed => "failed",
        }
    }

    const fn code(self) -> u64 {
        match self {
            Self::Succeeded => 1,
            Self::Denied => 2,
            Self::Failed => 3,
        }
    }

    fn from_code(code: u64) -> Result<Self, AuditError> {
        match code {
            1 => Ok(Self::Succeeded),
            2 => Ok(Self::Denied),
            3 => Ok(Self::Failed),
            _ => Err(AuditError::Unsupported),
        }
    }
}

/// Closed, payload-free audit failures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuditError {
    /// Caller-provided event fields violate the V1 contract.
    InvalidInput,
    /// A fixed V1 collection or byte bound was exceeded.
    BoundExceeded,
    /// The encoded version, action, or outcome is not supported.
    Unsupported,
    /// Canonical structure, identity binding, or field shape is invalid.
    IntegrityFailure,
    /// The acting device signature did not verify.
    SignatureFailure,
}

impl Display for AuditError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::InvalidInput => "invalid input",
            Self::BoundExceeded => "bound exceeded",
            Self::Unsupported => "unsupported",
            Self::IntegrityFailure => "integrity failure",
            Self::SignatureFailure => "signature failure",
        };
        write!(formatter, "vault-pm-audit: {label}")
    }
}

impl std::error::Error for AuditError {}

/// Unsigned canonical operation facts prepared before repository publication.
#[derive(Clone, PartialEq, Eq)]
pub struct AuditEventV1 {
    vault_id: VaultId,
    device_id: DeviceId,
    device_counter: u64,
    trace_id: OperationId,
    action: AuditActionV1,
    outcome: AuditOutcomeV1,
    item_id: Option<ItemId>,
    selected_revision: Option<RevisionId>,
    result_revision: Option<RevisionId>,
    previous_event: Option<ObjectId>,
    basis_heads: Vec<ObjectId>,
    timestamp_ms: u64,
}

impl AuditEventV1 {
    /// Validate and construct one complete V1 operation event.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        vault_id: VaultId,
        device_id: DeviceId,
        device_counter: u64,
        trace_id: OperationId,
        action: AuditActionV1,
        outcome: AuditOutcomeV1,
        item_id: Option<ItemId>,
        selected_revision: Option<RevisionId>,
        result_revision: Option<RevisionId>,
        previous_event: Option<ObjectId>,
        basis_heads: Vec<ObjectId>,
        timestamp_ms: u64,
    ) -> Result<Self, AuditError> {
        let value = Self {
            vault_id,
            device_id,
            device_counter,
            trace_id,
            action,
            outcome,
            item_id,
            selected_revision,
            result_revision,
            previous_event,
            basis_heads,
            timestamp_ms,
        };
        value.validate()?;
        Ok(value)
    }

    /// Return the vault whose encrypted repository will contain the event.
    pub const fn vault_id(&self) -> VaultId {
        self.vault_id
    }

    /// Return the certified acting device.
    pub const fn device_id(&self) -> DeviceId {
        self.device_id
    }

    /// Return the device commit counter reserved for this operation.
    pub const fn device_counter(&self) -> u64 {
        self.device_counter
    }

    /// Return the random correlation identity for the high-level operation.
    pub const fn trace_id(&self) -> OperationId {
        self.trace_id
    }

    /// Return the closed operation label.
    pub const fn action(&self) -> AuditActionV1 {
        self.action
    }

    /// Return the closed result class.
    pub const fn outcome(&self) -> AuditOutcomeV1 {
        self.outcome
    }

    /// Return the redacted stable item identity, when item-scoped.
    pub const fn item_id(&self) -> Option<ItemId> {
        self.item_id
    }

    /// Return the exact revision selected by the operation, when applicable.
    pub const fn selected_revision(&self) -> Option<RevisionId> {
        self.selected_revision
    }

    /// Return the new revision produced by a successful item mutation.
    pub const fn result_revision(&self) -> Option<RevisionId> {
        self.result_revision
    }

    /// Return the preceding per-device audit event object.
    pub const fn previous_event(&self) -> Option<ObjectId> {
        self.previous_event
    }

    /// Return the repository heads observed before the operation.
    pub fn basis_heads(&self) -> &[ObjectId] {
        &self.basis_heads
    }

    /// Return the caller-supplied advisory wall time.
    pub const fn timestamp_ms(&self) -> u64 {
        self.timestamp_ms
    }

    /// Sign the canonical event with the acting device seed.
    pub fn sign(self, device_seed: &[u8; 32]) -> Result<SignedAuditEventV1, AuditError> {
        self.validate()?;
        let (_, device_secret) = generate_keypair(device_seed);
        let device_secret = Zeroizing::new(device_secret);
        let signature = Signature::new(sign(&self.signing_preimage(), &device_secret));
        Ok(SignedAuditEventV1 {
            event: self,
            signature,
        })
    }

    fn validate(&self) -> Result<(), AuditError> {
        if self.device_counter == 0 {
            return Err(AuditError::InvalidInput);
        }
        if self.basis_heads.len() > MAX_COMMIT_PARENTS {
            return Err(AuditError::BoundExceeded);
        }
        if self.basis_heads.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(AuditError::InvalidInput);
        }
        let starts_new_vault = self.action == AuditActionV1::VaultInitialize;
        let starts_audit_epoch = self.action == AuditActionV1::AuditEpochStart;
        if starts_new_vault {
            if self.device_counter != 1
                || self.previous_event.is_some()
                || !self.basis_heads.is_empty()
                || self.outcome != AuditOutcomeV1::Succeeded
            {
                return Err(AuditError::InvalidInput);
            }
        } else if starts_audit_epoch {
            if self.previous_event.is_some()
                || self.basis_heads.is_empty()
                || self.outcome != AuditOutcomeV1::Succeeded
            {
                return Err(AuditError::InvalidInput);
            }
        } else if self.previous_event.is_none() || self.basis_heads.is_empty() {
            return Err(AuditError::InvalidInput);
        }

        if self.action.is_item_scoped() != self.item_id.is_some() {
            return Err(AuditError::InvalidInput);
        }
        if self.selected_revision.is_some() && self.item_id.is_none() {
            return Err(AuditError::InvalidInput);
        }
        if self.result_revision.is_some() && self.item_id.is_none() {
            return Err(AuditError::InvalidInput);
        }
        if self.outcome == AuditOutcomeV1::Succeeded
            && self.action.selects_revision_on_success()
            && self.selected_revision.is_none()
        {
            return Err(AuditError::InvalidInput);
        }
        if self.outcome == AuditOutcomeV1::Succeeded
            && self.action.is_item_mutation()
            && self.result_revision.is_none()
        {
            return Err(AuditError::InvalidInput);
        }
        if (self.outcome != AuditOutcomeV1::Succeeded || !self.action.is_item_mutation())
            && self.result_revision.is_some()
        {
            return Err(AuditError::InvalidInput);
        }
        if !self.action.permits_selected_revision() && self.selected_revision.is_some() {
            return Err(AuditError::InvalidInput);
        }
        Ok(())
    }

    fn signing_preimage(&self) -> Vec<u8> {
        let body = self.to_cbor(None);
        let encoded = encode(&body);
        let mut preimage = Vec::with_capacity(SIGNING_DOMAIN.len() + encoded.len());
        preimage.extend_from_slice(SIGNING_DOMAIN);
        preimage.extend_from_slice(&encoded);
        preimage
    }

    fn to_cbor(&self, signature: Option<Signature>) -> CborValue {
        let mut fields = vec![
            field(1, CborValue::Unsigned(VERSION)),
            field(2, CborValue::Unsigned(KIND_OPERATION_AUDIT)),
            field(3, bytes(self.vault_id.as_bytes())),
            field(4, bytes(self.device_id.as_bytes())),
            field(5, CborValue::Unsigned(self.device_counter)),
            field(6, bytes(self.trace_id.as_bytes())),
            field(7, CborValue::Unsigned(self.action.code())),
            field(8, CborValue::Unsigned(self.outcome.code())),
            field(
                9,
                optional_bytes(self.item_id.map(|value| *value.as_bytes())),
            ),
            field(
                10,
                optional_bytes(self.selected_revision.map(|value| *value.as_bytes())),
            ),
            field(
                11,
                optional_bytes(self.result_revision.map(|value| *value.as_bytes())),
            ),
            field(
                12,
                optional_bytes(self.previous_event.map(|value| *value.as_bytes())),
            ),
            field(
                13,
                CborValue::Array(
                    self.basis_heads
                        .iter()
                        .map(|value| bytes(value.as_bytes()))
                        .collect(),
                ),
            ),
            field(14, CborValue::Unsigned(self.timestamp_ms)),
        ];
        if let Some(signature) = signature {
            fields.push(field(15, bytes(signature.as_bytes())));
        }
        CborValue::Map(fields)
    }
}

impl Debug for AuditEventV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuditEventV1")
            .field("device_counter", &self.device_counter)
            .field("action", &self.action)
            .field("outcome", &self.outcome)
            .field("item_scoped", &self.item_id.is_some())
            .field("selected_revision", &self.selected_revision.is_some())
            .field("result_revision", &self.result_revision.is_some())
            .field("has_previous", &self.previous_event.is_some())
            .field("basis_head_count", &self.basis_heads.len())
            .field("timestamp_ms", &self.timestamp_ms)
            .finish()
    }
}

/// One canonical operation event signed by its certified acting device.
#[derive(Clone, PartialEq, Eq)]
pub struct SignedAuditEventV1 {
    event: AuditEventV1,
    signature: Signature,
}

impl SignedAuditEventV1 {
    /// Borrow the verified or not-yet-verified operation facts.
    pub const fn event(&self) -> &AuditEventV1 {
        &self.event
    }

    /// Borrow the detached device signature.
    pub const fn signature(&self) -> Signature {
        self.signature
    }

    /// Encode the exact closed canonical V1 signed event.
    pub fn encode(&self) -> Vec<u8> {
        encode(&self.event.to_cbor(Some(self.signature)))
    }

    /// Strictly decode one exact closed V1 signed event.
    pub fn decode(encoded: &[u8]) -> Result<Self, AuditError> {
        if encoded.len() > MAX_ENCODED_BYTES {
            return Err(AuditError::BoundExceeded);
        }
        let value = decode(encoded).map_err(|_| AuditError::IntegrityFailure)?;
        let mut fields = value_fields(value, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15])?;
        if take_uint(&mut fields, 1)? != VERSION {
            return Err(AuditError::Unsupported);
        }
        if take_uint(&mut fields, 2)? != KIND_OPERATION_AUDIT {
            return Err(AuditError::IntegrityFailure);
        }
        let vault_id = VaultId::new(take_fixed(&mut fields, 3)?);
        let device_id = DeviceId::new(take_fixed(&mut fields, 4)?);
        let device_counter = take_uint(&mut fields, 5)?;
        let trace_id = OperationId::new(take_fixed(&mut fields, 6)?);
        let action = AuditActionV1::from_code(take_uint(&mut fields, 7)?)?;
        let outcome = AuditOutcomeV1::from_code(take_uint(&mut fields, 8)?)?;
        let item_id = take_optional_fixed(&mut fields, 9)?.map(ItemId::new);
        let selected_revision = take_optional_fixed(&mut fields, 10)?.map(RevisionId::new);
        let result_revision = take_optional_fixed(&mut fields, 11)?.map(RevisionId::new);
        let previous_event = take_optional_fixed(&mut fields, 12)?.map(ObjectId::new);
        let basis_heads = take_array(&mut fields, 13)?
            .into_iter()
            .map(fixed_value::<32>)
            .map(|result| result.map(ObjectId::new))
            .collect::<Result<Vec<_>, _>>()?;
        let timestamp_ms = take_uint(&mut fields, 14)?;
        let signature = Signature::new(take_fixed(&mut fields, 15)?);
        let event = AuditEventV1::new(
            vault_id,
            device_id,
            device_counter,
            trace_id,
            action,
            outcome,
            item_id,
            selected_revision,
            result_revision,
            previous_event,
            basis_heads,
            timestamp_ms,
        )
        .map_err(|error| match error {
            AuditError::BoundExceeded => AuditError::BoundExceeded,
            _ => AuditError::IntegrityFailure,
        })?;
        let signed = Self { event, signature };
        if signed.encode() != encoded {
            return Err(AuditError::IntegrityFailure);
        }
        Ok(signed)
    }

    /// Verify the signature against the public key from the event's certified device.
    pub fn verify(&self, device_public_key: &[u8; 32]) -> Result<(), AuditError> {
        self.event.validate()?;
        verify(
            &self.event.signing_preimage(),
            self.signature.as_bytes(),
            device_public_key,
        )
        .then_some(())
        .ok_or(AuditError::SignatureFailure)
    }
}

impl Debug for SignedAuditEventV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SignedAuditEventV1")
            .field("event", &self.event)
            .field("signature", &"<redacted>")
            .finish()
    }
}

fn field(key: u64, value: CborValue) -> (CborValue, CborValue) {
    (CborValue::Unsigned(key), value)
}

fn bytes(value: &[u8]) -> CborValue {
    CborValue::Bytes(value.to_vec())
}

fn optional_bytes<const N: usize>(value: Option<[u8; N]>) -> CborValue {
    CborValue::Array(value.into_iter().map(|value| bytes(&value)).collect())
}

fn value_fields(
    value: CborValue,
    expected: &[u64],
) -> Result<BTreeMap<u64, CborValue>, AuditError> {
    let CborValue::Map(entries) = value else {
        return Err(AuditError::IntegrityFailure);
    };
    if entries.len() != expected.len() {
        return Err(AuditError::IntegrityFailure);
    }
    let mut fields = BTreeMap::new();
    for (key, value) in entries {
        let CborValue::Unsigned(key) = key else {
            return Err(AuditError::IntegrityFailure);
        };
        if !expected.contains(&key) || fields.insert(key, value).is_some() {
            return Err(AuditError::IntegrityFailure);
        }
    }
    Ok(fields)
}

fn take_uint(fields: &mut BTreeMap<u64, CborValue>, key: u64) -> Result<u64, AuditError> {
    match fields.remove(&key) {
        Some(CborValue::Unsigned(value)) => Ok(value),
        _ => Err(AuditError::IntegrityFailure),
    }
}

fn take_fixed<const N: usize>(
    fields: &mut BTreeMap<u64, CborValue>,
    key: u64,
) -> Result<[u8; N], AuditError> {
    fixed_value(fields.remove(&key).ok_or(AuditError::IntegrityFailure)?)
}

fn fixed_value<const N: usize>(value: CborValue) -> Result<[u8; N], AuditError> {
    match value {
        CborValue::Bytes(value) => value.try_into().map_err(|_| AuditError::IntegrityFailure),
        _ => Err(AuditError::IntegrityFailure),
    }
}

fn take_array(
    fields: &mut BTreeMap<u64, CborValue>,
    key: u64,
) -> Result<Vec<CborValue>, AuditError> {
    match fields.remove(&key) {
        Some(CborValue::Array(values)) => Ok(values),
        _ => Err(AuditError::IntegrityFailure),
    }
}

fn take_optional_fixed<const N: usize>(
    fields: &mut BTreeMap<u64, CborValue>,
    key: u64,
) -> Result<Option<[u8; N]>, AuditError> {
    let values = take_array(fields, key)?;
    match values.as_slice() {
        [] => Ok(None),
        [_] => fixed_value(values.into_iter().next().expect("one value")).map(Some),
        _ => Err(AuditError::IntegrityFailure),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn head(value: u8) -> ObjectId {
        ObjectId::new([value; 32])
    }

    fn item(value: u8) -> ItemId {
        ItemId::new([value; 16])
    }

    fn revision(value: u8) -> RevisionId {
        RevisionId::new([value; 32])
    }

    fn successful_read() -> AuditEventV1 {
        AuditEventV1::new(
            VaultId::new([1; 16]),
            DeviceId::new([2; 16]),
            7,
            OperationId::new([3; 32]),
            AuditActionV1::ItemRead,
            AuditOutcomeV1::Succeeded,
            Some(item(4)),
            Some(revision(5)),
            None,
            Some(head(6)),
            vec![head(7), head(8)],
            1_700_000_000_000,
        )
        .unwrap()
    }

    #[test]
    fn signed_event_round_trips_and_verifies_deterministically() {
        let seed = [9; 32];
        let (public, _) = generate_keypair(&seed);
        let signed = successful_read().sign(&seed).unwrap();
        let encoded = signed.encode();
        let decoded = SignedAuditEventV1::decode(&encoded).unwrap();
        assert_eq!(decoded, signed);
        assert_eq!(decoded.encode(), encoded);
        assert_eq!(decoded.verify(&public), Ok(()));
        assert_eq!(decoded.event().action(), AuditActionV1::ItemRead);
        assert_eq!(decoded.event().item_id(), Some(item(4)));
        assert_eq!(decoded.event().selected_revision(), Some(revision(5)));
        assert_eq!(decoded.event().basis_heads(), &[head(7), head(8)]);
        assert_eq!(decoded.event().action().label(), "item_read");
        assert_eq!(decoded.event().outcome().label(), "succeeded");
    }

    #[test]
    fn redacted_surface_labels_are_stable_and_complete() {
        let actions = [
            (AuditActionV1::AuditEpochStart, "audit_epoch_start"),
            (AuditActionV1::VaultInitialize, "vault_initialize"),
            (AuditActionV1::VaultVerify, "vault_verify"),
            (AuditActionV1::VaultDiagnose, "vault_diagnose"),
            (AuditActionV1::AuditRead, "audit_read"),
            (AuditActionV1::PassphraseRotate, "passphrase_rotate"),
            (AuditActionV1::ItemCreate, "item_create"),
            (AuditActionV1::ItemList, "item_list"),
            (AuditActionV1::ItemRead, "item_read"),
            (AuditActionV1::ItemUpdate, "item_update"),
            (AuditActionV1::ItemDelete, "item_delete"),
            (AuditActionV1::ItemRestore, "item_restore"),
            (AuditActionV1::ItemHistoryRead, "item_history_read"),
            (AuditActionV1::ItemSearch, "item_search"),
            (AuditActionV1::ItemConflictResolve, "item_conflict_resolve"),
            (AuditActionV1::ItemConflictMerge, "item_conflict_merge"),
            (AuditActionV1::PortableImport, "portable_import"),
            (AuditActionV1::PortableExport, "portable_export"),
            (
                AuditActionV1::PortableRestoreVerify,
                "portable_restore_verify",
            ),
        ];
        for (action, label) in actions {
            assert_eq!(action.label(), label);
        }

        assert_eq!(AuditOutcomeV1::Succeeded.label(), "succeeded");
        assert_eq!(AuditOutcomeV1::Denied.label(), "denied");
        assert_eq!(AuditOutcomeV1::Failed.label(), "failed");
    }

    #[test]
    fn tampering_and_wrong_device_key_fail_signature_verification() {
        let seed = [10; 32];
        let signed = successful_read().sign(&seed).unwrap();
        let mut encoded = signed.encode();
        let last = encoded.len() - 1;
        encoded[last] ^= 1;
        let tampered = SignedAuditEventV1::decode(&encoded).unwrap();
        let (public, _) = generate_keypair(&seed);
        assert_eq!(tampered.verify(&public), Err(AuditError::SignatureFailure));
        let (wrong, _) = generate_keypair(&[11; 32]);
        assert_eq!(signed.verify(&wrong), Err(AuditError::SignatureFailure));
    }

    #[test]
    fn initialization_and_migration_epoch_have_explicit_genesis_rules() {
        let initialized = AuditEventV1::new(
            VaultId::new([1; 16]),
            DeviceId::new([2; 16]),
            1,
            OperationId::new([3; 32]),
            AuditActionV1::VaultInitialize,
            AuditOutcomeV1::Succeeded,
            None,
            None,
            None,
            None,
            vec![],
            1,
        );
        assert!(initialized.is_ok());
        let epoch = AuditEventV1::new(
            VaultId::new([1; 16]),
            DeviceId::new([2; 16]),
            99,
            OperationId::new([3; 32]),
            AuditActionV1::AuditEpochStart,
            AuditOutcomeV1::Succeeded,
            None,
            None,
            None,
            None,
            vec![head(1)],
            2,
        );
        assert!(epoch.is_ok());
        assert_eq!(
            AuditEventV1::new(
                VaultId::new([1; 16]),
                DeviceId::new([2; 16]),
                2,
                OperationId::new([3; 32]),
                AuditActionV1::ItemList,
                AuditOutcomeV1::Succeeded,
                None,
                None,
                None,
                None,
                vec![head(1)],
                3,
            ),
            Err(AuditError::InvalidInput)
        );
    }

    #[test]
    fn action_resource_and_result_shapes_fail_closed() {
        let base = successful_read();
        assert!(AuditEventV1::new(
            base.vault_id,
            base.device_id,
            base.device_counter,
            base.trace_id,
            AuditActionV1::ItemConflictMerge,
            AuditOutcomeV1::Succeeded,
            Some(item(4)),
            None,
            Some(revision(6)),
            base.previous_event,
            base.basis_heads.clone(),
            base.timestamp_ms,
        )
        .is_ok());
        assert_eq!(
            AuditEventV1::new(
                base.vault_id,
                base.device_id,
                base.device_counter,
                base.trace_id,
                AuditActionV1::ItemRead,
                AuditOutcomeV1::Succeeded,
                None,
                Some(revision(5)),
                None,
                base.previous_event,
                base.basis_heads.clone(),
                base.timestamp_ms,
            ),
            Err(AuditError::InvalidInput)
        );
        assert_eq!(
            AuditEventV1::new(
                base.vault_id,
                base.device_id,
                base.device_counter,
                base.trace_id,
                AuditActionV1::ItemUpdate,
                AuditOutcomeV1::Succeeded,
                Some(item(4)),
                Some(revision(5)),
                None,
                base.previous_event,
                base.basis_heads.clone(),
                base.timestamp_ms,
            ),
            Err(AuditError::InvalidInput)
        );
        assert_eq!(
            AuditEventV1::new(
                base.vault_id,
                base.device_id,
                base.device_counter,
                base.trace_id,
                AuditActionV1::ItemList,
                AuditOutcomeV1::Succeeded,
                None,
                None,
                Some(revision(6)),
                base.previous_event,
                base.basis_heads.clone(),
                base.timestamp_ms,
            ),
            Err(AuditError::InvalidInput)
        );
    }

    #[test]
    fn basis_heads_are_bounded_sorted_and_unique() {
        let base = successful_read();
        for heads in [vec![head(8), head(7)], vec![head(7), head(7)]] {
            assert_eq!(
                AuditEventV1::new(
                    base.vault_id,
                    base.device_id,
                    base.device_counter,
                    base.trace_id,
                    base.action,
                    base.outcome,
                    base.item_id,
                    base.selected_revision,
                    base.result_revision,
                    base.previous_event,
                    heads,
                    base.timestamp_ms,
                ),
                Err(AuditError::InvalidInput)
            );
        }
        let too_many = (0..=MAX_COMMIT_PARENTS)
            .map(|index| {
                let mut bytes = [0; 32];
                bytes[31] = index as u8;
                ObjectId::new(bytes)
            })
            .collect();
        assert_eq!(
            AuditEventV1::new(
                base.vault_id,
                base.device_id,
                base.device_counter,
                base.trace_id,
                base.action,
                base.outcome,
                base.item_id,
                base.selected_revision,
                base.result_revision,
                base.previous_event,
                too_many,
                base.timestamp_ms,
            ),
            Err(AuditError::BoundExceeded)
        );
    }

    #[test]
    fn decoder_rejects_unknown_fields_and_oversize_inputs() {
        let signed = successful_read().sign(&[12; 32]).unwrap();
        let mut value = decode(&signed.encode()).unwrap();
        let CborValue::Map(ref mut fields) = value else {
            panic!("fixture must be a map")
        };
        fields.push(field(99, CborValue::Unsigned(1)));
        assert_eq!(
            SignedAuditEventV1::decode(&encode(&value)),
            Err(AuditError::IntegrityFailure)
        );
        assert_eq!(
            SignedAuditEventV1::decode(&vec![0; MAX_ENCODED_BYTES + 1]),
            Err(AuditError::BoundExceeded)
        );
    }

    #[test]
    fn debug_is_redacted() {
        let signed = successful_read().sign(&[13; 32]).unwrap();
        let debug = format!("{signed:?}");
        assert!(debug.contains("ItemRead"));
        for secret_identity in [
            successful_read().trace_id().to_user_string(),
            item(4).to_user_string(),
            revision(5).to_user_string(),
        ] {
            assert!(!debug.contains(&secret_identity));
        }
    }
}
