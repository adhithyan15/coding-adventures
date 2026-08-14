//! Generate the deterministic shared D18P version 1 fixture manifest.

use std::collections::VecDeque;
use std::env;
use std::fs;
use std::path::Path;
use std::sync::Mutex;

use chief_of_staff_channel_crypto::profile::message_serialize;
use chief_of_staff_channel_crypto::wire::{
    channel_definition_record_key, encode_message_header, key_grant_record_key, message_record_key,
    message_record_prefix, receiver_ack_record_key, sequence_state_record_key,
    CHANNEL_STORAGE_NAMESPACE,
};
use chief_of_staff_channel_crypto::{
    prepare_message_header, ChannelId, ChannelMasterKey, KeyEpoch, MessageFields,
    OriginatorSigningKey, ReceiverKeyPair, Sequence,
};
use chief_of_staff_channel_endpoints::profile::{
    channel_definition_deserialize, channel_definition_serialize, channel_endpoint_error_code,
    CHANNEL_DEFINITION_CONTENT_TYPE, MAX_CHANNEL_RECEIVERS, MAX_DEFINITION_CAS_ATTEMPTS,
};
use chief_of_staff_channel_endpoints::{
    AgentId, ChannelDefinition, ChannelDefinitionStore, DurableOriginator, DurableReceiver,
    MessageId, MessageMetadata, MessageMetadataError, MessageMetadataSource, Originator,
    OriginatorIdentity, Receiver, ReceiverIdentity,
};
use chief_of_staff_channel_store::profile::{
    channel_state_serialize, channel_store_error_code, receiver_cursor_serialize,
    CHANNEL_ACK_CONTENT_TYPE, CHANNEL_GRANT_CONTENT_TYPE, CHANNEL_MESSAGE_CONTENT_TYPE,
    CHANNEL_STATE_CONTENT_TYPE, MAX_CHANNEL_CAS_ATTEMPTS, MAX_PENDING_HEADER_BYTES,
};
use chief_of_staff_channel_store::{AppendRequest, ChannelState, ChannelStore};
use coding_adventures_json_serializer::serialize;
use coding_adventures_json_value::JsonValue;
use storage_core::{InMemoryStorageBackend, StorageBackend, StoragePutInput};

const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const CHANNEL_ID: [u8; 16] = [
    0x01, 0x8f, 0x47, 0xa0, 0x9b, 0x6c, 0x7d, 0xef, 0x92, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
];
const ORIGINATOR_ID: &[u8] = b"fixture-originator";
const BINARY_RECEIVER_ID: &[u8] = &[0x00, 0xff, 0x01];
const TEXT_RECEIVER_ID: &[u8] = b"zed";
const SIGNING_SEED: [u8; 32] = [0x11; 32];
const CHANNEL_MASTER_KEY: [u8; 32] = [0x22; 32];
const BINARY_RECEIVER_PRIVATE_KEY: [u8; 32] = [0x44; 32];
const TEXT_RECEIVER_PRIVATE_KEY: [u8; 32] = [0x55; 32];

struct FixedMetadataSource {
    values: Mutex<VecDeque<MessageMetadata>>,
}

impl FixedMetadataSource {
    fn new(values: Vec<MessageMetadata>) -> Self {
        Self {
            values: Mutex::new(values.into()),
        }
    }
}

impl MessageMetadataSource for FixedMetadataSource {
    fn next_metadata(&self) -> Result<MessageMetadata, MessageMetadataError> {
        self.values
            .lock()
            .expect("fixture metadata mutex poisoned")
            .pop_front()
            .ok_or_else(|| MessageMetadataError::new("fixture metadata exhausted"))
    }
}

#[allow(dead_code)]
fn main() {
    let mut arguments = env::args().skip(1);
    let output = arguments
        .next()
        .expect("usage: generate_d18p_fixtures OUTPUT GENERATOR_BLOB_SHA1");
    let generator_blob_sha1 = arguments
        .next()
        .expect("usage: generate_d18p_fixtures OUTPUT GENERATOR_BLOB_SHA1");
    assert!(arguments.next().is_none(), "unexpected extra argument");
    let encoded = generate_manifest(&generator_blob_sha1);
    if let Some(parent) = Path::new(&output).parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(output, encoded).unwrap();
}

/// Generate the complete canonical fixture manifest.
pub fn generate_manifest(generator_blob_sha1: &str) -> Vec<u8> {
    assert_eq!(generator_blob_sha1.len(), 40);
    let definition = fixture_definition(1_725_000_000_000_000_000);
    let active_definition = channel_definition_serialize(&definition);
    let mut destroyed_definition = active_definition.clone();
    *destroyed_definition.last_mut().unwrap() = 1;
    let destroyed = channel_definition_deserialize(&destroyed_definition).unwrap();

    let pending_header = prepare_message_header(
        MessageFields::new(
            uuid_v7(0x70),
            9_000_000_007,
            ORIGINATOR_ID.to_vec(),
            ChannelId(CHANNEL_ID),
            Sequence(7),
            KeyEpoch(3),
            "application/octet-stream".to_owned(),
        ),
        b"fixture pending payload",
    );
    let initial_state = ChannelState {
        next_sequence: Sequence(0),
        pending_header: None,
    };
    let pending_state = ChannelState {
        next_sequence: Sequence(8),
        pending_header: Some(pending_header.clone()),
    };
    let initial_state_bytes = channel_state_serialize(&initial_state).unwrap();
    let pending_state_bytes = channel_state_serialize(&pending_state).unwrap();
    let pending_header_bytes = encode_message_header(&pending_header).unwrap();

    let (positive_operations, negative_operations) = operation_cases();
    let manifest = JsonValue::Object(vec![
        (
            "fixture_format".into(),
            string("D18P-durable-channel-fixtures-v1"),
        ),
        (
            "spec".into(),
            string("code/specs/D18P-chief-of-staff-durable-channel-profile.md"),
        ),
        (
            "generator_blob_sha1".into(),
            string(generator_blob_sha1),
        ),
        (
            "warning".into(),
            string("All private keys and channel master keys are deterministic test-only material. Never use them outside conformance tests."),
        ),
        ("constants".into(), constants()),
        ("test_keys".into(), test_keys()),
        (
            "definition_cases".into(),
            JsonValue::Array(vec![
                JsonValue::Object(vec![
                    ("name".into(), string("active-binary-sorted-receivers")),
                    ("lifecycle".into(), string("active")),
                    (
                        "canonical_receiver_ids_b64".into(),
                        JsonValue::Array(
                            definition
                                .receivers()
                                .iter()
                                .map(|receiver| string(encode_base64(receiver.agent_id.as_bytes())))
                                .collect(),
                        ),
                    ),
                    (
                        "d18c_b64".into(),
                        string(encode_base64(&active_definition)),
                    ),
                ]),
                JsonValue::Object(vec![
                    ("name".into(), string("destroyed")),
                    ("lifecycle".into(), string("destroyed")),
                    (
                        "d18c_b64".into(),
                        string(encode_base64(&channel_definition_serialize(&destroyed))),
                    ),
                ]),
            ]),
        ),
        (
            "state_cases".into(),
            JsonValue::Array(vec![
                JsonValue::Object(vec![
                    ("name".into(), string("initial")),
                    ("next_sequence".into(), string("0")),
                    ("pending".into(), JsonValue::Bool(false)),
                    (
                        "d18s_b64".into(),
                        string(encode_base64(&initial_state_bytes)),
                    ),
                ]),
                JsonValue::Object(vec![
                    ("name".into(), string("pending")),
                    ("next_sequence".into(), string("8")),
                    ("pending".into(), JsonValue::Bool(true)),
                    (
                        "d18h_b64".into(),
                        string(encode_base64(&pending_header_bytes)),
                    ),
                    (
                        "d18s_b64".into(),
                        string(encode_base64(&pending_state_bytes)),
                    ),
                ]),
            ]),
        ),
        (
            "cursor_cases".into(),
            JsonValue::Array(
                [0u64, 1, 42, u64::MAX]
                    .into_iter()
                    .map(|cursor| {
                        JsonValue::Object(vec![
                            ("first_unread_sequence".into(), string(cursor.to_string())),
                            (
                                "d18a_b64".into(),
                                string(encode_base64(&receiver_cursor_serialize(Sequence(cursor)))),
                            ),
                        ])
                    })
                    .collect(),
            ),
        ),
        ("storage_key_cases".into(), storage_key_cases()),
        (
            "codec_negative_cases".into(),
            codec_negative_cases(
                &active_definition,
                &pending_state_bytes,
                &receiver_cursor_serialize(Sequence(42)),
            ),
        ),
        (
            "operation_cases".into(),
            JsonValue::Array(positive_operations),
        ),
        (
            "operation_negative_cases".into(),
            JsonValue::Array(negative_operations),
        ),
        (
            "stable_error_codes".into(),
            JsonValue::Array(
                STABLE_ERROR_CODES
                    .iter()
                    .map(|code| string(*code))
                    .collect(),
            ),
        ),
        (
            "oversize_recipes".into(),
            JsonValue::Array(vec![
                recipe("agent-id", "4097", "invalid_definition"),
                recipe("receiver-count", "1025", "invalid_definition"),
                recipe("pending-header", "16385", "corrupt_record"),
            ]),
        ),
    ]);
    let mut encoded = serialize(&manifest).unwrap();
    encoded.push('\n');
    encoded.into_bytes()
}

const STABLE_ERROR_CODES: &[&str] = &[
    "invalid_definition",
    "invalid_message_id",
    "definition_not_found",
    "conflicting_definition",
    "corrupt_definition",
    "definition_changed",
    "channel_destroyed",
    "unauthorized_originator",
    "unauthorized_receiver",
    "public_key_mismatch",
    "missing_key_grant",
    "unknown_message_id",
    "unauthorized_message",
    "not_initialized",
    "corrupt_record",
    "pending_append",
    "no_pending_append",
    "pending_header_mismatch",
    "conflicting_record",
    "concurrent_update",
    "invalid_receiver_id",
    "invalid_page_size",
    "acknowledgement_regression",
    "acknowledgement_ahead",
    "acknowledgement_pending",
    "sequence_exhausted",
    "storage_error",
    "wire_error",
    "crypto_error",
    "metadata_error",
];

fn constants() -> JsonValue {
    JsonValue::Object(vec![
        (
            "storage_namespace".into(),
            string(CHANNEL_STORAGE_NAMESPACE),
        ),
        (
            "content_types".into(),
            JsonValue::Object(vec![
                ("definition".into(), string(CHANNEL_DEFINITION_CONTENT_TYPE)),
                ("state".into(), string(CHANNEL_STATE_CONTENT_TYPE)),
                ("message".into(), string(CHANNEL_MESSAGE_CONTENT_TYPE)),
                ("grant".into(), string(CHANNEL_GRANT_CONTENT_TYPE)),
                ("ack".into(), string(CHANNEL_ACK_CONTENT_TYPE)),
            ]),
        ),
        (
            "max_receivers".into(),
            string(MAX_CHANNEL_RECEIVERS.to_string()),
        ),
        (
            "max_pending_header_bytes".into(),
            string(MAX_PENDING_HEADER_BYTES.to_string()),
        ),
        (
            "max_store_cas_attempts".into(),
            string(MAX_CHANNEL_CAS_ATTEMPTS.to_string()),
        ),
        (
            "max_definition_cas_attempts".into(),
            string(MAX_DEFINITION_CAS_ATTEMPTS.to_string()),
        ),
    ])
}

fn test_keys() -> JsonValue {
    let signing_key = OriginatorSigningKey::from_seed(SIGNING_SEED);
    let binary_receiver = ReceiverKeyPair::from_private_key(BINARY_RECEIVER_PRIVATE_KEY).unwrap();
    let text_receiver = ReceiverKeyPair::from_private_key(TEXT_RECEIVER_PRIVATE_KEY).unwrap();
    JsonValue::Object(vec![
        (
            "originator_signing_seed_hex".into(),
            string(encode_hex(&SIGNING_SEED)),
        ),
        (
            "originator_public_key_hex".into(),
            string(encode_hex(&signing_key.public_key())),
        ),
        (
            "channel_master_key_hex".into(),
            string(encode_hex(&CHANNEL_MASTER_KEY)),
        ),
        (
            "binary_receiver_private_key_hex".into(),
            string(encode_hex(&BINARY_RECEIVER_PRIVATE_KEY)),
        ),
        (
            "binary_receiver_public_key_hex".into(),
            string(encode_hex(&binary_receiver.public_key())),
        ),
        (
            "text_receiver_private_key_hex".into(),
            string(encode_hex(&TEXT_RECEIVER_PRIVATE_KEY)),
        ),
        (
            "text_receiver_public_key_hex".into(),
            string(encode_hex(&text_receiver.public_key())),
        ),
    ])
}

fn storage_key_cases() -> JsonValue {
    let channel_id = ChannelId(CHANNEL_ID);
    JsonValue::Array(vec![
        key_case("definition", channel_definition_record_key(channel_id)),
        key_case("state", sequence_state_record_key(channel_id)),
        key_case("message-zero", message_record_key(channel_id, Sequence(0))),
        key_case(
            "message-max",
            message_record_key(channel_id, Sequence(u64::MAX)),
        ),
        key_case("message-prefix", message_record_prefix(channel_id)),
        key_case(
            "grant",
            key_grant_record_key(channel_id, KeyEpoch(7), BINARY_RECEIVER_ID),
        ),
        key_case(
            "ack-binary-receiver",
            receiver_ack_record_key(channel_id, BINARY_RECEIVER_ID),
        ),
    ])
}

fn codec_negative_cases(definition: &[u8], pending_state: &[u8], cursor: &[u8]) -> JsonValue {
    let mut cases = Vec::new();
    cases.push(codec_mutation(
        "definition-invalid-magic",
        "definition",
        definition,
        0,
        b'X',
        "corrupt_definition",
    ));
    cases.push(codec_mutation(
        "definition-unsupported-version",
        "definition",
        definition,
        4,
        2,
        "corrupt_definition",
    ));
    cases.push(codec_record(
        "definition-truncated",
        "definition",
        &definition[..definition.len() - 1],
        "corrupt_definition",
    ));
    let mut definition_trailing = definition.to_vec();
    definition_trailing.push(0);
    cases.push(codec_record(
        "definition-trailing",
        "definition",
        &definition_trailing,
        "corrupt_definition",
    ));
    cases.push(codec_mutation(
        "definition-invalid-channel-uuid",
        "definition",
        definition,
        11,
        0x40,
        "corrupt_definition",
    ));
    let originator_length = read_u32(definition, 21) as usize;
    let receiver_count = 25 + originator_length + 32;
    let mut zero_receivers = definition.to_vec();
    zero_receivers[receiver_count..receiver_count + 4].copy_from_slice(&0u32.to_be_bytes());
    cases.push(codec_record(
        "definition-zero-receivers",
        "definition",
        &zero_receivers,
        "corrupt_definition",
    ));
    cases.push(codec_mutation(
        "definition-invalid-lifecycle",
        "definition",
        definition,
        definition.len() - 1,
        2,
        "corrupt_definition",
    ));

    cases.push(codec_mutation(
        "state-invalid-magic",
        "state",
        pending_state,
        0,
        b'X',
        "corrupt_record",
    ));
    cases.push(codec_mutation(
        "state-unsupported-version",
        "state",
        pending_state,
        4,
        2,
        "corrupt_record",
    ));
    cases.push(codec_record(
        "state-truncated",
        "state",
        &pending_state[..pending_state.len() - 1],
        "corrupt_record",
    ));
    let mut state_trailing = pending_state.to_vec();
    state_trailing.push(0);
    cases.push(codec_record(
        "state-trailing",
        "state",
        &state_trailing,
        "corrupt_record",
    ));
    cases.push(codec_mutation(
        "state-invalid-pending-flag",
        "state",
        pending_state,
        13,
        2,
        "corrupt_record",
    ));
    let mut oversized_header = pending_state[..18].to_vec();
    oversized_header[14..18]
        .copy_from_slice(&((MAX_PENDING_HEADER_BYTES + 1) as u32).to_be_bytes());
    cases.push(codec_record(
        "state-oversized-header",
        "state",
        &oversized_header,
        "corrupt_record",
    ));
    let mut wrong_next = pending_state.to_vec();
    wrong_next[5..13].copy_from_slice(&9u64.to_be_bytes());
    cases.push(codec_record(
        "state-sequence-invariant",
        "state",
        &wrong_next,
        "corrupt_record",
    ));
    let originator_length = read_u32(pending_state, 18 + 29) as usize;
    let embedded_channel = 18 + 33 + originator_length;
    cases.push(codec_mutation(
        "state-channel-invariant",
        "state",
        pending_state,
        embedded_channel,
        0xff,
        "corrupt_record",
    ));

    cases.push(codec_mutation(
        "cursor-invalid-magic",
        "cursor",
        cursor,
        0,
        b'X',
        "corrupt_record",
    ));
    cases.push(codec_mutation(
        "cursor-unsupported-version",
        "cursor",
        cursor,
        4,
        2,
        "corrupt_record",
    ));
    cases.push(codec_record(
        "cursor-truncated",
        "cursor",
        &cursor[..cursor.len() - 1],
        "corrupt_record",
    ));
    let mut cursor_trailing = cursor.to_vec();
    cursor_trailing.push(0);
    cases.push(codec_record(
        "cursor-trailing",
        "cursor",
        &cursor_trailing,
        "corrupt_record",
    ));
    JsonValue::Array(cases)
}

fn operation_cases() -> (Vec<JsonValue>, Vec<JsonValue>) {
    let mut positives = Vec::new();
    let mut negatives = Vec::new();
    definition_and_endpoint_operations(&mut positives, &mut negatives);
    store_operations(&mut positives, &mut negatives);
    storage_corruption_cases(&mut negatives);
    (positives, negatives)
}

fn definition_and_endpoint_operations(
    positives: &mut Vec<JsonValue>,
    negatives: &mut Vec<JsonValue>,
) {
    let backend = InMemoryStorageBackend::new();
    let definition = fixture_definition(1_725_000_000_000_000_000);
    let definitions = ChannelDefinitionStore::new(&backend);
    let created = definitions.create(&definition).unwrap();
    let retried = definitions.create(&definition).unwrap();
    positives.push(operation_case(
        "definition-create-idempotent",
        vec![
            ("definitions_equal", JsonValue::Bool(created == retried)),
            ("initial_next_sequence", string("0")),
        ],
    ));
    let conflict = definitions
        .create(&fixture_definition(1_725_000_000_000_000_001))
        .unwrap_err();
    negatives.push(endpoint_error_case(
        "conflicting-definition",
        "create",
        &conflict,
    ));

    let signing_key = OriginatorSigningKey::from_seed(SIGNING_SEED);
    let cmk = ChannelMasterKey::from_bytes(CHANNEL_MASTER_KEY);
    let metadata = FixedMetadataSource::new(vec![
        metadata(1, 10_000_000_001),
        metadata(2, 10_000_000_002),
        metadata(3, 10_000_000_003),
    ]);
    let originator_id = AgentId::new(ORIGINATOR_ID.to_vec()).unwrap();
    let originator = DurableOriginator::open(
        &backend,
        ChannelId(CHANNEL_ID),
        &originator_id,
        &signing_key,
        &cmk,
        &metadata,
    )
    .unwrap();

    let binary_id = AgentId::new(BINARY_RECEIVER_ID.to_vec()).unwrap();
    let text_id = AgentId::new(TEXT_RECEIVER_ID.to_vec()).unwrap();
    originator.grant_receiver(&binary_id).unwrap();
    originator.grant_receiver(&text_id).unwrap();
    let first = originator.publish(b"message zero", "text/plain").unwrap();
    let second = originator
        .publish(b"message one", "application/octet-stream")
        .unwrap();

    let mut binary_receiver = DurableReceiver::open(
        &backend,
        ChannelId(CHANNEL_ID),
        binary_id.clone(),
        ReceiverKeyPair::from_private_key(BINARY_RECEIVER_PRIVATE_KEY).unwrap(),
    )
    .unwrap();
    let delivered_zero = binary_receiver.receive(1).unwrap();
    let binary_after_zero = binary_receiver
        .acknowledge(delivered_zero[0].message_id)
        .unwrap();
    let delivered_one = binary_receiver.receive(10).unwrap();
    let binary_after_one = binary_receiver
        .acknowledge(delivered_one[0].message_id)
        .unwrap();
    let binary_after_one_retry = binary_receiver
        .acknowledge(delivered_one[0].message_id)
        .unwrap();
    let binary_empty = binary_receiver.receive(10).unwrap();

    let mut text_receiver = DurableReceiver::open(
        &backend,
        ChannelId(CHANNEL_ID),
        text_id.clone(),
        ReceiverKeyPair::from_private_key(TEXT_RECEIVER_PRIVATE_KEY).unwrap(),
    )
    .unwrap();
    let text_delivered = text_receiver.receive(10).unwrap();
    let text_after_zero = text_receiver
        .acknowledge(text_delivered[0].message_id)
        .unwrap();
    positives.push(operation_case(
        "encrypted-endpoint-round-trip-independent-cursors",
        vec![
            (
                "published_sequences",
                sequences([first.sequence.0, second.sequence.0]),
            ),
            (
                "binary_receiver_delivered_sequences",
                sequences([delivered_zero[0].sequence.0, delivered_one[0].sequence.0]),
            ),
            (
                "text_receiver_delivered_sequences",
                sequences(text_delivered.iter().map(|message| message.sequence.0)),
            ),
            (
                "binary_first_unread_after_zero",
                string(binary_after_zero.0.to_string()),
            ),
            (
                "binary_first_unread_after_one",
                string(binary_after_one.0.to_string()),
            ),
            (
                "binary_first_unread_after_retry",
                string(binary_after_one_retry.0.to_string()),
            ),
            (
                "binary_empty_continuation",
                JsonValue::Bool(binary_empty.is_empty()),
            ),
            (
                "text_first_unread_after_zero",
                string(text_after_zero.0.to_string()),
            ),
        ],
    ));

    let mut fresh_binary_receiver = DurableReceiver::open(
        &backend,
        ChannelId(CHANNEL_ID),
        binary_id.clone(),
        ReceiverKeyPair::from_private_key(BINARY_RECEIVER_PRIVATE_KEY).unwrap(),
    )
    .unwrap();
    let unknown = fresh_binary_receiver
        .acknowledge(first.message_id)
        .unwrap_err();
    negatives.push(endpoint_error_case(
        "session-delivery-enforcement",
        "acknowledge",
        &unknown,
    ));
    let wrong_originator = DurableOriginator::open(
        &backend,
        ChannelId(CHANNEL_ID),
        &AgentId::new(b"intruder".to_vec()).unwrap(),
        &signing_key,
        &cmk,
        &metadata,
    )
    .err()
    .unwrap();
    negatives.push(endpoint_error_case(
        "unauthorized-originator",
        "open-originator",
        &wrong_originator,
    ));
    let unknown_receiver = DurableReceiver::open(
        &backend,
        ChannelId(CHANNEL_ID),
        AgentId::new(b"intruder".to_vec()).unwrap(),
        ReceiverKeyPair::from_private_key([0x77; 32]).unwrap(),
    )
    .err()
    .unwrap();
    negatives.push(endpoint_error_case(
        "unauthorized-receiver",
        "open-receiver",
        &unknown_receiver,
    ));
    let wrong_key = DurableReceiver::open(
        &backend,
        ChannelId(CHANNEL_ID),
        binary_id,
        ReceiverKeyPair::from_private_key([0x77; 32]).unwrap(),
    )
    .err()
    .unwrap();
    negatives.push(endpoint_error_case(
        "receiver-public-key-mismatch",
        "open-receiver",
        &wrong_key,
    ));

    let first_destroy = definitions.destroy(ChannelId(CHANNEL_ID)).unwrap();
    let second_destroy = definitions.destroy(ChannelId(CHANNEL_ID)).unwrap();
    let history_count = ChannelStore::new(&backend, ChannelId(CHANNEL_ID))
        .read_messages(Sequence(0), 10)
        .unwrap()
        .messages
        .len();
    positives.push(operation_case(
        "destroy-idempotent-history-preserved",
        vec![
            (
                "definitions_equal",
                JsonValue::Bool(first_destroy == second_destroy),
            ),
            ("history_count", string(history_count.to_string())),
        ],
    ));
    let destroyed_error = originator.publish(b"denied", "text/plain").unwrap_err();
    negatives.push(endpoint_error_case(
        "channel-destroyed",
        "publish",
        &destroyed_error,
    ));

    missing_grant_case(negatives);
}

fn missing_grant_case(negatives: &mut Vec<JsonValue>) {
    let backend = InMemoryStorageBackend::new();
    let definition = fixture_definition(1_725_000_000_000_000_000);
    ChannelDefinitionStore::new(&backend)
        .create(&definition)
        .unwrap();
    let signing_key = OriginatorSigningKey::from_seed(SIGNING_SEED);
    let cmk = ChannelMasterKey::from_bytes(CHANNEL_MASTER_KEY);
    let metadata = FixedMetadataSource::new(vec![metadata(9, 10_000_000_009)]);
    let originator = DurableOriginator::open(
        &backend,
        ChannelId(CHANNEL_ID),
        &AgentId::new(ORIGINATOR_ID.to_vec()).unwrap(),
        &signing_key,
        &cmk,
        &metadata,
    )
    .unwrap();
    originator.publish(b"no grant", "text/plain").unwrap();
    let mut receiver = DurableReceiver::open(
        &backend,
        ChannelId(CHANNEL_ID),
        AgentId::new(BINARY_RECEIVER_ID.to_vec()).unwrap(),
        ReceiverKeyPair::from_private_key(BINARY_RECEIVER_PRIVATE_KEY).unwrap(),
    )
    .unwrap();
    let error = receiver.receive(1).err().unwrap();
    negatives.push(endpoint_error_case("missing-key-grant", "receive", &error));
}

fn store_operations(positives: &mut Vec<JsonValue>, negatives: &mut Vec<JsonValue>) {
    let backend = InMemoryStorageBackend::new();
    let store = ChannelStore::new(&backend, ChannelId(CHANNEL_ID));
    store.initialize().unwrap();
    let request = append_request(20, 20_000_000_020);
    let header = store
        .reserve_append(request.clone(), b"recoverable")
        .unwrap();
    let recovered = ChannelStore::new(&backend, ChannelId(CHANNEL_ID));
    let recovered_state = recovered.initialize().unwrap();

    let pending = store
        .reserve_append(append_request(21, 20_000_000_021), b"pending")
        .unwrap_err();
    negatives.push(store_error_case("pending-append", "reserve", &pending));
    let ack_pending = store
        .acknowledge(BINARY_RECEIVER_ID, Sequence(0))
        .unwrap_err();
    negatives.push(store_error_case(
        "acknowledgement-pending",
        "acknowledge",
        &ack_pending,
    ));
    let mismatched = prepare_message_header(
        MessageFields::new(
            uuid_v7(22),
            20_000_000_022,
            ORIGINATOR_ID.to_vec(),
            ChannelId(CHANNEL_ID),
            Sequence(0),
            KeyEpoch(0),
            "text/plain".to_owned(),
        ),
        b"recoverable",
    );
    let cmk = ChannelMasterKey::from_bytes(CHANNEL_MASTER_KEY);
    let signing_key = OriginatorSigningKey::from_seed(SIGNING_SEED);
    let mismatch = recovered
        .commit_reserved(&mismatched, b"recoverable", &cmk, &signing_key)
        .err()
        .unwrap();
    negatives.push(store_error_case(
        "pending-header-mismatch",
        "complete",
        &mismatch,
    ));
    let first_commit = recovered
        .commit_reserved(&header, b"recoverable", &cmk, &signing_key)
        .unwrap();
    let retry = recovered
        .commit_reserved(&header, b"recoverable", &cmk, &signing_key)
        .unwrap();

    let abandoned_header = recovered
        .reserve_append(append_request(23, 20_000_000_023), b"abandoned")
        .unwrap();
    let abandoned = recovered.abandon_pending().unwrap().unwrap();
    let no_pending = recovered
        .commit_reserved(&abandoned_header, b"abandoned", &cmk, &signing_key)
        .err()
        .unwrap();
    negatives.push(store_error_case(
        "no-pending-append",
        "complete",
        &no_pending,
    ));
    let after_gap = recovered
        .append(
            append_request(24, 20_000_000_024),
            b"after gap",
            &cmk,
            &signing_key,
        )
        .unwrap();
    let read_sequences: Vec<u64> = recovered
        .read_messages(Sequence(0), 10)
        .unwrap()
        .messages
        .iter()
        .map(|message| message.header().fields().sequence().0)
        .collect();
    let first_page = recovered.read_messages(Sequence(0), 1).unwrap();
    let continuation = first_page.next_start.unwrap();
    let second_page = recovered.read_messages(continuation, 1).unwrap();
    let random_access_sequences: Vec<u64> = recovered
        .read_messages(Sequence(2), 10)
        .unwrap()
        .messages
        .iter()
        .map(|message| message.header().fields().sequence().0)
        .collect();
    let empty_continuation = recovered.read_messages(Sequence(3), 10).unwrap();
    positives.push(operation_case(
        "reserve-recover-complete-retry-abandon-gap",
        vec![
            (
                "recovered_pending_equal",
                JsonValue::Bool(recovered_state.pending_header.as_ref() == Some(&header)),
            ),
            ("commit_retry_equal", JsonValue::Bool(first_commit == retry)),
            (
                "first_d18m_b64",
                string(encode_base64(&message_serialize(&first_commit).unwrap())),
            ),
            (
                "abandoned_sequence",
                string(abandoned.fields().sequence().0.to_string()),
            ),
            (
                "after_gap_sequence",
                string(after_gap.header().fields().sequence().0.to_string()),
            ),
            ("read_sequences", sequences(read_sequences)),
            (
                "first_page_sequences",
                sequences(
                    first_page
                        .messages
                        .iter()
                        .map(|message| message.header().fields().sequence().0),
                ),
            ),
            ("first_page_next_start", string(continuation.0.to_string())),
            (
                "second_page_sequences",
                sequences(
                    second_page
                        .messages
                        .iter()
                        .map(|message| message.header().fields().sequence().0),
                ),
            ),
            (
                "random_access_sequences",
                sequences(random_access_sequences),
            ),
            (
                "empty_continuation",
                JsonValue::Bool(
                    empty_continuation.messages.is_empty()
                        && empty_continuation.next_start.is_none(),
                ),
            ),
        ],
    ));

    let invalid_page = recovered.read_messages(Sequence(0), 0).err().unwrap();
    negatives.push(store_error_case("invalid-page-size", "read", &invalid_page));
    let invalid_receiver = recovered.receiver_cursor(b"").unwrap_err();
    negatives.push(store_error_case(
        "invalid-receiver-id",
        "receiver-cursor",
        &invalid_receiver,
    ));
    let ack_ahead = recovered
        .acknowledge(BINARY_RECEIVER_ID, Sequence(3))
        .unwrap_err();
    negatives.push(store_error_case(
        "acknowledgement-ahead",
        "acknowledge",
        &ack_ahead,
    ));
    recovered
        .acknowledge(BINARY_RECEIVER_ID, Sequence(2))
        .unwrap();
    let ack_regression = recovered
        .acknowledge(BINARY_RECEIVER_ID, Sequence(0))
        .unwrap_err();
    negatives.push(store_error_case(
        "acknowledgement-regression",
        "acknowledge",
        &ack_regression,
    ));
}

fn storage_corruption_cases(negatives: &mut Vec<JsonValue>) {
    let channel_id = ChannelId(CHANNEL_ID);
    let cmk = ChannelMasterKey::from_bytes(CHANNEL_MASTER_KEY);
    let signing_key = OriginatorSigningKey::from_seed(SIGNING_SEED);

    let key_mismatch_backend = InMemoryStorageBackend::new();
    let key_mismatch_store = ChannelStore::new(&key_mismatch_backend, channel_id);
    key_mismatch_store.initialize().unwrap();
    let message = key_mismatch_store
        .append(
            append_request(30, 30_000_000_030),
            b"mis-keyed",
            &cmk,
            &signing_key,
        )
        .unwrap();
    key_mismatch_backend
        .put(
            StoragePutInput::new(
                CHANNEL_STORAGE_NAMESPACE,
                message_record_key(channel_id, Sequence(1)),
                CHANNEL_MESSAGE_CONTENT_TYPE,
                JsonValue::Object(vec![]),
                message_serialize(&message).unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
    let key_mismatch = key_mismatch_store
        .read_messages(Sequence(1), 10)
        .err()
        .unwrap();
    negatives.push(store_error_case(
        "message-key-body-mismatch",
        "read",
        &key_mismatch,
    ));

    let content_type_backend = InMemoryStorageBackend::new();
    let content_type_store = ChannelStore::new(&content_type_backend, channel_id);
    content_type_store.initialize().unwrap();
    let message = content_type_store
        .append(
            append_request(31, 30_000_000_031),
            b"wrong content type",
            &cmk,
            &signing_key,
        )
        .unwrap();
    content_type_backend
        .put(
            StoragePutInput::new(
                CHANNEL_STORAGE_NAMESPACE,
                message_record_key(channel_id, Sequence(0)),
                "application/octet-stream",
                JsonValue::Object(vec![]),
                message_serialize(&message).unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
    let content_type = content_type_store
        .read_messages(Sequence(0), 10)
        .err()
        .unwrap();
    negatives.push(store_error_case(
        "message-content-type-mismatch",
        "read",
        &content_type,
    ));
}

fn fixture_definition(created_at_ns: u64) -> ChannelDefinition {
    let signing_key = OriginatorSigningKey::from_seed(SIGNING_SEED);
    let binary_receiver = ReceiverKeyPair::from_private_key(BINARY_RECEIVER_PRIVATE_KEY).unwrap();
    let text_receiver = ReceiverKeyPair::from_private_key(TEXT_RECEIVER_PRIVATE_KEY).unwrap();
    ChannelDefinition::new(
        ChannelId(CHANNEL_ID),
        OriginatorIdentity {
            agent_id: AgentId::new(ORIGINATOR_ID.to_vec()).unwrap(),
            public_key: signing_key.public_key(),
        },
        vec![
            ReceiverIdentity {
                agent_id: AgentId::new(TEXT_RECEIVER_ID.to_vec()).unwrap(),
                public_key: text_receiver.public_key(),
            },
            ReceiverIdentity {
                agent_id: AgentId::new(BINARY_RECEIVER_ID.to_vec()).unwrap(),
                public_key: binary_receiver.public_key(),
            },
        ],
        created_at_ns,
        KeyEpoch(0),
    )
    .unwrap()
}

fn append_request(byte: u8, timestamp_ns: u64) -> AppendRequest {
    AppendRequest {
        message_id: uuid_v7(byte),
        timestamp_ns,
        originator_id: ORIGINATOR_ID.to_vec(),
        key_epoch: KeyEpoch(0),
        content_type: "text/plain".to_owned(),
    }
}

fn metadata(byte: u8, timestamp_ns: u64) -> MessageMetadata {
    MessageMetadata {
        message_id: MessageId::from_uuid_v7(uuid_v7(byte)).unwrap(),
        timestamp_ns,
    }
}

fn uuid_v7(byte: u8) -> [u8; 16] {
    let mut bytes = [byte; 16];
    bytes[6] = 0x70 | (byte & 0x0f);
    bytes[8] = 0x80 | (byte & 0x3f);
    bytes
}

fn operation_case(name: &str, fields: Vec<(&str, JsonValue)>) -> JsonValue {
    let mut object = vec![("name".into(), string(name))];
    object.extend(
        fields
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value)),
    );
    JsonValue::Object(object)
}

fn endpoint_error_case(
    name: &str,
    operation: &str,
    error: &chief_of_staff_channel_endpoints::ChannelEndpointError,
) -> JsonValue {
    error_case(name, operation, channel_endpoint_error_code(error))
}

fn store_error_case(
    name: &str,
    operation: &str,
    error: &chief_of_staff_channel_store::ChannelStoreError,
) -> JsonValue {
    error_case(name, operation, channel_store_error_code(error))
}

fn error_case(name: &str, operation: &str, expected_error: &str) -> JsonValue {
    JsonValue::Object(vec![
        ("name".into(), string(name)),
        ("operation".into(), string(operation)),
        ("expected_error".into(), string(expected_error)),
    ])
}

fn codec_mutation(
    name: &str,
    kind: &str,
    base: &[u8],
    offset: usize,
    value: u8,
    expected_error: &str,
) -> JsonValue {
    let mut record = base.to_vec();
    record[offset] = value;
    codec_record(name, kind, &record, expected_error)
}

fn codec_record(name: &str, kind: &str, record: &[u8], expected_error: &str) -> JsonValue {
    JsonValue::Object(vec![
        ("name".into(), string(name)),
        ("kind".into(), string(kind)),
        ("record_b64".into(), string(encode_base64(record))),
        ("expected_error".into(), string(expected_error)),
    ])
}

fn key_case(name: &str, key: String) -> JsonValue {
    JsonValue::Object(vec![
        ("name".into(), string(name)),
        ("expected_key".into(), string(key)),
    ])
}

fn recipe(field: &str, declared_length: &str, expected_error: &str) -> JsonValue {
    JsonValue::Object(vec![
        ("field".into(), string(field)),
        ("declared_length".into(), string(declared_length)),
        ("expected_error".into(), string(expected_error)),
    ])
}

fn sequences(values: impl IntoIterator<Item = u64>) -> JsonValue {
    JsonValue::Array(
        values
            .into_iter()
            .map(|value| string(value.to_string()))
            .collect(),
    )
}

fn string(value: impl Into<String>) -> JsonValue {
    JsonValue::String(value.into())
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn encode_base64(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let a = u32::from(chunk[0]);
        let b = u32::from(*chunk.get(1).unwrap_or(&0));
        let c = u32::from(*chunk.get(2).unwrap_or(&0));
        let word = (a << 16) | (b << 8) | c;
        output.push(BASE64[((word >> 18) & 63) as usize] as char);
        output.push(BASE64[((word >> 12) & 63) as usize] as char);
        output.push(if chunk.len() > 1 {
            BASE64[((word >> 6) & 63) as usize] as char
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            BASE64[(word & 63) as usize] as char
        } else {
            '='
        });
    }
    output
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
