use crate::initialize::verify_active_bootstrap;
use crate::{encode_item_revision, ApplicationError};
use coding_adventures_argon2id::{argon2id, Options as Argon2idOptions};
use coding_adventures_canonical_cbor::{encode, CborValue};
use coding_adventures_chacha20_poly1305::xchacha20_poly1305_aead_encrypt;
use coding_adventures_sha256::sha256;
use coding_adventures_vault_pm_domain::{ItemCandidate, ItemId};
use coding_adventures_vault_pm_format::{Argon2idParametersV1, CRYPTO_SUITE_V1};
use coding_adventures_zeroize::{Zeroize, Zeroizing};
use core::fmt::{self, Debug, Formatter};
use std::collections::BTreeMap;

const VERSION: u64 = 1;
const PASSPHRASE_PROTECTION: u64 = 1;
const KDF_SALT_BYTES: usize = 16;
const NONCE_BYTES: usize = 24;
const SNAPSHOT_HASH_DOMAIN: &[u8] = b"VPM-PORTABLE-SNAPSHOT-v1";
const EXPORT_AAD_DOMAIN: &[u8] = b"VPM-PORTABLE-EXPORT-AAD-v1";
const CANDIDATE_ESTIMATE_OVERHEAD: usize = 128;
const SNAPSHOT_ESTIMATE_OVERHEAD: usize = 1_024;

/// Exact host-supplied CSPRNG bytes consumed by one passphrase export.
pub const PORTABLE_EXPORT_RANDOM_BYTES: usize = KDF_SALT_BYTES + NONCE_BYTES;
/// Maximum separately collected export-passphrase bytes accepted by V1.
pub const MAX_PORTABLE_EXPORT_PASSPHRASE_BYTES: usize = 1_024;
/// Maximum canonical plaintext snapshot size accepted by V1.
pub const MAX_PORTABLE_EXPORT_PLAINTEXT_BYTES: usize = 512 * 1024 * 1024;

/// Bounded caller-calibrated Argon2id policy for a portable export.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PortableExportPolicyV1 {
    memory_kib: u32,
    iterations: u32,
    lanes: u8,
}

impl PortableExportPolicyV1 {
    /// Validate and retain one V1 export-passphrase policy.
    pub fn new(memory_kib: u32, iterations: u32, lanes: u8) -> Result<Self, ApplicationError> {
        Argon2idParametersV1 {
            memory_kib,
            iterations,
            lanes,
            salt: [0; KDF_SALT_BYTES],
        }
        .validate()
        .map_err(|_| ApplicationError::InvalidInput)?;
        Ok(Self {
            memory_kib,
            iterations,
            lanes,
        })
    }

    /// Return the Argon2id memory cost in KiB.
    pub const fn memory_kib(&self) -> u32 {
        self.memory_kib
    }

    /// Return the Argon2id iteration count.
    pub const fn iterations(&self) -> u32 {
        self.iterations
    }

    /// Return the Argon2id lane count.
    pub const fn lanes(&self) -> u8 {
        self.lanes
    }
}

impl Debug for PortableExportPolicyV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PortableExportPolicyV1")
            .field("memory_kib", &self.memory_kib)
            .field("iterations", &self.iterations)
            .field("lanes", &self.lanes)
            .finish()
    }
}

/// One owned wipe-on-drop CSPRNG block for export salt and nonce.
pub struct PortableExportRandomnessV1 {
    bytes: [u8; PORTABLE_EXPORT_RANDOM_BYTES],
}

impl PortableExportRandomnessV1 {
    /// Take one exact block filled by the host cryptographic entropy source.
    pub const fn new(bytes: [u8; PORTABLE_EXPORT_RANDOM_BYTES]) -> Self {
        Self { bytes }
    }

    fn salt(&self) -> [u8; KDF_SALT_BYTES] {
        self.bytes[..KDF_SALT_BYTES]
            .try_into()
            .expect("portable export salt partition is constant")
    }

    fn nonce(&self) -> [u8; NONCE_BYTES] {
        self.bytes[KDF_SALT_BYTES..]
            .try_into()
            .expect("portable export nonce partition is constant")
    }
}

impl Zeroize for PortableExportRandomnessV1 {
    fn zeroize(&mut self) {
        self.bytes.zeroize();
    }
}

impl Drop for PortableExportRandomnessV1 {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl Debug for PortableExportRandomnessV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("PortableExportRandomnessV1(<redacted>)")
    }
}

/// One canonical authenticated encrypted portable export artifact.
///
/// The bytes contain no plaintext vault record, owner-private state, local
/// pins, provider credential, or search projection. The host may write them to
/// an explicitly selected destination without interpreting their contents.
pub struct PortableExportArtifactV1 {
    bytes: Vec<u8>,
}

impl PortableExportArtifactV1 {
    /// Borrow the exact canonical encrypted artifact bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Consume the wrapper and return the exact bytes for host persistence.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

impl Debug for PortableExportArtifactV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("PortableExportArtifactV1(<encrypted>)")
    }
}

pub(crate) fn export_portable_with_passphrase(
    candidates: &BTreeMap<ItemId, Vec<ItemCandidate>>,
    active: &crate::ActiveStateV1,
    exact_bootstrap: &[u8],
    passphrase: Zeroizing<Vec<u8>>,
    policy: PortableExportPolicyV1,
    randomness: PortableExportRandomnessV1,
) -> Result<PortableExportArtifactV1, ApplicationError> {
    if passphrase.is_empty() || passphrase.len() > MAX_PORTABLE_EXPORT_PASSPHRASE_BYTES {
        return Err(ApplicationError::InvalidInput);
    }
    verify_active_bootstrap(active, exact_bootstrap)?;

    let salt = randomness.salt();
    let nonce = randomness.nonce();
    let kdf = Argon2idParametersV1 {
        memory_kib: policy.memory_kib,
        iterations: policy.iterations,
        lanes: policy.lanes,
        salt,
    };
    kdf.validate().map_err(|_| ApplicationError::InvalidInput)?;

    let mut entries = SecretCborValues::default();
    let mut estimated_size = exact_bootstrap
        .len()
        .checked_add(SNAPSHOT_ESTIMATE_OVERHEAD)
        .ok_or(ApplicationError::BoundExceeded)?;
    for (item_id, item_candidates) in candidates {
        let mut ordered = item_candidates.iter().collect::<Vec<_>>();
        ordered.sort_unstable_by_key(|candidate| candidate.revision_id());
        for candidate in ordered {
            if candidate.item_id() != *item_id {
                return Err(ApplicationError::IntegrityFailure);
            }
            let revision = Zeroizing::new(encode_item_revision(
                candidate.causal_parents(),
                candidate.state(),
            )?);
            estimated_size = estimated_size
                .checked_add(revision.len())
                .and_then(|size| size.checked_add(CANDIDATE_ESTIMATE_OVERHEAD))
                .ok_or(ApplicationError::BoundExceeded)?;
            if estimated_size > MAX_PORTABLE_EXPORT_PLAINTEXT_BYTES {
                return Err(ApplicationError::BoundExceeded);
            }
            entries.push(CborValue::Map(vec![
                field(1, bytes(item_id.as_bytes())),
                field(2, bytes(candidate.revision_id().as_bytes())),
                field(3, CborValue::Bytes(revision.into_inner())),
            ]));
        }
    }

    let candidate_count =
        u64::try_from(entries.len()).map_err(|_| ApplicationError::BoundExceeded)?;
    let mut entries_value = SecretCborValue::new(CborValue::Array(entries.into_inner()));
    let encoded_entries = Zeroizing::new(encode(entries_value.get()));
    let snapshot_hash = snapshot_hash(exact_bootstrap, &encoded_entries)?;
    let snapshot = SecretCborValue::new(CborValue::Map(vec![
        field(1, CborValue::Unsigned(VERSION)),
        field(2, CborValue::Bytes(exact_bootstrap.to_vec())),
        field(3, entries_value.take()),
        field(4, CborValue::Unsigned(candidate_count)),
        field(5, CborValue::Bytes(snapshot_hash.to_vec())),
    ]));
    let plaintext = Zeroizing::new(encode(snapshot.get()));
    if plaintext.len() > MAX_PORTABLE_EXPORT_PLAINTEXT_BYTES {
        return Err(ApplicationError::BoundExceeded);
    }

    let derived = Zeroizing::new(
        argon2id(
            &passphrase,
            &kdf.salt,
            kdf.iterations,
            kdf.memory_kib,
            kdf.lanes.into(),
            32,
            &Argon2idOptions::default(),
        )
        .map_err(|_| ApplicationError::InvalidInput)?,
    );
    let mut key = Zeroizing::new([0; 32]);
    key.copy_from_slice(&derived);
    let aad = artifact_aad(&kdf, nonce);
    let (ciphertext, tag) = xchacha20_poly1305_aead_encrypt(&plaintext, &key, &nonce, &aad);
    let artifact = CborValue::Map(vec![
        field(1, CborValue::Unsigned(VERSION)),
        field(2, CborValue::Unsigned(PASSPHRASE_PROTECTION)),
        field(3, CborValue::Unsigned(CRYPTO_SUITE_V1.into())),
        field(4, kdf_value(&kdf)),
        field(5, CborValue::Bytes(nonce.to_vec())),
        field(6, CborValue::Bytes(ciphertext)),
        field(7, CborValue::Bytes(tag.to_vec())),
    ]);
    Ok(PortableExportArtifactV1 {
        bytes: encode(&artifact),
    })
}

pub(crate) fn snapshot_hash(
    exact_bootstrap: &[u8],
    encoded_entries: &[u8],
) -> Result<[u8; 32], ApplicationError> {
    let bootstrap_len =
        u64::try_from(exact_bootstrap.len()).map_err(|_| ApplicationError::BoundExceeded)?;
    let mut preimage = Zeroizing::new(Vec::with_capacity(
        SNAPSHOT_HASH_DOMAIN.len() + 8 + exact_bootstrap.len() + encoded_entries.len(),
    ));
    preimage.extend_from_slice(SNAPSHOT_HASH_DOMAIN);
    preimage.extend_from_slice(&bootstrap_len.to_be_bytes());
    preimage.extend_from_slice(exact_bootstrap);
    preimage.extend_from_slice(encoded_entries);
    Ok(sha256(&preimage))
}

fn artifact_aad(kdf: &Argon2idParametersV1, nonce: [u8; NONCE_BYTES]) -> Vec<u8> {
    let header = CborValue::Map(vec![
        field(1, CborValue::Unsigned(VERSION)),
        field(2, CborValue::Unsigned(PASSPHRASE_PROTECTION)),
        field(3, CborValue::Unsigned(CRYPTO_SUITE_V1.into())),
        field(4, kdf_value(kdf)),
        field(5, CborValue::Bytes(nonce.to_vec())),
    ]);
    let encoded = encode(&header);
    let mut aad = Vec::with_capacity(EXPORT_AAD_DOMAIN.len() + encoded.len());
    aad.extend_from_slice(EXPORT_AAD_DOMAIN);
    aad.extend_from_slice(&encoded);
    aad
}

fn kdf_value(kdf: &Argon2idParametersV1) -> CborValue {
    CborValue::Map(vec![
        field(1, CborValue::Unsigned(kdf.memory_kib.into())),
        field(2, CborValue::Unsigned(kdf.iterations.into())),
        field(3, CborValue::Unsigned(kdf.lanes.into())),
        field(4, CborValue::Bytes(kdf.salt.to_vec())),
    ])
}

fn field(key: u64, value: CborValue) -> (CborValue, CborValue) {
    (CborValue::Unsigned(key), value)
}

fn bytes<const N: usize>(value: &[u8; N]) -> CborValue {
    CborValue::Bytes(value.to_vec())
}

fn zeroize_cbor_values(values: &mut [CborValue]) {
    for value in values {
        zeroize_cbor(value);
    }
}

#[derive(Default)]
struct SecretCborValues(Option<Vec<CborValue>>);

impl SecretCborValues {
    fn push(&mut self, value: CborValue) {
        self.0.get_or_insert_with(Vec::new).push(value);
    }

    fn len(&self) -> usize {
        self.0.as_ref().map_or(0, Vec::len)
    }

    fn into_inner(mut self) -> Vec<CborValue> {
        self.0.take().unwrap_or_default()
    }
}

impl Drop for SecretCborValues {
    fn drop(&mut self) {
        if let Some(values) = &mut self.0 {
            zeroize_cbor_values(values);
        }
    }
}

struct SecretCborValue(Option<CborValue>);

impl SecretCborValue {
    fn new(value: CborValue) -> Self {
        Self(Some(value))
    }

    fn get(&self) -> &CborValue {
        self.0.as_ref().expect("secret CBOR value is present")
    }

    fn take(&mut self) -> CborValue {
        self.0.take().expect("secret CBOR value is present")
    }
}

impl Drop for SecretCborValue {
    fn drop(&mut self) {
        if let Some(value) = &mut self.0 {
            zeroize_cbor(value);
        }
    }
}

pub(crate) fn zeroize_cbor(value: &mut CborValue) {
    match value {
        CborValue::Bytes(bytes) => bytes.zeroize(),
        CborValue::Text(text) => text.zeroize(),
        CborValue::Array(values) => zeroize_cbor_values(values),
        CborValue::Map(entries) => {
            for (key, value) in entries {
                zeroize_cbor(key);
                zeroize_cbor(value);
            }
        }
        CborValue::Tag(_, value) => zeroize_cbor(value),
        CborValue::Unsigned(_) | CborValue::Negative(_) | CborValue::Bool(_) | CborValue::Null => {}
    }
}

#[cfg(test)]
pub(crate) fn decrypt_portable_for_test(
    artifact: &[u8],
    passphrase: Zeroizing<Vec<u8>>,
) -> Option<Zeroizing<Vec<u8>>> {
    use coding_adventures_canonical_cbor::decode;
    use coding_adventures_chacha20_poly1305::xchacha20_poly1305_aead_decrypt;

    let CborValue::Map(mut fields) = decode(artifact).ok()? else {
        return None;
    };
    let version = take_test_uint(&mut fields, 1)?;
    let protection = take_test_uint(&mut fields, 2)?;
    let suite = take_test_uint(&mut fields, 3)?;
    if version != VERSION
        || protection != PASSPHRASE_PROTECTION
        || suite != u64::from(CRYPTO_SUITE_V1)
    {
        return None;
    }
    let CborValue::Map(mut kdf_fields) = take_test_field(&mut fields, 4)? else {
        return None;
    };
    let memory_kib = u32::try_from(take_test_uint(&mut kdf_fields, 1)?).ok()?;
    let iterations = u32::try_from(take_test_uint(&mut kdf_fields, 2)?).ok()?;
    let lanes = u8::try_from(take_test_uint(&mut kdf_fields, 3)?).ok()?;
    let salt = take_test_bytes(&mut kdf_fields, 4)?.try_into().ok()?;
    let kdf = Argon2idParametersV1 {
        memory_kib,
        iterations,
        lanes,
        salt,
    };
    kdf.validate().ok()?;
    let nonce = take_test_bytes(&mut fields, 5)?.try_into().ok()?;
    let ciphertext = take_test_bytes(&mut fields, 6)?;
    let tag = take_test_bytes(&mut fields, 7)?.try_into().ok()?;
    if !fields.is_empty() || !kdf_fields.is_empty() {
        return None;
    }
    let derived = Zeroizing::new(
        argon2id(
            &passphrase,
            &kdf.salt,
            kdf.iterations,
            kdf.memory_kib,
            kdf.lanes.into(),
            32,
            &Argon2idOptions::default(),
        )
        .ok()?,
    );
    let mut key = Zeroizing::new([0; 32]);
    key.copy_from_slice(&derived);
    xchacha20_poly1305_aead_decrypt(&ciphertext, &key, &nonce, &artifact_aad(&kdf, nonce), &tag)
        .map(Zeroizing::new)
}

#[cfg(test)]
fn take_test_field(fields: &mut Vec<(CborValue, CborValue)>, key: u64) -> Option<CborValue> {
    let index = fields
        .iter()
        .position(|(candidate, _)| candidate == &CborValue::Unsigned(key))?;
    Some(fields.remove(index).1)
}

#[cfg(test)]
fn take_test_uint(fields: &mut Vec<(CborValue, CborValue)>, key: u64) -> Option<u64> {
    match take_test_field(fields, key)? {
        CborValue::Unsigned(value) => Some(value),
        _ => None,
    }
}

#[cfg(test)]
fn take_test_bytes(fields: &mut Vec<(CborValue, CborValue)>, key: u64) -> Option<Vec<u8>> {
    match take_test_field(fields, key)? {
        CborValue::Bytes(value) => Some(value),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_randomness_and_artifact_diagnostics_are_closed() {
        let policy = PortableExportPolicyV1::new(8 * 1024, 1, 1).unwrap();
        assert_eq!(policy.memory_kib(), 8 * 1024);
        assert_eq!(policy.iterations(), 1);
        assert_eq!(policy.lanes(), 1);
        assert_eq!(
            format!("{policy:?}"),
            "PortableExportPolicyV1 { memory_kib: 8192, iterations: 1, lanes: 1 }"
        );
        assert_eq!(
            PortableExportPolicyV1::new(1, 1, 1),
            Err(ApplicationError::InvalidInput)
        );

        let randomness = PortableExportRandomnessV1::new([0x5a; PORTABLE_EXPORT_RANDOM_BYTES]);
        assert_eq!(
            format!("{randomness:?}"),
            "PortableExportRandomnessV1(<redacted>)"
        );
        let artifact = PortableExportArtifactV1 {
            bytes: vec![1, 2, 3],
        };
        assert_eq!(
            format!("{artifact:?}"),
            "PortableExportArtifactV1(<encrypted>)"
        );
        assert_eq!(artifact.as_bytes(), &[1, 2, 3]);
        assert_eq!(artifact.into_bytes(), vec![1, 2, 3]);
    }

    #[test]
    fn recursive_cbor_wipe_clears_secret_shaped_allocations() {
        let mut value = CborValue::Map(vec![field(
            1,
            CborValue::Array(vec![
                CborValue::Bytes(b"secret bytes".to_vec()),
                CborValue::Text("secret text".to_owned()),
                CborValue::Tag(7, Box::new(CborValue::Bytes(vec![9; 8]))),
            ]),
        )]);
        zeroize_cbor(&mut value);
        let CborValue::Map(entries) = value else {
            panic!()
        };
        let CborValue::Array(values) = &entries[0].1 else {
            panic!()
        };
        assert_eq!(values[0], CborValue::Bytes(Vec::new()));
        assert_eq!(values[1], CborValue::Text(String::new()));
        assert_eq!(
            values[2],
            CborValue::Tag(7, Box::new(CborValue::Bytes(Vec::new())))
        );
    }
}
