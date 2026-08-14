use std::fs;
use std::path::PathBuf;

use chief_of_staff_channel_crypto::profile::{
    message_authenticated_header, message_deserialize, message_from_json, message_serialize,
    message_to_json, message_verify_with_key_resolver, MessageProfileError, MAX_MESSAGE_JSON_BYTES,
};
use chief_of_staff_channel_crypto::{ChannelMasterKey, KeyEpoch};
use coding_adventures_json_parser::try_parse_json;
use coding_adventures_json_value::{from_ast, JsonValue};

const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

#[test]
fn positive_fixtures_lock_binary_header_json_and_verification() {
    let manifest = manifest();
    let root = object(&manifest);
    assert_eq!(string(root, "fixture_format"), "D18F-message-fixtures-v1");
    assert!(string(root, "warning").contains("test-only"));
    assert_eq!(string(root, "generator_blob_sha1").len(), 40);

    let keys = object(field(root, "keys"));
    let public_key: [u8; 32] = decode_hex(string(keys, "originator_public_key_hex"))
        .try_into()
        .unwrap();
    let master_keys = channel_keys(keys);
    let positives = array(field(root, "positive_cases"));
    assert_eq!(positives.len(), 8);

    for case in positives {
        let case = object(case);
        let name = string(case, "name");
        let plaintext = decode_base64(string(case, "plaintext_b64"));
        let header = decode_base64(string(case, "authenticated_header_b64"));
        let record = decode_base64(string(case, "d18m_b64"));
        let json = decode_base64(string(case, "canonical_json_b64"));
        let message = message_deserialize(&record)
            .unwrap_or_else(|error| panic!("{name}: deserialize: {}", error.code()));

        assert_eq!(
            message_serialize(&message).unwrap(),
            record,
            "{name}: binary"
        );
        assert_eq!(
            message_authenticated_header(&message),
            header,
            "{name}: header"
        );
        assert_eq!(message_to_json(&message).unwrap(), json, "{name}: JSON");
        assert_eq!(
            message_serialize(&message_from_json(&json).unwrap()).unwrap(),
            record,
            "{name}: JSON to binary"
        );
        let recovered = message_verify_with_key_resolver(&message, &public_key, |epoch| {
            master_keys
                .iter()
                .find(|(candidate, _)| *candidate == epoch)
                .map(|(_, key)| key)
        })
        .unwrap_or_else(|error| panic!("{name}: verify: {}", error.code()));
        assert_eq!(recovered, plaintext, "{name}: plaintext");
    }
}

#[test]
fn negative_fixtures_produce_the_declared_stable_error() {
    let manifest = manifest();
    let root = object(&manifest);
    let keys = object(field(root, "keys"));
    let public_key: [u8; 32] = decode_hex(string(keys, "originator_public_key_hex"))
        .try_into()
        .unwrap();
    let master_keys = channel_keys(keys);

    for case in array(field(root, "binary_negative_cases")) {
        let case = object(case);
        let name = string(case, "name");
        let record = decode_base64(string(case, "d18m_b64"));
        let expected = string(case, "expected_error");
        let error = if string(case, "phase") == "deserialize" {
            match message_deserialize(&record) {
                Ok(_) => panic!("{name}: unexpectedly deserialized"),
                Err(error) => error,
            }
        } else {
            let message = message_deserialize(&record)
                .unwrap_or_else(|error| panic!("{name}: structural decode: {}", error.code()));
            message_verify_with_key_resolver(&message, &public_key, |epoch| {
                master_keys
                    .iter()
                    .find(|(candidate, _)| *candidate == epoch)
                    .map(|(_, key)| key)
            })
            .unwrap_err()
        };
        assert_eq!(error.code(), expected, "{name}");
    }

    for case in array(field(root, "json_negative_cases")) {
        let case = object(case);
        let name = string(case, "name");
        let json = decode_base64(string(case, "json_b64"));
        let error = match message_from_json(&json) {
            Ok(_) => panic!("{name}: unexpectedly decoded"),
            Err(error) => error,
        };
        assert_eq!(error.code(), string(case, "expected_error"), "{name}");
    }
}

#[test]
fn compact_oversize_recipes_hit_each_declared_bound_without_large_blobs() {
    let manifest = manifest();
    let root = object(&manifest);
    let recipes = array(field(root, "oversize_recipes"));
    let positive = object(&array(field(root, "positive_cases"))[1]);
    let base = decode_base64(string(positive, "d18m_b64"));

    for recipe in recipes {
        let recipe = object(recipe);
        let field_name = string(recipe, "field");
        let length: usize = string(recipe, "declared_length").parse().unwrap();
        assert_eq!(string(recipe, "expected_error"), "length_limit_exceeded");
        if field_name == "json-input" {
            assert_eq!(length, MAX_MESSAGE_JSON_BYTES + 1);
            continue;
        }
        let record = oversize_record(&base, field_name, length);
        let error = match message_deserialize(&record) {
            Ok(_) => panic!("{field_name}: unexpectedly deserialized"),
            Err(error) => error,
        };
        assert_eq!(
            error.code(),
            MessageProfileError::LengthLimitExceeded.code(),
            "{field_name}"
        );
    }
}

fn manifest() -> JsonValue {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/chief-of-staff-message/v1/manifest.json");
    let source = fs::read_to_string(path).unwrap();
    let ast = try_parse_json(&source).unwrap();
    from_ast(&ast).unwrap()
}

fn channel_keys(keys: &[(String, JsonValue)]) -> Vec<(KeyEpoch, ChannelMasterKey)> {
    array(field(keys, "channel_master_keys"))
        .iter()
        .map(|entry| {
            let entry = object(entry);
            let epoch = KeyEpoch(string(entry, "key_epoch").parse().unwrap());
            let bytes: [u8; 32] = decode_hex(string(entry, "key_hex")).try_into().unwrap();
            (epoch, ChannelMasterKey::from_bytes(bytes))
        })
        .collect()
}

fn oversize_record(base: &[u8], field_name: &str, length: usize) -> Vec<u8> {
    let originator_length = read_u32(base, 29) as usize;
    let channel = 33 + originator_length;
    let content_length = channel + 16 + 8 + 8;
    let content = content_length + 4;
    let content_bytes = read_u32(base, content_length) as usize;
    let ciphertext_length = content + content_bytes + 32;
    match field_name {
        "originator-id" => {
            let mut record = base[..33].to_vec();
            record[29..33].copy_from_slice(&(length as u32).to_be_bytes());
            record
        }
        "content-type" => {
            let mut record = base[..content].to_vec();
            record[content_length..content].copy_from_slice(&(length as u32).to_be_bytes());
            record
        }
        "ciphertext" => {
            let mut record = base[..ciphertext_length + 8].to_vec();
            record[ciphertext_length..ciphertext_length + 8]
                .copy_from_slice(&(length as u64).to_be_bytes());
            record
        }
        _ => panic!("unknown oversize recipe {field_name}"),
    }
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

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap())
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

fn decode_hex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| hex_digit(pair[0]) * 16 + hex_digit(pair[1]))
        .collect()
}

fn hex_digit(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => panic!("invalid fixture hex"),
    }
}
