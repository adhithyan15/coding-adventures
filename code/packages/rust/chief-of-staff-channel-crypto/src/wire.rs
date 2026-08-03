//! Versioned wire records and stable storage keys for encrypted channels.
//!
//! Decoding is purely structural. Callers must still pass decoded grants to
//! [`crate::open_channel_key_grant`] and decoded messages to
//! [`crate::decrypt_message`] before trusting their contents.

use crate::{
    ChannelId, EncryptedMessage, KeyEpoch, MessageFields, MessageHeader, SealedChannelKeyGrant,
    Sequence,
};
use coding_adventures_sha256::sha256;

const GRANT_MAGIC: &[u8; 4] = b"D18G";
const HEADER_MAGIC: &[u8; 4] = b"D18H";
const MESSAGE_MAGIC: &[u8; 4] = b"D18M";

/// Current Chief channel record version.
pub const WIRE_VERSION: u8 = 1;

/// Maximum encoded originator or receiver identifier length.
pub const MAX_IDENTITY_BYTES: usize = 4 * 1024;

/// Maximum encoded MIME content-type length.
pub const MAX_CONTENT_TYPE_BYTES: usize = 1024;

/// Maximum ciphertext carried by one channel record.
pub const MAX_CIPHERTEXT_BYTES: usize = 64 * 1024 * 1024;

/// Storage namespace shared by durable Chief channel records.
pub const CHANNEL_STORAGE_NAMESPACE: &str = "chief-channels";

/// Structural errors produced by the channel record codec.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChannelWireError {
    /// The record ended before a complete field could be read.
    Truncated,
    /// The record did not begin with the expected record-type magic.
    InvalidMagic,
    /// The record uses a newer or unknown version.
    UnsupportedVersion(u8),
    /// A variable-width field exceeded its declared safety bound.
    LengthLimitExceeded {
        /// Logical field whose length was rejected.
        field: &'static str,
        /// Length declared by the record.
        length: u64,
        /// Maximum accepted length.
        maximum: u64,
    },
    /// A text field was not valid UTF-8.
    InvalidUtf8(&'static str),
    /// Complete logical fields were followed by unrecognized bytes.
    TrailingBytes,
}

impl core::fmt::Display for ChannelWireError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Truncated => f.write_str("channel wire record truncated"),
            Self::InvalidMagic => f.write_str("channel wire record magic invalid"),
            Self::UnsupportedVersion(version) => {
                write!(f, "channel wire record version {version} unsupported")
            }
            Self::LengthLimitExceeded {
                field,
                length,
                maximum,
            } => write!(
                f,
                "channel wire field {field} length {length} exceeds {maximum}"
            ),
            Self::InvalidUtf8(field) => {
                write!(f, "channel wire field {field} is not valid UTF-8")
            }
            Self::TrailingBytes => f.write_str("channel wire record has trailing bytes"),
        }
    }
}

impl std::error::Error for ChannelWireError {}

/// Encode a signed receiver key grant as one bounded binary record.
pub fn encode_key_grant(grant: &SealedChannelKeyGrant) -> Result<Vec<u8>, ChannelWireError> {
    validate_length(
        "originator_id",
        grant.originator_id.len(),
        MAX_IDENTITY_BYTES,
    )?;
    validate_length("receiver_id", grant.receiver_id.len(), MAX_IDENTITY_BYTES)?;
    let mut encoded = Vec::with_capacity(
        5 + 4
            + grant.originator_id.len()
            + 4
            + grant.receiver_id.len()
            + 16
            + 8
            + 32
            + 24
            + 48
            + 64,
    );
    encoded.extend_from_slice(GRANT_MAGIC);
    encoded.push(WIRE_VERSION);
    put_bytes_u32(&mut encoded, &grant.originator_id);
    put_bytes_u32(&mut encoded, &grant.receiver_id);
    encoded.extend_from_slice(&grant.channel_id.0);
    encoded.extend_from_slice(&grant.key_epoch.0.to_be_bytes());
    encoded.extend_from_slice(&grant.ephemeral_public_key);
    encoded.extend_from_slice(&grant.wrapping_nonce);
    encoded.extend_from_slice(&grant.wrapped_cmk);
    encoded.extend_from_slice(&grant.originator_signature);
    Ok(encoded)
}

/// Decode one structurally valid signed key-grant record.
pub fn decode_key_grant(bytes: &[u8]) -> Result<SealedChannelKeyGrant, ChannelWireError> {
    let mut decoder = Decoder::new(bytes);
    decoder.expect_magic(GRANT_MAGIC)?;
    decoder.expect_version()?;
    let originator_id = decoder.read_vec_u32("originator_id", MAX_IDENTITY_BYTES)?;
    let receiver_id = decoder.read_vec_u32("receiver_id", MAX_IDENTITY_BYTES)?;
    let channel_id = ChannelId(decoder.read_array()?);
    let key_epoch = KeyEpoch(decoder.read_u64()?);
    let ephemeral_public_key = decoder.read_array()?;
    let wrapping_nonce = decoder.read_array()?;
    let wrapped_cmk = decoder.read_array()?;
    let originator_signature = decoder.read_array()?;
    decoder.finish()?;
    Ok(SealedChannelKeyGrant {
        originator_id,
        receiver_id,
        channel_id,
        key_epoch,
        ephemeral_public_key,
        wrapping_nonce,
        wrapped_cmk,
        originator_signature,
    })
}

/// Encode an authenticated message header for durable pre-encryption reservation.
pub fn encode_message_header(header: &MessageHeader) -> Result<Vec<u8>, ChannelWireError> {
    validate_length(
        "originator_id",
        header.fields.originator_id.len(),
        MAX_IDENTITY_BYTES,
    )?;
    validate_length(
        "content_type",
        header.fields.content_type.len(),
        MAX_CONTENT_TYPE_BYTES,
    )?;
    let mut encoded = Vec::with_capacity(
        5 + 16
            + 8
            + 4
            + header.fields.originator_id.len()
            + 16
            + 8
            + 8
            + 4
            + header.fields.content_type.len()
            + 32,
    );
    encoded.extend_from_slice(HEADER_MAGIC);
    encoded.push(WIRE_VERSION);
    encoded.extend_from_slice(&header.fields.message_id);
    encoded.extend_from_slice(&header.fields.timestamp_ns.to_be_bytes());
    put_bytes_u32(&mut encoded, &header.fields.originator_id);
    encoded.extend_from_slice(&header.fields.channel_id.0);
    encoded.extend_from_slice(&header.fields.sequence.0.to_be_bytes());
    encoded.extend_from_slice(&header.fields.key_epoch.0.to_be_bytes());
    put_bytes_u32(&mut encoded, header.fields.content_type.as_bytes());
    encoded.extend_from_slice(&header.plaintext_hash);
    Ok(encoded)
}

/// Decode one structurally valid authenticated message header.
pub fn decode_message_header(bytes: &[u8]) -> Result<MessageHeader, ChannelWireError> {
    let mut decoder = Decoder::new(bytes);
    decoder.expect_magic(HEADER_MAGIC)?;
    decoder.expect_version()?;
    let message_id = decoder.read_array()?;
    let timestamp_ns = decoder.read_u64()?;
    let originator_id = decoder.read_vec_u32("originator_id", MAX_IDENTITY_BYTES)?;
    let channel_id = ChannelId(decoder.read_array()?);
    let sequence = Sequence(decoder.read_u64()?);
    let key_epoch = KeyEpoch(decoder.read_u64()?);
    let content_type_bytes = decoder.read_vec_u32("content_type", MAX_CONTENT_TYPE_BYTES)?;
    let content_type = String::from_utf8(content_type_bytes)
        .map_err(|_| ChannelWireError::InvalidUtf8("content_type"))?;
    let plaintext_hash = decoder.read_array()?;
    decoder.finish()?;
    Ok(MessageHeader {
        fields: MessageFields {
            message_id,
            timestamp_ns,
            originator_id,
            channel_id,
            sequence,
            key_epoch,
            content_type,
        },
        plaintext_hash,
    })
}

/// Encode an encrypted append-log message as one bounded binary record.
pub fn encode_message(message: &EncryptedMessage) -> Result<Vec<u8>, ChannelWireError> {
    validate_length(
        "originator_id",
        message.header.fields.originator_id.len(),
        MAX_IDENTITY_BYTES,
    )?;
    validate_length(
        "content_type",
        message.header.fields.content_type.len(),
        MAX_CONTENT_TYPE_BYTES,
    )?;
    validate_length("ciphertext", message.ciphertext.len(), MAX_CIPHERTEXT_BYTES)?;
    let mut encoded = Vec::with_capacity(
        5 + 16
            + 8
            + 4
            + message.header.fields.originator_id.len()
            + 16
            + 8
            + 8
            + 4
            + message.header.fields.content_type.len()
            + 32
            + 8
            + message.ciphertext.len()
            + 16
            + 64,
    );
    encoded.extend_from_slice(MESSAGE_MAGIC);
    encoded.push(WIRE_VERSION);
    encoded.extend_from_slice(&message.header.fields.message_id);
    encoded.extend_from_slice(&message.header.fields.timestamp_ns.to_be_bytes());
    put_bytes_u32(&mut encoded, &message.header.fields.originator_id);
    encoded.extend_from_slice(&message.header.fields.channel_id.0);
    encoded.extend_from_slice(&message.header.fields.sequence.0.to_be_bytes());
    encoded.extend_from_slice(&message.header.fields.key_epoch.0.to_be_bytes());
    put_bytes_u32(&mut encoded, message.header.fields.content_type.as_bytes());
    encoded.extend_from_slice(&message.header.plaintext_hash);
    put_bytes_u64(&mut encoded, &message.ciphertext);
    encoded.extend_from_slice(&message.authentication_tag);
    encoded.extend_from_slice(&message.originator_signature);
    Ok(encoded)
}

/// Decode one structurally valid encrypted-message record.
pub fn decode_message(bytes: &[u8]) -> Result<EncryptedMessage, ChannelWireError> {
    let mut decoder = Decoder::new(bytes);
    decoder.expect_magic(MESSAGE_MAGIC)?;
    decoder.expect_version()?;
    let message_id = decoder.read_array()?;
    let timestamp_ns = decoder.read_u64()?;
    let originator_id = decoder.read_vec_u32("originator_id", MAX_IDENTITY_BYTES)?;
    let channel_id = ChannelId(decoder.read_array()?);
    let sequence = Sequence(decoder.read_u64()?);
    let key_epoch = KeyEpoch(decoder.read_u64()?);
    let content_type_bytes = decoder.read_vec_u32("content_type", MAX_CONTENT_TYPE_BYTES)?;
    let content_type = String::from_utf8(content_type_bytes)
        .map_err(|_| ChannelWireError::InvalidUtf8("content_type"))?;
    let plaintext_hash = decoder.read_array()?;
    let ciphertext = decoder.read_vec_u64("ciphertext", MAX_CIPHERTEXT_BYTES)?;
    let authentication_tag = decoder.read_array()?;
    let originator_signature = decoder.read_array()?;
    decoder.finish()?;
    Ok(EncryptedMessage {
        header: MessageHeader {
            fields: MessageFields {
                message_id,
                timestamp_ns,
                originator_id,
                channel_id,
                sequence,
                key_epoch,
                content_type,
            },
            plaintext_hash,
        },
        ciphertext,
        authentication_tag,
        originator_signature,
    })
}

/// Stable, lexicographically ordered storage key for one encrypted message.
pub fn message_record_key(channel_id: ChannelId, sequence: Sequence) -> String {
    format!("{}{:020}", message_record_prefix(channel_id), sequence.0)
}

/// Stable storage-key prefix for all encrypted messages in one channel.
pub fn message_record_prefix(channel_id: ChannelId) -> String {
    format!("{}/messages/", encode_hex(&channel_id.0))
}

/// Stable storage key for the durable next-sequence record of one channel.
pub fn sequence_state_record_key(channel_id: ChannelId) -> String {
    format!("{}/state/next-sequence", encode_hex(&channel_id.0))
}

/// Stable storage key for a receiver's sealed grant at one key epoch.
pub fn key_grant_record_key(
    channel_id: ChannelId,
    key_epoch: KeyEpoch,
    receiver_id: &[u8],
) -> String {
    format!(
        "{}/grants/{:020}/{}",
        encode_hex(&channel_id.0),
        key_epoch.0,
        encode_hex(&sha256(receiver_id))
    )
}

/// Stable storage key for one receiver's acknowledgement state.
pub fn receiver_ack_record_key(channel_id: ChannelId, receiver_id: &[u8]) -> String {
    format!(
        "{}/receivers/{}/ack",
        encode_hex(&channel_id.0),
        encode_hex(&sha256(receiver_id))
    )
}

fn validate_length(
    field: &'static str,
    length: usize,
    maximum: usize,
) -> Result<(), ChannelWireError> {
    if length > maximum {
        return Err(ChannelWireError::LengthLimitExceeded {
            field,
            length: length as u64,
            maximum: maximum as u64,
        });
    }
    Ok(())
}

fn put_bytes_u32(output: &mut Vec<u8>, bytes: &[u8]) {
    let length = u32::try_from(bytes.len()).expect("validated bounded field fits in u32");
    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(bytes);
}

fn put_bytes_u64(output: &mut Vec<u8>, bytes: &[u8]) {
    let length = u64::try_from(bytes.len()).expect("usize fits in u64 on supported targets");
    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(bytes);
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

struct Decoder<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Decoder<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn expect_magic(&mut self, expected: &[u8; 4]) -> Result<(), ChannelWireError> {
        let actual: [u8; 4] = self.read_array()?;
        if &actual != expected {
            return Err(ChannelWireError::InvalidMagic);
        }
        Ok(())
    }

    fn expect_version(&mut self) -> Result<(), ChannelWireError> {
        let version = self.read_array::<1>()?[0];
        if version != WIRE_VERSION {
            return Err(ChannelWireError::UnsupportedVersion(version));
        }
        Ok(())
    }

    fn read_u32(&mut self) -> Result<u32, ChannelWireError> {
        Ok(u32::from_be_bytes(self.read_array()?))
    }

    fn read_u64(&mut self) -> Result<u64, ChannelWireError> {
        Ok(u64::from_be_bytes(self.read_array()?))
    }

    fn read_vec_u32(
        &mut self,
        field: &'static str,
        maximum: usize,
    ) -> Result<Vec<u8>, ChannelWireError> {
        let length = self.read_u32()? as u64;
        self.read_bounded_vec(field, length, maximum)
    }

    fn read_vec_u64(
        &mut self,
        field: &'static str,
        maximum: usize,
    ) -> Result<Vec<u8>, ChannelWireError> {
        let length = self.read_u64()?;
        self.read_bounded_vec(field, length, maximum)
    }

    fn read_bounded_vec(
        &mut self,
        field: &'static str,
        length: u64,
        maximum: usize,
    ) -> Result<Vec<u8>, ChannelWireError> {
        if length > maximum as u64 {
            return Err(ChannelWireError::LengthLimitExceeded {
                field,
                length,
                maximum: maximum as u64,
            });
        }
        let length =
            usize::try_from(length).map_err(|_| ChannelWireError::LengthLimitExceeded {
                field,
                length,
                maximum: maximum as u64,
            })?;
        Ok(self.take(length)?.to_vec())
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], ChannelWireError> {
        self.take(N)?
            .try_into()
            .map_err(|_| ChannelWireError::Truncated)
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], ChannelWireError> {
        let end = self
            .position
            .checked_add(length)
            .ok_or(ChannelWireError::Truncated)?;
        let bytes = self
            .bytes
            .get(self.position..end)
            .ok_or(ChannelWireError::Truncated)?;
        self.position = end;
        Ok(bytes)
    }

    fn finish(self) -> Result<(), ChannelWireError> {
        if self.position != self.bytes.len() {
            return Err(ChannelWireError::TrailingBytes);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        decrypt_message, encrypt_message, open_channel_key_grant, seal_channel_key,
        ChannelMasterKey, OriginatorSigningKey, ReceiverKeyPair,
    };

    fn channel_id() -> ChannelId {
        ChannelId([0x42; 16])
    }

    fn encrypted_message() -> (EncryptedMessage, ChannelMasterKey, OriginatorSigningKey) {
        let key = ChannelMasterKey::from_bytes([0xa5; 32]);
        let signer = OriginatorSigningKey::from_seed([0x17; 32]);
        let fields = MessageFields {
            message_id: [0x71; 16],
            timestamp_ns: 1_725_000_000_000_000_000,
            originator_id: b"originator".to_vec(),
            channel_id: channel_id(),
            sequence: Sequence(42),
            key_epoch: KeyEpoch(3),
            content_type: "application/octet-stream".to_owned(),
        };
        (
            encrypt_message(fields, b"encrypted record", &key, &signer),
            key,
            signer,
        )
    }

    #[test]
    fn key_grant_record_round_trip_remains_cryptographically_valid() {
        let signer = OriginatorSigningKey::from_seed([0x17; 32]);
        let receiver = ReceiverKeyPair::from_private_key([0x33; 32]).unwrap();
        let grant = seal_channel_key(
            b"originator",
            b"receiver",
            channel_id(),
            KeyEpoch(3),
            &ChannelMasterKey::from_bytes([0xa5; 32]),
            &receiver.public_key(),
            &signer,
        )
        .unwrap();
        let encoded = encode_key_grant(&grant).unwrap();
        let decoded = decode_key_grant(&encoded).unwrap();
        assert!(decoded == grant);
        let opened = open_channel_key_grant(
            &decoded,
            b"originator",
            b"receiver",
            channel_id(),
            &receiver,
            &signer.public_key(),
        )
        .unwrap();
        assert_eq!(opened.as_bytes(), &[0xa5; 32]);
    }

    #[test]
    fn encrypted_message_record_round_trip_remains_decryptable() {
        let (message, key, signer) = encrypted_message();
        let encoded = encode_message(&message).unwrap();
        let decoded = decode_message(&encoded).unwrap();
        assert!(decoded == message);
        assert_eq!(
            decrypt_message(&decoded, &key, &signer.public_key()).unwrap(),
            b"encrypted record"
        );
    }

    #[test]
    fn authenticated_header_round_trip_is_exact() {
        let (message, _, _) = encrypted_message();
        let encoded = encode_message_header(&message.header).unwrap();
        assert_eq!(decode_message_header(&encoded).unwrap(), message.header);
    }

    #[test]
    fn every_truncated_record_prefix_is_rejected() {
        let (message, _, _) = encrypted_message();
        let header_bytes = encode_message_header(&message.header).unwrap();
        for end in 0..header_bytes.len() {
            assert!(
                decode_message_header(&header_bytes[..end]).is_err(),
                "header prefix {end}"
            );
        }

        let message_bytes = encode_message(&message).unwrap();
        for end in 0..message_bytes.len() {
            assert!(
                decode_message(&message_bytes[..end]).is_err(),
                "prefix {end}"
            );
        }

        let signer = OriginatorSigningKey::from_seed([0x17; 32]);
        let receiver = ReceiverKeyPair::from_private_key([0x33; 32]).unwrap();
        let grant = seal_channel_key(
            b"originator",
            b"receiver",
            channel_id(),
            KeyEpoch(0),
            &ChannelMasterKey::from_bytes([0xa5; 32]),
            &receiver.public_key(),
            &signer,
        )
        .unwrap();
        let grant_bytes = encode_key_grant(&grant).unwrap();
        for end in 0..grant_bytes.len() {
            assert!(
                decode_key_grant(&grant_bytes[..end]).is_err(),
                "prefix {end}"
            );
        }
    }

    #[test]
    fn rejects_wrong_magic_version_and_trailing_bytes() {
        let (message, _, _) = encrypted_message();
        let mut encoded = encode_message(&message).unwrap();
        encoded[0] ^= 1;
        assert_eq!(
            decode_message(&encoded).err().unwrap(),
            ChannelWireError::InvalidMagic
        );

        let mut encoded = encode_message(&message).unwrap();
        encoded[4] = WIRE_VERSION + 1;
        assert_eq!(
            decode_message(&encoded).err().unwrap(),
            ChannelWireError::UnsupportedVersion(WIRE_VERSION + 1)
        );

        let mut encoded = encode_message(&message).unwrap();
        encoded.push(0);
        assert_eq!(
            decode_message(&encoded).err().unwrap(),
            ChannelWireError::TrailingBytes
        );
    }

    #[test]
    fn rejects_oversized_and_non_utf8_fields_before_allocation() {
        let (mut message, _, _) = encrypted_message();
        message.header.fields.originator_id = vec![0; MAX_IDENTITY_BYTES + 1];
        assert_eq!(
            encode_message(&message).unwrap_err(),
            ChannelWireError::LengthLimitExceeded {
                field: "originator_id",
                length: (MAX_IDENTITY_BYTES + 1) as u64,
                maximum: MAX_IDENTITY_BYTES as u64,
            }
        );

        let (message, _, _) = encrypted_message();
        let mut encoded = encode_message(&message).unwrap();
        let content_type_offset = 5 + 16 + 8 + 4 + b"originator".len() + 16 + 8 + 8;
        let content_type_length = u32::from_be_bytes(
            encoded[content_type_offset..content_type_offset + 4]
                .try_into()
                .unwrap(),
        ) as usize;
        encoded[content_type_offset + 4..content_type_offset + 4 + content_type_length].fill(0xff);
        assert_eq!(
            decode_message(&encoded).err().unwrap(),
            ChannelWireError::InvalidUtf8("content_type")
        );

        let mut oversized = vec![0u8; 5 + 4];
        oversized[..4].copy_from_slice(GRANT_MAGIC);
        oversized[4] = WIRE_VERSION;
        oversized[5..9].copy_from_slice(&((MAX_IDENTITY_BYTES as u32) + 1).to_be_bytes());
        assert_eq!(
            decode_key_grant(&oversized).err().unwrap(),
            ChannelWireError::LengthLimitExceeded {
                field: "originator_id",
                length: (MAX_IDENTITY_BYTES + 1) as u64,
                maximum: MAX_IDENTITY_BYTES as u64,
            }
        );
    }

    #[test]
    fn storage_keys_are_stable_ordered_and_path_safe() {
        let first = message_record_key(channel_id(), Sequence(9));
        let second = message_record_key(channel_id(), Sequence(10));
        assert!(first < second);
        assert!(first.ends_with("/messages/00000000000000000009"));
        assert_eq!(
            sequence_state_record_key(channel_id()),
            format!("{}/state/next-sequence", encode_hex(&channel_id().0))
        );

        let unsafe_receiver = b"../../receiver\nname";
        let grant_key = key_grant_record_key(channel_id(), KeyEpoch(2), unsafe_receiver);
        let ack_key = receiver_ack_record_key(channel_id(), unsafe_receiver);
        assert!(!grant_key.contains(".."));
        assert!(!grant_key.contains('\n'));
        assert!(!ack_key.contains(".."));
        assert!(!ack_key.contains('\n'));
        assert!(grant_key.contains("/grants/00000000000000000002/"));
    }
}
