use std::fs;
use std::path::PathBuf;

use chief_of_staff_channel_crypto::{ChannelId, KeyEpoch, Sequence};
use chief_of_staff_channel_epoch_activation::{
    activation_plan_deserialize, epoch_state_deserialize, ACTIVATION_PLAN_CONTENT_TYPE,
    EPOCH_STATE_CONTENT_TYPE,
};
use chief_of_staff_channel_store::profile::channel_state_deserialize;
use coding_adventures_json_parser::try_parse_json;
use coding_adventures_json_value::{from_ast, JsonValue};
use coding_adventures_sha1::sum1;

#[path = "../examples/generate_d18t_fixtures.rs"]
mod fixture_generator;

const CHANNEL_ID: [u8; 16] = [
    0x01, 0x8f, 0x47, 0xa0, 0x9b, 0x6c, 0x7d, 0xef, 0x92, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
];

#[test]
fn checked_manifest_is_byte_identical_to_its_recorded_generator() {
    let bytes = manifest_bytes();
    let manifest = parse_manifest(&bytes);
    let root = object(&manifest);
    assert_eq!(
        string(root, "fixture_format"),
        "D18T-durable-epoch-activation-fixtures-v1"
    );
    let generator_blob_sha1 = string(root, "generator_blob_sha1");
    let generator_source = include_bytes!("../examples/generate_d18t_fixtures.rs");
    let mut git_blob = format!("blob {}\0", generator_source.len()).into_bytes();
    git_blob.extend_from_slice(generator_source);
    assert_eq!(encode_hex(&sum1(&git_blob)), generator_blob_sha1);
    assert_eq!(
        fixture_generator::generate_manifest(generator_blob_sha1),
        bytes
    );
}

#[test]
fn migration_vectors_preserve_pending_header_and_add_only_active_epoch() {
    let manifest = parse_manifest(&manifest_bytes());
    let root = object(&manifest);
    let migrations = array(field(root, "state_migrations"));
    assert_eq!(migrations.len(), 2);
    for migration in migrations {
        let migration = object(migration);
        let v1 = channel_state_deserialize(
            &decode_base64(string(migration, "d18s_v1_b64")),
            ChannelId(CHANNEL_ID),
        )
        .unwrap();
        let v2 = epoch_state_deserialize(
            &decode_base64(string(migration, "d18s_v2_b64")),
            ChannelId(CHANNEL_ID),
        )
        .unwrap();
        assert_eq!(v2.active_epoch(), KeyEpoch(0));
        assert_eq!(v2.next_sequence(), v1.next_sequence);
        assert_eq!(v2.pending_header(), v1.pending_header.as_ref());
    }
    let pending = object(&migrations[1]);
    assert_eq!(string(pending, "next_sequence"), "8");
    assert_eq!(
        epoch_state_deserialize(
            &decode_base64(string(pending, "d18s_v2_b64")),
            ChannelId(CHANNEL_ID),
        )
        .unwrap()
        .next_sequence(),
        Sequence(8)
    );
}

#[test]
fn activation_vector_commits_exact_plan_and_b_only_successor_grant() {
    let manifest = parse_manifest(&manifest_bytes());
    let root = object(&manifest);
    let constants = object(field(root, "constants"));
    assert_eq!(
        string(constants, "state_content_type"),
        EPOCH_STATE_CONTENT_TYPE
    );
    assert_eq!(
        string(constants, "plan_content_type"),
        ACTIVATION_PLAN_CONTENT_TYPE
    );
    let activation = object(field(root, "activation_case"));
    let plan = activation_plan_deserialize(&decode_base64(string(activation, "plan_b64"))).unwrap();
    assert_eq!(plan.channel_id(), ChannelId(CHANNEL_ID));
    assert_eq!(plan.base_epoch(), KeyEpoch(0));
    assert_eq!(plan.new_epoch(), KeyEpoch(1));
    assert_eq!(plan.receivers().len(), 1);
    assert_eq!(array(field(activation, "grant_b64")).len(), 1);
    assert!(matches!(
        field(activation, "receiver_a_new_grant"),
        JsonValue::Null
    ));
    assert_eq!(
        array(field(activation, "receiver_a_retains_epochs"))
            .iter()
            .map(json_string)
            .collect::<Vec<_>>(),
        vec!["0"]
    );
    assert_eq!(
        array(field(activation, "receiver_b_retains_epochs"))
            .iter()
            .map(json_string)
            .collect::<Vec<_>>(),
        vec!["0", "1"]
    );
}

#[test]
fn replay_races_errors_and_secret_boundary_are_closed() {
    let bytes = manifest_bytes();
    let text = std::str::from_utf8(&bytes).unwrap();
    let manifest = parse_manifest(&bytes);
    let root = object(&manifest);
    assert!(string(root, "warning").contains("Never log"));
    let crash_names = names(array(field(root, "crash_replay_traces")));
    assert_eq!(
        crash_names,
        vec![
            "after-custody-selection",
            "after-plan-write",
            "after-first-grant",
            "after-all-grants",
            "after-activation-cas",
        ]
    );
    assert_eq!(names(array(field(root, "race_traces"))).len(), 4);
    assert_eq!(names(array(field(root, "negative_scenarios"))).len(), 6);
    assert_eq!(string(root, "secret_erasure_capability"), "guaranteed");
    let errors = array(field(root, "stable_error_codes"))
        .iter()
        .map(json_string)
        .collect::<Vec<_>>();
    assert_eq!(errors.len(), 19);
    assert!(errors.contains(&"concurrent_update"));
    assert!(errors.contains(&"preparation_missing"));

    for (_, value) in object(field(root, "test_only_secrets")) {
        let secret = json_string(value);
        assert_eq!(
            text.matches(secret).count(),
            1,
            "test-only secret escaped its dedicated object"
        );
    }
}

fn manifest_bytes() -> Vec<u8> {
    fs::read(manifest_path()).unwrap()
}

fn manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/chief-of-staff-channel-epoch-activation/v1/manifest.json")
}

fn parse_manifest(bytes: &[u8]) -> JsonValue {
    let text = std::str::from_utf8(bytes).unwrap();
    from_ast(&try_parse_json(text).unwrap()).unwrap()
}

fn object(value: &JsonValue) -> &Vec<(String, JsonValue)> {
    match value {
        JsonValue::Object(fields) => fields,
        _ => panic!("expected object"),
    }
}

fn array(value: &JsonValue) -> &Vec<JsonValue> {
    match value {
        JsonValue::Array(values) => values,
        _ => panic!("expected array"),
    }
}

fn field<'a>(fields: &'a [(String, JsonValue)], name: &str) -> &'a JsonValue {
    fields
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value)
        .unwrap_or_else(|| panic!("missing field {name}"))
}

fn string<'a>(fields: &'a [(String, JsonValue)], name: &str) -> &'a str {
    json_string(field(fields, name))
}

fn json_string(value: &JsonValue) -> &str {
    match value {
        JsonValue::String(value) => value,
        _ => panic!("expected string"),
    }
}

fn names(values: &[JsonValue]) -> Vec<&str> {
    values
        .iter()
        .map(|value| string(object(value), "name"))
        .collect()
}

fn decode_base64(value: &str) -> Vec<u8> {
    let bytes = value.as_bytes();
    assert_eq!(bytes.len() % 4, 0);
    let mut output = Vec::with_capacity(bytes.len() / 4 * 3);
    for chunk in bytes.chunks_exact(4) {
        let a = base64_value(chunk[0]);
        let b = base64_value(chunk[1]);
        let c = if chunk[2] == b'=' {
            0
        } else {
            base64_value(chunk[2])
        };
        let d = if chunk[3] == b'=' {
            0
        } else {
            base64_value(chunk[3])
        };
        let word = ((a as u32) << 18) | ((b as u32) << 12) | ((c as u32) << 6) | d as u32;
        output.push((word >> 16) as u8);
        if chunk[2] != b'=' {
            output.push((word >> 8) as u8);
        }
        if chunk[3] != b'=' {
            output.push(word as u8);
        }
    }
    output
}

fn base64_value(byte: u8) -> u8 {
    match byte {
        b'A'..=b'Z' => byte - b'A',
        b'a'..=b'z' => byte - b'a' + 26,
        b'0'..=b'9' => byte - b'0' + 52,
        b'+' => 62,
        b'/' => 63,
        _ => panic!("invalid base64"),
    }
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
