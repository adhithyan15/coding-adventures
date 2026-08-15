//! Generate the deterministic shared D18T version 1 fixture manifest.

use std::env;
use std::fs;
use std::path::Path;

use chief_of_staff_channel_crypto::grant_profile::{plan_rotation, RotationReceiver};
use chief_of_staff_channel_crypto::{
    prepare_message_header, ChannelId, ChannelMasterKey, KeyEpoch, MessageFields,
    OriginatorSigningKey, ReceiverKeyPair, Sequence,
};
use chief_of_staff_channel_endpoints::{
    AgentId, ChannelDefinition, OriginatorIdentity, ReceiverIdentity,
};
use chief_of_staff_channel_epoch_activation::{
    activation_plan_record_key, epoch_state_serialize, prepare_rotation_candidate, EpochState,
    ACTIVATION_PLAN_CONTENT_TYPE, EPOCH_STATE_CONTENT_TYPE,
};
use chief_of_staff_channel_store::profile::channel_state_serialize;
use chief_of_staff_channel_store::ChannelState;
use coding_adventures_json_serializer::{serialize_pretty, SerializerConfig};
use coding_adventures_json_value::JsonValue;

const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const CHANNEL_ID: [u8; 16] = [
    0x01, 0x8f, 0x47, 0xa0, 0x9b, 0x6c, 0x7d, 0xef, 0x92, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
];
const CURRENT_CMK: [u8; 32] = [0x22; 32];
const NEXT_CMK: [u8; 32] = [0x33; 32];
const SIGNING_SEED: [u8; 32] = [0x11; 32];
const RECEIVER_A_PRIVATE: [u8; 32] = [0x41; 32];
const RECEIVER_B_PRIVATE: [u8; 32] = [0x42; 32];

#[allow(dead_code)]
fn main() {
    let mut arguments = env::args().skip(1);
    let output = arguments
        .next()
        .expect("usage: generate_d18t_fixtures OUTPUT GENERATOR_BLOB_SHA1");
    let generator_blob_sha1 = arguments
        .next()
        .expect("usage: generate_d18t_fixtures OUTPUT GENERATOR_BLOB_SHA1");
    assert!(arguments.next().is_none(), "unexpected extra argument");
    let encoded = generate_manifest(&generator_blob_sha1);
    if let Some(parent) = Path::new(&output).parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(output, encoded).unwrap();
}

/// Generate the complete canonical D18T fixture manifest.
pub fn generate_manifest(generator_blob_sha1: &str) -> Vec<u8> {
    assert_eq!(generator_blob_sha1.len(), 40);
    let signer = OriginatorSigningKey::from_seed(SIGNING_SEED);
    let receiver_a_key = ReceiverKeyPair::from_private_key(RECEIVER_A_PRIVATE).unwrap();
    let receiver_b_key = ReceiverKeyPair::from_private_key(RECEIVER_B_PRIVATE).unwrap();
    let receiver_a = ReceiverIdentity {
        agent_id: AgentId::new(b"receiver-a".to_vec()).unwrap(),
        public_key: receiver_a_key.public_key(),
    };
    let receiver_b = ReceiverIdentity {
        agent_id: AgentId::new(b"receiver-b".to_vec()).unwrap(),
        public_key: receiver_b_key.public_key(),
    };
    let definition = ChannelDefinition::new(
        ChannelId(CHANNEL_ID),
        OriginatorIdentity {
            agent_id: AgentId::new(b"originator".to_vec()).unwrap(),
            public_key: signer.public_key(),
        },
        vec![receiver_a, receiver_b.clone()],
        1_725_000_000_000_000_000,
        KeyEpoch(0),
    )
    .unwrap();
    let pending = prepare_message_header(
        MessageFields::new(
            message_id(7),
            1_725_000_000_000_000_007,
            b"originator".to_vec(),
            ChannelId(CHANNEL_ID),
            Sequence(7),
            KeyEpoch(0),
            "application/octet-stream".to_owned(),
        ),
        b"pending-before-activation",
    );
    let no_pending_v1 = channel_state_serialize(&ChannelState {
        next_sequence: Sequence(0),
        pending_header: None,
    })
    .unwrap();
    let no_pending_v2 = epoch_state_serialize(
        &EpochState::new(ChannelId(CHANNEL_ID), KeyEpoch(0), Sequence(0), None).unwrap(),
    )
    .unwrap();
    let pending_v1 = channel_state_serialize(&ChannelState {
        next_sequence: Sequence(8),
        pending_header: Some(pending.clone()),
    })
    .unwrap();
    let pending_v2 = epoch_state_serialize(
        &EpochState::new(
            ChannelId(CHANNEL_ID),
            KeyEpoch(0),
            Sequence(8),
            Some(pending),
        )
        .unwrap(),
    )
    .unwrap();
    let rotation = plan_rotation(
        b"originator",
        ChannelId(CHANNEL_ID),
        KeyEpoch(0),
        ChannelMasterKey::from_bytes(NEXT_CMK),
        vec![RotationReceiver::with_material(
            b"receiver-b".to_vec(),
            receiver_b.public_key,
            [0x51; 32],
            [0x61; 24],
        )
        .unwrap()],
        &signer,
    )
    .unwrap();
    let prepared = prepare_rotation_candidate(
        &definition,
        KeyEpoch(0),
        std::slice::from_ref(&receiver_b),
        rotation,
    )
    .unwrap();
    let public = prepared.public();

    let manifest = JsonValue::Object(vec![
        (
            "fixture_format".into(),
            string("D18T-durable-epoch-activation-fixtures-v1"),
        ),
        (
            "spec".into(),
            string("code/specs/D18T-chief-of-staff-durable-epoch-activation-profile.md"),
        ),
        (
            "generator_blob_sha1".into(),
            string(generator_blob_sha1),
        ),
        (
            "warning".into(),
            string("The test_only_secrets object contains deterministic conformance-only secrets. Never log or use them outside tests."),
        ),
        (
            "constants".into(),
            object(vec![
                ("state_magic_ascii", string("D18S")),
                ("state_version", string("2")),
                ("plan_magic_ascii", string("D18T")),
                ("plan_version", string("1")),
                ("state_content_type", string(EPOCH_STATE_CONTENT_TYPE)),
                ("plan_content_type", string(ACTIVATION_PLAN_CONTENT_TYPE)),
                ("max_cas_attempts", string("16")),
            ]),
        ),
        (
            "test_only_secrets".into(),
            object(vec![
                ("current_cmk_hex", string(encode_hex(&CURRENT_CMK))),
                ("next_cmk_hex", string(encode_hex(&NEXT_CMK))),
                ("originator_signing_seed_hex", string(encode_hex(&SIGNING_SEED))),
                ("receiver_a_private_key_hex", string(encode_hex(&RECEIVER_A_PRIVATE))),
                ("receiver_b_private_key_hex", string(encode_hex(&RECEIVER_B_PRIVATE))),
                ("ephemeral_private_key_hex", string(encode_hex(&[0x51; 32]))),
                ("wrapping_nonce_hex", string(encode_hex(&[0x61; 24]))),
            ]),
        ),
        (
            "state_migrations".into(),
            JsonValue::Array(vec![
                migration("no-pending", &no_pending_v1, &no_pending_v2, "0", "0"),
                migration("pending-d18h", &pending_v1, &pending_v2, "0", "8"),
            ]),
        ),
        (
            "activation_case".into(),
            object(vec![
                ("name", string("receivers-a-plus-b-to-b-only")),
                ("base_epoch", string("0")),
                ("new_epoch", string("1")),
                (
                    "plan_record_key",
                    string(activation_plan_record_key(ChannelId(CHANNEL_ID), KeyEpoch(1))),
                ),
                ("plan_content_type", string(ACTIVATION_PLAN_CONTENT_TYPE)),
                ("plan_b64", string(encode_base64(public.plan_bytes()))),
                (
                    "grant_b64",
                    JsonValue::Array(
                        public
                            .grants()
                            .iter()
                            .map(|grant| string(encode_base64(grant)))
                            .collect(),
                    ),
                ),
                (
                    "receiver_a_retains_epochs",
                    JsonValue::Array(vec![string("0")]),
                ),
                (
                    "receiver_b_retains_epochs",
                    JsonValue::Array(vec![string("0"), string("1")]),
                ),
                ("receiver_a_new_grant", JsonValue::Null),
            ]),
        ),
        (
            "crash_replay_traces".into(),
            JsonValue::Array(vec![
                trace("after-custody-selection", "replay-plan-and-all-grants", "prepared"),
                trace("after-plan-write", "replay-all-grants", "prepared"),
                trace("after-first-grant", "replay-remaining-grants", "prepared"),
                trace("after-all-grants", "verify-and-activate", "activated"),
                trace("after-activation-cas", "verify-exact-plan", "idempotent"),
            ]),
        ),
        (
            "race_traces".into(),
            JsonValue::Array(vec![
                trace("publish-reservation-wins", "activation", "pending_append"),
                trace("activation-wins", "next-publish", "epoch-1-sequence-preserved"),
                trace("same-candidate-retry", "custody-selection", "idempotent"),
                trace("different-candidate-loses", "custody-selection", "conflicting_preparation"),
            ]),
        ),
        (
            "stable_error_codes".into(),
            JsonValue::Array(
                [
                    "not_initialized", "channel_destroyed", "invalid_plan", "corrupt_record",
                    "pending_append", "unactivated_epoch", "active_key_missing",
                    "conflicting_active_key", "preparation_missing", "conflicting_preparation",
                    "conflicting_plan", "conflicting_grant", "unexpected_epoch",
                    "decreasing_epoch", "epoch_exhausted", "concurrent_update", "storage_error",
                    "custody_error", "crypto_error",
                ]
                .into_iter()
                .map(string)
                .collect(),
            ),
        ),
        (
            "negative_scenarios".into(),
            JsonValue::Array(vec![
                trace("pending-append", "activation", "pending_append"),
                trace("corrupt-public-record", "recovery", "corrupt_record"),
                trace("missing-custody", "activation", "preparation_missing"),
                trace("destroyed-channel", "activation", "channel_destroyed"),
                trace("epoch-exhaustion", "preparation", "epoch_exhausted"),
                trace("sixteen-cas-conflicts", "activation", "concurrent_update"),
            ]),
        ),
        ("secret_erasure_capability".into(), string("guaranteed")),
    ]);
    let mut encoded = serialize_pretty(&manifest, &SerializerConfig::default())
        .unwrap()
        .into_bytes();
    encoded.push(b'\n');
    encoded
}

fn migration(name: &str, v1: &[u8], v2: &[u8], epoch: &str, sequence: &str) -> JsonValue {
    object(vec![
        ("name", string(name)),
        ("d18s_v1_b64", string(encode_base64(v1))),
        ("d18s_v2_b64", string(encode_base64(v2))),
        ("active_epoch", string(epoch)),
        ("next_sequence", string(sequence)),
    ])
}

fn trace(name: &str, operation: &str, expected: &str) -> JsonValue {
    object(vec![
        ("name", string(name)),
        ("operation", string(operation)),
        ("expected", string(expected)),
    ])
}

fn object(fields: Vec<(&str, JsonValue)>) -> JsonValue {
    JsonValue::Object(
        fields
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
}

fn string(value: impl Into<String>) -> JsonValue {
    JsonValue::String(value.into())
}

fn message_id(byte: u8) -> [u8; 16] {
    let mut bytes = [byte; 16];
    bytes[6] = 0x70 | (byte & 0x0f);
    bytes[8] = 0x80 | (byte & 0x3f);
    bytes
}

fn encode_base64(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let word = ((chunk[0] as u32) << 16)
            | ((chunk.get(1).copied().unwrap_or(0) as u32) << 8)
            | chunk.get(2).copied().unwrap_or(0) as u32;
        output.push(BASE64[((word >> 18) & 0x3f) as usize] as char);
        output.push(BASE64[((word >> 12) & 0x3f) as usize] as char);
        output.push(if chunk.len() > 1 {
            BASE64[((word >> 6) & 0x3f) as usize] as char
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            BASE64[(word & 0x3f) as usize] as char
        } else {
            '='
        });
    }
    output
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}
