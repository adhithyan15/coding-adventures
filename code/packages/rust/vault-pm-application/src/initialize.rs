use crate::{
    decode_device_certificate, encode_device_certificate, encode_signed_audit_event,
    encode_signed_commit, open_local_secret, open_object, seal_local_secret, seal_object,
    ActiveStateV1, ApplicationError, ApplicationRepositoryError, ApplicationRepositoryFactory,
    AuthorityFingerprint, BootstrapLocator, BootstrapStore, BootstrapStoreError, CatalogV1,
    LocalSecretRandomness, LocalSecretV1, LocalStateStore, LocalStateStoreError, LocalVaultStateV1,
    ObjectKind, ObjectRandomness, PreparedInitV1, PublicationJournalV1, V1Keys,
    V1SingleDeviceVerifier,
};
use coding_adventures_argon2id::{argon2id, Options as Argon2idOptions};
use coding_adventures_chacha20_poly1305::{
    xchacha20_poly1305_aead_decrypt, xchacha20_poly1305_aead_encrypt,
};
use coding_adventures_ed25519::{generate_keypair, is_valid_public_key, sign, verify};
use coding_adventures_vault_pm_audit::{AuditActionV1, AuditEventV1, AuditOutcomeV1};
use coding_adventures_vault_pm_domain::OperationId;
use coding_adventures_vault_pm_format::{
    AeadEnvelopeV1, AnnouncementV1, Argon2idParametersV1, BootstrapV1, CommitV1,
    DeviceCertificateV1, DeviceId, FormatError, PublicKey, Signature, VaultId, CRYPTO_SUITE_V1,
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
const OPERATION_ID_BYTES: usize = 32;
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

/// Exact caller-filled CSPRNG bytes consumed by audited generation zero.
pub const AUDITED_GENERATION_ZERO_RANDOM_BYTES: usize =
    GENERATION_ZERO_RANDOM_BYTES + OPERATION_ID_BYTES + OBJECT_RANDOM_BYTES;

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

/// One owned, wipe-on-drop CSPRNG block for audit-first generation zero.
pub struct AuditedGenerationZeroRandomness {
    bytes: [u8; AUDITED_GENERATION_ZERO_RANDOM_BYTES],
}

impl AuditedGenerationZeroRandomness {
    /// Take one exact block filled by the host's cryptographic entropy source.
    pub const fn new(bytes: [u8; AUDITED_GENERATION_ZERO_RANDOM_BYTES]) -> Self {
        Self { bytes }
    }
}

impl Zeroize for AuditedGenerationZeroRandomness {
    fn zeroize(&mut self) {
        self.bytes.zeroize();
    }
}

impl Drop for AuditedGenerationZeroRandomness {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl Debug for AuditedGenerationZeroRandomness {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("AuditedGenerationZeroRandomness(<redacted>)")
    }
}

/// Complete pure preparation result consumed by crash-resumable side effects.
pub struct PreparedGenerationZero {
    bootstrap_locator: BootstrapLocator,
    owner_state: LocalVaultStateV1,
    repository_address: RepositoryAddress,
    verifier: V1SingleDeviceVerifier,
}

pub(crate) struct UnlockedActiveMaterial {
    pub repository_address: RepositoryAddress,
    pub keys: V1Keys,
    pub local_secret: LocalSecretV1,
    pub verifier: V1SingleDeviceVerifier,
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
    prepare_generation_zero_inner(passphrase, policy, &randomness.bytes, None)
}

/// Deterministically prepare an audit-first generation zero whose initial
/// commit contains the signed, encrypted `VaultInitialize` genesis event.
pub fn prepare_audited_generation_zero(
    passphrase: Zeroizing<Vec<u8>>,
    policy: GenerationZeroPolicyV1,
    randomness: AuditedGenerationZeroRandomness,
) -> Result<PreparedGenerationZero, ApplicationError> {
    let mut audit_offset = GENERATION_ZERO_RANDOM_BYTES;
    let trace_id = OperationId::new(take(&randomness.bytes, &mut audit_offset));
    let audit_randomness = take_object_randomness(&randomness.bytes, &mut audit_offset);
    debug_assert_eq!(audit_offset, AUDITED_GENERATION_ZERO_RANDOM_BYTES);
    prepare_generation_zero_inner(
        passphrase,
        policy,
        &randomness.bytes[..GENERATION_ZERO_RANDOM_BYTES],
        Some((trace_id, audit_randomness)),
    )
}

fn prepare_generation_zero_inner(
    passphrase: Zeroizing<Vec<u8>>,
    policy: GenerationZeroPolicyV1,
    randomness: &[u8],
    audit_randomness: Option<(OperationId, ObjectRandomness)>,
) -> Result<PreparedGenerationZero, ApplicationError> {
    let mut offset = 0;
    let bootstrap_locator = BootstrapLocator::new(take(randomness, &mut offset));
    let vault_id = VaultId::new(take(randomness, &mut offset));
    let vault_root_key = Zeroizing::new(take(randomness, &mut offset));
    let kdf = Argon2idParametersV1 {
        memory_kib: policy.memory_kib,
        iterations: policy.iterations,
        lanes: policy.lanes,
        salt: take(randomness, &mut offset),
    };
    let root_wrap_nonce = take(randomness, &mut offset);
    let authority_seed = Zeroizing::new(take(randomness, &mut offset));
    let device_id = DeviceId::new(take(randomness, &mut offset));
    let device_signing_seed = Zeroizing::new(take(randomness, &mut offset));
    let device_x25519_secret = Zeroizing::new(take(randomness, &mut offset));
    let local_secret_randomness = LocalSecretRandomness::new(take(randomness, &mut offset));
    let certificate_randomness = take_object_randomness(randomness, &mut offset);
    let catalog_randomness = take_object_randomness(randomness, &mut offset);
    let commit_randomness = take_object_randomness(randomness, &mut offset);
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

    let audit_frame = audit_randomness
        .map(|(trace_id, randomness)| {
            let event = AuditEventV1::new(
                vault_id,
                device_id,
                1,
                trace_id,
                AuditActionV1::VaultInitialize,
                AuditOutcomeV1::Succeeded,
                None,
                None,
                None,
                None,
                Vec::new(),
                policy.created_at_ms,
            )
            .map_err(|_| ApplicationError::InternalInvariant)?
            .sign(&device_signing_seed)
            .map_err(|_| ApplicationError::InternalInvariant)?;
            let plaintext = Zeroizing::new(encode_signed_audit_event(&event)?);
            seal_object(&keys, ObjectKind::AuditEvent, &plaintext, &randomness)
        })
        .transpose()?;
    let audit_id = audit_frame
        .as_ref()
        .map(|frame| frame.id().map_err(|_| ApplicationError::InternalInvariant))
        .transpose()?;

    let mut added_objects = vec![certificate_id, catalog_id];
    if let Some(audit_id) = audit_id {
        added_objects.push(audit_id);
    }
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
    let mut objects = vec![certificate_frame.clone(), catalog_frame];
    if let Some(frame) = audit_frame {
        objects.push(frame);
    }
    let publication = PublicationJournalV1::new(
        objects,
        commit_frame,
        announcement,
        PinnedHeads::empty(),
        expected_heads.clone(),
        1,
        catalog_id,
    )?;
    let publication = match audit_id {
        Some(audit_id) => publication.with_audit_event_head(audit_id)?,
        None => publication,
    };
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
    let intended_active = match audit_id {
        Some(audit_id) => intended_active.with_audit_event_head(audit_id)?,
        None => intended_active,
    };
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
    let material = unlock_active_material(passphrase, active, prepared.bootstrap())?;

    Ok(PreparedGenerationZero {
        bootstrap_locator: active.bootstrap_locator(),
        owner_state,
        repository_address: material.repository_address,
        verifier: material.verifier,
    })
}

pub(crate) fn unlock_active_material(
    passphrase: Zeroizing<Vec<u8>>,
    active: &ActiveStateV1,
    exact_bootstrap: &[u8],
) -> Result<UnlockedActiveMaterial, ApplicationError> {
    let bootstrap = verify_active_bootstrap(active, exact_bootstrap)?;
    let vault_root_key = unwrap_root_key(&passphrase, &bootstrap)?;
    let keys = V1Keys::derive(bootstrap.vault_id, &vault_root_key)?;
    let verifier_keys = V1Keys::derive(bootstrap.vault_id, &vault_root_key)?;
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
        verifier_keys,
        authority_public,
        active.device_certificate_id(),
        active.device_certificate_frame(),
    )?;

    Ok(UnlockedActiveMaterial {
        repository_address,
        keys,
        local_secret,
        verifier,
    })
}

pub(crate) fn verify_active_bootstrap(
    active: &ActiveStateV1,
    exact_bootstrap: &[u8],
) -> Result<BootstrapV1, ApplicationError> {
    let bootstrap = verify_signed_bootstrap(exact_bootstrap)?;
    let bootstrap_id = bootstrap
        .id()
        .map_err(|_| ApplicationError::IntegrityFailure)?;
    if bootstrap_id != active.bootstrap_id()
        || bootstrap.vault_id != active.vault_id()
        || AuthorityFingerprint::for_public_key(bootstrap.authority_public_key)
            != active.authority_fingerprint()
    {
        return Err(ApplicationError::IntegrityFailure);
    }
    Ok(bootstrap)
}

pub(crate) fn verify_signed_bootstrap(
    exact_bootstrap: &[u8],
) -> Result<BootstrapV1, ApplicationError> {
    let bootstrap = BootstrapV1::decode(exact_bootstrap).map_err(map_bootstrap_format)?;
    let bootstrap_preimage = bootstrap
        .signing_preimage()
        .map_err(|_| ApplicationError::IntegrityFailure)?;
    if !is_valid_public_key(bootstrap.authority_public_key.as_bytes())
        || !verify(
            &bootstrap_preimage,
            bootstrap.signature.as_bytes(),
            bootstrap.authority_public_key.as_bytes(),
        )
    {
        return Err(ApplicationError::IntegrityFailure);
    }
    Ok(bootstrap)
}

fn map_bootstrap_format(error: FormatError) -> ApplicationError {
    match error {
        FormatError::UnsupportedVersion | FormatError::UnsupportedSuite => {
            ApplicationError::Unsupported
        }
        _ => ApplicationError::IntegrityFailure,
    }
}

/// Durably install and idempotently complete one exact generation-zero
/// journal through injected local, bootstrap, and repository authorities.
///
/// The exact `PreparedInit` bytes are atomically installed before the first
/// external effect. Every retry reuses the same signed and randomized bytes,
/// and the intended `Active` bytes replace the journal only after exact
/// bootstrap read-back and repository receipt verification.
pub fn complete_generation_zero(
    prepared: PreparedGenerationZero,
    local_state_store: &dyn LocalStateStore,
    bootstrap_store: &dyn BootstrapStore,
    repository_factory: &dyn ApplicationRepositoryFactory,
) -> Result<ActiveStateV1, ApplicationError> {
    let (locator, owner_state, address, verifier) = prepared.into_parts();
    let LocalVaultStateV1::PreparedInit(journal) = owner_state else {
        return Err(ApplicationError::InternalInvariant);
    };
    let prepared_state = LocalVaultStateV1::PreparedInit(journal.clone());
    let exact_prepared = prepared_state.encode()?;
    let intended_active = journal.intended_active().clone();
    let exact_active = LocalVaultStateV1::Active(intended_active.clone()).encode()?;

    if ensure_prepared_state(
        local_state_store,
        locator,
        &exact_prepared,
        &exact_active,
        &intended_active,
    )? {
        return Ok(intended_active);
    }

    bootstrap_store
        .put_generation(locator, None, journal.bootstrap())
        .map_err(map_bootstrap_store)?;
    let observed_bootstrap = bootstrap_store
        .load_latest(locator)
        .map_err(map_bootstrap_store)?
        .ok_or(ApplicationError::IntegrityFailure)?;
    if observed_bootstrap != journal.bootstrap() {
        return Err(ApplicationError::IntegrityFailure);
    }

    let repository = repository_factory
        .connect(address, Box::new(verifier))
        .map_err(map_application_repository)?;
    repository
        .initialize()
        .map_err(map_application_repository)?;
    let receipt = repository
        .publish(
            journal.publication().publication(),
            journal.publication().base_heads(),
        )
        .map_err(map_application_repository)?;
    if receipt.heads() != journal.publication().expected_heads() {
        return Err(ApplicationError::IntegrityFailure);
    }

    install_active_state(
        local_state_store,
        locator,
        &exact_prepared,
        &exact_active,
        intended_active,
    )
}

fn ensure_prepared_state(
    store: &dyn LocalStateStore,
    locator: BootstrapLocator,
    exact_prepared: &[u8],
    exact_active: &[u8],
    intended_active: &ActiveStateV1,
) -> Result<bool, ApplicationError> {
    match store.load(locator).map_err(map_local_state_store)? {
        Some(observed) if observed == exact_prepared => return Ok(false),
        Some(observed) if observed == exact_active => {
            let state = LocalVaultStateV1::decode(&observed)?;
            if state == LocalVaultStateV1::Active(intended_active.clone()) {
                return Ok(true);
            }
            return Err(ApplicationError::IntegrityFailure);
        }
        Some(observed) => {
            LocalVaultStateV1::decode(&observed)?;
            return Err(ApplicationError::AlreadyInitialized);
        }
        None => {}
    }

    match store.compare_exchange(locator, None, exact_prepared) {
        Ok(()) => Ok(false),
        Err(LocalStateStoreError::ConcurrentHost) => {
            match store.load(locator).map_err(map_local_state_store)? {
                Some(observed) if observed == exact_prepared => Ok(false),
                Some(observed) if observed == exact_active => Ok(true),
                _ => Err(ApplicationError::ConcurrentHost),
            }
        }
        Err(error) => Err(map_local_state_store(error)),
    }
}

fn install_active_state(
    store: &dyn LocalStateStore,
    locator: BootstrapLocator,
    exact_prepared: &[u8],
    exact_active: &[u8],
    intended_active: ActiveStateV1,
) -> Result<ActiveStateV1, ApplicationError> {
    match store.compare_exchange(locator, Some(exact_prepared), exact_active) {
        Ok(()) => Ok(intended_active),
        Err(LocalStateStoreError::ConcurrentHost) => {
            match store.load(locator).map_err(map_local_state_store)? {
                Some(observed) if observed == exact_active => Ok(intended_active),
                _ => Err(ApplicationError::ConcurrentHost),
            }
        }
        Err(error) => Err(map_local_state_store(error)),
    }
}

fn map_bootstrap_store(error: BootstrapStoreError) -> ApplicationError {
    match error {
        BootstrapStoreError::Unavailable => ApplicationError::StorageUnavailable,
        BootstrapStoreError::Conflict | BootstrapStoreError::Corruption => {
            ApplicationError::IntegrityFailure
        }
    }
}

fn map_local_state_store(error: LocalStateStoreError) -> ApplicationError {
    match error {
        LocalStateStoreError::Unavailable => ApplicationError::StorageUnavailable,
        LocalStateStoreError::ConcurrentHost => ApplicationError::ConcurrentHost,
        LocalStateStoreError::Corruption => ApplicationError::IntegrityFailure,
    }
}

fn map_application_repository(error: ApplicationRepositoryError) -> ApplicationError {
    match error {
        ApplicationRepositoryError::NotInitialized => ApplicationError::NotInitialized,
        ApplicationRepositoryError::InvalidInput => ApplicationError::InvalidInput,
        ApplicationRepositoryError::BoundExceeded => ApplicationError::BoundExceeded,
        ApplicationRepositoryError::StorageUnavailable => ApplicationError::StorageUnavailable,
        ApplicationRepositoryError::IntegrityFailure => ApplicationError::IntegrityFailure,
    }
}

pub(crate) fn wrap_root_key(
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

pub(crate) fn unwrap_root_key(
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

pub(crate) fn sign_bootstrap(value: BootstrapV1, secret: &[u8; 64]) -> Result<BootstrapV1, ApplicationError> {
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

fn take_object_randomness(bytes: &[u8], offset: &mut usize) -> ObjectRandomness {
    ObjectRandomness::new(
        take(bytes, offset),
        take(bytes, offset),
        take(bytes, offset),
    )
}

fn take<const N: usize>(bytes: &[u8], offset: &mut usize) -> [u8; N] {
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
    use coding_adventures_vault_pm_format::{BootstrapId, BootstrapV1};
    use coding_adventures_vault_pm_storage::{
        FaultAction, FaultEffect, FaultInjectingObjectStore, InMemoryObjectStore, StoreError,
        StoreOperation,
    };
    use std::sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    };

    #[derive(Default)]
    struct MemoryLocalStateStore {
        state: Mutex<Option<Vec<u8>>>,
        compare_calls: AtomicUsize,
        fail_compare_call: AtomicUsize,
    }

    impl MemoryLocalStateStore {
        fn fail_compare_call(&self, call: usize) {
            self.fail_compare_call.store(call, Ordering::SeqCst);
        }

        fn stored(&self) -> Option<Vec<u8>> {
            self.state.lock().unwrap().clone()
        }
    }

    impl LocalStateStore for MemoryLocalStateStore {
        fn load(
            &self,
            _locator: BootstrapLocator,
        ) -> Result<Option<Vec<u8>>, LocalStateStoreError> {
            Ok(self.stored())
        }

        fn compare_exchange(
            &self,
            _locator: BootstrapLocator,
            expected: Option<&[u8]>,
            replacement: &[u8],
        ) -> Result<(), LocalStateStoreError> {
            let call = self.compare_calls.fetch_add(1, Ordering::SeqCst) + 1;
            if self.fail_compare_call.load(Ordering::SeqCst) == call {
                return Err(LocalStateStoreError::Unavailable);
            }
            let mut state = self
                .state
                .lock()
                .map_err(|_| LocalStateStoreError::Unavailable)?;
            if state.as_deref() != expected {
                return Err(LocalStateStoreError::ConcurrentHost);
            }
            *state = Some(replacement.to_vec());
            Ok(())
        }
    }

    #[derive(Default)]
    struct MemoryBootstrapStore {
        bootstrap: Mutex<Option<Vec<u8>>>,
        put_calls: AtomicUsize,
        fail_next_put: AtomicBool,
        corrupt_next_read: AtomicBool,
    }

    impl MemoryBootstrapStore {
        fn fail_next_put(&self) {
            self.fail_next_put.store(true, Ordering::SeqCst);
        }

        fn corrupt_next_read(&self) {
            self.corrupt_next_read.store(true, Ordering::SeqCst);
        }

        fn put_calls(&self) -> usize {
            self.put_calls.load(Ordering::SeqCst)
        }

        fn stored(&self) -> Option<Vec<u8>> {
            self.bootstrap.lock().unwrap().clone()
        }
    }

    impl BootstrapStore for MemoryBootstrapStore {
        fn load_latest(
            &self,
            _locator: BootstrapLocator,
        ) -> Result<Option<Vec<u8>>, BootstrapStoreError> {
            let mut value = self.stored();
            if self.corrupt_next_read.swap(false, Ordering::SeqCst) {
                if let Some(bytes) = &mut value {
                    bytes[0] ^= 1;
                }
            }
            Ok(value)
        }

        fn put_generation(
            &self,
            _locator: BootstrapLocator,
            expected_previous: Option<BootstrapId>,
            exact_bootstrap: &[u8],
        ) -> Result<(), BootstrapStoreError> {
            self.put_calls.fetch_add(1, Ordering::SeqCst);
            if expected_previous.is_some() {
                return Err(BootstrapStoreError::Conflict);
            }
            if self.fail_next_put.swap(false, Ordering::SeqCst) {
                return Err(BootstrapStoreError::Unavailable);
            }
            let mut bootstrap = self
                .bootstrap
                .lock()
                .map_err(|_| BootstrapStoreError::Unavailable)?;
            match &*bootstrap {
                Some(existing) if existing == exact_bootstrap => Ok(()),
                Some(_) => Err(BootstrapStoreError::Conflict),
                None => {
                    *bootstrap = Some(exact_bootstrap.to_vec());
                    Ok(())
                }
            }
        }

        fn supersede_generation(
            &self,
            _locator: BootstrapLocator,
            _superseded: BootstrapId,
        ) -> Result<(), BootstrapStoreError> {
            // Generation zero never supersedes anything, and this fixture is
            // deliberately single-slot, so the only correct answer here is the
            // refusal `BootstrapStore` requires for the live generation.
            Err(BootstrapStoreError::Conflict)
        }
    }

    fn fixture_randomness() -> GenerationZeroRandomness {
        let mut bytes = [0; GENERATION_ZERO_RANDOM_BYTES];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = (index as u8).wrapping_mul(17).wrapping_add(1);
        }
        GenerationZeroRandomness::new(bytes)
    }

    fn fixture_audited_randomness() -> AuditedGenerationZeroRandomness {
        let mut bytes = [0; AUDITED_GENERATION_ZERO_RANDOM_BYTES];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = (index as u8).wrapping_mul(17).wrapping_add(1);
        }
        AuditedGenerationZeroRandomness::new(bytes)
    }

    fn policy() -> GenerationZeroPolicyV1 {
        GenerationZeroPolicyV1::new(8 * 1024, 1, 1, 1_700_000_000_000).unwrap()
    }

    fn prepared(passphrase: &[u8]) -> PreparedGenerationZero {
        prepare_generation_zero(
            Zeroizing::new(passphrase.to_vec()),
            policy(),
            fixture_randomness(),
        )
        .unwrap()
    }

    fn rehydrated(passphrase: &[u8], store: &MemoryLocalStateStore) -> PreparedGenerationZero {
        rehydrate_prepared_init(
            Zeroizing::new(passphrase.to_vec()),
            LocalVaultStateV1::decode(&store.stored().unwrap()).unwrap(),
        )
        .unwrap()
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
    fn audited_preparation_binds_vault_initialize_as_generation_zero_head() {
        let prepared = prepare_audited_generation_zero(
            Zeroizing::new(b"audit first passphrase".to_vec()),
            policy(),
            fixture_audited_randomness(),
        )
        .unwrap();
        let LocalVaultStateV1::PreparedInit(journal) = prepared.owner_state() else {
            panic!("generation zero must be prepared")
        };
        let audit_head = journal.intended_active().audit_event_head().unwrap();
        assert_eq!(journal.publication().audit_event_head(), Some(audit_head));
        assert_eq!(journal.publication().objects().len(), 3);
        assert!(journal
            .publication()
            .objects()
            .iter()
            .any(|frame| frame.id().unwrap() == audit_head));
        let material = unlock_active_material(
            Zeroizing::new(b"audit first passphrase".to_vec()),
            journal.intended_active(),
            journal.bootstrap(),
        )
        .unwrap();
        let audit_frame = journal
            .publication()
            .objects()
            .iter()
            .find(|frame| frame.id().unwrap() == audit_head)
            .unwrap();
        let plaintext = open_object(&material.keys, ObjectKind::AuditEvent, audit_frame).unwrap();
        let signed = crate::decode_signed_audit_event(&plaintext).unwrap();
        assert_eq!(signed.event().device_counter(), 1);
        assert_eq!(signed.event().action(), AuditActionV1::VaultInitialize);
        assert_eq!(signed.event().outcome(), AuditOutcomeV1::Succeeded);
        assert_eq!(signed.event().previous_event(), None);
        assert!(signed.event().basis_heads().is_empty());
        assert_eq!(
            format!("{:?}", fixture_audited_randomness()),
            "AuditedGenerationZeroRandomness(<redacted>)"
        );
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
    fn completion_persists_first_finishes_exactly_and_is_idempotent() {
        let passphrase = b"completion passphrase";
        let initial = prepared(passphrase);
        let expected_active = match initial.owner_state() {
            LocalVaultStateV1::PreparedInit(journal) => journal.intended_active().clone(),
            _ => panic!("generation zero must be prepared"),
        };
        let expected_bootstrap = match initial.owner_state() {
            LocalVaultStateV1::PreparedInit(journal) => journal.bootstrap().to_vec(),
            _ => unreachable!(),
        };
        let local = MemoryLocalStateStore::default();
        let bootstrap = MemoryBootstrapStore::default();
        let factory = V1ApplicationRepositoryFactory::new(InMemoryObjectStore::new());

        let active = complete_generation_zero(initial, &local, &bootstrap, &factory).unwrap();
        assert_eq!(active, expected_active);
        assert_eq!(bootstrap.stored(), Some(expected_bootstrap));
        assert_eq!(
            LocalVaultStateV1::decode(&local.stored().unwrap()).unwrap(),
            LocalVaultStateV1::Active(expected_active.clone())
        );

        let put_calls = bootstrap.put_calls();
        assert_eq!(
            complete_generation_zero(prepared(passphrase), &local, &bootstrap, &factory).unwrap(),
            expected_active
        );
        assert_eq!(bootstrap.put_calls(), put_calls);
    }

    #[test]
    fn completion_performs_no_external_effect_when_prepared_state_is_not_durable() {
        let local = MemoryLocalStateStore::default();
        local.fail_compare_call(1);
        let bootstrap = MemoryBootstrapStore::default();
        let factory = V1ApplicationRepositoryFactory::new(InMemoryObjectStore::new());

        assert_eq!(
            complete_generation_zero(prepared(b"persist-first"), &local, &bootstrap, &factory,)
                .err(),
            Some(ApplicationError::StorageUnavailable)
        );
        assert_eq!(local.stored(), None);
        assert_eq!(bootstrap.put_calls(), 0);
    }

    #[test]
    fn completion_resumes_exactly_after_bootstrap_and_readback_failures() {
        let passphrase = b"bootstrap recovery";
        for corrupt_readback in [false, true] {
            let local = MemoryLocalStateStore::default();
            let bootstrap = MemoryBootstrapStore::default();
            if corrupt_readback {
                bootstrap.corrupt_next_read();
            } else {
                bootstrap.fail_next_put();
            }
            let factory = V1ApplicationRepositoryFactory::new(InMemoryObjectStore::new());

            let error =
                complete_generation_zero(prepared(passphrase), &local, &bootstrap, &factory).err();
            assert_eq!(
                error,
                Some(if corrupt_readback {
                    ApplicationError::IntegrityFailure
                } else {
                    ApplicationError::StorageUnavailable
                })
            );
            assert!(matches!(
                LocalVaultStateV1::decode(&local.stored().unwrap()).unwrap(),
                LocalVaultStateV1::PreparedInit(_)
            ));

            complete_generation_zero(rehydrated(passphrase, &local), &local, &bootstrap, &factory)
                .unwrap();
            assert!(matches!(
                LocalVaultStateV1::decode(&local.stored().unwrap()).unwrap(),
                LocalVaultStateV1::Active(_)
            ));
        }
    }

    #[test]
    fn completion_recovers_exact_publication_after_provider_failures() {
        let passphrase = b"repository recovery";
        for fault in [
            FaultAction {
                operation: StoreOperation::Initialize,
                effect: FaultEffect::Return(StoreError::Network),
            },
            FaultAction {
                operation: StoreOperation::PutImmutable,
                effect: FaultEffect::CommitPutThenNetwork,
            },
        ] {
            let local = MemoryLocalStateStore::default();
            let bootstrap = MemoryBootstrapStore::default();
            let backend = Arc::new(FaultInjectingObjectStore::new(InMemoryObjectStore::new()));
            backend.enqueue(fault).unwrap();
            let factory = V1ApplicationRepositoryFactory::from_shared(Arc::clone(&backend));

            assert_eq!(
                complete_generation_zero(prepared(passphrase), &local, &bootstrap, &factory,).err(),
                Some(ApplicationError::StorageUnavailable)
            );
            assert!(matches!(
                LocalVaultStateV1::decode(&local.stored().unwrap()).unwrap(),
                LocalVaultStateV1::PreparedInit(_)
            ));

            complete_generation_zero(rehydrated(passphrase, &local), &local, &bootstrap, &factory)
                .unwrap();
            assert_eq!(backend.pending_faults().unwrap(), 0);
            assert!(matches!(
                LocalVaultStateV1::decode(&local.stored().unwrap()).unwrap(),
                LocalVaultStateV1::Active(_)
            ));
        }
    }

    #[test]
    fn completion_recovers_after_active_compare_exchange_failure() {
        let passphrase = b"local commit recovery";
        let local = MemoryLocalStateStore::default();
        local.fail_compare_call(2);
        let bootstrap = MemoryBootstrapStore::default();
        let factory = V1ApplicationRepositoryFactory::new(InMemoryObjectStore::new());

        assert_eq!(
            complete_generation_zero(prepared(passphrase), &local, &bootstrap, &factory,).err(),
            Some(ApplicationError::StorageUnavailable)
        );
        assert!(matches!(
            LocalVaultStateV1::decode(&local.stored().unwrap()).unwrap(),
            LocalVaultStateV1::PreparedInit(_)
        ));

        complete_generation_zero(rehydrated(passphrase, &local), &local, &bootstrap, &factory)
            .unwrap();
        assert!(matches!(
            LocalVaultStateV1::decode(&local.stored().unwrap()).unwrap(),
            LocalVaultStateV1::Active(_)
        ));
    }

    #[test]
    fn completion_rejects_occupied_or_corrupt_local_state_before_external_effects() {
        for (occupied, expected) in [
            (
                prepared(b"different initialization")
                    .owner_state()
                    .encode()
                    .unwrap(),
                ApplicationError::AlreadyInitialized,
            ),
            (vec![0xff], ApplicationError::IntegrityFailure),
        ] {
            let local = MemoryLocalStateStore::default();
            *local.state.lock().unwrap() = Some(occupied);
            let bootstrap = MemoryBootstrapStore::default();
            let factory = V1ApplicationRepositoryFactory::new(InMemoryObjectStore::new());

            assert_eq!(
                complete_generation_zero(
                    prepared(b"requested initialization"),
                    &local,
                    &bootstrap,
                    &factory,
                )
                .err(),
                Some(expected)
            );
            assert_eq!(bootstrap.put_calls(), 0);
        }
    }

    #[test]
    fn completion_error_translation_is_closed() {
        assert_eq!(
            [
                BootstrapStoreError::Unavailable,
                BootstrapStoreError::Conflict,
                BootstrapStoreError::Corruption,
            ]
            .map(map_bootstrap_store),
            [
                ApplicationError::StorageUnavailable,
                ApplicationError::IntegrityFailure,
                ApplicationError::IntegrityFailure,
            ]
        );
        assert_eq!(
            [
                LocalStateStoreError::Unavailable,
                LocalStateStoreError::ConcurrentHost,
                LocalStateStoreError::Corruption,
            ]
            .map(map_local_state_store),
            [
                ApplicationError::StorageUnavailable,
                ApplicationError::ConcurrentHost,
                ApplicationError::IntegrityFailure,
            ]
        );
        assert_eq!(
            [
                ApplicationRepositoryError::NotInitialized,
                ApplicationRepositoryError::InvalidInput,
                ApplicationRepositoryError::BoundExceeded,
                ApplicationRepositoryError::StorageUnavailable,
                ApplicationRepositoryError::IntegrityFailure,
            ]
            .map(map_application_repository),
            [
                ApplicationError::NotInitialized,
                ApplicationError::InvalidInput,
                ApplicationError::BoundExceeded,
                ApplicationError::StorageUnavailable,
                ApplicationError::IntegrityFailure,
            ]
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
        let mut audited = fixture_audited_randomness();
        assert_eq!(
            format!("{audited:?}"),
            "AuditedGenerationZeroRandomness(<redacted>)"
        );
        audited.zeroize();
        assert!(audited.bytes.iter().all(|byte| *byte == 0));
    }
}
