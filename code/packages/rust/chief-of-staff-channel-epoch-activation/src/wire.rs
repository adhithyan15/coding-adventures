//! Canonical D18S version 2 state and D18T version 1 plan codecs.
//!
//! D18T changes no D18F, D18G, or D18H bytes. It adds one active-epoch field
//! to the D18P reservation record and one immutable public plan that commits
//! the exact receiver grants selected before activation.

use chief_of_staff_channel_crypto::wire::{decode_message_header, encode_message_header};
use chief_of_staff_channel_crypto::{ChannelId, KeyEpoch, MessageHeader, Sequence};

const STATE_MAGIC: &[u8; 4] = b"D18S";
const STATE_VERSION: u8 = 2;
const PLAN_MAGIC: &[u8; 4] = b"D18T";
const PLAN_VERSION: u8 = 1;

/// Maximum D18H bytes accepted inside D18S version 2.
pub const MAX_PENDING_HEADER_BYTES: usize = 16 * 1024;
/// Maximum receiver commitments in one activation plan.
pub const MAX_PLAN_RECEIVERS: usize = 1024;
/// D18S version 2 content type.
pub const EPOCH_STATE_CONTENT_TYPE: &str =
    "application/vnd.coding-adventures.chief-channel-state-v2";
/// D18T version 1 activation-plan content type.
pub const ACTIVATION_PLAN_CONTENT_TYPE: &str =
    "application/vnd.coding-adventures.chief-channel-epoch-activation-v1";

/// Strict structural failures for D18T public bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EpochWireError {
    /// The record magic did not identify its declared kind.
    InvalidMagic,
    /// The record used another version.
    UnsupportedVersion,
    /// The record ended before a complete field was available.
    Truncated,
    /// Bytes remained after the complete canonical record.
    TrailingBytes,
    /// A bounded length or count was invalid.
    LengthLimitExceeded,
    /// A logical field relation was invalid.
    InvalidField,
    /// The embedded D18H record was invalid.
    InvalidPendingHeader,
}

impl core::fmt::Display for EpochWireError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidMagic => "invalid D18T record magic",
            Self::UnsupportedVersion => "unsupported D18T record version",
            Self::Truncated => "truncated D18T record",
            Self::TrailingBytes => "trailing D18T record bytes",
            Self::LengthLimitExceeded => "D18T length limit exceeded",
            Self::InvalidField => "invalid D18T field",
            Self::InvalidPendingHeader => "invalid D18T pending header",
        })
    }
}

impl std::error::Error for EpochWireError {}

/// Durable active epoch plus the D18P sequence/pending reservation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EpochState {
    active_epoch: KeyEpoch,
    next_sequence: Sequence,
    pending_header: Option<MessageHeader>,
}

impl EpochState {
    /// Create one validated D18S version 2 state value.
    pub fn new(
        channel_id: ChannelId,
        active_epoch: KeyEpoch,
        next_sequence: Sequence,
        pending_header: Option<MessageHeader>,
    ) -> Result<Self, EpochWireError> {
        if let Some(header) = &pending_header {
            if header.fields().channel_id() != channel_id
                || header.fields().key_epoch() != active_epoch
                || header.fields().sequence().0.checked_add(1).map(Sequence) != Some(next_sequence)
            {
                return Err(EpochWireError::InvalidField);
            }
            if encode_message_header(header)
                .map_err(|_| EpochWireError::InvalidPendingHeader)?
                .len()
                > MAX_PENDING_HEADER_BYTES
            {
                return Err(EpochWireError::LengthLimitExceeded);
            }
        }
        Ok(Self {
            active_epoch,
            next_sequence,
            pending_header,
        })
    }

    /// Return the only epoch allowed for a new publish.
    pub fn active_epoch(&self) -> KeyEpoch {
        self.active_epoch
    }

    /// Return the first sequence not yet reserved.
    pub fn next_sequence(&self) -> Sequence {
        self.next_sequence
    }

    /// Borrow the exact pending D18H reservation, if present.
    pub fn pending_header(&self) -> Option<&MessageHeader> {
        self.pending_header.as_ref()
    }

    pub(crate) fn with_pending(
        &self,
        channel_id: ChannelId,
        next_sequence: Sequence,
        pending_header: Option<MessageHeader>,
    ) -> Result<Self, EpochWireError> {
        Self::new(channel_id, self.active_epoch, next_sequence, pending_header)
    }

    pub(crate) fn with_active_epoch(
        &self,
        channel_id: ChannelId,
        active_epoch: KeyEpoch,
    ) -> Result<Self, EpochWireError> {
        Self::new(
            channel_id,
            active_epoch,
            self.next_sequence,
            self.pending_header.clone(),
        )
    }
}

/// Serialize canonical D18S version 2 bytes.
pub fn epoch_state_serialize(state: &EpochState) -> Result<Vec<u8>, EpochWireError> {
    let mut bytes = Vec::with_capacity(22);
    bytes.extend_from_slice(STATE_MAGIC);
    bytes.push(STATE_VERSION);
    bytes.extend_from_slice(&state.active_epoch.0.to_be_bytes());
    bytes.extend_from_slice(&state.next_sequence.0.to_be_bytes());
    match &state.pending_header {
        None => bytes.push(0),
        Some(header) => {
            let header =
                encode_message_header(header).map_err(|_| EpochWireError::InvalidPendingHeader)?;
            if header.len() > MAX_PENDING_HEADER_BYTES {
                return Err(EpochWireError::LengthLimitExceeded);
            }
            bytes.push(1);
            bytes.extend_from_slice(&(header.len() as u32).to_be_bytes());
            bytes.extend_from_slice(&header);
        }
    }
    Ok(bytes)
}

/// Decode and validate canonical D18S version 2 bytes for one channel.
pub fn epoch_state_deserialize(
    bytes: &[u8],
    channel_id: ChannelId,
) -> Result<EpochState, EpochWireError> {
    if bytes.len() < 5 {
        return Err(EpochWireError::Truncated);
    }
    if &bytes[..4] != STATE_MAGIC {
        return Err(EpochWireError::InvalidMagic);
    }
    if bytes[4] != STATE_VERSION {
        return Err(EpochWireError::UnsupportedVersion);
    }
    if bytes.len() < 22 {
        return Err(EpochWireError::Truncated);
    }
    let active_epoch = KeyEpoch(read_u64(bytes, 5)?);
    let next_sequence = Sequence(read_u64(bytes, 13)?);
    let pending_header = match bytes[21] {
        0 => {
            if bytes.len() != 22 {
                return Err(EpochWireError::TrailingBytes);
            }
            None
        }
        1 => {
            if bytes.len() < 26 {
                return Err(EpochWireError::Truncated);
            }
            let length = read_u32(bytes, 22)? as usize;
            if length > MAX_PENDING_HEADER_BYTES {
                return Err(EpochWireError::LengthLimitExceeded);
            }
            let end = 26usize
                .checked_add(length)
                .ok_or(EpochWireError::LengthLimitExceeded)?;
            if bytes.len() < end {
                return Err(EpochWireError::Truncated);
            }
            if bytes.len() > end {
                return Err(EpochWireError::TrailingBytes);
            }
            Some(
                decode_message_header(&bytes[26..end])
                    .map_err(|_| EpochWireError::InvalidPendingHeader)?,
            )
        }
        _ => return Err(EpochWireError::InvalidField),
    };
    EpochState::new(channel_id, active_epoch, next_sequence, pending_header)
}

/// One receiver/grant commitment in an activation plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActivationPlanEntry {
    receiver_id_hash: [u8; 32],
    grant_hash: [u8; 32],
}

impl ActivationPlanEntry {
    /// Construct one public commitment pair.
    pub const fn new(receiver_id_hash: [u8; 32], grant_hash: [u8; 32]) -> Self {
        Self {
            receiver_id_hash,
            grant_hash,
        }
    }

    /// Return SHA-256 of the raw receiver identifier.
    pub const fn receiver_id_hash(&self) -> [u8; 32] {
        self.receiver_id_hash
    }

    /// Return SHA-256 of the exact canonical D18G bytes.
    pub const fn grant_hash(&self) -> [u8; 32] {
        self.grant_hash
    }
}

/// Immutable canonical D18T version 1 plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActivationPlan {
    channel_id: ChannelId,
    base_epoch: KeyEpoch,
    new_epoch: KeyEpoch,
    receivers: Vec<ActivationPlanEntry>,
}

impl ActivationPlan {
    /// Validate and own one activation plan, sorting commitments canonically.
    pub fn new(
        channel_id: ChannelId,
        base_epoch: KeyEpoch,
        new_epoch: KeyEpoch,
        mut receivers: Vec<ActivationPlanEntry>,
    ) -> Result<Self, EpochWireError> {
        validate_channel_id(channel_id)?;
        if base_epoch.0.checked_add(1).map(KeyEpoch) != Some(new_epoch)
            || receivers.is_empty()
            || receivers.len() > MAX_PLAN_RECEIVERS
        {
            return Err(EpochWireError::InvalidField);
        }
        receivers.sort_by_key(ActivationPlanEntry::receiver_id_hash);
        for (index, entry) in receivers.iter().enumerate() {
            if index > 0 && receivers[index - 1].receiver_id_hash == entry.receiver_id_hash
                || receivers[..index]
                    .iter()
                    .any(|prior| prior.grant_hash == entry.grant_hash)
            {
                return Err(EpochWireError::InvalidField);
            }
        }
        Ok(Self {
            channel_id,
            base_epoch,
            new_epoch,
            receivers,
        })
    }

    /// Return the channel identifier.
    pub const fn channel_id(&self) -> ChannelId {
        self.channel_id
    }

    /// Return the epoch this plan advances from.
    pub const fn base_epoch(&self) -> KeyEpoch {
        self.base_epoch
    }

    /// Return the selected successor epoch.
    pub const fn new_epoch(&self) -> KeyEpoch {
        self.new_epoch
    }

    /// Borrow entries sorted by receiver-ID hash.
    pub fn receivers(&self) -> &[ActivationPlanEntry] {
        &self.receivers
    }
}

/// Serialize canonical D18T version 1 plan bytes.
pub fn activation_plan_serialize(plan: &ActivationPlan) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(41 + plan.receivers.len() * 64);
    bytes.extend_from_slice(PLAN_MAGIC);
    bytes.push(PLAN_VERSION);
    bytes.extend_from_slice(&plan.channel_id.0);
    bytes.extend_from_slice(&plan.base_epoch.0.to_be_bytes());
    bytes.extend_from_slice(&plan.new_epoch.0.to_be_bytes());
    bytes.extend_from_slice(&(plan.receivers.len() as u32).to_be_bytes());
    for receiver in &plan.receivers {
        bytes.extend_from_slice(&receiver.receiver_id_hash);
        bytes.extend_from_slice(&receiver.grant_hash);
    }
    bytes
}

/// Decode and validate canonical D18T version 1 plan bytes.
pub fn activation_plan_deserialize(bytes: &[u8]) -> Result<ActivationPlan, EpochWireError> {
    if bytes.len() < 5 {
        return Err(EpochWireError::Truncated);
    }
    if &bytes[..4] != PLAN_MAGIC {
        return Err(EpochWireError::InvalidMagic);
    }
    if bytes[4] != PLAN_VERSION {
        return Err(EpochWireError::UnsupportedVersion);
    }
    if bytes.len() < 41 {
        return Err(EpochWireError::Truncated);
    }
    let channel_id = ChannelId(
        bytes[5..21]
            .try_into()
            .map_err(|_| EpochWireError::Truncated)?,
    );
    let base_epoch = KeyEpoch(read_u64(bytes, 21)?);
    let new_epoch = KeyEpoch(read_u64(bytes, 29)?);
    let count = read_u32(bytes, 37)? as usize;
    if count == 0 || count > MAX_PLAN_RECEIVERS {
        return Err(EpochWireError::LengthLimitExceeded);
    }
    let expected = 41usize
        .checked_add(
            count
                .checked_mul(64)
                .ok_or(EpochWireError::LengthLimitExceeded)?,
        )
        .ok_or(EpochWireError::LengthLimitExceeded)?;
    if bytes.len() < expected {
        return Err(EpochWireError::Truncated);
    }
    if bytes.len() > expected {
        return Err(EpochWireError::TrailingBytes);
    }
    let mut receivers = Vec::with_capacity(count);
    for chunk in bytes[41..].chunks_exact(64) {
        receivers.push(ActivationPlanEntry::new(
            chunk[..32]
                .try_into()
                .map_err(|_| EpochWireError::Truncated)?,
            chunk[32..]
                .try_into()
                .map_err(|_| EpochWireError::Truncated)?,
        ));
    }
    if receivers
        .windows(2)
        .any(|pair| pair[0].receiver_id_hash >= pair[1].receiver_id_hash)
        || receivers.iter().enumerate().any(|(index, entry)| {
            receivers[..index]
                .iter()
                .any(|prior| prior.grant_hash == entry.grant_hash)
        })
    {
        return Err(EpochWireError::InvalidField);
    }
    ActivationPlan::new(channel_id, base_epoch, new_epoch, receivers)
}

/// Return the deterministic public activation-plan storage key.
pub fn activation_plan_record_key(channel_id: ChannelId, new_epoch: KeyEpoch) -> String {
    format!(
        "{}/epochs/{:020}/activation",
        encode_hex(&channel_id.0),
        new_epoch.0
    )
}

fn validate_channel_id(channel_id: ChannelId) -> Result<(), EpochWireError> {
    if channel_id.0[6] >> 4 != 7 || channel_id.0[8] >> 6 != 2 {
        return Err(EpochWireError::InvalidField);
    }
    Ok(())
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, EpochWireError> {
    Ok(u32::from_be_bytes(
        bytes
            .get(offset..offset + 4)
            .ok_or(EpochWireError::Truncated)?
            .try_into()
            .map_err(|_| EpochWireError::Truncated)?,
    ))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, EpochWireError> {
    Ok(u64::from_be_bytes(
        bytes
            .get(offset..offset + 8)
            .ok_or(EpochWireError::Truncated)?
            .try_into()
            .map_err(|_| EpochWireError::Truncated)?,
    ))
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;
    use chief_of_staff_channel_crypto::{prepare_message_header, MessageFields};

    fn channel_id() -> ChannelId {
        ChannelId([
            0x01, 0x8f, 0x47, 0xa0, 0x9b, 0x6c, 0x7d, 0xef, 0x92, 0x34, 0x56, 0x78, 0x9a, 0xbc,
            0xde, 0xf0,
        ])
    }

    #[test]
    fn state_round_trip_is_exact_and_rejects_epoch_mismatch() {
        let header = prepare_message_header(
            MessageFields::new(
                [0x22; 16],
                7,
                b"originator".to_vec(),
                channel_id(),
                Sequence(3),
                KeyEpoch(9),
                "text/plain".into(),
            ),
            b"hello",
        );
        let state = EpochState::new(channel_id(), KeyEpoch(9), Sequence(4), Some(header)).unwrap();
        let bytes = epoch_state_serialize(&state).unwrap();
        assert_eq!(&bytes[..5], b"D18S\x02");
        assert_eq!(
            epoch_state_deserialize(&bytes, channel_id()).unwrap(),
            state
        );

        let mut wrong = bytes;
        wrong[12] = 8;
        assert_eq!(
            epoch_state_deserialize(&wrong, channel_id()),
            Err(EpochWireError::InvalidField)
        );
    }

    #[test]
    fn plan_round_trip_sorts_commitments_and_rejects_trailing_bytes() {
        let plan = ActivationPlan::new(
            channel_id(),
            KeyEpoch(9),
            KeyEpoch(10),
            vec![
                ActivationPlanEntry::new([2; 32], [4; 32]),
                ActivationPlanEntry::new([1; 32], [3; 32]),
            ],
        )
        .unwrap();
        assert_eq!(plan.receivers()[0].receiver_id_hash(), [1; 32]);
        let bytes = activation_plan_serialize(&plan);
        assert_eq!(&bytes[..5], b"D18T\x01");
        assert_eq!(activation_plan_deserialize(&bytes).unwrap(), plan);
        let mut padded = bytes;
        padded.push(0);
        assert_eq!(
            activation_plan_deserialize(&padded),
            Err(EpochWireError::TrailingBytes)
        );
    }
}
