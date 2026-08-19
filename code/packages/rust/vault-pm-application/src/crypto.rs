use crate::{ApplicationError, LocalSecretV1};
use coding_adventures_canonical_cbor::{decode, CborValue};
use coding_adventures_chacha20_poly1305::{
    xchacha20_poly1305_aead_decrypt, xchacha20_poly1305_aead_encrypt,
};
use coding_adventures_hkdf::{hkdf, HashAlgorithm};
use coding_adventures_vault_pm_format::{AeadEnvelopeV1, ObjectFrameV1, VaultId, CRYPTO_SUITE_V1};
use coding_adventures_zeroize::{Zeroize, Zeroizing};
use core::fmt::{self, Debug, Formatter};

const LOCATOR_KEY_LABEL: &[u8] = b"vpm/locator-key/v1";
const OBJECT_WRAP_KEY_LABEL: &[u8] = b"vpm/object-wrap-key/v1";
const LOCAL_STATE_KEY_LABEL: &[u8] = b"vpm/local-state-key/v1";
const AUDIT_KEY_LABEL: &[u8] = b"vpm/audit-key/v1";
const WRAP_AAD_PREFIX: &[u8] = b"VPM-OBJECT-DEK-WRAP-v1";
const PAYLOAD_AAD_PREFIX: &[u8] = b"VPM-OBJECT-PAYLOAD-v1";
const LOCAL_SECRET_AAD_PREFIX: &[u8] = b"VPM-LOCAL-SECRET-v1";
const MAX_PLAINTEXT_BYTES: usize = 16 * 1024 * 1024;

/// Closed registry of authenticated application object kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjectKind {
    /// One live or tombstone item revision.
    ItemRevision,
    /// One complete immutable catalog snapshot.
    Catalog,
    /// One exact authority-signed VLT-PM01 device certificate.
    DeviceCertificate,
    /// One exact device-signed VLT-PM01 repository commit.
    Commit,
    /// One exact device-signed VLT-PM15 operation-audit event.
    AuditEvent,
    /// One attachment's metadata, key, and ordered chunk references.
    ///
    /// VLT-PM47 §4.3. The manifest is what makes an attachment cost the item
    /// revision a fixed forty-eight bytes instead of growing with the file.
    AttachmentManifest,
    /// One VLT14 sealed attachment chunk.
    ///
    /// VLT-PM47 §4.2. The kind is bound into both associated-data strings, so
    /// a chunk frame presented where a manifest is expected is an integrity
    /// failure rather than a value that decodes into the wrong shape.
    AttachmentChunk,
}

impl ObjectKind {
    /// Return the stable V1 registry code.
    pub const fn code(self) -> u64 {
        match self {
            Self::ItemRevision => 1,
            Self::Catalog => 2,
            Self::DeviceCertificate => 3,
            Self::Commit => 4,
            Self::AuditEvent => 5,
            Self::AttachmentManifest => 6,
            Self::AttachmentChunk => 7,
        }
    }
}

/// Four live V1 subkeys derived from one vault root key.
pub struct V1Keys {
    vault_id: VaultId,
    locator_key: [u8; 32],
    object_wrap_key: [u8; 32],
    local_state_key: [u8; 32],
    audit_key: [u8; 32],
}

impl V1Keys {
    /// Derive every V1 32-byte subkey using the vault ID as HKDF-SHA-256 salt.
    pub fn derive(vault_id: VaultId, vault_root_key: &[u8; 32]) -> Result<Self, ApplicationError> {
        Ok(Self {
            vault_id,
            locator_key: derive_key(vault_id, vault_root_key, LOCATOR_KEY_LABEL)?,
            object_wrap_key: derive_key(vault_id, vault_root_key, OBJECT_WRAP_KEY_LABEL)?,
            local_state_key: derive_key(vault_id, vault_root_key, LOCAL_STATE_KEY_LABEL)?,
            audit_key: derive_key(vault_id, vault_root_key, AUDIT_KEY_LABEL)?,
        })
    }

    /// Return the vault identity bound into every derived key and object AAD.
    pub const fn vault_id(&self) -> VaultId {
        self.vault_id
    }

    /// Borrow the opaque repository locator key for VLT-PM04 construction.
    pub const fn locator_key(&self) -> &[u8; 32] {
        &self.locator_key
    }

    /// Borrow the local-state encryption key for the recovery-journal codec.
    pub const fn local_state_key(&self) -> &[u8; 32] {
        &self.local_state_key
    }

    /// Borrow the audit derivation key for the later audit workflow.
    pub const fn audit_key(&self) -> &[u8; 32] {
        &self.audit_key
    }
}

impl Zeroize for V1Keys {
    fn zeroize(&mut self) {
        self.locator_key.zeroize();
        self.object_wrap_key.zeroize();
        self.local_state_key.zeroize();
        self.audit_key.zeroize();
    }
}

impl Drop for V1Keys {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl Debug for V1Keys {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("V1Keys(<redacted>)")
    }
}

/// Independently generated randomness consumed by one object seal operation.
pub struct ObjectRandomness {
    object_dek: [u8; 32],
    wrap_nonce: [u8; 24],
    payload_nonce: [u8; 24],
}

impl ObjectRandomness {
    /// Construct caller-supplied independent DEK and nonce material.
    pub const fn new(object_dek: [u8; 32], wrap_nonce: [u8; 24], payload_nonce: [u8; 24]) -> Self {
        Self {
            object_dek,
            wrap_nonce,
            payload_nonce,
        }
    }
}

impl Zeroize for ObjectRandomness {
    fn zeroize(&mut self) {
        self.object_dek.zeroize();
        self.wrap_nonce.zeroize();
        self.payload_nonce.zeroize();
    }
}

impl Drop for ObjectRandomness {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl Debug for ObjectRandomness {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("ObjectRandomness(<redacted>)")
    }
}

/// Independently generated nonce consumed by one local-secret seal operation.
pub struct LocalSecretRandomness {
    nonce: [u8; 24],
}

impl LocalSecretRandomness {
    /// Construct caller-supplied nonce material.
    pub const fn new(nonce: [u8; 24]) -> Self {
        Self { nonce }
    }
}

impl Zeroize for LocalSecretRandomness {
    fn zeroize(&mut self) {
        self.nonce.zeroize();
    }
}

impl Drop for LocalSecretRandomness {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl Debug for LocalSecretRandomness {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("LocalSecretRandomness(<redacted>)")
    }
}

/// Encrypt owner-private seeds into a bounded local-state envelope.
pub fn seal_local_secret(
    keys: &V1Keys,
    secret: &LocalSecretV1,
    randomness: &LocalSecretRandomness,
) -> Result<AeadEnvelopeV1, ApplicationError> {
    if secret.vault_id() != keys.vault_id {
        return Err(ApplicationError::InvalidInput);
    }
    let plaintext = Zeroizing::new(secret.encode());
    let aad = local_secret_aad(keys.vault_id);
    let (ciphertext, tag) =
        xchacha20_poly1305_aead_encrypt(&plaintext, &keys.local_state_key, &randomness.nonce, &aad);
    let envelope = AeadEnvelopeV1 {
        suite: CRYPTO_SUITE_V1,
        nonce: randomness.nonce,
        ciphertext,
        tag,
    };
    envelope
        .validate()
        .map_err(|_| ApplicationError::InternalInvariant)?;
    Ok(envelope)
}

/// Authenticate, decrypt, and identity-bind owner-private local seeds.
pub fn open_local_secret(
    keys: &V1Keys,
    envelope: &AeadEnvelopeV1,
) -> Result<LocalSecretV1, ApplicationError> {
    if envelope.suite != CRYPTO_SUITE_V1 {
        return Err(ApplicationError::Unsupported);
    }
    envelope
        .validate()
        .map_err(|_| ApplicationError::IntegrityFailure)?;
    let aad = local_secret_aad(keys.vault_id);
    let plaintext = xchacha20_poly1305_aead_decrypt(
        &envelope.ciphertext,
        &keys.local_state_key,
        &envelope.nonce,
        &aad,
        &envelope.tag,
    )
    .ok_or(ApplicationError::IntegrityFailure)?;
    let plaintext = Zeroizing::new(plaintext);
    let secret = LocalSecretV1::decode(&plaintext)?;
    if secret.vault_id() != keys.vault_id {
        return Err(ApplicationError::IntegrityFailure);
    }
    Ok(secret)
}

/// Encrypt one canonical application object into an exact VLT-PM01 frame.
pub fn seal_object(
    keys: &V1Keys,
    kind: ObjectKind,
    plaintext: &[u8],
    randomness: &ObjectRandomness,
) -> Result<ObjectFrameV1, ApplicationError> {
    if plaintext.len() > MAX_PLAINTEXT_BYTES {
        return Err(ApplicationError::BoundExceeded);
    }
    validate_plaintext_header(plaintext, kind)?;
    let wrap_aad = object_aad(WRAP_AAD_PREFIX, keys.vault_id, kind);
    let payload_aad = object_aad(PAYLOAD_AAD_PREFIX, keys.vault_id, kind);
    let (wrapped_dek, wrap_tag) = xchacha20_poly1305_aead_encrypt(
        &randomness.object_dek,
        &keys.object_wrap_key,
        &randomness.wrap_nonce,
        &wrap_aad,
    );
    let wrapped_dek: [u8; 32] = wrapped_dek
        .try_into()
        .map_err(|_| ApplicationError::InternalInvariant)?;
    let (ciphertext, payload_tag) = xchacha20_poly1305_aead_encrypt(
        plaintext,
        &randomness.object_dek,
        &randomness.payload_nonce,
        &payload_aad,
    );
    Ok(ObjectFrameV1 {
        suite: CRYPTO_SUITE_V1,
        wrap_nonce: randomness.wrap_nonce,
        wrapped_dek,
        wrap_tag,
        payload_nonce: randomness.payload_nonce,
        ciphertext,
        payload_tag,
    })
}

/// Authenticate and decrypt one exact VLT-PM01 frame for an expected kind.
pub fn open_object(
    keys: &V1Keys,
    expected_kind: ObjectKind,
    frame: &ObjectFrameV1,
) -> Result<Zeroizing<Vec<u8>>, ApplicationError> {
    if frame.suite != CRYPTO_SUITE_V1 {
        return Err(ApplicationError::Unsupported);
    }
    if frame.ciphertext.len() > MAX_PLAINTEXT_BYTES {
        return Err(ApplicationError::BoundExceeded);
    }
    frame
        .validate()
        .map_err(|_| ApplicationError::IntegrityFailure)?;
    let wrap_aad = object_aad(WRAP_AAD_PREFIX, keys.vault_id, expected_kind);
    let payload_aad = object_aad(PAYLOAD_AAD_PREFIX, keys.vault_id, expected_kind);
    let dek = xchacha20_poly1305_aead_decrypt(
        &frame.wrapped_dek,
        &keys.object_wrap_key,
        &frame.wrap_nonce,
        &wrap_aad,
        &frame.wrap_tag,
    )
    .ok_or(ApplicationError::IntegrityFailure)?;
    let dek: [u8; 32] = dek
        .try_into()
        .map_err(|_| ApplicationError::IntegrityFailure)?;
    let dek = Zeroizing::new(dek);
    let plaintext = xchacha20_poly1305_aead_decrypt(
        &frame.ciphertext,
        &dek,
        &frame.payload_nonce,
        &payload_aad,
        &frame.payload_tag,
    )
    .ok_or(ApplicationError::IntegrityFailure)?;
    let plaintext = Zeroizing::new(plaintext);
    validate_plaintext_header(&plaintext, expected_kind)?;
    Ok(plaintext)
}

fn derive_key(
    vault_id: VaultId,
    vault_root_key: &[u8; 32],
    label: &[u8],
) -> Result<[u8; 32], ApplicationError> {
    let mut derived = hkdf(
        vault_id.as_bytes(),
        vault_root_key,
        label,
        32,
        HashAlgorithm::Sha256,
    )
    .map_err(|_| ApplicationError::InternalInvariant)?;
    let mut key = [0u8; 32];
    key.copy_from_slice(&derived);
    derived.zeroize();
    Ok(key)
}

fn object_aad(prefix: &[u8], vault_id: VaultId, kind: ObjectKind) -> Vec<u8> {
    let mut aad = Vec::with_capacity(prefix.len() + 2 + 16 + 8);
    aad.extend_from_slice(prefix);
    aad.extend_from_slice(&CRYPTO_SUITE_V1.to_be_bytes());
    aad.extend_from_slice(vault_id.as_bytes());
    aad.extend_from_slice(&kind.code().to_be_bytes());
    aad
}

fn local_secret_aad(vault_id: VaultId) -> Vec<u8> {
    let mut aad = Vec::with_capacity(LOCAL_SECRET_AAD_PREFIX.len() + 2 + 16);
    aad.extend_from_slice(LOCAL_SECRET_AAD_PREFIX);
    aad.extend_from_slice(&CRYPTO_SUITE_V1.to_be_bytes());
    aad.extend_from_slice(vault_id.as_bytes());
    aad
}

fn validate_plaintext_header(
    plaintext: &[u8],
    expected_kind: ObjectKind,
) -> Result<(), ApplicationError> {
    let entries = match decode(plaintext).map_err(|_| ApplicationError::IntegrityFailure)? {
        CborValue::Map(entries) => entries,
        _ => return Err(ApplicationError::IntegrityFailure),
    };
    let mut version = None;
    let mut kind = None;
    for (key, value) in entries {
        match (key, value) {
            (CborValue::Unsigned(1), CborValue::Unsigned(value)) => version = Some(value),
            (CborValue::Unsigned(2), CborValue::Unsigned(value)) => kind = Some(value),
            _ => {}
        }
    }
    if version != Some(1) {
        return Err(ApplicationError::Unsupported);
    }
    if kind != Some(expected_kind.code()) {
        return Err(ApplicationError::IntegrityFailure);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{encode_signed_audit_event, ApplicationError, CatalogV1};
    use coding_adventures_canonical_cbor::{encode, CborValue};
    use coding_adventures_vault_pm_audit::{AuditActionV1, AuditEventV1, AuditOutcomeV1};
    use coding_adventures_vault_pm_domain::OperationId;
    use coding_adventures_vault_pm_format::{DeviceId, ObjectId};

    fn keys() -> V1Keys {
        V1Keys::derive(VaultId::new([0x11; 16]), &[0x22; 32]).unwrap()
    }

    fn randomness() -> ObjectRandomness {
        ObjectRandomness::new([0x33; 32], [0x44; 24], [0x55; 24])
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    #[test]
    fn key_derivation_is_deterministic_domain_separated_and_redacted() {
        let keys = keys();
        assert_eq!(
            hex(keys.locator_key()),
            "d101b6c82a864a3f089331d6125248c9d0d5481c606895bbba86e5159e18f9fc"
        );
        assert_ne!(keys.locator_key(), keys.local_state_key());
        assert_ne!(keys.local_state_key(), keys.audit_key());
        assert_eq!(keys.vault_id(), VaultId::new([0x11; 16]));
        assert_eq!(format!("{keys:?}"), "V1Keys(<redacted>)");
    }

    #[test]
    fn catalog_object_seals_and_opens_exactly() {
        let keys = keys();
        let randomness = randomness();
        let plaintext = CatalogV1::empty().encode().unwrap();
        let frame = seal_object(&keys, ObjectKind::Catalog, &plaintext, &randomness).unwrap();
        assert_eq!(
            hex(&frame.encode().unwrap()),
            "56504f3100014444444444444444444444444444444444444444444444441b67e22415d0bc3f692497398ebb3872d35da3289ae7cb35ddc51e1b82bdc1d39535f3b6fcd552db76ddb9227a3598145555555555555555555555555555555555555555555555550000000000000007d44c77f95a1d570be73353ddd1f6389c1a46960932b52c"
        );
        assert_eq!(frame.suite, CRYPTO_SUITE_V1);
        assert_ne!(frame.ciphertext, plaintext);
        assert_eq!(
            &*open_object(&keys, ObjectKind::Catalog, &frame).unwrap(),
            &plaintext
        );
        assert_eq!(format!("{randomness:?}"), "ObjectRandomness(<redacted>)");
    }

    #[test]
    fn signed_audit_event_seals_under_its_own_authenticated_kind() {
        let keys = keys();
        let event = AuditEventV1::new(
            keys.vault_id(),
            DeviceId::new([0x66; 16]),
            2,
            OperationId::new([0x77; 32]),
            AuditActionV1::AuditEpochStart,
            AuditOutcomeV1::Succeeded,
            None,
            None,
            None,
            None,
            vec![ObjectId::new([0x88; 32])],
            9,
        )
        .unwrap()
        .sign(&[0x99; 32])
        .unwrap();
        let plaintext = encode_signed_audit_event(&event).unwrap();
        let frame = seal_object(&keys, ObjectKind::AuditEvent, &plaintext, &randomness()).unwrap();
        assert_eq!(
            &*open_object(&keys, ObjectKind::AuditEvent, &frame).unwrap(),
            &plaintext
        );
        assert_eq!(
            open_object(&keys, ObjectKind::Commit, &frame).err(),
            Some(ApplicationError::IntegrityFailure)
        );
    }

    #[test]
    fn local_secret_seals_opens_and_is_identity_bound() {
        let keys = keys();
        let secret = LocalSecretV1::new(
            keys.vault_id(),
            coding_adventures_vault_pm_format::DeviceId::new([0x66; 16]),
            [0x77; 32],
            [0x88; 32],
            [0x99; 32],
        );
        let randomness = LocalSecretRandomness::new([0xaa; 24]);
        let envelope = seal_local_secret(&keys, &secret, &randomness).unwrap();
        assert_eq!(
            hex(&envelope.ciphertext),
            "0b0579aa801c68ba91ec18ab4417c88a223e185be2ba0a63dd072c7a81f84c26fc8fc83155822ad1dfc259f69caabc10424afdbb816e205d2cd75744e04ae6ff23e14300e170124a8f074881ebf80f965a13ae002a53cbc028d36d5ff7470ff9d5fd5489478d8176733076427e14901868e301842608f9952463d4f468e1e1f086447059c85e31ac396dad367a603c2a"
        );
        assert_eq!(open_local_secret(&keys, &envelope).unwrap(), secret);
        assert_eq!(
            format!("{randomness:?}"),
            "LocalSecretRandomness(<redacted>)"
        );

        let other_keys = V1Keys::derive(VaultId::new([0x12; 16]), &[0x22; 32]).unwrap();
        assert_eq!(
            open_local_secret(&other_keys, &envelope).err(),
            Some(ApplicationError::IntegrityFailure)
        );
        let mut tampered = envelope.clone();
        tampered.tag[0] ^= 1;
        assert_eq!(
            open_local_secret(&keys, &tampered).err(),
            Some(ApplicationError::IntegrityFailure)
        );
        let mut unsupported = envelope;
        unsupported.suite = CRYPTO_SUITE_V1 + 1;
        assert_eq!(
            open_local_secret(&keys, &unsupported).err(),
            Some(ApplicationError::Unsupported)
        );
        let wrong_vault = LocalSecretV1::new(
            VaultId::new([0x12; 16]),
            coding_adventures_vault_pm_format::DeviceId::new([0x66; 16]),
            [0x77; 32],
            [0x88; 32],
            [0x99; 32],
        );
        assert_eq!(
            seal_local_secret(&keys, &wrong_vault, &LocalSecretRandomness::new([1; 24])),
            Err(ApplicationError::InvalidInput)
        );
        let mut wipe = LocalSecretRandomness::new([0xbb; 24]);
        wipe.zeroize();
        assert_eq!(wipe.nonce, [0; 24]);
    }

    #[test]
    fn kind_vault_and_tags_are_authenticated() {
        let keys = keys();
        let plaintext = CatalogV1::empty().encode().unwrap();
        let frame = seal_object(&keys, ObjectKind::Catalog, &plaintext, &randomness()).unwrap();
        assert_eq!(
            open_object(&keys, ObjectKind::ItemRevision, &frame).err(),
            Some(ApplicationError::IntegrityFailure)
        );
        let other_keys = V1Keys::derive(VaultId::new([0x12; 16]), &[0x22; 32]).unwrap();
        assert_eq!(
            open_object(&other_keys, ObjectKind::Catalog, &frame).err(),
            Some(ApplicationError::IntegrityFailure)
        );

        let mut bad_wrap = frame.clone();
        bad_wrap.wrap_tag[0] ^= 1;
        assert_eq!(
            open_object(&keys, ObjectKind::Catalog, &bad_wrap).err(),
            Some(ApplicationError::IntegrityFailure)
        );
        let mut bad_payload = frame;
        bad_payload.payload_tag[0] ^= 1;
        assert_eq!(
            open_object(&keys, ObjectKind::Catalog, &bad_payload).err(),
            Some(ApplicationError::IntegrityFailure)
        );
    }

    #[test]
    fn seal_and_open_reject_bad_headers_suites_and_bounds() {
        let keys = keys();
        let randomness = randomness();
        let wrong_kind = encode(&CborValue::Map(vec![
            (CborValue::Unsigned(1), CborValue::Unsigned(1)),
            (
                CborValue::Unsigned(2),
                CborValue::Unsigned(ObjectKind::ItemRevision.code()),
            ),
        ]));
        assert_eq!(
            seal_object(&keys, ObjectKind::Catalog, &wrong_kind, &randomness),
            Err(ApplicationError::IntegrityFailure)
        );
        let wrong_version = encode(&CborValue::Map(vec![
            (CborValue::Unsigned(1), CborValue::Unsigned(2)),
            (
                CborValue::Unsigned(2),
                CborValue::Unsigned(ObjectKind::Catalog.code()),
            ),
        ]));
        assert_eq!(
            seal_object(&keys, ObjectKind::Catalog, &wrong_version, &randomness),
            Err(ApplicationError::Unsupported)
        );

        let plaintext = CatalogV1::empty().encode().unwrap();
        let mut frame = seal_object(&keys, ObjectKind::Catalog, &plaintext, &randomness).unwrap();
        frame.suite = 2;
        assert_eq!(
            open_object(&keys, ObjectKind::Catalog, &frame).err(),
            Some(ApplicationError::Unsupported)
        );
        let oversized = vec![0u8; MAX_PLAINTEXT_BYTES + 1];
        assert_eq!(
            seal_object(&keys, ObjectKind::Catalog, &oversized, &randomness),
            Err(ApplicationError::BoundExceeded)
        );
        let mut oversized_frame = frame;
        oversized_frame.suite = CRYPTO_SUITE_V1;
        oversized_frame.ciphertext = oversized;
        assert_eq!(
            open_object(&keys, ObjectKind::Catalog, &oversized_frame).err(),
            Some(ApplicationError::BoundExceeded)
        );
        assert_eq!(
            seal_object(
                &keys,
                ObjectKind::Catalog,
                &encode(&CborValue::Bool(false)),
                &randomness,
            ),
            Err(ApplicationError::IntegrityFailure)
        );
    }

    #[test]
    fn live_key_and_randomness_containers_can_be_wiped() {
        let mut keys = keys();
        keys.zeroize();
        assert_eq!(keys.locator_key, [0; 32]);
        assert_eq!(keys.object_wrap_key, [0; 32]);
        assert_eq!(keys.local_state_key, [0; 32]);
        assert_eq!(keys.audit_key, [0; 32]);

        let mut randomness = randomness();
        randomness.zeroize();
        assert_eq!(randomness.object_dek, [0; 32]);
        assert_eq!(randomness.wrap_nonce, [0; 24]);
        assert_eq!(randomness.payload_nonce, [0; 24]);
    }

    #[test]
    fn all_object_kind_codes_are_stable() {
        assert_eq!(ObjectKind::ItemRevision.code(), 1);
        assert_eq!(ObjectKind::Catalog.code(), 2);
        assert_eq!(ObjectKind::DeviceCertificate.code(), 3);
        assert_eq!(ObjectKind::Commit.code(), 4);
        assert_eq!(ObjectKind::AuditEvent.code(), 5);
    }
}
