use std::fs;
use std::path::PathBuf;

use chief_of_staff_channel_crypto::{ChannelId, Sequence};
use chief_of_staff_channel_endpoints::profile::{
    channel_definition_deserialize, channel_definition_serialize, channel_endpoint_error_code,
    CHANNEL_DEFINITION_CONTENT_TYPE, MAX_CHANNEL_RECEIVERS, MAX_DEFINITION_CAS_ATTEMPTS,
};
use chief_of_staff_channel_store::profile::{
    channel_state_deserialize, channel_state_serialize, channel_store_error_code,
    receiver_cursor_deserialize, receiver_cursor_serialize, CHANNEL_ACK_CONTENT_TYPE,
    CHANNEL_GRANT_CONTENT_TYPE, CHANNEL_MESSAGE_CONTENT_TYPE, CHANNEL_STATE_CONTENT_TYPE,
    MAX_CHANNEL_CAS_ATTEMPTS, MAX_PENDING_HEADER_BYTES,
};
use coding_adventures_json_parser::try_parse_json;
use coding_adventures_json_value::{from_ast, JsonValue};
use coding_adventures_sha1::sum1;

#[path = "../examples/generate_d18p_fixtures.rs"]
mod fixture_generator;

const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const CHANNEL_ID: [u8; 16] = [
    0x01, 0x8f, 0x47, 0xa0, 0x9b, 0x6c, 0x7d, 0xef, 0x92, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
];

#[test]
fn checked_manifest_is_byte_identical_to_the_recorded_generator() {
    let bytes = manifest_bytes();
    let manifest = parse_manifest(&bytes);
    let root = object(&manifest);
    assert_eq!(
        string(root, "fixture_format"),
        "D18P-durable-channel-fixtures-v1"
    );
    assert!(string(root, "warning").contains("test-only"));
    let generator_blob_sha1 = string(root, "generator_blob_sha1");
    assert_eq!(generator_blob_sha1.len(), 40);
    let generator_source = include_bytes!("../examples/generate_d18p_fixtures.rs");
    let mut git_blob = format!("blob {}\0", generator_source.len()).into_bytes();
    git_blob.extend_from_slice(generator_source);
    assert_eq!(encode_hex(&sum1(&git_blob)), generator_blob_sha1);
    assert_eq!(
        fixture_generator::generate_manifest(generator_blob_sha1),
        bytes
    );
}

#[test]
fn positive_codec_cases_preserve_exact_production_bytes() {
    let manifest = parse_manifest(&manifest_bytes());
    let root = object(&manifest);

    for case in array(field(root, "definition_cases")) {
        let case = object(case);
        let bytes = decode_base64(string(case, "d18c_b64"));
        let definition = channel_definition_deserialize(&bytes).unwrap_or_else(|error| {
            panic!(
                "{}: {}",
                string(case, "name"),
                channel_endpoint_error_code(&error)
            )
        });
        assert_eq!(
            channel_definition_serialize(&definition),
            bytes,
            "{}",
            string(case, "name")
        );
    }

    for case in array(field(root, "state_cases")) {
        let case = object(case);
        let bytes = decode_base64(string(case, "d18s_b64"));
        let state =
            channel_state_deserialize(&bytes, ChannelId(CHANNEL_ID)).unwrap_or_else(|error| {
                panic!(
                    "{}: {}",
                    string(case, "name"),
                    channel_store_error_code(&error)
                )
            });
        assert_eq!(
            channel_state_serialize(&state).unwrap(),
            bytes,
            "{}",
            string(case, "name")
        );
    }

    for case in array(field(root, "cursor_cases")) {
        let case = object(case);
        let bytes = decode_base64(string(case, "d18a_b64"));
        let cursor = receiver_cursor_deserialize(&bytes)
            .unwrap_or_else(|error| panic!("cursor: {}", channel_store_error_code(&error)));
        assert_eq!(cursor.0.to_string(), string(case, "first_unread_sequence"));
        assert_eq!(receiver_cursor_serialize(cursor), bytes);
    }
}

#[test]
fn negative_codec_cases_produce_declared_stable_errors() {
    let manifest = parse_manifest(&manifest_bytes());
    let root = object(&manifest);

    for case in array(field(root, "codec_negative_cases")) {
        let case = object(case);
        let name = string(case, "name");
        let bytes = decode_base64(string(case, "record_b64"));
        let actual = match string(case, "kind") {
            "definition" => match channel_definition_deserialize(&bytes) {
                Ok(_) => panic!("{name}: unexpectedly decoded"),
                Err(error) => channel_endpoint_error_code(&error),
            },
            "state" => match channel_state_deserialize(&bytes, ChannelId(CHANNEL_ID)) {
                Ok(_) => panic!("{name}: unexpectedly decoded"),
                Err(error) => channel_store_error_code(&error),
            },
            "cursor" => match receiver_cursor_deserialize(&bytes) {
                Ok(_) => panic!("{name}: unexpectedly decoded"),
                Err(error) => channel_store_error_code(&error),
            },
            kind => panic!("{name}: unknown codec kind {kind}"),
        };
        assert_eq!(actual, string(case, "expected_error"), "{name}");
    }
}

#[test]
fn constants_keys_traces_and_error_rosters_are_closed() {
    let manifest = parse_manifest(&manifest_bytes());
    let root = object(&manifest);
    let constants = object(field(root, "constants"));
    let content_types = object(field(constants, "content_types"));
    assert_eq!(string(constants, "storage_namespace"), "chief-channels");
    assert_eq!(
        string(content_types, "definition"),
        CHANNEL_DEFINITION_CONTENT_TYPE
    );
    assert_eq!(string(content_types, "state"), CHANNEL_STATE_CONTENT_TYPE);
    assert_eq!(
        string(content_types, "message"),
        CHANNEL_MESSAGE_CONTENT_TYPE
    );
    assert_eq!(string(content_types, "grant"), CHANNEL_GRANT_CONTENT_TYPE);
    assert_eq!(string(content_types, "ack"), CHANNEL_ACK_CONTENT_TYPE);
    assert_eq!(
        usize_field(constants, "max_receivers"),
        MAX_CHANNEL_RECEIVERS
    );
    assert_eq!(
        usize_field(constants, "max_pending_header_bytes"),
        MAX_PENDING_HEADER_BYTES
    );
    assert_eq!(
        usize_field(constants, "max_store_cas_attempts"),
        MAX_CHANNEL_CAS_ATTEMPTS
    );
    assert_eq!(
        usize_field(constants, "max_definition_cas_attempts"),
        MAX_DEFINITION_CAS_ATTEMPTS
    );

    assert_eq!(array(field(root, "definition_cases")).len(), 2);
    assert_eq!(array(field(root, "state_cases")).len(), 2);
    assert_eq!(array(field(root, "cursor_cases")).len(), 4);
    assert_eq!(array(field(root, "storage_key_cases")).len(), 7);
    assert_eq!(array(field(root, "codec_negative_cases")).len(), 19);
    assert_eq!(array(field(root, "operation_cases")).len(), 4);
    assert_eq!(array(field(root, "operation_negative_cases")).len(), 17);
    assert_eq!(array(field(root, "stable_error_codes")).len(), 30);
    assert_eq!(array(field(root, "oversize_recipes")).len(), 3);

    let operation_errors: Vec<&str> = array(field(root, "operation_negative_cases"))
        .iter()
        .map(|case| string(object(case), "expected_error"))
        .collect();
    for required in [
        "conflicting_definition",
        "channel_destroyed",
        "missing_key_grant",
        "pending_append",
        "pending_header_mismatch",
        "corrupt_record",
        "acknowledgement_regression",
        "acknowledgement_ahead",
        "acknowledgement_pending",
    ] {
        assert!(
            operation_errors.contains(&required),
            "missing operation error {required}"
        );
    }

    let cursor_42 = receiver_cursor_deserialize(&decode_base64(string(
        object(&array(field(root, "cursor_cases"))[2]),
        "d18a_b64",
    )))
    .unwrap();
    assert_eq!(cursor_42, Sequence(42));
}

fn manifest_bytes() -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/chief-of-staff-channel/v1/manifest.json");
    fs::read(path).unwrap()
}

fn parse_manifest(bytes: &[u8]) -> JsonValue {
    let source = std::str::from_utf8(bytes).unwrap();
    let ast = try_parse_json(source).unwrap();
    from_ast(&ast).unwrap()
}

fn object(value: &JsonValue) -> &[(String, JsonValue)] {
    match value {
        JsonValue::Object(value) => value,
        _ => panic!("expected object"),
    }
}

fn array(value: &JsonValue) -> &[JsonValue] {
    match value {
        JsonValue::Array(value) => value,
        _ => panic!("expected array"),
    }
}

fn field<'a>(object: &'a [(String, JsonValue)], name: &str) -> &'a JsonValue {
    &object.iter().find(|(key, _)| key == name).unwrap().1
}

fn string<'a>(object: &'a [(String, JsonValue)], name: &str) -> &'a str {
    match field(object, name) {
        JsonValue::String(value) => value,
        _ => panic!("expected string field {name}"),
    }
}

fn usize_field(object: &[(String, JsonValue)], name: &str) -> usize {
    string(object, name).parse().unwrap()
}

fn decode_base64(value: &str) -> Vec<u8> {
    assert!(value.len().is_multiple_of(4));
    let mut output = Vec::with_capacity(value.len() / 4 * 3);
    for chunk in value.as_bytes().chunks_exact(4) {
        let a = base64_digit(chunk[0]);
        let b = base64_digit(chunk[1]);
        let c = if chunk[2] == b'=' {
            0
        } else {
            base64_digit(chunk[2])
        };
        let d = if chunk[3] == b'=' {
            0
        } else {
            base64_digit(chunk[3])
        };
        let word = (u32::from(a) << 18) | (u32::from(b) << 12) | (u32::from(c) << 6) | u32::from(d);
        output.push(((word >> 16) & 255) as u8);
        if chunk[2] != b'=' {
            output.push(((word >> 8) & 255) as u8);
        }
        if chunk[3] != b'=' {
            output.push((word & 255) as u8);
        }
    }
    output
}

fn base64_digit(byte: u8) -> u8 {
    BASE64
        .iter()
        .position(|candidate| *candidate == byte)
        .unwrap() as u8
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
