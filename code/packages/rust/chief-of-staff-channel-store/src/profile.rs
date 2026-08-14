//! Portable D18P adapters over the production channel store.
//!
//! These helpers expose the already-shipped D18S reservation-state and D18A
//! receiver-cursor codecs without creating a second serialization path. They
//! also collapse implementation-specific errors into the stable D18P codes
//! consumed by cross-language fixtures.

use chief_of_staff_channel_crypto::ChannelCryptoError;

use super::{
    decode_cursor, decode_state, encode_cursor, encode_state, ChannelState, ChannelStoreError,
    ACK_CONTENT_TYPE, GRANT_CONTENT_TYPE, MAX_CAS_ATTEMPTS, MAX_STATE_HEADER_BYTES,
    MESSAGE_CONTENT_TYPE, STATE_CONTENT_TYPE,
};
use chief_of_staff_channel_crypto::{ChannelId, Sequence};

/// D18P content type for a durable reservation-state record.
pub const CHANNEL_STATE_CONTENT_TYPE: &str = STATE_CONTENT_TYPE;

/// D18P content type for an immutable encrypted-message record.
pub const CHANNEL_MESSAGE_CONTENT_TYPE: &str = MESSAGE_CONTENT_TYPE;

/// D18P content type for an immutable sealed receiver-grant record.
pub const CHANNEL_GRANT_CONTENT_TYPE: &str = GRANT_CONTENT_TYPE;

/// D18P content type for a receiver acknowledgement cursor.
pub const CHANNEL_ACK_CONTENT_TYPE: &str = ACK_CONTENT_TYPE;

/// Maximum encoded D18H bytes accepted inside D18S version 1.
pub const MAX_PENDING_HEADER_BYTES: usize = MAX_STATE_HEADER_BYTES;

/// Maximum revision-CAS attempts made by one D18P store operation.
pub const MAX_CHANNEL_CAS_ATTEMPTS: usize = MAX_CAS_ATTEMPTS;

/// Serialize one reservation state as the production D18S version 1 bytes.
pub fn channel_state_serialize(state: &ChannelState) -> Result<Vec<u8>, ChannelStoreError> {
    encode_state(state)
}

/// Decode and validate production D18S version 1 bytes for one channel.
pub fn channel_state_deserialize(
    bytes: &[u8],
    channel_id: ChannelId,
) -> Result<ChannelState, ChannelStoreError> {
    decode_state(bytes, channel_id)
}

/// Serialize a first-unread sequence as production D18A version 1 bytes.
pub fn receiver_cursor_serialize(first_unread: Sequence) -> Vec<u8> {
    encode_cursor(first_unread)
}

/// Decode production D18A version 1 bytes into a first-unread sequence.
pub fn receiver_cursor_deserialize(bytes: &[u8]) -> Result<Sequence, ChannelStoreError> {
    decode_cursor(bytes)
}

/// Return the normative D18P machine-readable code for a store failure.
pub const fn channel_store_error_code(error: &ChannelStoreError) -> &'static str {
    match error {
        ChannelStoreError::Storage(_) => "storage_error",
        ChannelStoreError::Wire(_) => "wire_error",
        ChannelStoreError::Crypto(ChannelCryptoError::SequenceExhausted) => "sequence_exhausted",
        ChannelStoreError::Crypto(_) => "crypto_error",
        ChannelStoreError::NotInitialized => "not_initialized",
        ChannelStoreError::CorruptRecord(_) => "corrupt_record",
        ChannelStoreError::PendingAppend(_) => "pending_append",
        ChannelStoreError::NoPendingAppend => "no_pending_append",
        ChannelStoreError::PendingHeaderMismatch => "pending_header_mismatch",
        ChannelStoreError::ConflictingRecord(_) => "conflicting_record",
        ChannelStoreError::ConcurrentUpdate => "concurrent_update",
        ChannelStoreError::InvalidReceiverId => "invalid_receiver_id",
        ChannelStoreError::InvalidPageSize => "invalid_page_size",
        ChannelStoreError::AcknowledgementRegression { .. } => "acknowledgement_regression",
        ChannelStoreError::AcknowledgementAhead { .. } => "acknowledgement_ahead",
        ChannelStoreError::AcknowledgementPending { .. } => "acknowledgement_pending",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chief_of_staff_channel_crypto::{
        prepare_message_header, ChannelId, KeyEpoch, MessageFields, Sequence,
    };

    fn channel_id() -> ChannelId {
        let mut bytes = [0x31; 16];
        bytes[6] = 0x71;
        bytes[8] = 0x91;
        ChannelId(bytes)
    }

    #[test]
    fn portable_state_and_cursor_helpers_preserve_production_bytes() {
        let initial = ChannelState {
            next_sequence: Sequence(0),
            pending_header: None,
        };
        let initial_bytes = channel_state_serialize(&initial).unwrap();
        assert_eq!(&initial_bytes[..5], b"D18S\x01");
        assert_eq!(
            channel_state_deserialize(&initial_bytes, channel_id()).unwrap(),
            initial
        );

        let pending = ChannelState {
            next_sequence: Sequence(8),
            pending_header: Some(prepare_message_header(
                MessageFields::new(
                    [0x71; 16],
                    99,
                    b"originator".to_vec(),
                    channel_id(),
                    Sequence(7),
                    KeyEpoch(3),
                    "application/octet-stream".to_owned(),
                ),
                b"pending",
            )),
        };
        let pending_bytes = channel_state_serialize(&pending).unwrap();
        assert_eq!(
            channel_state_deserialize(&pending_bytes, channel_id()).unwrap(),
            pending
        );

        let cursor_bytes = receiver_cursor_serialize(Sequence(42));
        assert_eq!(&cursor_bytes[..5], b"D18A\x01");
        assert_eq!(
            receiver_cursor_deserialize(&cursor_bytes).unwrap(),
            Sequence(42)
        );
    }

    #[test]
    fn portable_codec_failures_use_stable_codes() {
        assert_eq!(
            channel_store_error_code(
                &channel_state_deserialize(b"not state", channel_id()).unwrap_err()
            ),
            "corrupt_record"
        );
        assert_eq!(
            channel_store_error_code(&receiver_cursor_deserialize(b"bad").unwrap_err()),
            "corrupt_record"
        );
        assert_eq!(
            channel_store_error_code(&ChannelStoreError::Crypto(
                ChannelCryptoError::SequenceExhausted
            )),
            "sequence_exhausted"
        );
    }
}
