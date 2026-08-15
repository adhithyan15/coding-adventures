//! Portable D18Q channel-key grant compatibility profile.
//!
//! This module validates the high-level D18Q contract while delegating the
//! shipped D18G version 1 cryptographic bytes to the crate's production
//! primitives. Structural decoding remains deliberately separate from trust.

use std::collections::BTreeSet;

use coding_adventures_csprng::random_array;
use coding_adventures_zeroize::Zeroizing;

use crate::wire::{self, MAX_IDENTITY_BYTES};
use crate::{
    open_channel_key_grant as open_raw_grant, seal_channel_key as seal_raw_grant,
    seal_channel_key_with_material_raw,
    verify_channel_key_grant_signature as verify_raw_grant_signature, ChannelCryptoError,
    ChannelId, ChannelMasterKey, GrantInstallOutcome, KeyEpoch, OriginatorSigningKey,
    ReceiverEpochKeys as RawReceiverEpochKeys, ReceiverKeyPair, SealedChannelKeyGrant,
};

/// Stable machine-readable failures defined by D18Q.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyGrantProfileError {
    /// Record magic is not `D18G`.
    InvalidMagic,
    /// Record version is not supported.
    UnsupportedVersion,
    /// The record ended before a complete field could be read.
    TruncatedRecord,
    /// Bytes remained after one complete record.
    TrailingBytes,
    /// An identity exceeded the D18Q bound.
    LengthLimitExceeded,
    /// A high-level field was empty or malformed.
    InvalidField,
    /// The operating-system CSPRNG was unavailable.
    RandomnessUnavailable,
    /// X25519 rejected the supplied key material.
    InvalidKeyAgreement,
    /// HKDF-SHA256 could not derive the wrapping key.
    KeyDerivationFailed,
    /// The canonical Ed25519 grant signature was invalid.
    InvalidSignature,
    /// The grant named another originator.
    UnexpectedOriginator,
    /// The grant named another receiver.
    UnexpectedReceiver,
    /// The grant named another channel.
    UnexpectedChannel,
    /// XChaCha20-Poly1305 authentication failed.
    AuthenticationFailed,
    /// The opened key was not exactly 32 bytes.
    InvalidWrappedKey,
    /// Another grant already occupies the newest epoch.
    ConflictingGrant,
    /// A receiver attempted to install an older epoch.
    DecreasingEpoch,
    /// Rotation attempted to advance `u64::MAX`.
    EpochExhausted,
    /// The receiver has no retained key for the requested epoch.
    MissingEpochKey,
}

impl KeyGrantProfileError {
    /// Return the stable D18Q machine-readable code.
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidMagic => "invalid_magic",
            Self::UnsupportedVersion => "unsupported_version",
            Self::TruncatedRecord => "truncated_record",
            Self::TrailingBytes => "trailing_bytes",
            Self::LengthLimitExceeded => "length_limit_exceeded",
            Self::InvalidField => "invalid_field",
            Self::RandomnessUnavailable => "randomness_unavailable",
            Self::InvalidKeyAgreement => "invalid_key_agreement",
            Self::KeyDerivationFailed => "key_derivation_failed",
            Self::InvalidSignature => "invalid_signature",
            Self::UnexpectedOriginator => "unexpected_originator",
            Self::UnexpectedReceiver => "unexpected_receiver",
            Self::UnexpectedChannel => "unexpected_channel",
            Self::AuthenticationFailed => "authentication_failed",
            Self::InvalidWrappedKey => "invalid_wrapped_key",
            Self::ConflictingGrant => "conflicting_grant",
            Self::DecreasingEpoch => "decreasing_epoch",
            Self::EpochExhausted => "epoch_exhausted",
            Self::MissingEpochKey => "missing_epoch_key",
        }
    }
}

impl core::fmt::Display for KeyGrantProfileError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for KeyGrantProfileError {}

impl From<wire::ChannelWireError> for KeyGrantProfileError {
    fn from(error: wire::ChannelWireError) -> Self {
        match error {
            wire::ChannelWireError::InvalidMagic => Self::InvalidMagic,
            wire::ChannelWireError::UnsupportedVersion(_) => Self::UnsupportedVersion,
            wire::ChannelWireError::Truncated => Self::TruncatedRecord,
            wire::ChannelWireError::TrailingBytes => Self::TrailingBytes,
            wire::ChannelWireError::LengthLimitExceeded { .. } => Self::LengthLimitExceeded,
            wire::ChannelWireError::InvalidUtf8(_) => Self::InvalidField,
        }
    }
}

impl From<ChannelCryptoError> for KeyGrantProfileError {
    fn from(error: ChannelCryptoError) -> Self {
        match error {
            ChannelCryptoError::RandomnessUnavailable => Self::RandomnessUnavailable,
            ChannelCryptoError::InvalidKeyAgreement => Self::InvalidKeyAgreement,
            ChannelCryptoError::KeyDerivationFailed => Self::KeyDerivationFailed,
            ChannelCryptoError::InvalidGrantSignature => Self::InvalidSignature,
            ChannelCryptoError::UnexpectedOriginator => Self::UnexpectedOriginator,
            ChannelCryptoError::UnexpectedReceiver => Self::UnexpectedReceiver,
            ChannelCryptoError::UnexpectedChannel => Self::UnexpectedChannel,
            ChannelCryptoError::AuthenticationFailed => Self::AuthenticationFailed,
            ChannelCryptoError::InvalidWrappedKey => Self::InvalidWrappedKey,
            ChannelCryptoError::ConflictingGrant => Self::ConflictingGrant,
            ChannelCryptoError::DecreasingEpoch => Self::DecreasingEpoch,
            ChannelCryptoError::InvalidMessageSignature
            | ChannelCryptoError::PlaintextHashMismatch
            | ChannelCryptoError::MissingDurableSequence
            | ChannelCryptoError::DecreasingSequence
            | ChannelCryptoError::DurableSequenceMismatch
            | ChannelCryptoError::SequenceExhausted => Self::InvalidField,
        }
    }
}

/// Immutable high-level fields for one receiver-bound grant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyGrantFields {
    originator_id: Vec<u8>,
    receiver_id: Vec<u8>,
    channel_id: ChannelId,
    key_epoch: KeyEpoch,
}

impl KeyGrantFields {
    /// Validate and construct high-level D18Q grant fields.
    pub fn new(
        originator_id: Vec<u8>,
        receiver_id: Vec<u8>,
        channel_id: ChannelId,
        key_epoch: KeyEpoch,
    ) -> Result<Self, KeyGrantProfileError> {
        validate_identity(&originator_id)?;
        validate_identity(&receiver_id)?;
        validate_channel_id(channel_id)?;
        Ok(Self {
            originator_id,
            receiver_id,
            channel_id,
            key_epoch,
        })
    }

    /// Borrow the originator identity.
    pub fn originator_id(&self) -> &[u8] {
        &self.originator_id
    }

    /// Borrow the receiver identity.
    pub fn receiver_id(&self) -> &[u8] {
        &self.receiver_id
    }

    /// Return the channel UUID-v7 bytes.
    pub fn channel_id(&self) -> ChannelId {
        self.channel_id
    }

    /// Return the channel-key epoch.
    pub fn key_epoch(&self) -> KeyEpoch {
        self.key_epoch
    }
}

/// Immutable profile wrapper around one shipped D18G grant.
#[derive(Clone, PartialEq, Eq)]
pub struct PortableKeyGrant {
    grant: SealedChannelKeyGrant,
}

impl core::fmt::Debug for PortableKeyGrant {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("PortableKeyGrant")
            .field("originator_id", &self.grant.originator_id)
            .field("receiver_id", &self.grant.receiver_id)
            .field("channel_id", &self.grant.channel_id)
            .field("key_epoch", &self.grant.key_epoch)
            .field("ephemeral_public_key", &self.grant.ephemeral_public_key)
            .field("wrapping_nonce", &self.grant.wrapping_nonce)
            .field("wrapped_cmk", &self.grant.wrapped_cmk)
            .field("originator_signature", &self.grant.originator_signature)
            .finish()
    }
}

impl PortableKeyGrant {
    /// Borrow the originator identity.
    pub fn originator_id(&self) -> &[u8] {
        &self.grant.originator_id
    }

    /// Borrow the receiver identity.
    pub fn receiver_id(&self) -> &[u8] {
        &self.grant.receiver_id
    }

    /// Return the channel UUID bytes.
    pub fn channel_id(&self) -> ChannelId {
        self.grant.channel_id
    }

    /// Return the key epoch.
    pub fn key_epoch(&self) -> KeyEpoch {
        self.grant.key_epoch
    }

    /// Return the one-time X25519 public key.
    pub fn ephemeral_public_key(&self) -> [u8; 32] {
        self.grant.ephemeral_public_key
    }

    /// Return the wrapping nonce.
    pub fn wrapping_nonce(&self) -> [u8; 24] {
        self.grant.wrapping_nonce
    }

    /// Return the encrypted CMK and tag.
    pub fn wrapped_cmk(&self) -> [u8; 48] {
        self.grant.wrapped_cmk
    }

    /// Return the originator signature.
    pub fn originator_signature(&self) -> [u8; 64] {
        self.grant.originator_signature
    }
}

/// Structurally decode one D18G record without claiming cryptographic trust.
pub fn grant_deserialize(bytes: &[u8]) -> Result<PortableKeyGrant, KeyGrantProfileError> {
    Ok(PortableKeyGrant {
        grant: wire::decode_key_grant(bytes)?,
    })
}

/// Validate and serialize one immutable grant as shipped D18G version 1 bytes.
pub fn grant_serialize(grant: &PortableKeyGrant) -> Result<Vec<u8>, KeyGrantProfileError> {
    validate_grant(&grant.grant)?;
    Ok(wire::encode_key_grant(&grant.grant)?)
}

/// Seal one grant using production CSPRNG material.
pub fn seal_channel_key(
    fields: &KeyGrantFields,
    cmk: &ChannelMasterKey,
    receiver_public_key: &[u8; 32],
    signing_key: &OriginatorSigningKey,
) -> Result<PortableKeyGrant, KeyGrantProfileError> {
    Ok(PortableKeyGrant {
        grant: seal_raw_grant(
            fields.originator_id(),
            fields.receiver_id(),
            fields.channel_id(),
            fields.key_epoch(),
            cmk,
            receiver_public_key,
            signing_key,
        )?,
    })
}

/// Seal one grant with explicit deterministic material.
#[allow(clippy::too_many_arguments)]
pub fn seal_channel_key_with_material(
    fields: &KeyGrantFields,
    cmk: &ChannelMasterKey,
    receiver_public_key: &[u8; 32],
    signing_key: &OriginatorSigningKey,
    ephemeral_private_key: [u8; 32],
    wrapping_nonce: [u8; 24],
) -> Result<PortableKeyGrant, KeyGrantProfileError> {
    Ok(PortableKeyGrant {
        grant: seal_channel_key_with_material_raw(
            fields.originator_id(),
            fields.receiver_id(),
            fields.channel_id(),
            fields.key_epoch(),
            cmk,
            receiver_public_key,
            signing_key,
            Zeroizing::new(ephemeral_private_key),
            wrapping_nonce,
        )?,
    })
}

/// Verify and open one immutable receiver-bound grant.
pub fn open_channel_key_grant(
    grant: &PortableKeyGrant,
    expected_originator_id: &[u8],
    expected_receiver_id: &[u8],
    expected_channel_id: ChannelId,
    receiver_key_pair: &ReceiverKeyPair,
    originator_public_key: &[u8; 32],
) -> Result<ChannelMasterKey, KeyGrantProfileError> {
    validate_grant(&grant.grant)?;
    Ok(open_raw_grant(
        &grant.grant,
        expected_originator_id,
        expected_receiver_id,
        expected_channel_id,
        receiver_key_pair,
        originator_public_key,
    )?)
}

/// Verify a D18G signature and its expected public identities without opening
/// the receiver-bound CMK.
pub fn verify_grant_signature(
    grant: &PortableKeyGrant,
    expected_originator_id: &[u8],
    expected_receiver_id: &[u8],
    expected_channel_id: ChannelId,
    originator_public_key: &[u8; 32],
) -> Result<(), KeyGrantProfileError> {
    validate_grant(&grant.grant)?;
    verify_raw_grant_signature(
        &grant.grant,
        expected_originator_id,
        expected_receiver_id,
        expected_channel_id,
        originator_public_key,
    )?;
    Ok(())
}

/// Receiver-local D18Q epoch state with immutable grant inputs.
pub struct ReceiverEpochKeys {
    state: RawReceiverEpochKeys,
    latest_grant: Option<PortableKeyGrant>,
}

impl ReceiverEpochKeys {
    /// Validate and create empty state for one receiver/channel tuple.
    pub fn new(
        originator_id: Vec<u8>,
        receiver_id: Vec<u8>,
        channel_id: ChannelId,
        receiver_key_pair: ReceiverKeyPair,
        originator_public_key: [u8; 32],
    ) -> Result<Self, KeyGrantProfileError> {
        validate_identity(&originator_id)?;
        validate_identity(&receiver_id)?;
        validate_channel_id(channel_id)?;
        Ok(Self {
            state: RawReceiverEpochKeys::new(
                originator_id,
                receiver_id,
                channel_id,
                receiver_key_pair,
                originator_public_key,
            ),
            latest_grant: None,
        })
    }

    /// Return the public key used to prepare grants for this receiver.
    pub fn receiver_public_key(&self) -> [u8; 32] {
        self.state.receiver_public_key()
    }

    /// Verify and install a grant without mutating state on failure.
    pub fn install_grant(
        &mut self,
        grant: PortableKeyGrant,
    ) -> Result<GrantInstallOutcome, KeyGrantProfileError> {
        if let Some(latest) = &self.latest_grant {
            if grant.key_epoch() < latest.key_epoch() {
                return Err(KeyGrantProfileError::DecreasingEpoch);
            }
            if grant.key_epoch() == latest.key_epoch() {
                return if grant == *latest {
                    Ok(GrantInstallOutcome::Idempotent)
                } else {
                    Err(KeyGrantProfileError::ConflictingGrant)
                };
            }
        }
        validate_grant(&grant.grant)?;
        let retained = grant.clone();
        let outcome = self.state.install_grant(grant.grant)?;
        if outcome == GrantInstallOutcome::Installed {
            self.latest_grant = Some(retained);
        }
        Ok(outcome)
    }

    /// Borrow one retained CMK or return `missing_epoch_key`.
    pub fn key(&self, epoch: KeyEpoch) -> Result<&ChannelMasterKey, KeyGrantProfileError> {
        self.state
            .epoch_key(epoch)
            .ok_or(KeyGrantProfileError::MissingEpochKey)
    }

    /// Return the newest installed epoch.
    pub fn latest_epoch(&self) -> Option<KeyEpoch> {
        self.state.latest_epoch()
    }
}

/// One already-authorized receiver and its independent deterministic material.
pub struct RotationReceiver {
    receiver_id: Vec<u8>,
    public_key: [u8; 32],
    ephemeral_private_key: Zeroizing<[u8; 32]>,
    wrapping_nonce: [u8; 24],
}

impl RotationReceiver {
    /// Validate one receiver binding for deterministic rotation planning.
    pub fn with_material(
        receiver_id: Vec<u8>,
        public_key: [u8; 32],
        ephemeral_private_key: [u8; 32],
        wrapping_nonce: [u8; 24],
    ) -> Result<Self, KeyGrantProfileError> {
        validate_identity(&receiver_id)?;
        Ok(Self {
            receiver_id,
            public_key,
            ephemeral_private_key: Zeroizing::new(ephemeral_private_key),
            wrapping_nonce,
        })
    }

    /// Create one production receiver binding with independent CSPRNG material.
    pub fn generate(
        receiver_id: Vec<u8>,
        public_key: [u8; 32],
    ) -> Result<Self, KeyGrantProfileError> {
        validate_identity(&receiver_id)?;
        Ok(Self {
            receiver_id,
            public_key,
            ephemeral_private_key: Zeroizing::new(
                random_array().map_err(|_| KeyGrantProfileError::RandomnessUnavailable)?,
            ),
            wrapping_nonce: random_array()
                .map_err(|_| KeyGrantProfileError::RandomnessUnavailable)?,
        })
    }

    /// Borrow the canonical receiver identifier.
    pub fn receiver_id(&self) -> &[u8] {
        &self.receiver_id
    }
}

/// Pure D18Q rotation result; durable activation is caller-owned.
pub struct RotationPlan {
    new_epoch: KeyEpoch,
    new_cmk: ChannelMasterKey,
    grants: Vec<PortableKeyGrant>,
}

impl RotationPlan {
    /// Return the exactly-once successor epoch.
    pub fn new_epoch(&self) -> KeyEpoch {
        self.new_epoch
    }

    /// Borrow the new originator-held CMK.
    pub fn new_cmk(&self) -> &ChannelMasterKey {
        &self.new_cmk
    }

    /// Borrow grants sorted by raw receiver identifier bytes.
    pub fn grants(&self) -> &[PortableKeyGrant] {
        &self.grants
    }

    /// Consume the plan into its durable-composition parts.
    pub fn into_parts(self) -> (KeyEpoch, ChannelMasterKey, Vec<PortableKeyGrant>) {
        (self.new_epoch, self.new_cmk, self.grants)
    }
}

/// Create a complete, ordered rotation plan or return no partial plan.
pub fn plan_rotation(
    originator_id: &[u8],
    channel_id: ChannelId,
    current_epoch: KeyEpoch,
    new_cmk: ChannelMasterKey,
    mut receivers: Vec<RotationReceiver>,
    signing_key: &OriginatorSigningKey,
) -> Result<RotationPlan, KeyGrantProfileError> {
    validate_identity(originator_id)?;
    validate_channel_id(channel_id)?;
    let new_epoch = KeyEpoch(
        current_epoch
            .0
            .checked_add(1)
            .ok_or(KeyGrantProfileError::EpochExhausted)?,
    );
    if receivers.is_empty() {
        return Err(KeyGrantProfileError::InvalidField);
    }
    receivers.sort_by(|left, right| left.receiver_id.cmp(&right.receiver_id));
    let mut seen = BTreeSet::new();
    let mut grants = Vec::with_capacity(receivers.len());
    for receiver in receivers {
        if !seen.insert(receiver.receiver_id.clone()) {
            return Err(KeyGrantProfileError::InvalidField);
        }
        let fields = KeyGrantFields::new(
            originator_id.to_vec(),
            receiver.receiver_id,
            channel_id,
            new_epoch,
        )?;
        grants.push(seal_channel_key_with_material(
            &fields,
            &new_cmk,
            &receiver.public_key,
            signing_key,
            *receiver.ephemeral_private_key,
            receiver.wrapping_nonce,
        )?);
    }
    Ok(RotationPlan {
        new_epoch,
        new_cmk,
        grants,
    })
}

/// Closed D18Q vocabulary for implementation erasure strength.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecretErasureCapability {
    /// Controlled Rust destruction paths use volatile overwrites and a fence.
    Guaranteed,
    /// Mutable buffers are overwritten where the runtime permits it.
    BestEffort,
    /// The runtime cannot enforce physical overwrite of secret values.
    NotEnforceable,
}

impl SecretErasureCapability {
    /// Return the stable cross-language capability label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Guaranteed => "guaranteed",
            Self::BestEffort => "best_effort",
            Self::NotEnforceable => "not_enforceable",
        }
    }
}

/// Report the Rust implementation's controlled-destruction capability.
pub const fn secret_erasure_capability() -> SecretErasureCapability {
    SecretErasureCapability::Guaranteed
}

fn validate_identity(identity: &[u8]) -> Result<(), KeyGrantProfileError> {
    if identity.is_empty() {
        return Err(KeyGrantProfileError::InvalidField);
    }
    if identity.len() > MAX_IDENTITY_BYTES {
        return Err(KeyGrantProfileError::LengthLimitExceeded);
    }
    Ok(())
}

fn validate_channel_id(channel_id: ChannelId) -> Result<(), KeyGrantProfileError> {
    if channel_id.0[6] >> 4 != 7 || channel_id.0[8] >> 6 != 2 {
        return Err(KeyGrantProfileError::InvalidField);
    }
    Ok(())
}

fn validate_grant(grant: &SealedChannelKeyGrant) -> Result<(), KeyGrantProfileError> {
    validate_identity(&grant.originator_id)?;
    validate_identity(&grant.receiver_id)?;
    validate_channel_id(grant.channel_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn channel_id() -> ChannelId {
        ChannelId([
            0x01, 0x8f, 0x47, 0xa0, 0x9b, 0x6c, 0x7d, 0xef, 0x92, 0x34, 0x56, 0x78, 0x9a, 0xbc,
            0xde, 0xf0,
        ])
    }

    #[test]
    fn explicit_material_and_production_primitives_share_d18g_bytes() {
        let receiver = ReceiverKeyPair::from_private_key([0x33; 32]).unwrap();
        let signer = OriginatorSigningKey::from_seed([0x11; 32]);
        let fields = KeyGrantFields::new(
            b"originator".to_vec(),
            b"receiver".to_vec(),
            channel_id(),
            KeyEpoch(0),
        )
        .unwrap();
        let profile = seal_channel_key_with_material(
            &fields,
            &ChannelMasterKey::from_bytes([0x22; 32]),
            &receiver.public_key(),
            &signer,
            [0x44; 32],
            [0x55; 24],
        )
        .unwrap();
        let raw = seal_channel_key_with_material_raw(
            fields.originator_id(),
            fields.receiver_id(),
            fields.channel_id(),
            fields.key_epoch(),
            &ChannelMasterKey::from_bytes([0x22; 32]),
            &receiver.public_key(),
            &signer,
            Zeroizing::new([0x44; 32]),
            [0x55; 24],
        )
        .unwrap();
        assert_eq!(
            grant_serialize(&profile).unwrap(),
            wire::encode_key_grant(&raw).unwrap()
        );
    }

    #[test]
    fn rotation_sorts_receivers_and_rejects_exhaustion() {
        let receiver_a = ReceiverKeyPair::from_private_key([0x31; 32]).unwrap();
        let receiver_b = ReceiverKeyPair::from_private_key([0x32; 32]).unwrap();
        let plan = plan_rotation(
            b"originator",
            channel_id(),
            KeyEpoch(4),
            ChannelMasterKey::from_bytes([0x66; 32]),
            vec![
                RotationReceiver::with_material(
                    b"receiver-b".to_vec(),
                    receiver_b.public_key(),
                    [0x42; 32],
                    [0x52; 24],
                )
                .unwrap(),
                RotationReceiver::with_material(
                    b"receiver-a".to_vec(),
                    receiver_a.public_key(),
                    [0x41; 32],
                    [0x51; 24],
                )
                .unwrap(),
            ],
            &OriginatorSigningKey::from_seed([0x11; 32]),
        )
        .unwrap();
        assert_eq!(plan.new_epoch(), KeyEpoch(5));
        assert_eq!(plan.grants()[0].receiver_id(), b"receiver-a");
        assert_eq!(plan.grants()[1].receiver_id(), b"receiver-b");
        assert_eq!(secret_erasure_capability().as_str(), "guaranteed");
        let error = match plan_rotation(
            b"originator",
            channel_id(),
            KeyEpoch(u64::MAX),
            ChannelMasterKey::from_bytes([0x66; 32]),
            vec![RotationReceiver::with_material(
                b"receiver-a".to_vec(),
                receiver_a.public_key(),
                [0x41; 32],
                [0x51; 24],
            )
            .unwrap()],
            &OriginatorSigningKey::from_seed([0x11; 32]),
        ) {
            Ok(_) => panic!("exhausted rotation unexpectedly succeeded"),
            Err(error) => error,
        };
        assert_eq!(error, KeyGrantProfileError::EpochExhausted);
    }
}
