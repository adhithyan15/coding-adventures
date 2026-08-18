//! VLT-PM43 passphrase rotation: re-wrap the root key, touch no item body.
//!
//! # The one thing this module is for
//!
//! `VLT-PM00-local-first-password-manager.md` §14.8 requires that
//!
//! > password rotation rewraps the VRK without re-encrypting every item body.
//!
//! That is a claim about *cost and blast radius*, not merely about
//! functionality. A password manager that derived item encryption from the
//! master passphrase would have to decrypt and re-encrypt every record on every
//! password change — minutes of work, a window in which a crash leaves half the
//! vault under each key, and a strong reason never to rotate at all.
//!
//! This product avoids that by construction. The passphrase never encrypts
//! anything except one 32-byte value:
//!
//! ```text
//!   passphrase ──argon2id(salt, m, t, p)──▶ KEK
//!                                            │  XChaCha20-Poly1305
//!                                            ▼
//!                       BootstrapV1.passphrase_root_wrap  (32 bytes of ciphertext)
//!                                            │  unwraps
//!                                            ▼
//!                                       VRK (random 256-bit)
//!                                            │  HKDF, closed purpose labels
//!            ┌───────────────┬───────────────┼───────────────┐
//!            ▼               ▼               ▼               ▼
//!       locator key    object wrap key  local state key   audit key
//!                            │
//!                            ▼
//!                 per-object random DEKs ──▶ every ObjectFrameV1
//! ```
//!
//! So a rotation is: unwrap the VRK under the old KEK, wrap *the same VRK*
//! under a new KEK derived with a fresh salt, publish the new bootstrap, and
//! retire the old one. One Argon2id derivation, one AEAD open, one AEAD seal,
//! all on 32 bytes. Nothing below the VRK is read, rewritten, or even opened.
//!
//! # What "durably rotated" has to mean
//!
//! Two independent durable stores hold the two halves of the answer to "which
//! bootstrap does this vault use": the bootstrap store serves one, and the
//! owner-private local state pins the ID it will accept. Moving them one at a
//! time without a journal has a landing point that wedges the vault outright —
//! see [`crate::PendingRotationV1`], which exists for exactly that reason.
//!
//! The rules this module implements, in order of how much they matter:
//!
//! 1. **The journal is the commit point.** Before it is durable the vault
//!    belongs to the old passphrase; at and after it, to the new one.
//! 2. **Recovery rolls forward, and consumes no passphrase.** Every step after
//!    the journal is a pure function of the journal, so a killed process is
//!    finished by the next ordinary command rather than by a special verb.
//! 3. **The retired wrap is deleted, not merely unpointed-at.** A superseded
//!    generation left on disk would still surrender the unchanged VRK to
//!    anyone holding the old passphrase.

use crate::initialize::{sign_bootstrap, unwrap_root_key, verify_active_bootstrap, wrap_root_key};
use crate::{
    open_local_secret, ActiveStateV1, ApplicationError, BootstrapLocator, BootstrapStore,
    BootstrapStoreError, LocalSecretV1, LocalStateStore, LocalStateStoreError, LocalVaultStateV1,
    PendingRotationV1, V1Keys,
};
use coding_adventures_ed25519::generate_keypair;
use coding_adventures_vault_pm_format::{
    Argon2idParametersV1, BootstrapV1, PublicKey, Signature, CRYPTO_SUITE_V1,
};
use coding_adventures_zeroize::{Zeroize, Zeroizing};
use core::fmt::{self, Debug, Formatter};

const KDF_SALT_BYTES: usize = 16;
const ROOT_WRAP_NONCE_BYTES: usize = 24;

/// Exact caller-filled CSPRNG bytes consumed by one passphrase rotation.
///
/// One fresh 16-byte Argon2id salt and one fresh 24-byte AEAD nonce. Both must
/// be new: reusing the salt would let an attacker who precomputed against the
/// old one carry that work forward, and reusing the nonce under a different key
/// is not a break but is a habit this codebase refuses to form.
pub const PASSPHRASE_ROTATION_RANDOM_BYTES: usize = KDF_SALT_BYTES + ROOT_WRAP_NONCE_BYTES;

/// One owned, wipe-on-drop CSPRNG block for a rotation's salt and nonce.
pub struct PassphraseRotationRandomnessV1 {
    bytes: [u8; PASSPHRASE_ROTATION_RANDOM_BYTES],
}

impl PassphraseRotationRandomnessV1 {
    /// Take one exact block filled by the host's cryptographic entropy source.
    pub const fn new(bytes: [u8; PASSPHRASE_ROTATION_RANDOM_BYTES]) -> Self {
        Self { bytes }
    }
}

impl Zeroize for PassphraseRotationRandomnessV1 {
    fn zeroize(&mut self) {
        self.bytes.zeroize();
    }
}

impl Drop for PassphraseRotationRandomnessV1 {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl Debug for PassphraseRotationRandomnessV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("PassphraseRotationRandomnessV1(<redacted>)")
    }
}

/// Bounded Argon2id policy applied to the newly collected passphrase.
///
/// A rotation adopts the host's *current* calibration rather than copying the
/// parameters the vault was created with, so a person who rotates on a faster
/// machine gets stronger parameters as a side effect. There is deliberately no
/// way to change parameters without changing the passphrase: a verb that
/// re-derived a KEK from a passphrase the person did not just retype would have
/// to hold that passphrase somewhere, and nothing in this product does.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PassphraseRotationPolicyV1 {
    memory_kib: u32,
    iterations: u32,
    lanes: u8,
}

impl PassphraseRotationPolicyV1 {
    /// Validate one caller-calibrated Argon2id policy against the V1 bounds.
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
}

impl Debug for PassphraseRotationPolicyV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PassphraseRotationPolicyV1")
            .field("memory_kib", &self.memory_kib)
            .field("iterations", &self.iterations)
            .field("lanes", &self.lanes)
            .finish()
    }
}

/// Complete pure preparation result consumed by the durable rotation.
///
/// It holds exactly one thing: the authority-signed bytes of the next
/// bootstrap. Those bytes are a public, provider-discoverable record — the
/// wrapped root key inside them is ciphertext no passphrase-free reader can
/// open — so this value carries no live secret, and both passphrases and the
/// root key are already wiped by the time it exists.
///
/// It deliberately does *not* carry the owner state the rotation will replace.
/// An audited rotation publishes its audit event between preparation and
/// commitment, and that publication advances the owner state; a preparation
/// that had captured the pre-audit state would either have to be rebased or
/// would fail its compare-exchange. Committing against whatever state is
/// current at commit time removes the question.
pub struct PreparedPassphraseRotationV1 {
    bootstrap: Vec<u8>,
}

impl PreparedPassphraseRotationV1 {
    /// Borrow the exact authority-signed next bootstrap bytes.
    pub fn bootstrap(&self) -> &[u8] {
        &self.bootstrap
    }
}

impl Debug for PreparedPassphraseRotationV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedPassphraseRotationV1")
            .field("bootstrap_bytes", &self.bootstrap.len())
            .finish()
    }
}

/// Deterministically build the next signed bootstrap without any external write.
///
/// # Arguments that look redundant and are not
///
/// `current_passphrase` is collected a second time by the caller even though
/// the session it belongs to is already unlocked. An [`crate::UnlockedVaultV1`]
/// retains the *derived* subkeys and not the vault root key, deliberately, so
/// that the root's lifetime is one function call rather than one session.
/// Rotation is the only operation that needs the root, so it is the only
/// operation that pays a second Argon2id derivation for it.
///
/// `expected_local_secret` is the session's own decrypted owner secret. After
/// unwrapping the root key this function derives subkeys from it and opens the
/// active state's sealed local secret with them; the result must equal the
/// session's. An AEAD that opens under the derived local-state key and yields
/// the identical owner secret proves the root behind it is the same root, so
/// the check binds the rotation to the session without comparing key bytes.
///
/// # Failure
///
/// A wrong current passphrase is `AuthenticationFailed` — the same closed class
/// an ordinary unlock returns, from the same unwrap. Anything that shows
/// persisted state disagreeing with itself is `IntegrityFailure`: the
/// passphrase has already authenticated by then, so a disagreement is not the
/// person's mistake to be told about.
///
/// Nothing durable happens here. Every failure leaves a vault that the current
/// passphrase still opens.
pub fn prepare_passphrase_rotation(
    active: &ActiveStateV1,
    exact_bootstrap: &[u8],
    current_passphrase: &Zeroizing<Vec<u8>>,
    new_passphrase: &Zeroizing<Vec<u8>>,
    policy: PassphraseRotationPolicyV1,
    randomness: &PassphraseRotationRandomnessV1,
    expected_local_secret: &LocalSecretV1,
) -> Result<PreparedPassphraseRotationV1, ApplicationError> {
    let bootstrap = verify_active_bootstrap(active, exact_bootstrap)?;
    let next_generation = bootstrap
        .generation
        .checked_add(1)
        .ok_or(ApplicationError::BoundExceeded)?;

    // The root key exists from here to the re-wrap below and nowhere else.
    // `unwrap_root_key` wipes the KEK it derived from the *old* passphrase
    // before it returns, so the retired credential's derived key is already
    // gone by the time the new one is derived.
    let vault_root_key = unwrap_root_key(current_passphrase, &bootstrap)?;

    let keys = V1Keys::derive(bootstrap.vault_id, &vault_root_key)?;
    let observed_local_secret = open_local_secret(&keys, active.local_secret())?;
    if &observed_local_secret != expected_local_secret {
        return Err(ApplicationError::IntegrityFailure);
    }

    // `generate_keypair` returns the secret *by value*, and `[u8; 64]` is
    // `Copy`, so wrapping it would leave the original array on this frame
    // un-wiped. Bind it mutably, take the owned copy, and wipe the original.
    let (authority_public, mut raw_authority_secret) =
        generate_keypair(observed_local_secret.authority_seed());
    let mut authority_secret = Zeroizing::new(raw_authority_secret);
    raw_authority_secret.zeroize();
    if PublicKey::new(authority_public) != bootstrap.authority_public_key {
        authority_secret.zeroize();
        return Err(ApplicationError::IntegrityFailure);
    }

    let mut offset = 0;
    let salt = take(&randomness.bytes, &mut offset);
    let nonce = take(&randomness.bytes, &mut offset);
    debug_assert_eq!(offset, PASSPHRASE_ROTATION_RANDOM_BYTES);

    // A rotation may raise the KDF cost and may never lower it. The doc on
    // `PassphraseRotationPolicyV1` promises stronger parameters as a side
    // effect of rotating on a faster machine; without this floor the same
    // parameter path would also let a caller hand the person a *weaker*
    // credential than the one they already had, in the ceremony they ran
    // specifically to improve their security. The shipped CLI passes a fixed
    // production policy, so this guards embedders and any future host that
    // calibrates at run time.
    if policy.memory_kib < bootstrap.kdf.memory_kib || policy.iterations < bootstrap.kdf.iterations
    {
        authority_secret.zeroize();
        return Err(ApplicationError::InvalidInput);
    }

    let kdf = Argon2idParametersV1 {
        memory_kib: policy.memory_kib,
        iterations: policy.iterations,
        lanes: policy.lanes,
        salt,
    };
    let passphrase_root_wrap = wrap_root_key(
        new_passphrase,
        &kdf,
        bootstrap.vault_id,
        &vault_root_key,
        nonce,
    )?;
    drop(vault_root_key);

    let next = sign_bootstrap(
        BootstrapV1 {
            vault_id: bootstrap.vault_id,
            generation: next_generation,
            previous_bootstrap: Some(active.bootstrap_id()),
            crypto_suite: CRYPTO_SUITE_V1,
            kdf,
            passphrase_root_wrap,
            authority_public_key: bootstrap.authority_public_key,
            // Recovery recipients wrap the same unchanged root key, so a
            // passphrase rotation neither invalidates nor rebuilds them.
            recovery_wraps: bootstrap.recovery_wraps.clone(),
            signature: Signature::new([0; 64]),
        },
        &authority_secret,
    )?;
    authority_secret.zeroize();

    Ok(PreparedPassphraseRotationV1 {
        bootstrap: next
            .encode()
            .map_err(|_| ApplicationError::InternalInvariant)?,
    })
}

/// Durably journal, install, supersede, and activate one prepared rotation.
///
/// The compare-exchange from `Active` to `PendingRotation` is the commit point
/// described in this module's header. A concurrent local writer is accepted
/// only when it installed the identical journal or the identical intended
/// state; everything else fails closed as `ConcurrentHost`.
pub fn commit_passphrase_rotation(
    active: &ActiveStateV1,
    prepared: PreparedPassphraseRotationV1,
    local_state_store: &dyn LocalStateStore,
    bootstrap_store: &dyn BootstrapStore,
) -> Result<ActiveStateV1, ApplicationError> {
    let journal = PendingRotationV1::new(active.clone(), prepared.bootstrap)?;
    let locator = active.bootstrap_locator();
    let exact_active = LocalVaultStateV1::Active(active.clone()).encode()?;
    let exact_pending = LocalVaultStateV1::PendingRotation(journal.clone()).encode()?;
    let exact_intended = LocalVaultStateV1::Active(journal.intended_active()?).encode()?;

    match local_state_store.compare_exchange(locator, Some(&exact_active), &exact_pending) {
        Ok(()) => {}
        Err(LocalStateStoreError::ConcurrentHost) => {
            match local_state_store
                .load(locator)
                .map_err(map_local_state_store)?
            {
                Some(observed) if observed == exact_pending => {}
                Some(observed) if observed == exact_intended => return journal.intended_active(),
                _ => return Err(ApplicationError::ConcurrentHost),
            }
        }
        Err(error) => return Err(map_local_state_store(error)),
    }

    finish_pending_rotation(
        locator,
        &journal,
        &exact_pending,
        local_state_store,
        bootstrap_store,
    )
}

/// Finish one exact durable `PendingRotation` without collecting a passphrase.
///
/// # Why no passphrase
///
/// Everything left to do is a function of the journal: the signed bootstrap
/// bytes are in it, the ID to retire is in it, and the intended owner state is
/// computed from it. Asking for a secret here would create a much worse
/// problem than it solved, because at this point in the ceremony *which*
/// passphrase is correct depends on how far the interrupted process had got —
/// which is precisely the ambiguity the journal exists to remove.
///
/// Recovery therefore always rolls forward, and the rule a person can hold is
/// simple: once the machine accepted the new passphrase, it is the passphrase.
///
/// Replay is idempotent at every step. `put_generation` accepts the identical
/// already-installed generation as success, `supersede_generation` accepts an
/// already-absent record as success, and the final compare-exchange re-reads
/// and accepts the value it intended to write.
pub fn recover_pending_rotation(
    locator: BootstrapLocator,
    local_state_store: &dyn LocalStateStore,
    bootstrap_store: &dyn BootstrapStore,
) -> Result<ActiveStateV1, ApplicationError> {
    let exact_pending = local_state_store
        .load(locator)
        .map_err(map_local_state_store)?
        .ok_or(ApplicationError::NotInitialized)?;
    let LocalVaultStateV1::PendingRotation(journal) = LocalVaultStateV1::decode(&exact_pending)?
    else {
        return Err(ApplicationError::InvalidInput);
    };
    if journal.active().bootstrap_locator() != locator {
        return Err(ApplicationError::IntegrityFailure);
    }
    finish_pending_rotation(
        locator,
        &journal,
        &exact_pending,
        local_state_store,
        bootstrap_store,
    )
}

fn finish_pending_rotation(
    locator: BootstrapLocator,
    journal: &PendingRotationV1,
    exact_pending: &[u8],
    local_state_store: &dyn LocalStateStore,
    bootstrap_store: &dyn BootstrapStore,
) -> Result<ActiveStateV1, ApplicationError> {
    let superseded = journal.superseded_bootstrap_id();

    if let Err(error) =
        bootstrap_store.put_generation(locator, Some(superseded), journal.bootstrap())
    {
        return Err(abandon_uninstalled_rotation(
            locator,
            journal,
            exact_pending,
            local_state_store,
            bootstrap_store,
            map_bootstrap_store(error),
        ));
    }
    // Read the provider back before retiring anything. The old wrap is the
    // only remaining way into this vault until the new one is actually being
    // served, so the delete below must never run on the strength of a write
    // that merely returned success.
    let observed = bootstrap_store
        .load_latest(locator)
        .map_err(map_bootstrap_store)?
        .ok_or(ApplicationError::IntegrityFailure)?;
    if observed != journal.bootstrap() {
        return Err(ApplicationError::IntegrityFailure);
    }
    bootstrap_store
        .supersede_generation(locator, superseded)
        .map_err(map_bootstrap_store)?;

    let intended = journal.intended_active()?;
    let exact_intended = LocalVaultStateV1::Active(intended.clone()).encode()?;
    match local_state_store.compare_exchange(locator, Some(exact_pending), &exact_intended) {
        Ok(()) => Ok(intended),
        Err(LocalStateStoreError::ConcurrentHost) => {
            match local_state_store
                .load(locator)
                .map_err(map_local_state_store)?
            {
                Some(observed) if observed == exact_intended => Ok(intended),
                _ => Err(ApplicationError::ConcurrentHost),
            }
        }
        Err(error) => Err(map_local_state_store(error)),
    }
}

/// Undo a journalled rotation that provably never reached the provider.
///
/// # The one landing point that would otherwise have no exit
///
/// Rolling forward is the rule (see [`recover_pending_rotation`]) precisely
/// because, once anything has been installed, no passphrase-free code can tell
/// how far the ceremony got. There is exactly one shape of failure where that
/// uncertainty does not exist: `put_generation` refused, *and* the store's
/// latest is still the generation this rotation intended to retire. Then the
/// install demonstrably did not happen, and neither did the supersession that
/// strictly follows it — so the durable world is byte-for-byte the one the
/// rotation started from, and the old passphrase is still the vault's
/// passphrase.
///
/// Without this, a `Conflict` from the bootstrap store after the journal landed
/// would be terminal. `map_bootstrap_store` turns it into `IntegrityFailure`,
/// which is not a wait-and-retry class, and every later command re-enters the
/// roll-forward through the same door and fails the same way — a vault bricked
/// by a transient answer from a provider, with no verb to escape it. A
/// contract that only ever rolls forward is worth less than one that also
/// knows when it is safe to stand still.
///
/// # Why this is not a hole in "the journal is the commit point"
///
/// The rule the product actually needs is *exactly one passphrase works, and a
/// person can find out which*. Rolling back here preserves it: the old
/// passphrase opens the vault, the new one does not, and the command reports
/// failure rather than success. The rotation simply did not happen, and the
/// person retries.
///
/// Anything that stops this function from *proving* the install never happened
/// — an unreadable provider, a latest pointer naming something else, a failed
/// compare-exchange — leaves the journal exactly where it was and returns the
/// original error. Guessing here would be the one mistake with no recovery.
fn abandon_uninstalled_rotation(
    locator: BootstrapLocator,
    journal: &PendingRotationV1,
    exact_pending: &[u8],
    local_state_store: &dyn LocalStateStore,
    bootstrap_store: &dyn BootstrapStore,
    original: ApplicationError,
) -> ApplicationError {
    let Ok(Some(observed)) = bootstrap_store.load_latest(locator) else {
        return original;
    };
    let Ok(observed) = BootstrapV1::decode(&observed) else {
        return original;
    };
    if observed.id().ok() != Some(journal.superseded_bootstrap_id()) {
        return original;
    }
    let Ok(exact_active) = LocalVaultStateV1::Active(journal.active().clone()).encode() else {
        return original;
    };
    match local_state_store.compare_exchange(locator, Some(exact_pending), &exact_active) {
        Ok(()) => original,
        Err(_) => original,
    }
}

fn take<const N: usize>(bytes: &[u8], offset: &mut usize) -> [u8; N] {
    let end = *offset + N;
    let value = bytes[*offset..end]
        .try_into()
        .expect("the rotation partition lengths are constant");
    *offset = end;
    value
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
