use crate::{
    decode_device_certificate, encode_device_certificate, encode_signed_commit, open_local_secret,
    open_object, seal_local_secret, seal_object, ActiveStateV1, ApplicationError,
    AuthorityFingerprint, BootstrapLocator, CatalogV1, LocalSecretRandomness, LocalSecretV1,
    LocalVaultStateV1, ObjectKind, ObjectRandomness, PreparedInitV1, PublicationJournalV1, V1Keys,
    V1SingleDeviceVerifier,
};
use coding_adventures_argon2id::{argon2id, Options as Argon2idOptions};
use coding_adventures_chacha20_poly1305::{
    xchacha20_poly1305_aead_decrypt, xchacha20_poly1305_aead_encrypt,
};
use coding_adventures_ed25519::{generate_keypair, sign};
use coding_adventures_vault_pm_format::{
    AeadEnvelopeV1, AnnouncementV1, Argon2idParametersV1, BootstrapV1, CommitV1,
    DeviceCertificateV1, DeviceId, PublicKey, Signature, VaultId, CRYPTO_SUITE_V1,
};
use coding_adventures_vault_pm_repository::{PinnedHeads, RepositoryAddress};
use coding_adventures_x25519::generate_keypair as generate_x25519_public_key;
use coding_adventures_zeroize::{Zeroize, Zeroizing};
use core::fmt::{self, Debug, Formatter};

const ROOT_WRAP_AAD_PREFIX: &[u8] = b"VPM-ROOT-WRAP-v1";
const BOOTSTRAP_LOCATOR_BYTES: usize = 32;
const VAULT_ID_BYTES: usize = 16;
const VAULT_ROOT_KEY_BYTES: usize = 32;
const KDF_SALT_BYTES: usize = 16;
const ROOT_WRAP_NONCE_BYTES: usize = 24;
const AUTHORITY_SEED_BYTES: usize = 32;
const DEVICE_ID_BYTES: usize = 16;
const DEVICE_SIGNING_SEED_BYTES: usize = 32;
const DEVICE_X25519_SECRET_BYTES: usize = 32;
const LOCAL_SECRET_NONCE_BYTES: usize = 24;
const OBJECT_RANDOM_BYTES: usize = 32 + 24 + 24;

/// Exact caller-filled CSPRNG bytes consumed by generation-zero preparation.
pub const GENERATION_ZERO_RANDOM_BYTES: usize = BOOTSTRAP_LOCATOR_BYTES
    + VAULT_ID_BYTES
    + VAULT_ROOT_KEY_BYTES
    + KDF_SALT_BYTES
    + ROOT_WRAP_NONCE_BYTES
    + AUTHORITY_SEED_BYTES
    + DEVICE_ID_BYTES
    + DEVICE_SIGNING_SEED_BYTES
    + DEVICE_X25519_SECRET_BYTES
    + LOCAL_SECRET_NONCE_BYTES
    + 3 * OBJECT_RANDOM_BYTES;

/// Bounded password-KDF policy and advisory time for generation zero.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct GenerationZeroPolicyV1 {
    memory_kib: u32,
    iterations: u32,
    lanes: u8,
    created_at_ms: u64,
}

impl GenerationZeroPolicyV1 {
    /// Validate one caller-calibrated Argon2id policy and advisory timestamp.
    pub fn new(
        memory_kib: u32,
        iterations: u32,
        lanes: u8,
        created_at_ms: u64,
    ) -> Result<Self, ApplicationError> {
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
            created_at_ms,
        })
    }

    /// Return the bounded Argon2id memory cost in KiB.
    pub const fn memory_kib(&self) -> u32 {
        self.memory_kib
    }

    /// Return the bounded Argon2id iteration count.
    pub const fn iterations(&self) -> u32 {
        self.iterations
    }

    /// Return the bounded Argon2id lane count.
    pub const fn lanes(&self) -> u8 {
        self.lanes
    }

    /// Return the advisory certificate and initial-commit time.
    pub const fn created_at_ms(&self) -> u64 {
        self.created_at_ms
    }
}

impl Debug for GenerationZeroPolicyV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GenerationZeroPolicyV1")
            .field("memory_kib", &self.memory_kib)
            .field("iterations", &self.iterations)
            .field("lanes", &self.lanes)
            .finish_non_exhaustive()
    }
}

/// One owned, wipe-on-drop CSPRNG block partitioned into independent values.
pub struct GenerationZeroRandomness {
    bytes: [u8; GENERATION_ZERO_RANDOM_BYTES],
}

impl GenerationZeroRandomness {
    /// Take one exact block filled by the host's cryptographic entropy source.
    pub const fn new(bytes: [u8; GENERATION_ZERO_RANDOM_BYTES]) -> Self {
        Self { bytes }
    }
}

impl Zeroize for GenerationZeroRandomness {
    fn zeroize(&mut self) {
        self.bytes.zeroize();
    }
}

impl Drop for GenerationZeroRandomness {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl Debug for GenerationZeroRandomness {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("GenerationZeroRandomness(<redacted>)")
    }
}

/// Complete pure preparation result consumed by crash-resumable side effects.
pub struct PreparedGenerationZero {
    bootstrap_locator: BootstrapLocator,
    owner_state: LocalVaultStateV1,
    repository_address: RepositoryAddress,
    verifier: V1SingleDeviceVerifier,
}

impl PreparedGenerationZero {
    /// Return the random provider-independent owner/bootstrap locator.
    pub const fn bootstrap_locator(&self) -> BootstrapLocator {
        self.bootstrap_locator
    }

    /// Borrow the exact prepared owner-state machine value.
    pub const fn owner_state(&self) -> &LocalVaultStateV1 {
        &self.owner_state
    }

    /// Return the opaque repository address derived from the live VRK.
    pub const fn repository_address(&self) -> RepositoryAddress {
        self.repository_address
    }

    /// Consume the preparation for persistence and verified repository use.
    pub fn into_parts(
        self,
    ) -> (
        BootstrapLocator,
        LocalVaultStateV1,
        RepositoryAddress,
        V1SingleDeviceVerifier,
    ) {
        (
            self.bootstrap_locator,
            self.owner_state,
            self.repository_address,
            self.verifier,
        )
    }
}

impl Debug for PreparedGenerationZero {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("PreparedGenerationZero(<redacted>)")
    }
}

/// Deterministically prepare every generation-zero byte before external writes.
pub fn prepare_generation_zero(
    passphrase: Zeroizing<Vec<u8>>,
    policy: GenerationZeroPolicyV1,
    randomness: GenerationZeroRandomness,
) -> Result<PreparedGenerationZero, ApplicationError> {
    let mut offset = 0;
    let bootstrap_locator = BootstrapLocator::new(take(&randomness.bytes, &mut offset));
    let vault_id = VaultId::new(take(&randomness.bytes, &mut offset));
    let vault_root_key = Zeroizing::new(take(&randomness.bytes, &mut offset));
    let kdf = Argon2idParametersV1 {
        memory_kib: policy.memory_kib,
        iterations: policy.iterations,
        lanes: policy.lanes,
        salt: take(&randomness.bytes, &mut offset),
    };
    let root_wrap_nonce = take(&randomness.bytes, &mut offset);
    let authority_seed = Zeroizing::new(take(&randomness.bytes, &mut offset));
    let device_id = DeviceId::new(take(&randomness.bytes, &mut offset));
    let device_signing_seed = Zeroizing::new(take(&randomness.bytes, &mut offset));
    let device_x25519_secret = Zeroizing::new(take(&randomness.bytes, &mut offset));
    let local_secret_randomness = LocalSecretRandomness::new(take(&randomness.bytes, &mut offset));
    let certificate_randomness = take_object_randomness(&randomness.bytes, &mut offset);
    let catalog_randomness = take_object_randomness(&randomness.bytes, &mut offset);
    let commit_randomness = take_object_randomness(&randomness.bytes, &mut offset);
    debug_assert_eq!(offset, GENERATION_ZERO_RANDOM_BYTES);

    let keys = V1Keys::derive(vault_id, &vault_root_key)?;
    let repository_address = RepositoryAddress::derive(keys.locator_key());
    let root_wrap = wrap_root_key(
        &passphrase,
        &kdf,
        vault_id,
        &vault_root_key,
        root_wrap_nonce,
    )?;

    let (authority_public, authority_secret) = generate_keypair(&authority_seed);
    let authority_secret = Zeroizing::new(authority_secret);
    let authority_public = PublicKey::new(authority_public);
    let (device_signing_public, device_signing_secret) = generate_keypair(&device_signing_seed);
    let device_signing_secret = Zeroizing::new(device_signing_secret);
    let device_wrapping_public = PublicKey::new(
        generate_x25519_public_key(&device_x25519_secret)
            .map_err(|_| ApplicationError::InternalInvariant)?,
    );

    let bootstrap = sign_bootstrap(
        BootstrapV1 {
            vault_id,
            generation: 0,
            previous_bootstrap: None,
            crypto_suite: CRYPTO_SUITE_V1,
            kdf,
            passphrase_root_wrap: root_wrap,
            authority_public_key: authority_public,
            recovery_wraps: Vec::new(),
            signature: Signature::new([0; 64]),
        },
        &authority_secret,
    )?;
    let bootstrap_id = bootstrap
        .id()
        .map_err(|_| ApplicationError::InternalInvariant)?;
    let bootstrap_bytes = bootstrap
        .encode()
        .map_err(|_| ApplicationError::InternalInvariant)?;

    let certificate = sign_certificate(
        DeviceCertificateV1 {
            vault_id,
            device_id,
            signing_public_key: PublicKey::new(device_signing_public),
            wrapping_public_key: device_wrapping_public,
            created_at_ms: policy.created_at_ms,
            capabilities: Vec::new(),
            signature: Signature::new([0; 64]),
        },
        &authority_secret,
    )?;
    let certificate_plaintext = Zeroizing::new(encode_device_certificate(&certificate)?);
    let certificate_frame = seal_object(
        &keys,
        ObjectKind::DeviceCertificate,
        &certificate_plaintext,
        &certificate_randomness,
    )?;
    let certificate_id = certificate_frame
        .id()
        .map_err(|_| ApplicationError::InternalInvariant)?;

    let catalog_plaintext = Zeroizing::new(CatalogV1::empty().encode()?);
    let catalog_frame = seal_object(
        &keys,
        ObjectKind::Catalog,
        &catalog_plaintext,
        &catalog_randomness,
    )?;
    let catalog_id = catalog_frame
        .id()
        .map_err(|_| ApplicationError::InternalInvariant)?;

    let mut added_objects = vec![certificate_id, catalog_id];
    added_objects.sort_unstable();
    let commit = sign_commit(
        CommitV1 {
            vault_id,
            device_id,
            device_counter: 1,
            parents: Vec::new(),
            catalog_root: catalog_id,
            added_objects,
            tombstone_root: None,
            wall_time_ms: policy.created_at_ms,
            device_certificate: certificate_id,
            signature: Signature::new([0; 64]),
        },
        &device_signing_secret,
    )?;
    let commit_plaintext = Zeroizing::new(encode_signed_commit(&commit)?);
    let commit_frame = seal_object(
        &keys,
        ObjectKind::Commit,
        &commit_plaintext,
        &commit_randomness,
    )?;
    let commit_id = commit_frame
        .id()
        .map_err(|_| ApplicationError::InternalInvariant)?;
    let announcement = sign_announcement(
        AnnouncementV1 {
            vault_id,
            device_id,
            device_counter: 1,
            commit_id,
            device_certificate: certificate_id,
            signature: Signature::new([0; 64]),
        },
        &device_signing_secret,
    )?
    .encode()
    .map_err(|_| ApplicationError::InternalInvariant)?;

    let local_secret = LocalSecretV1::new(
        vault_id,
        device_id,
        *authority_seed,
        *device_signing_seed,
        *device_x25519_secret,
    );
    let sealed_local_secret = seal_local_secret(&keys, &local_secret, &local_secret_randomness)?;
    let expected_heads =
        PinnedHeads::new([commit_id]).map_err(|_| ApplicationError::InternalInvariant)?;
    let publication = PublicationJournalV1::new(
        vec![certificate_frame.clone(), catalog_frame],
        commit_frame,
        announcement,
        PinnedHeads::empty(),
        expected_heads.clone(),
        1,
        catalog_id,
    )?;
    let intended_active = ActiveStateV1::new(
        bootstrap_locator,
        vault_id,
        bootstrap_id,
        AuthorityFingerprint::for_public_key(authority_public),
        device_id,
        certificate_id,
        certificate_frame.clone(),
        sealed_local_secret,
        expected_heads,
        1,
        catalog_id,
    )?;
    let prepared = PreparedInitV1::new(bootstrap_bytes, intended_active, publication)?;
    let verifier = V1SingleDeviceVerifier::authorize(
        keys,
        authority_public,
        certificate_id,
        &certificate_frame,
    )?;

    Ok(PreparedGenerationZero {
        bootstrap_locator,
        owner_state: LocalVaultStateV1::PreparedInit(prepared),
        repository_address,
        verifier,
    })
}

/// Reconstruct the generation-zero repository authority from a durable
/// `PreparedInit` journal after process loss, without performing external
/// writes.
///
/// A wrong passphrase and an unauthenticatable passphrase root wrap share the
/// same closed failure. All other persisted identity or signature mismatches
/// are integrity failures.
pub fn rehydrate_prepared_init(
    passphrase: Zeroizing<Vec<u8>>,
    owner_state: LocalVaultStateV1,
) -> Result<PreparedGenerationZero, ApplicationError> {
    let LocalVaultStateV1::PreparedInit(prepared) = &owner_state else {
        return Err(ApplicationError::InvalidInput);
    };
    let active = prepared.intended_active();
    let bootstrap = BootstrapV1::decode(prepared.bootstrap())
        .map_err(|_| ApplicationError::IntegrityFailure)?;
    let vault_root_key = unwrap_root_key(&passphrase, &bootstrap)?;
    let keys = V1Keys::derive(bootstrap.vault_id, &vault_root_key)?;
    let repository_address = RepositoryAddress::derive(keys.locator_key());
    let local_secret = open_local_secret(&keys, active.local_secret())?;

    if local_secret.vault_id() != active.vault_id()
        || local_secret.device_id() != active.device_id()
    {
        return Err(ApplicationError::IntegrityFailure);
    }

    let (authority_public, authority_secret) = generate_keypair(local_secret.authority_seed());
    let mut authority_secret = Zeroizing::new(authority_secret);
    authority_secret.zeroize();
    let authority_public = PublicKey::new(authority_public);
    let (device_signing_public, device_signing_secret) =
        generate_keypair(local_secret.device_signing_seed());
    let mut device_signing_secret = Zeroizing::new(device_signing_secret);
    device_signing_secret.zeroize();
    let device_wrapping_public = PublicKey::new(
        generate_x25519_public_key(local_secret.device_x25519_secret())
            .map_err(|_| ApplicationError::IntegrityFailure)?,
    );
    let certificate_plaintext = open_object(
        &keys,
        ObjectKind::DeviceCertificate,
        active.device_certificate_frame(),
    )?;
    let certificate = decode_device_certificate(&certificate_plaintext)?;

    if authority_public != bootstrap.authority_public_key
        || certificate.vault_id != active.vault_id()
        || certificate.device_id != active.device_id()
        || certificate.signing_public_key != PublicKey::new(device_signing_public)
        || certificate.wrapping_public_key != device_wrapping_public
    {
        return Err(ApplicationError::IntegrityFailure);
    }

    let verifier = V1SingleDeviceVerifier::authorize(
        keys,
        authority_public,
        active.device_certificate_id(),
        active.device_certificate_frame(),
    )?;

    Ok(PreparedGenerationZero {
        bootstrap_locator: active.bootstrap_locator(),
        owner_state,
        repository_address,
        verifier,
    })
}

fn wrap_root_key(
    passphrase: &[u8],
    kdf: &Argon2idParametersV1,
    vault_id: VaultId,
    vault_root_key: &[u8; 32],
    nonce: [u8; 24],
) -> Result<AeadEnvelopeV1, ApplicationError> {
    kdf.validate().map_err(|_| ApplicationError::InvalidInput)?;
    let derived = Zeroizing::new(
        argon2id(
            passphrase,
            &kdf.salt,
            kdf.iterations,
            kdf.memory_kib,
            kdf.lanes.into(),
            32,
            &Argon2idOptions::default(),
        )
        .map_err(|_| ApplicationError::InvalidInput)?,
    );
    let mut kek = Zeroizing::new([0; 32]);
    kek.copy_from_slice(&derived);
    let aad = root_wrap_aad(vault_id);
    let (ciphertext, tag) = xchacha20_poly1305_aead_encrypt(vault_root_key, &kek, &nonce, &aad);
    Ok(AeadEnvelopeV1 {
        suite: CRYPTO_SUITE_V1,
        nonce,
        ciphertext,
        tag,
    })
}

fn unwrap_root_key(
    passphrase: &[u8],
    bootstrap: &BootstrapV1,
) -> Result<Zeroizing<[u8; 32]>, ApplicationError> {
    bootstrap
        .kdf
        .validate()
        .map_err(|_| ApplicationError::IntegrityFailure)?;
    let root_wrap = &bootstrap.passphrase_root_wrap;
    if root_wrap.suite != CRYPTO_SUITE_V1 || root_wrap.validate().is_err() {
        return Err(ApplicationError::AuthenticationFailed);
    }
    let derived = Zeroizing::new(
        argon2id(
            passphrase,
            &bootstrap.kdf.salt,
            bootstrap.kdf.iterations,
            bootstrap.kdf.memory_kib,
            bootstrap.kdf.lanes.into(),
            32,
            &Argon2idOptions::default(),
        )
        .map_err(|_| ApplicationError::IntegrityFailure)?,
    );
    let mut kek = Zeroizing::new([0; 32]);
    kek.copy_from_slice(&derived);
    let opened = xchacha20_poly1305_aead_decrypt(
        &root_wrap.ciphertext,
        &kek,
        &root_wrap.nonce,
        &root_wrap_aad(bootstrap.vault_id),
        &root_wrap.tag,
    )
    .ok_or(ApplicationError::AuthenticationFailed)?;
    let opened = Zeroizing::new(opened);
    let mut vault_root_key = Zeroizing::new([0; 32]);
    if opened.len() != vault_root_key.len() {
        return Err(ApplicationError::AuthenticationFailed);
    }
    vault_root_key.copy_from_slice(&opened);
    Ok(vault_root_key)
}

fn root_wrap_aad(vault_id: VaultId) -> Vec<u8> {
    let mut aad = Vec::with_capacity(ROOT_WRAP_AAD_PREFIX.len() + 2 + 16);
    aad.extend_from_slice(ROOT_WRAP_AAD_PREFIX);
    aad.extend_from_slice(&CRYPTO_SUITE_V1.to_be_bytes());
    aad.extend_from_slice(vault_id.as_bytes());
    aad
}

fn sign_bootstrap(value: BootstrapV1, secret: &[u8; 64]) -> Result<BootstrapV1, ApplicationError> {
    let preimage = value
        .signing_preimage()
        .map_err(|_| ApplicationError::InternalInvariant)?;
    Ok(value.with_signature(Signature::new(sign(&preimage, secret))))
}

fn sign_certificate(
    value: DeviceCertificateV1,
    secret: &[u8; 64],
) -> Result<DeviceCertificateV1, ApplicationError> {
    let preimage = value
        .signing_preimage()
        .map_err(|_| ApplicationError::InternalInvariant)?;
    Ok(value.with_signature(Signature::new(sign(&preimage, secret))))
}

fn sign_commit(value: CommitV1, secret: &[u8; 64]) -> Result<CommitV1, ApplicationError> {
    let preimage = value
        .signing_preimage()
        .map_err(|_| ApplicationError::InternalInvariant)?;
    Ok(value.with_signature(Signature::new(sign(&preimage, secret))))
}

fn sign_announcement(
    value: AnnouncementV1,
    secret: &[u8; 64],
) -> Result<AnnouncementV1, ApplicationError> {
    let preimage = value
        .signing_preimage()
        .map_err(|_| ApplicationError::InternalInvariant)?;
    Ok(value.with_signature(Signature::new(sign(&preimage, secret))))
}

fn take_object_randomness(
    bytes: &[u8; GENERATION_ZERO_RANDOM_BYTES],
    offset: &mut usize,
) -> ObjectRandomness {
    ObjectRandomness::new(
        take(bytes, offset),
        take(bytes, offset),
        take(bytes, offset),
    )
}

fn take<const N: usize>(bytes: &[u8; GENERATION_ZERO_RANDOM_BYTES], offset: &mut usize) -> [u8; N] {
    let end = *offset + N;
    let value = bytes[*offset..end]
        .try_into()
        .expect("generation-zero partition lengths are constant");
    *offset = end;
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{open_local_secret, ApplicationRepositoryFactory, V1ApplicationRepositoryFactory};
    use coding_adventures_argon2id::argon2id;
    use coding_adventures_chacha20_poly1305::xchacha20_poly1305_aead_decrypt;
    use coding_adventures_ed25519::verify;
    use coding_adventures_vault_pm_format::BootstrapV1;
    use coding_adventures_vault_pm_storage::InMemoryObjectStore;

    fn fixture_randomness() -> GenerationZeroRandomness {
        let mut bytes = [0; GENERATION_ZERO_RANDOM_BYTES];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = (index as u8).wrapping_mul(17).wrapping_add(1);
        }
        GenerationZeroRandomness::new(bytes)
    }

    fn policy() -> GenerationZeroPolicyV1 {
        GenerationZeroPolicyV1::new(8 * 1024, 1, 1, 1_700_000_000_000).unwrap()
    }

    #[test]
    fn preparation_is_complete_and_immediately_publishable() {
        let prepared = prepare_generation_zero(
            Zeroizing::new(b"correct horse battery staple".to_vec()),
            policy(),
            fixture_randomness(),
        )
        .unwrap();
        assert_eq!(
            format!("{prepared:?}"),
            "PreparedGenerationZero(<redacted>)"
        );
        let exact_state = prepared.owner_state().encode().unwrap();
        assert_eq!(
            LocalVaultStateV1::decode(&exact_state).unwrap(),
            *prepared.owner_state()
        );

        let (locator, state, address, verifier) = prepared.into_parts();
        let LocalVaultStateV1::PreparedInit(journal) = state else {
            panic!("generation zero must be prepared")
        };
        assert_eq!(locator, journal.intended_active().bootstrap_locator());
        let bootstrap = BootstrapV1::decode(journal.bootstrap()).unwrap();
        assert_eq!(bootstrap.generation, 0);
        assert_eq!(bootstrap.kdf.memory_kib, 8 * 1024);
        assert!(verify(
            &bootstrap.signing_preimage().unwrap(),
            bootstrap.signature.as_bytes(),
            bootstrap.authority_public_key.as_bytes(),
        ));

        let derived = Zeroizing::new(
            argon2id(
                b"correct horse battery staple",
                &bootstrap.kdf.salt,
                bootstrap.kdf.iterations,
                bootstrap.kdf.memory_kib,
                bootstrap.kdf.lanes.into(),
                32,
                &Argon2idOptions::default(),
            )
            .unwrap(),
        );
        let mut kek = Zeroizing::new([0; 32]);
        kek.copy_from_slice(&derived);
        let opened_root = Zeroizing::new(
            xchacha20_poly1305_aead_decrypt(
                &bootstrap.passphrase_root_wrap.ciphertext,
                &kek,
                &bootstrap.passphrase_root_wrap.nonce,
                &root_wrap_aad(bootstrap.vault_id),
                &bootstrap.passphrase_root_wrap.tag,
            )
            .unwrap(),
        );
        let mut root_key = Zeroizing::new([0; 32]);
        root_key.copy_from_slice(&opened_root);
        let keys = V1Keys::derive(bootstrap.vault_id, &root_key).unwrap();
        let local_secret =
            open_local_secret(&keys, journal.intended_active().local_secret()).unwrap();
        assert_eq!(local_secret.vault_id(), bootstrap.vault_id);
        assert_eq!(
            local_secret.device_id(),
            journal.intended_active().device_id()
        );

        let factory = V1ApplicationRepositoryFactory::new(InMemoryObjectStore::new());
        let repository = factory.connect(address, Box::new(verifier)).unwrap();
        repository.initialize().unwrap();
        let receipt = repository
            .publish(
                journal.publication().publication(),
                journal.publication().base_heads(),
            )
            .unwrap();
        assert_eq!(receipt.heads(), journal.publication().expected_heads());
    }

    #[test]
    fn preparation_is_deterministic_for_identical_owned_inputs() {
        let first = prepare_generation_zero(
            Zeroizing::new(b"same input".to_vec()),
            policy(),
            fixture_randomness(),
        )
        .unwrap();
        let second = prepare_generation_zero(
            Zeroizing::new(b"same input".to_vec()),
            policy(),
            fixture_randomness(),
        )
        .unwrap();
        assert_eq!(first.bootstrap_locator(), second.bootstrap_locator());
        assert_eq!(first.repository_address(), second.repository_address());
        assert_eq!(
            first.owner_state().encode().unwrap(),
            second.owner_state().encode().unwrap()
        );
    }

    #[test]
    fn persisted_preparation_rehydrates_after_process_loss_and_publishes() {
        let prepared = prepare_generation_zero(
            Zeroizing::new(b"restart-safe passphrase".to_vec()),
            policy(),
            fixture_randomness(),
        )
        .unwrap();
        let locator = prepared.bootstrap_locator();
        let address = prepared.repository_address();
        let exact_state = prepared.owner_state().encode().unwrap();
        drop(prepared);

        let recovered = rehydrate_prepared_init(
            Zeroizing::new(b"restart-safe passphrase".to_vec()),
            LocalVaultStateV1::decode(&exact_state).unwrap(),
        )
        .unwrap();
        assert_eq!(recovered.bootstrap_locator(), locator);
        assert_eq!(recovered.repository_address(), address);
        assert_eq!(recovered.owner_state().encode().unwrap(), exact_state);

        let (_, state, recovered_address, verifier) = recovered.into_parts();
        let LocalVaultStateV1::PreparedInit(journal) = state else {
            panic!("rehydration must retain the prepared journal")
        };
        let factory = V1ApplicationRepositoryFactory::new(InMemoryObjectStore::new());
        let repository = factory
            .connect(recovered_address, Box::new(verifier))
            .unwrap();
        repository.initialize().unwrap();
        let receipt = repository
            .publish(
                journal.publication().publication(),
                journal.publication().base_heads(),
            )
            .unwrap();
        assert_eq!(receipt.heads(), journal.publication().expected_heads());
    }

    #[test]
    fn rehydration_closes_authentication_and_state_failures() {
        let prepared = prepare_generation_zero(
            Zeroizing::new(b"correct passphrase".to_vec()),
            policy(),
            fixture_randomness(),
        )
        .unwrap();
        let state = LocalVaultStateV1::decode(&prepared.owner_state().encode().unwrap()).unwrap();
        let active = match &state {
            LocalVaultStateV1::PreparedInit(journal) => journal.intended_active().clone(),
            _ => panic!("generation zero must be prepared"),
        };

        assert_eq!(
            rehydrate_prepared_init(Zeroizing::new(b"wrong passphrase".to_vec()), state).err(),
            Some(ApplicationError::AuthenticationFailed)
        );
        assert_eq!(
            rehydrate_prepared_init(
                Zeroizing::new(b"correct passphrase".to_vec()),
                LocalVaultStateV1::Active(active),
            )
            .err(),
            Some(ApplicationError::InvalidInput)
        );
    }

    #[test]
    fn rehydration_rejects_local_private_seeds_that_do_not_match_public_identity() {
        let passphrase = b"identity-bound passphrase";
        let prepared = prepare_generation_zero(
            Zeroizing::new(passphrase.to_vec()),
            policy(),
            fixture_randomness(),
        )
        .unwrap();
        let (_, state, _, _) = prepared.into_parts();
        let LocalVaultStateV1::PreparedInit(journal) = state else {
            panic!("generation zero must be prepared")
        };
        let bootstrap = BootstrapV1::decode(journal.bootstrap()).unwrap();
        let root_key = unwrap_root_key(passphrase, &bootstrap).unwrap();
        let keys = V1Keys::derive(bootstrap.vault_id, &root_key).unwrap();
        let active = journal.intended_active();
        let mismatched_secret = LocalSecretV1::new(
            active.vault_id(),
            active.device_id(),
            [0xa1; 32],
            [0xb2; 32],
            [0xc3; 32],
        );
        let mismatched_envelope = seal_local_secret(
            &keys,
            &mismatched_secret,
            &LocalSecretRandomness::new([0xd4; 24]),
        )
        .unwrap();
        let mismatched_active = ActiveStateV1::new(
            active.bootstrap_locator(),
            active.vault_id(),
            active.bootstrap_id(),
            active.authority_fingerprint(),
            active.device_id(),
            active.device_certificate_id(),
            active.device_certificate_frame().clone(),
            mismatched_envelope,
            active.pinned_heads().clone(),
            active.last_device_counter(),
            active.catalog_root(),
        )
        .unwrap();
        let mismatched_state = LocalVaultStateV1::PreparedInit(
            PreparedInitV1::new(
                journal.bootstrap().to_vec(),
                mismatched_active,
                journal.publication().clone(),
            )
            .unwrap(),
        );

        assert_eq!(
            rehydrate_prepared_init(Zeroizing::new(passphrase.to_vec()), mismatched_state).err(),
            Some(ApplicationError::IntegrityFailure)
        );
    }

    #[test]
    fn policy_randomness_and_diagnostics_are_closed() {
        assert_eq!(
            GenerationZeroPolicyV1::new(1024, 1, 1, 0).err(),
            Some(ApplicationError::InvalidInput)
        );
        let policy = policy();
        assert_eq!(policy.memory_kib(), 8 * 1024);
        assert_eq!(policy.iterations(), 1);
        assert_eq!(policy.lanes(), 1);
        assert_eq!(policy.created_at_ms(), 1_700_000_000_000);
        assert_eq!(
            format!("{policy:?}"),
            "GenerationZeroPolicyV1 { memory_kib: 8192, iterations: 1, lanes: 1, .. }"
        );
        let mut randomness = fixture_randomness();
        assert_eq!(
            format!("{randomness:?}"),
            "GenerationZeroRandomness(<redacted>)"
        );
        randomness.zeroize();
        assert!(randomness.bytes.iter().all(|byte| *byte == 0));
    }
}
