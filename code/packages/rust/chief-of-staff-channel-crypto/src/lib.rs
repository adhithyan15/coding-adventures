//! Cryptographic building blocks for D18 Chief of Staff channels.
//!
//! A channel uses one shared channel master key (CMK) per epoch. The
//! originator wraps that CMK independently for each authorized receiver and
//! signs every grant. Messages use the channel UUID and a never-resetting
//! sequence number as their XChaCha20-Poly1305 nonce.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::collections::BTreeMap;

use coding_adventures_chacha20_poly1305::{
    xchacha20_poly1305_aead_decrypt, xchacha20_poly1305_aead_encrypt,
};
use coding_adventures_csprng::random_array;
use coding_adventures_ed25519::{generate_keypair as ed25519_keypair, sign, verify};
use coding_adventures_hkdf::{hkdf, HashAlgorithm};
use coding_adventures_sha256::sha256;
use coding_adventures_x25519::{generate_keypair as x25519_public_key, x25519};
use coding_adventures_zeroize::Zeroizing;

const KEY_GRANT_CONTEXT: &[u8] = b"chief-channel-key-grant-v1";
const KEY_WRAP_CONTEXT: &[u8] = b"chief-channel-key-wrap-v1";
const MESSAGE_CONTEXT: &[u8] = b"chief-channel-message-v1";

/// Errors returned by Chief channel cryptographic operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChannelCryptoError {
    /// The operating system random-number generator was unavailable.
    RandomnessUnavailable,
    /// X25519 rejected a low-order public key or produced an invalid secret.
    InvalidKeyAgreement,
    /// HKDF could not derive the requested wrapping key.
    KeyDerivationFailed,
    /// A key grant's Ed25519 signature was invalid.
    InvalidGrantSignature,
    /// A key grant named a different originator than the receiver expected.
    UnexpectedOriginator,
    /// A key grant named a different receiver than the local receiver.
    UnexpectedReceiver,
    /// A key grant named a different channel than the receiver expected.
    UnexpectedChannel,
    /// The wrapped CMK or encrypted payload failed AEAD authentication.
    AuthenticationFailed,
    /// The wrapped plaintext was not exactly one 32-byte CMK.
    InvalidWrappedKey,
    /// A different key grant already exists for the current epoch.
    ConflictingGrant,
    /// A key grant attempted to move receiver state to an older epoch.
    DecreasingEpoch,
    /// A message's Ed25519 signature was invalid.
    InvalidMessageSignature,
    /// Decrypted bytes did not match the authenticated plaintext hash.
    PlaintextHashMismatch,
    /// Durable sequence state was missing during restart recovery.
    MissingDurableSequence,
    /// Durable sequence state moved behind the minimum safe value.
    DecreasingSequence,
    /// The persisted sequence reservation did not match the required advance.
    DurableSequenceMismatch,
    /// No sequence remains after `u64::MAX`.
    SequenceExhausted,
}

impl core::fmt::Display for ChannelCryptoError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::RandomnessUnavailable => "channel crypto randomness unavailable",
            Self::InvalidKeyAgreement => "channel crypto key agreement failed",
            Self::KeyDerivationFailed => "channel crypto key derivation failed",
            Self::InvalidGrantSignature => "channel key grant signature invalid",
            Self::UnexpectedOriginator => "channel key grant originator mismatch",
            Self::UnexpectedReceiver => "channel key grant receiver mismatch",
            Self::UnexpectedChannel => "channel key grant channel mismatch",
            Self::AuthenticationFailed => "channel ciphertext authentication failed",
            Self::InvalidWrappedKey => "channel key grant plaintext length invalid",
            Self::ConflictingGrant => "conflicting channel key grant for current epoch",
            Self::DecreasingEpoch => "channel key grant epoch decreased",
            Self::InvalidMessageSignature => "channel message signature invalid",
            Self::PlaintextHashMismatch => "channel message plaintext hash mismatch",
            Self::MissingDurableSequence => "durable channel sequence is missing",
            Self::DecreasingSequence => "durable channel sequence decreased",
            Self::DurableSequenceMismatch => "durable channel sequence reservation mismatch",
            Self::SequenceExhausted => "channel sequence exhausted",
        };
        f.write_str(message)
    }
}

impl std::error::Error for ChannelCryptoError {}

/// Canonical 16-byte channel UUID representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChannelId(pub [u8; 16]);

/// Monotonically increasing channel-key generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyEpoch(pub u64);

/// Globally monotonic message sequence within one channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Sequence(pub u64);

/// A 256-bit channel master key that is wiped when dropped.
pub struct ChannelMasterKey {
    bytes: Zeroizing<[u8; 32]>,
}

impl ChannelMasterKey {
    /// Generate a new CMK from the operating system CSPRNG.
    pub fn generate() -> Result<Self, ChannelCryptoError> {
        let bytes = random_array().map_err(|_| ChannelCryptoError::RandomnessUnavailable)?;
        Ok(Self::from_bytes(bytes))
    }

    /// Take ownership of existing CMK bytes.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self {
            bytes: Zeroizing::new(bytes),
        }
    }

    /// Borrow the CMK for an immediate cryptographic operation.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.bytes
    }
}

/// A receiver's X25519 key pair with zeroizing private-key storage.
pub struct ReceiverKeyPair {
    private_key: Zeroizing<[u8; 32]>,
    public_key: [u8; 32],
}

impl ReceiverKeyPair {
    /// Generate a receiver key pair from the operating system CSPRNG.
    pub fn generate() -> Result<Self, ChannelCryptoError> {
        let private_key = random_array().map_err(|_| ChannelCryptoError::RandomnessUnavailable)?;
        Self::from_private_key(private_key)
    }

    /// Derive a receiver key pair from caller-owned private bytes.
    pub fn from_private_key(private_key: [u8; 32]) -> Result<Self, ChannelCryptoError> {
        let private_key = Zeroizing::new(private_key);
        let public_key =
            x25519_public_key(&private_key).map_err(|_| ChannelCryptoError::InvalidKeyAgreement)?;
        Ok(Self {
            private_key,
            public_key,
        })
    }

    /// Return the public key that originators use when sealing grants.
    pub fn public_key(&self) -> [u8; 32] {
        self.public_key
    }
}

/// An originator's Ed25519 key pair with zeroizing secret-key storage.
pub struct OriginatorSigningKey {
    secret_key: Zeroizing<[u8; 64]>,
    public_key: [u8; 32],
}

impl OriginatorSigningKey {
    /// Generate a signing key from the operating system CSPRNG.
    pub fn generate() -> Result<Self, ChannelCryptoError> {
        let seed = random_array().map_err(|_| ChannelCryptoError::RandomnessUnavailable)?;
        Ok(Self::from_seed(seed))
    }

    /// Derive a signing key pair from a caller-owned Ed25519 seed.
    pub fn from_seed(seed: [u8; 32]) -> Self {
        let seed = Zeroizing::new(seed);
        let (public_key, secret_key) = ed25519_keypair(&seed);
        Self {
            secret_key: Zeroizing::new(secret_key),
            public_key,
        }
    }

    /// Return the Ed25519 public verification key.
    pub fn public_key(&self) -> [u8; 32] {
        self.public_key
    }
}

/// A CMK encrypted for one receiver and signed by its originator.
#[derive(Clone, PartialEq, Eq)]
pub struct SealedChannelKeyGrant {
    /// Entity that created and signed this grant.
    pub originator_id: Vec<u8>,
    /// Only entity whose X25519 private key can unwrap this grant.
    pub receiver_id: Vec<u8>,
    /// Channel whose CMK is wrapped by this grant.
    pub channel_id: ChannelId,
    /// CMK generation carried by this grant.
    pub key_epoch: KeyEpoch,
    /// One-time X25519 public key created by the originator.
    pub ephemeral_public_key: [u8; 32],
    /// Random XChaCha20-Poly1305 nonce used to wrap the CMK.
    pub wrapping_nonce: [u8; 24],
    /// Thirty-two encrypted CMK bytes followed by the 16-byte AEAD tag.
    pub wrapped_cmk: [u8; 48],
    /// Originator signature over every preceding logical field.
    pub originator_signature: [u8; 64],
}

/// Encrypt and sign one epoch CMK for an authorized receiver.
pub fn seal_channel_key(
    originator_id: &[u8],
    receiver_id: &[u8],
    channel_id: ChannelId,
    key_epoch: KeyEpoch,
    cmk: &ChannelMasterKey,
    receiver_public_key: &[u8; 32],
    signing_key: &OriginatorSigningKey,
) -> Result<SealedChannelKeyGrant, ChannelCryptoError> {
    let ephemeral_private =
        Zeroizing::new(random_array().map_err(|_| ChannelCryptoError::RandomnessUnavailable)?);
    let ephemeral_public_key = x25519_public_key(&ephemeral_private)
        .map_err(|_| ChannelCryptoError::InvalidKeyAgreement)?;
    let shared_secret = Zeroizing::new(
        x25519(&ephemeral_private, receiver_public_key)
            .map_err(|_| ChannelCryptoError::InvalidKeyAgreement)?,
    );
    let wrapping_key = derive_wrapping_key(&shared_secret, channel_id, key_epoch, receiver_id)?;
    let wrapping_nonce = random_array().map_err(|_| ChannelCryptoError::RandomnessUnavailable)?;
    let aad = grant_aad(
        originator_id,
        receiver_id,
        channel_id,
        key_epoch,
        &ephemeral_public_key,
    );
    let (ciphertext, tag) =
        xchacha20_poly1305_aead_encrypt(cmk.as_bytes(), &wrapping_key, &wrapping_nonce, &aad);
    let mut wrapped_cmk = [0u8; 48];
    wrapped_cmk[..32].copy_from_slice(&ciphertext);
    wrapped_cmk[32..].copy_from_slice(&tag);
    let signature_input = grant_signature_input(
        originator_id,
        receiver_id,
        channel_id,
        key_epoch,
        &ephemeral_public_key,
        &wrapping_nonce,
        &wrapped_cmk,
    );
    let originator_signature = sign(&signature_input, &signing_key.secret_key);

    Ok(SealedChannelKeyGrant {
        originator_id: originator_id.to_vec(),
        receiver_id: receiver_id.to_vec(),
        channel_id,
        key_epoch,
        ephemeral_public_key,
        wrapping_nonce,
        wrapped_cmk,
        originator_signature,
    })
}

/// Verify and unwrap a receiver-bound channel-key grant.
pub fn open_channel_key_grant(
    grant: &SealedChannelKeyGrant,
    expected_originator_id: &[u8],
    expected_receiver_id: &[u8],
    expected_channel_id: ChannelId,
    receiver_key_pair: &ReceiverKeyPair,
    originator_public_key: &[u8; 32],
) -> Result<ChannelMasterKey, ChannelCryptoError> {
    if grant.originator_id != expected_originator_id {
        return Err(ChannelCryptoError::UnexpectedOriginator);
    }
    if grant.receiver_id != expected_receiver_id {
        return Err(ChannelCryptoError::UnexpectedReceiver);
    }
    if grant.channel_id != expected_channel_id {
        return Err(ChannelCryptoError::UnexpectedChannel);
    }
    let signature_input = grant_signature_input(
        &grant.originator_id,
        &grant.receiver_id,
        grant.channel_id,
        grant.key_epoch,
        &grant.ephemeral_public_key,
        &grant.wrapping_nonce,
        &grant.wrapped_cmk,
    );
    if !verify(
        &signature_input,
        &grant.originator_signature,
        originator_public_key,
    ) {
        return Err(ChannelCryptoError::InvalidGrantSignature);
    }
    let shared_secret = Zeroizing::new(
        x25519(&receiver_key_pair.private_key, &grant.ephemeral_public_key)
            .map_err(|_| ChannelCryptoError::InvalidKeyAgreement)?,
    );
    let wrapping_key = derive_wrapping_key(
        &shared_secret,
        grant.channel_id,
        grant.key_epoch,
        &grant.receiver_id,
    )?;
    let aad = grant_aad(
        &grant.originator_id,
        &grant.receiver_id,
        grant.channel_id,
        grant.key_epoch,
        &grant.ephemeral_public_key,
    );
    let ciphertext: &[u8] = &grant.wrapped_cmk[..32];
    let tag: &[u8; 16] = grant.wrapped_cmk[32..]
        .try_into()
        .map_err(|_| ChannelCryptoError::InvalidWrappedKey)?;
    let plaintext = Zeroizing::new(
        xchacha20_poly1305_aead_decrypt(
            ciphertext,
            &wrapping_key,
            &grant.wrapping_nonce,
            &aad,
            tag,
        )
        .ok_or(ChannelCryptoError::AuthenticationFailed)?,
    );
    let cmk_bytes: [u8; 32] = plaintext
        .as_slice()
        .try_into()
        .map_err(|_| ChannelCryptoError::InvalidWrappedKey)?;
    Ok(ChannelMasterKey::from_bytes(cmk_bytes))
}

/// Result of installing a sealed key grant into receiver epoch state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrantInstallOutcome {
    /// A new epoch CMK was verified, unwrapped, and retained.
    Installed,
    /// The current grant was retransmitted byte-for-byte.
    Idempotent,
}

/// Receiver-owned CMKs and grant-ordering rules for one channel.
pub struct ReceiverEpochKeys {
    originator_id: Vec<u8>,
    receiver_id: Vec<u8>,
    channel_id: ChannelId,
    receiver_key_pair: ReceiverKeyPair,
    originator_public_key: [u8; 32],
    epoch_keys: BTreeMap<KeyEpoch, ChannelMasterKey>,
    latest_grant: Option<SealedChannelKeyGrant>,
}

impl ReceiverEpochKeys {
    /// Create empty receiver state for one originator/channel pair.
    pub fn new(
        originator_id: Vec<u8>,
        receiver_id: Vec<u8>,
        channel_id: ChannelId,
        receiver_key_pair: ReceiverKeyPair,
        originator_public_key: [u8; 32],
    ) -> Self {
        Self {
            originator_id,
            receiver_id,
            channel_id,
            receiver_key_pair,
            originator_public_key,
            epoch_keys: BTreeMap::new(),
            latest_grant: None,
        }
    }

    /// Return the receiver's public key for originator grant creation.
    pub fn receiver_public_key(&self) -> [u8; 32] {
        self.receiver_key_pair.public_key()
    }

    /// Verify and install a grant, enforcing monotonic epoch transitions.
    pub fn install_grant(
        &mut self,
        grant: SealedChannelKeyGrant,
    ) -> Result<GrantInstallOutcome, ChannelCryptoError> {
        if let Some(latest) = &self.latest_grant {
            if grant.key_epoch < latest.key_epoch {
                return Err(ChannelCryptoError::DecreasingEpoch);
            }
            if grant.key_epoch == latest.key_epoch {
                return if grant == *latest {
                    Ok(GrantInstallOutcome::Idempotent)
                } else {
                    Err(ChannelCryptoError::ConflictingGrant)
                };
            }
        }

        let key = open_channel_key_grant(
            &grant,
            &self.originator_id,
            &self.receiver_id,
            self.channel_id,
            &self.receiver_key_pair,
            &self.originator_public_key,
        )?;
        self.epoch_keys.insert(grant.key_epoch, key);
        self.latest_grant = Some(grant);
        Ok(GrantInstallOutcome::Installed)
    }

    /// Borrow the retained CMK for `epoch`, if this receiver was authorized.
    pub fn epoch_key(&self, epoch: KeyEpoch) -> Option<&ChannelMasterKey> {
        self.epoch_keys.get(&epoch)
    }

    /// Return the newest installed key epoch.
    pub fn latest_epoch(&self) -> Option<KeyEpoch> {
        self.latest_grant.as_ref().map(|grant| grant.key_epoch)
    }
}

/// Immutable message fields supplied before hashing and encryption.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MessageFields {
    /// Canonical 16-byte UUID v7 representation.
    pub message_id: [u8; 16],
    /// Monotonic nanosecond timestamp, encoded big-endian for authentication.
    pub timestamp_ns: u64,
    /// Entity that authored and signed this message.
    pub originator_id: Vec<u8>,
    /// Channel whose CMK protects this message.
    pub channel_id: ChannelId,
    /// Globally monotonic channel sequence.
    pub sequence: Sequence,
    /// CMK epoch used for this payload.
    pub key_epoch: KeyEpoch,
    /// MIME content type authenticated with the message.
    pub content_type: String,
}

/// Authenticated outer header stored with a channel-log payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MessageHeader {
    /// Immutable fields supplied by the originator.
    pub fields: MessageFields,
    /// SHA-256 digest of plaintext bytes computed before encryption.
    pub plaintext_hash: [u8; 32],
}

/// Ciphertext and authenticity data for one channel-log message.
#[derive(Clone, PartialEq, Eq)]
pub struct EncryptedMessage {
    /// Outer metadata used verbatim as AEAD additional data.
    pub header: MessageHeader,
    /// XChaCha20-encrypted payload bytes.
    pub ciphertext: Vec<u8>,
    /// Poly1305 authentication tag.
    pub authentication_tag: [u8; 16],
    /// Ed25519 signature over the canonical outer header.
    pub originator_signature: [u8; 64],
}

/// Build the deterministic 24-byte message nonce.
///
/// The key epoch is intentionally absent. Callers must never reset `sequence`
/// when rotating the CMK.
pub fn message_nonce(channel_id: ChannelId, sequence: Sequence) -> [u8; 24] {
    let mut nonce = [0u8; 24];
    nonce[..16].copy_from_slice(&channel_id.0);
    nonce[16..].copy_from_slice(&sequence.0.to_be_bytes());
    nonce
}

/// Hash, sign, and encrypt one message for an append-only channel log.
pub fn encrypt_message(
    fields: MessageFields,
    plaintext: &[u8],
    cmk: &ChannelMasterKey,
    signing_key: &OriginatorSigningKey,
) -> EncryptedMessage {
    let header = MessageHeader {
        fields,
        plaintext_hash: sha256(plaintext),
    };
    let authenticated_header = canonical_message_header(&header);
    let nonce = message_nonce(header.fields.channel_id, header.fields.sequence);
    let (ciphertext, authentication_tag) =
        xchacha20_poly1305_aead_encrypt(plaintext, cmk.as_bytes(), &nonce, &authenticated_header);
    let originator_signature = sign(&authenticated_header, &signing_key.secret_key);
    EncryptedMessage {
        header,
        ciphertext,
        authentication_tag,
        originator_signature,
    }
}

/// Verify and decrypt one channel-log message.
///
/// Any mutation to the outer header invalidates both the signature and the
/// AEAD additional data. The caller must select `cmk` using the header epoch.
pub fn decrypt_message(
    message: &EncryptedMessage,
    cmk: &ChannelMasterKey,
    originator_public_key: &[u8; 32],
) -> Result<Vec<u8>, ChannelCryptoError> {
    let authenticated_header = canonical_message_header(&message.header);
    if !verify(
        &authenticated_header,
        &message.originator_signature,
        originator_public_key,
    ) {
        return Err(ChannelCryptoError::InvalidMessageSignature);
    }
    let nonce = message_nonce(
        message.header.fields.channel_id,
        message.header.fields.sequence,
    );
    let plaintext = xchacha20_poly1305_aead_decrypt(
        &message.ciphertext,
        cmk.as_bytes(),
        &nonce,
        &authenticated_header,
        &message.authentication_tag,
    )
    .ok_or(ChannelCryptoError::AuthenticationFailed)?;
    if sha256(&plaintext) != message.header.plaintext_hash {
        return Err(ChannelCryptoError::PlaintextHashMismatch);
    }
    Ok(plaintext)
}

/// Two-phase cursor for durable, never-resetting channel sequences.
///
/// On restart, call [`SequenceCursor::recover`] with the durable next value.
/// Before encrypting, persist `next + 1`, then pass that persisted value to
/// [`SequenceCursor::reserve_after_persist`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SequenceCursor {
    next: Sequence,
}

impl SequenceCursor {
    /// Start a newly created channel at sequence zero.
    pub fn for_new_channel() -> Self {
        Self { next: Sequence(0) }
    }

    /// Recover a cursor from durable state, rejecting missing or stale state.
    pub fn recover(
        durable_next: Option<Sequence>,
        minimum_safe_next: Sequence,
    ) -> Result<Self, ChannelCryptoError> {
        let next = durable_next.ok_or(ChannelCryptoError::MissingDurableSequence)?;
        if next < minimum_safe_next {
            return Err(ChannelCryptoError::DecreasingSequence);
        }
        Ok(Self { next })
    }

    /// Return the next sequence that must be reserved durably.
    pub fn next(&self) -> Sequence {
        self.next
    }

    /// Complete a reservation only after the caller persisted `next + 1`.
    pub fn reserve_after_persist(
        &mut self,
        persisted_next: Sequence,
    ) -> Result<Sequence, ChannelCryptoError> {
        let required_next = self
            .next
            .0
            .checked_add(1)
            .map(Sequence)
            .ok_or(ChannelCryptoError::SequenceExhausted)?;
        if persisted_next != required_next {
            return Err(ChannelCryptoError::DurableSequenceMismatch);
        }
        let reserved = self.next;
        self.next = required_next;
        Ok(reserved)
    }
}

fn derive_wrapping_key(
    shared_secret: &[u8; 32],
    channel_id: ChannelId,
    key_epoch: KeyEpoch,
    receiver_id: &[u8],
) -> Result<Zeroizing<[u8; 32]>, ChannelCryptoError> {
    let epoch = key_epoch.0.to_be_bytes();
    let salt = frame(&[&channel_id.0, &epoch]);
    let info = frame(&[KEY_WRAP_CONTEXT, receiver_id]);
    let bytes = Zeroizing::new(
        hkdf(&salt, shared_secret, &info, 32, HashAlgorithm::Sha256)
            .map_err(|_| ChannelCryptoError::KeyDerivationFailed)?,
    );
    let mut key = Zeroizing::new([0u8; 32]);
    key.copy_from_slice(&bytes);
    Ok(key)
}

fn grant_aad(
    originator_id: &[u8],
    receiver_id: &[u8],
    channel_id: ChannelId,
    key_epoch: KeyEpoch,
    ephemeral_public_key: &[u8; 32],
) -> Vec<u8> {
    let epoch = key_epoch.0.to_be_bytes();
    frame(&[
        KEY_GRANT_CONTEXT,
        originator_id,
        &channel_id.0,
        &epoch,
        receiver_id,
        ephemeral_public_key,
    ])
}

#[allow(clippy::too_many_arguments)]
fn grant_signature_input(
    originator_id: &[u8],
    receiver_id: &[u8],
    channel_id: ChannelId,
    key_epoch: KeyEpoch,
    ephemeral_public_key: &[u8; 32],
    wrapping_nonce: &[u8; 24],
    wrapped_cmk: &[u8; 48],
) -> Vec<u8> {
    let epoch = key_epoch.0.to_be_bytes();
    frame(&[
        KEY_GRANT_CONTEXT,
        originator_id,
        &channel_id.0,
        &epoch,
        receiver_id,
        ephemeral_public_key,
        wrapping_nonce,
        wrapped_cmk,
    ])
}

fn canonical_message_header(header: &MessageHeader) -> Vec<u8> {
    let timestamp = header.fields.timestamp_ns.to_be_bytes();
    let sequence = header.fields.sequence.0.to_be_bytes();
    let epoch = header.fields.key_epoch.0.to_be_bytes();
    frame(&[
        MESSAGE_CONTEXT,
        &header.fields.message_id,
        &timestamp,
        &header.fields.originator_id,
        &header.fields.channel_id.0,
        &sequence,
        &epoch,
        header.fields.content_type.as_bytes(),
        &header.plaintext_hash,
    ])
}

fn frame(fields: &[&[u8]]) -> Vec<u8> {
    let capacity = fields
        .iter()
        .fold(0usize, |total, field| total.saturating_add(8 + field.len()));
    let mut encoded = Vec::with_capacity(capacity);
    for field in fields {
        let length = u64::try_from(field.len()).expect("usize always fits in u64");
        encoded.extend_from_slice(&length.to_be_bytes());
        encoded.extend_from_slice(field);
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    fn channel_id() -> ChannelId {
        ChannelId([0x44; 16])
    }

    fn signing_key(byte: u8) -> OriginatorSigningKey {
        OriginatorSigningKey::from_seed([byte; 32])
    }

    fn receiver_key(byte: u8) -> ReceiverKeyPair {
        ReceiverKeyPair::from_private_key([byte; 32]).unwrap()
    }

    fn message_fields(sequence: u64, epoch: u64) -> MessageFields {
        MessageFields {
            message_id: [sequence as u8; 16],
            timestamp_ns: 1_725_000_000_000_000_000 + sequence,
            originator_id: b"originator".to_vec(),
            channel_id: channel_id(),
            sequence: Sequence(sequence),
            key_epoch: KeyEpoch(epoch),
            content_type: "text/plain".to_owned(),
        }
    }

    #[test]
    fn two_receivers_unwrap_the_same_epoch_key() {
        let originator = signing_key(7);
        let receiver_a = receiver_key(11);
        let receiver_b = receiver_key(12);
        let cmk = ChannelMasterKey::from_bytes([0xa5; 32]);
        let grant_a = seal_channel_key(
            b"originator",
            b"receiver-a",
            channel_id(),
            KeyEpoch(0),
            &cmk,
            &receiver_a.public_key(),
            &originator,
        )
        .unwrap();
        let grant_b = seal_channel_key(
            b"originator",
            b"receiver-b",
            channel_id(),
            KeyEpoch(0),
            &cmk,
            &receiver_b.public_key(),
            &originator,
        )
        .unwrap();

        let opened_a = open_channel_key_grant(
            &grant_a,
            b"originator",
            b"receiver-a",
            channel_id(),
            &receiver_a,
            &originator.public_key(),
        )
        .unwrap();
        let opened_b = open_channel_key_grant(
            &grant_b,
            b"originator",
            b"receiver-b",
            channel_id(),
            &receiver_b,
            &originator.public_key(),
        )
        .unwrap();
        assert_eq!(opened_a.as_bytes(), cmk.as_bytes());
        assert_eq!(opened_b.as_bytes(), cmk.as_bytes());
        assert_ne!(grant_a.wrapped_cmk, grant_b.wrapped_cmk);
    }

    #[test]
    fn grant_is_bound_to_receiver_key_metadata_and_signature() {
        let originator = signing_key(7);
        let receiver = receiver_key(11);
        let outsider = receiver_key(99);
        let cmk = ChannelMasterKey::from_bytes([0xa5; 32]);
        let grant = seal_channel_key(
            b"originator",
            b"receiver",
            channel_id(),
            KeyEpoch(3),
            &cmk,
            &receiver.public_key(),
            &originator,
        )
        .unwrap();

        assert_eq!(
            open_channel_key_grant(
                &grant,
                b"originator",
                b"receiver",
                channel_id(),
                &outsider,
                &originator.public_key(),
            )
            .err()
            .unwrap(),
            ChannelCryptoError::AuthenticationFailed
        );
        assert_eq!(
            open_channel_key_grant(
                &grant,
                b"originator",
                b"someone-else",
                channel_id(),
                &receiver,
                &originator.public_key(),
            )
            .err()
            .unwrap(),
            ChannelCryptoError::UnexpectedReceiver
        );

        let mut tampered = grant.clone();
        tampered.key_epoch = KeyEpoch(4);
        assert_eq!(
            open_channel_key_grant(
                &tampered,
                b"originator",
                b"receiver",
                channel_id(),
                &receiver,
                &originator.public_key(),
            )
            .err()
            .unwrap(),
            ChannelCryptoError::InvalidGrantSignature
        );
    }

    #[test]
    fn low_order_x25519_public_keys_are_rejected_before_hkdf() {
        let result = seal_channel_key(
            b"originator",
            b"receiver",
            channel_id(),
            KeyEpoch(0),
            &ChannelMasterKey::from_bytes([0xa5; 32]),
            &[0u8; 32],
            &signing_key(7),
        );
        assert_eq!(
            result.err().unwrap(),
            ChannelCryptoError::InvalidKeyAgreement
        );
    }

    #[test]
    fn receiver_state_enforces_idempotency_conflicts_and_monotonic_epochs() {
        let originator = signing_key(7);
        let receiver = receiver_key(11);
        let receiver_public = receiver.public_key();
        let mut state = ReceiverEpochKeys::new(
            b"originator".to_vec(),
            b"receiver".to_vec(),
            channel_id(),
            receiver,
            originator.public_key(),
        );
        let cmk0 = ChannelMasterKey::from_bytes([0x10; 32]);
        let grant0 = seal_channel_key(
            b"originator",
            b"receiver",
            channel_id(),
            KeyEpoch(0),
            &cmk0,
            &receiver_public,
            &originator,
        )
        .unwrap();
        assert_eq!(
            state.install_grant(grant0.clone()).unwrap(),
            GrantInstallOutcome::Installed
        );
        assert_eq!(
            state.install_grant(grant0.clone()).unwrap(),
            GrantInstallOutcome::Idempotent
        );

        let conflicting = seal_channel_key(
            b"originator",
            b"receiver",
            channel_id(),
            KeyEpoch(0),
            &ChannelMasterKey::from_bytes([0x20; 32]),
            &receiver_public,
            &originator,
        )
        .unwrap();
        assert_eq!(
            state.install_grant(conflicting).unwrap_err(),
            ChannelCryptoError::ConflictingGrant
        );

        let grant1 = seal_channel_key(
            b"originator",
            b"receiver",
            channel_id(),
            KeyEpoch(1),
            &ChannelMasterKey::from_bytes([0x30; 32]),
            &receiver_public,
            &originator,
        )
        .unwrap();
        state.install_grant(grant1).unwrap();
        assert_eq!(state.latest_epoch(), Some(KeyEpoch(1)));
        assert!(state.epoch_key(KeyEpoch(0)).is_some());
        assert!(state.epoch_key(KeyEpoch(1)).is_some());
        assert_eq!(
            state.install_grant(grant0).unwrap_err(),
            ChannelCryptoError::DecreasingEpoch
        );
    }

    #[test]
    fn message_round_trip_binds_every_header_field() {
        let originator = signing_key(7);
        let cmk = ChannelMasterKey::from_bytes([0xa5; 32]);
        let message = encrypt_message(
            message_fields(42, 3),
            b"confidential payload",
            &cmk,
            &originator,
        );
        assert_ne!(message.ciphertext, b"confidential payload");
        assert_eq!(
            decrypt_message(&message, &cmk, &originator.public_key()).unwrap(),
            b"confidential payload"
        );

        let mut tampered = message.clone();
        tampered.header.fields.content_type = "application/json".to_owned();
        assert_eq!(
            decrypt_message(&tampered, &cmk, &originator.public_key()).unwrap_err(),
            ChannelCryptoError::InvalidMessageSignature
        );
        assert_eq!(
            decrypt_message(
                &message,
                &ChannelMasterKey::from_bytes([0x55; 32]),
                &originator.public_key(),
            )
            .unwrap_err(),
            ChannelCryptoError::AuthenticationFailed
        );
        assert_eq!(
            decrypt_message(&message, &cmk, &signing_key(8).public_key()).unwrap_err(),
            ChannelCryptoError::InvalidMessageSignature
        );
    }

    #[test]
    fn message_plaintext_hash_is_checked_after_valid_decryption() {
        let originator = signing_key(7);
        let cmk = ChannelMasterKey::from_bytes([0xa5; 32]);
        let mut message = encrypt_message(message_fields(2, 0), b"payload", &cmk, &originator);
        message.header.plaintext_hash = [0x99; 32];
        let authenticated_header = canonical_message_header(&message.header);
        let nonce = message_nonce(
            message.header.fields.channel_id,
            message.header.fields.sequence,
        );
        let (ciphertext, tag) = xchacha20_poly1305_aead_encrypt(
            b"payload",
            cmk.as_bytes(),
            &nonce,
            &authenticated_header,
        );
        message.ciphertext = ciphertext;
        message.authentication_tag = tag;
        message.originator_signature = sign(&authenticated_header, &originator.secret_key);

        assert_eq!(
            decrypt_message(&message, &cmk, &originator.public_key()).unwrap_err(),
            ChannelCryptoError::PlaintextHashMismatch
        );
    }

    #[test]
    fn nonce_is_unique_only_when_sequence_never_resets() {
        let first = message_nonce(channel_id(), Sequence(0));
        let second = message_nonce(channel_id(), Sequence(1));
        assert_ne!(first, second);
        assert_eq!(&first[..16], &channel_id().0);
        assert_eq!(&first[16..], &0u64.to_be_bytes());
        assert_eq!(
            message_nonce(channel_id(), Sequence(7)),
            message_nonce(channel_id(), Sequence(7)),
            "rotating an epoch does not make sequence reuse safe"
        );
    }

    #[test]
    fn canonical_framing_has_unambiguous_boundaries() {
        assert_ne!(frame(&[b"ab", b"c"]), frame(&[b"a", b"bc"]));
    }

    #[test]
    fn durable_sequence_cursor_fails_closed() {
        assert_eq!(
            SequenceCursor::recover(None, Sequence(0)).unwrap_err(),
            ChannelCryptoError::MissingDurableSequence
        );
        assert_eq!(
            SequenceCursor::recover(Some(Sequence(4)), Sequence(5)).unwrap_err(),
            ChannelCryptoError::DecreasingSequence
        );

        let mut cursor = SequenceCursor::recover(Some(Sequence(5)), Sequence(5)).unwrap();
        assert_eq!(
            cursor.reserve_after_persist(Sequence(5)).unwrap_err(),
            ChannelCryptoError::DurableSequenceMismatch
        );
        assert_eq!(
            cursor.reserve_after_persist(Sequence(6)).unwrap(),
            Sequence(5)
        );
        assert_eq!(cursor.next(), Sequence(6));

        let mut exhausted = SequenceCursor {
            next: Sequence(u64::MAX),
        };
        assert_eq!(
            exhausted.reserve_after_persist(Sequence(0)).unwrap_err(),
            ChannelCryptoError::SequenceExhausted
        );
    }
}
