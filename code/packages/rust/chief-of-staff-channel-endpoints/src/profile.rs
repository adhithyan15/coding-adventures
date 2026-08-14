//! Portable D18P adapters over production channel membership and endpoints.
//!
//! The definition helpers delegate to the shipped D18C version 1 codec. The
//! error classifier preserves the production error types while giving every
//! language fixture one stable machine-readable result.

use chief_of_staff_channel_crypto::ChannelCryptoError;
use chief_of_staff_channel_store::profile::channel_store_error_code;

use super::{
    decode_definition, encode_definition, ChannelDefinition, ChannelEndpointError,
    DEFINITION_CONTENT_TYPE, MAX_CAS_ATTEMPTS, MAX_RECEIVERS,
};

/// D18P content type for an immutable channel definition.
pub const CHANNEL_DEFINITION_CONTENT_TYPE: &str = DEFINITION_CONTENT_TYPE;

/// Maximum receivers accepted by D18C version 1.
pub const MAX_CHANNEL_RECEIVERS: usize = MAX_RECEIVERS;

/// Maximum revision-CAS attempts made by one D18P lifecycle operation.
pub const MAX_DEFINITION_CAS_ATTEMPTS: usize = MAX_CAS_ATTEMPTS;

/// Serialize one canonical definition as production D18C version 1 bytes.
pub fn channel_definition_serialize(definition: &ChannelDefinition) -> Vec<u8> {
    encode_definition(definition)
}

/// Decode and validate production D18C version 1 bytes.
pub fn channel_definition_deserialize(
    bytes: &[u8],
) -> Result<ChannelDefinition, ChannelEndpointError> {
    decode_definition(bytes)
}

/// Return the normative D18P machine-readable code for an endpoint failure.
pub fn channel_endpoint_error_code(error: &ChannelEndpointError) -> &'static str {
    match error {
        ChannelEndpointError::Storage(_) => "storage_error",
        ChannelEndpointError::Store(error) => channel_store_error_code(error),
        ChannelEndpointError::Crypto(ChannelCryptoError::SequenceExhausted) => "sequence_exhausted",
        ChannelEndpointError::Crypto(_) => "crypto_error",
        ChannelEndpointError::Metadata(_) => "metadata_error",
        ChannelEndpointError::InvalidDefinition(_) => "invalid_definition",
        ChannelEndpointError::InvalidMessageId => "invalid_message_id",
        ChannelEndpointError::DefinitionNotFound => "definition_not_found",
        ChannelEndpointError::ConflictingDefinition => "conflicting_definition",
        ChannelEndpointError::CorruptDefinition(_) => "corrupt_definition",
        ChannelEndpointError::DefinitionChanged => "definition_changed",
        ChannelEndpointError::ChannelDestroyed => "channel_destroyed",
        ChannelEndpointError::UnauthorizedOriginator => "unauthorized_originator",
        ChannelEndpointError::UnauthorizedReceiver => "unauthorized_receiver",
        ChannelEndpointError::PublicKeyMismatch => "public_key_mismatch",
        ChannelEndpointError::MissingKeyGrant(_) => "missing_key_grant",
        ChannelEndpointError::UnknownMessageId(_) => "unknown_message_id",
        ChannelEndpointError::UnauthorizedMessage => "unauthorized_message",
        ChannelEndpointError::ConcurrentUpdate => "concurrent_update",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AgentId, ChannelDefinition, ChannelLifecycle, OriginatorIdentity, ReceiverIdentity,
    };
    use chief_of_staff_channel_crypto::{ChannelId, KeyEpoch};

    fn channel_id() -> ChannelId {
        ChannelId([
            0x01, 0x8f, 0x47, 0xa0, 0x9b, 0x6c, 0x7d, 0xef, 0x92, 0x34, 0x56, 0x78, 0x9a, 0xbc,
            0xde, 0xf0,
        ])
    }

    #[test]
    fn portable_definition_helper_is_canonical_and_lossless() {
        let definition = ChannelDefinition::new(
            channel_id(),
            OriginatorIdentity {
                agent_id: AgentId::new(b"originator".to_vec()).unwrap(),
                public_key: [0x11; 32],
            },
            vec![
                ReceiverIdentity {
                    agent_id: AgentId::new(vec![0xff, 0x00]).unwrap(),
                    public_key: [0x33; 32],
                },
                ReceiverIdentity {
                    agent_id: AgentId::new(b"alpha".to_vec()).unwrap(),
                    public_key: [0x22; 32],
                },
            ],
            1_725_000_000_000_000_000,
            KeyEpoch(7),
        )
        .unwrap();
        assert_eq!(definition.receivers()[0].agent_id.as_bytes(), b"alpha");
        let encoded = channel_definition_serialize(&definition);
        assert_eq!(&encoded[..5], b"D18C\x01");
        assert_eq!(
            channel_definition_deserialize(&encoded).unwrap(),
            definition
        );

        let mut destroyed = encoded;
        *destroyed.last_mut().unwrap() = 1;
        let destroyed = channel_definition_deserialize(&destroyed).unwrap();
        assert_eq!(destroyed.lifecycle(), ChannelLifecycle::Destroyed);
    }

    #[test]
    fn portable_definition_failures_use_stable_codes() {
        let error = channel_definition_deserialize(b"not definition").unwrap_err();
        assert_eq!(channel_endpoint_error_code(&error), "corrupt_definition");
        assert_eq!(
            channel_endpoint_error_code(&ChannelEndpointError::UnauthorizedReceiver),
            "unauthorized_receiver"
        );
        assert_eq!(
            channel_endpoint_error_code(&ChannelEndpointError::Crypto(
                ChannelCryptoError::SequenceExhausted
            )),
            "sequence_exhausted"
        );
    }
}
