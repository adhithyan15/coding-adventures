//! Generate the deterministic shared D18F version 1 fixture manifest.

use std::env;
use std::fs;
use std::path::Path;

use chief_of_staff_channel_crypto::profile::{
    message_authenticated_header, message_create, message_serialize, message_to_json,
    MonotonicUuidV7Generator,
};
use chief_of_staff_channel_crypto::{
    ChannelId, ChannelMasterKey, KeyEpoch, MessageFields, OriginatorSigningKey, Sequence,
};
use coding_adventures_chacha20_poly1305::xchacha20_poly1305_aead_encrypt;
use coding_adventures_ed25519::sign;
use coding_adventures_json_serializer::serialize;
use coding_adventures_json_value::JsonValue;
use coding_adventures_sha256::sha256;

const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const CONTEXT: &[u8] = b"chief-channel-message-v1";
const CHANNEL_ID: [u8; 16] = [
    0x01, 0x8f, 0x47, 0xa0, 0x9b, 0x6c, 0x7d, 0xef, 0x92, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
];

struct Case<'a> {
    name: &'a str,
    plaintext: &'a [u8],
    content_type: &'a str,
    epoch: u64,
}

fn main() {
    let mut arguments = env::args().skip(1);
    let output = arguments
        .next()
        .expect("usage: generate_d18f_fixtures OUTPUT GENERATOR_BLOB_SHA1");
    let generator_blob_sha1 = arguments
        .next()
        .expect("usage: generate_d18f_fixtures OUTPUT GENERATOR_BLOB_SHA1");
    assert!(arguments.next().is_none(), "unexpected extra argument");

    let signing_seed = [0x11; 32];
    let signing_key = OriginatorSigningKey::from_seed(signing_seed);
    let epoch_zero = ChannelMasterKey::from_bytes([0x22; 32]);
    let epoch_seven = ChannelMasterKey::from_bytes([0x77; 32]);
    let cases = [
        Case {
            name: "empty",
            plaintext: b"",
            content_type: "application/octet-stream",
            epoch: 0,
        },
        Case {
            name: "utf8-text",
            plaintext: "Hello, Chief! \u{1f680}".as_bytes(),
            content_type: "text/plain;charset=utf-8",
            epoch: 0,
        },
        Case {
            name: "structured-json",
            plaintext: br#"{"status":"ok","count":3}"#,
            content_type: "application/vnd.coding-adventures.result+json;version=1",
            epoch: 0,
        },
        Case {
            name: "arbitrary-binary",
            plaintext: &[0x00, 0xff, 0x80, 0x7f, 0x10, 0x0d, 0x0a],
            content_type: "application/octet-stream",
            epoch: 0,
        },
        Case {
            name: "multipart-related",
            plaintext: b"--chief-boundary\r\nContent-Type: text/plain\r\n\r\nhello\r\n--chief-boundary--\r\n",
            content_type: "multipart/related;boundary=\"chief-boundary\"",
            epoch: 0,
        },
        Case {
            name: "rotated-key-epoch",
            plaintext: b"epoch seven",
            content_type: "text/plain",
            epoch: 7,
        },
        Case {
            name: "stream-chunk-zero",
            plaintext: b"chunk zero",
            content_type: "application/octet-stream;stream-id=\"018f47a0-9b6c-7def-8123-456789abcdef\";chunk-index=0;final=false",
            epoch: 7,
        },
        Case {
            name: "stream-chunk-one-final",
            plaintext: b"chunk one",
            content_type: "application/octet-stream;stream-id=\"018f47a0-9b6c-7def-8123-456789abcdef\";chunk-index=1;final=true",
            epoch: 7,
        },
    ];

    let mut uuid_source = MonotonicUuidV7Generator::new();
    let mut positives = Vec::new();
    let mut binary_negatives = Vec::new();
    let mut json_negatives = Vec::new();
    let mut mutation_source = None;

    for (index, case) in cases.iter().enumerate() {
        let message_id = uuid_source
            .next(1_725_000_000_000, [index as u8; 10])
            .unwrap();
        let fields = MessageFields::new(
            message_id,
            9_000_000_000 + index as u64,
            b"fixture-originator".to_vec(),
            ChannelId(CHANNEL_ID),
            Sequence(index as u64),
            KeyEpoch(case.epoch),
            case.content_type.to_owned(),
        );
        let cmk = if case.epoch == 0 {
            &epoch_zero
        } else {
            &epoch_seven
        };
        let message = message_create(fields, case.plaintext, &signing_key, cmk).unwrap();
        let record = message_serialize(&message).unwrap();
        let json = message_to_json(&message).unwrap();
        if case.name == "utf8-text" {
            mutation_source = Some((record.clone(), json.clone()));
        }
        positives.push(JsonValue::Object(vec![
            ("name".into(), string(case.name)),
            (
                "plaintext_b64".into(),
                string(encode_base64(case.plaintext)),
            ),
            (
                "authenticated_header_b64".into(),
                string(encode_base64(&message_authenticated_header(&message))),
            ),
            ("d18m_b64".into(), string(encode_base64(&record))),
            ("canonical_json_b64".into(), string(encode_base64(&json))),
        ]));
    }

    let (base_record, base_json) = mutation_source.unwrap();
    add_binary_negatives(&mut binary_negatives, &base_record, &epoch_zero);
    add_json_negatives(&mut json_negatives, &base_json);

    let manifest = JsonValue::Object(vec![
        ("fixture_format".into(), string("D18F-message-fixtures-v1")),
        (
            "spec".into(),
            string("code/specs/D18F-chief-of-staff-message-profile.md"),
        ),
        (
            "generator_blob_sha1".into(),
            string(generator_blob_sha1),
        ),
        (
            "warning".into(),
            string("All private keys and channel master keys are deterministic test-only material. Never use them outside conformance tests."),
        ),
        (
            "keys".into(),
            JsonValue::Object(vec![
                ("originator_signing_seed_hex".into(), string(encode_hex(&signing_seed))),
                (
                    "originator_public_key_hex".into(),
                    string(encode_hex(&signing_key.public_key())),
                ),
                (
                    "channel_master_keys".into(),
                    JsonValue::Array(vec![
                        epoch_key(0, [0x22; 32]),
                        epoch_key(7, [0x77; 32]),
                    ]),
                ),
            ]),
        ),
        ("positive_cases".into(), JsonValue::Array(positives)),
        (
            "binary_negative_cases".into(),
            JsonValue::Array(binary_negatives),
        ),
        (
            "json_negative_cases".into(),
            JsonValue::Array(json_negatives),
        ),
        (
            "oversize_recipes".into(),
            JsonValue::Array(vec![
                recipe("originator-id", "4097", "length_limit_exceeded"),
                recipe("content-type", "1025", "length_limit_exceeded"),
                recipe("ciphertext", "67108865", "length_limit_exceeded"),
                recipe("json-input", "94371841", "length_limit_exceeded"),
            ]),
        ),
    ]);
    let mut encoded = serialize(&manifest).unwrap();
    encoded.push('\n');
    if let Some(parent) = Path::new(&output).parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(output, encoded).unwrap();
}

fn add_binary_negatives(output: &mut Vec<JsonValue>, base: &[u8], cmk: &ChannelMasterKey) {
    let layout = layout(base);
    let mutate = |name: &str, phase: &str, expected: &str, index: usize, value: u8| {
        let mut record = base.to_vec();
        record[index] = value;
        binary_negative(name, phase, expected, &record)
    };
    output.push(mutate(
        "invalid-magic",
        "deserialize",
        "invalid_magic",
        0,
        b'X',
    ));
    output.push(mutate(
        "unsupported-version",
        "deserialize",
        "unsupported_version",
        4,
        2,
    ));
    output.push(binary_negative(
        "truncated-record",
        "deserialize",
        "truncated_record",
        &base[..base.len() - 1],
    ));
    let mut trailing = base.to_vec();
    trailing.push(0);
    output.push(binary_negative(
        "trailing-byte",
        "deserialize",
        "trailing_bytes",
        &trailing,
    ));
    let mut oversized_identity = base[..33].to_vec();
    oversized_identity[29..33].copy_from_slice(&4097u32.to_be_bytes());
    output.push(binary_negative(
        "oversized-originator-length",
        "deserialize",
        "length_limit_exceeded",
        &oversized_identity,
    ));
    output.push(mutate(
        "invalid-content-type-utf8",
        "deserialize",
        "invalid_utf8",
        layout.content_type,
        0xff,
    ));
    output.push(mutate(
        "invalid-message-uuid",
        "verify",
        "invalid_field",
        11,
        0x40,
    ));
    output.push(mutate(
        "authenticated-message-id",
        "verify",
        "invalid_signature",
        20,
        base[20] ^ 1,
    ));
    output.push(mutate(
        "authenticated-originator-id",
        "verify",
        "invalid_signature",
        layout.originator,
        base[layout.originator] ^ 1,
    ));
    output.push(mutate(
        "authenticated-channel-id",
        "verify",
        "invalid_signature",
        layout.channel_id + 15,
        base[layout.channel_id + 15] ^ 1,
    ));
    output.push(mutate(
        "authenticated-sequence",
        "verify",
        "invalid_signature",
        layout.sequence + 7,
        base[layout.sequence + 7] ^ 1,
    ));
    output.push(mutate(
        "invalid-mime",
        "verify",
        "invalid_field",
        layout.content_type,
        b' ',
    ));
    output.push(mutate(
        "authenticated-content-type",
        "verify",
        "invalid_signature",
        layout.content_type,
        b'u',
    ));
    output.push(mutate(
        "authenticated-timestamp",
        "verify",
        "invalid_signature",
        28,
        base[28] ^ 1,
    ));
    output.push(mutate(
        "missing-key-epoch",
        "verify",
        "missing_epoch_key",
        layout.key_epoch + 7,
        99,
    ));
    output.push(mutate(
        "authenticated-plaintext-hash",
        "verify",
        "invalid_signature",
        layout.plaintext_hash,
        base[layout.plaintext_hash] ^ 1,
    ));
    output.push(mutate(
        "ciphertext",
        "verify",
        "authentication_failed",
        layout.ciphertext,
        base[layout.ciphertext] ^ 1,
    ));
    output.push(mutate(
        "authentication-tag",
        "verify",
        "authentication_failed",
        layout.authentication_tag,
        base[layout.authentication_tag] ^ 1,
    ));
    output.push(mutate(
        "originator-signature",
        "verify",
        "invalid_signature",
        layout.signature,
        base[layout.signature] ^ 1,
    ));
    output.push(binary_negative(
        "plaintext-hash-mismatch",
        "verify",
        "plaintext_hash_mismatch",
        &wrong_hash_record(cmk),
    ));
}

fn add_json_negatives(output: &mut Vec<JsonValue>, canonical: &[u8]) {
    let source = String::from_utf8(canonical.to_vec()).unwrap();
    let cases = [
        ("syntax", "invalid_json", "{".to_owned()),
        (
            "duplicate-key",
            "invalid_json",
            source.replacen(
                "\"record_type\":\"D18M\"",
                "\"record_type\":\"D18M\",\"record_type\":\"D18M\"",
                1,
            ),
        ),
        (
            "unknown-key",
            "invalid_json",
            source.replacen("{", "{\"unknown\":0,", 1),
        ),
        (
            "missing-key",
            "invalid_json",
            source.replacen("\"record_type\":\"D18M\",", "", 1),
        ),
        (
            "record-type",
            "invalid_magic",
            source.replacen("\"D18M\"", "\"ACTM\"", 1),
        ),
        (
            "wire-version",
            "unsupported_version",
            source.replacen("\"wire_version\":1", "\"wire_version\":2", 1),
        ),
        (
            "wire-version-type",
            "invalid_json",
            source.replacen("\"wire_version\":1", "\"wire_version\":\"1\"", 1),
        ),
        (
            "decimal-leading-zero",
            "invalid_field",
            source.replacen("\"sequence\":\"1\"", "\"sequence\":\"01\"", 1),
        ),
        (
            "uppercase-uuid",
            "invalid_field",
            source.replacen("018f47a0", "018F47A0", 1),
        ),
        (
            "base64-without-padding",
            "invalid_field",
            remove_base64_padding(&source, "\"authentication_tag_b64\":\""),
        ),
        (
            "uppercase-hash",
            "invalid_field",
            uppercase_first_hash_digit(&source),
        ),
    ];
    output.extend(cases.into_iter().map(|(name, expected, json)| {
        JsonValue::Object(vec![
            ("name".into(), string(name)),
            ("json_b64".into(), string(encode_base64(json.as_bytes()))),
            ("expected_error".into(), string(expected)),
        ])
    }));
}

fn wrong_hash_record(cmk: &ChannelMasterKey) -> Vec<u8> {
    let message_id = [
        0x01, 0x8f, 0x47, 0xa0, 0x9b, 0x6c, 0x7d, 0xef, 0x81, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd,
        0xef,
    ];
    let timestamp = 9_000_000_100u64;
    let originator = b"fixture-originator";
    let sequence = 100u64;
    let epoch = 0u64;
    let content_type = b"application/octet-stream";
    let plaintext = b"hash mismatch fixture";
    let mut declared_hash = sha256(plaintext);
    declared_hash[0] ^= 1;
    let aad = frame(&[
        CONTEXT,
        &message_id,
        &timestamp.to_be_bytes(),
        originator,
        &CHANNEL_ID,
        &sequence.to_be_bytes(),
        &epoch.to_be_bytes(),
        content_type,
        &declared_hash,
    ]);
    let mut nonce = [0u8; 24];
    nonce[..16].copy_from_slice(&CHANNEL_ID);
    nonce[16..].copy_from_slice(&sequence.to_be_bytes());
    let (ciphertext, tag) =
        xchacha20_poly1305_aead_encrypt(plaintext, cmk.as_bytes(), &nonce, &aad);
    let signature_seed = [0x11; 32];
    let (_, secret_key) = coding_adventures_ed25519::generate_keypair(&signature_seed);
    let signature = sign(&aad, &secret_key);

    let mut record = Vec::new();
    record.extend_from_slice(b"D18M\x01");
    record.extend_from_slice(&message_id);
    record.extend_from_slice(&timestamp.to_be_bytes());
    put_u32_bytes(&mut record, originator);
    record.extend_from_slice(&CHANNEL_ID);
    record.extend_from_slice(&sequence.to_be_bytes());
    record.extend_from_slice(&epoch.to_be_bytes());
    put_u32_bytes(&mut record, content_type);
    record.extend_from_slice(&declared_hash);
    record.extend_from_slice(&(ciphertext.len() as u64).to_be_bytes());
    record.extend_from_slice(&ciphertext);
    record.extend_from_slice(&tag);
    record.extend_from_slice(&signature);
    record
}

struct Layout {
    originator: usize,
    channel_id: usize,
    sequence: usize,
    key_epoch: usize,
    content_type: usize,
    plaintext_hash: usize,
    ciphertext: usize,
    authentication_tag: usize,
    signature: usize,
}

fn layout(record: &[u8]) -> Layout {
    let originator_length = read_u32(record, 29) as usize;
    let channel = 33 + originator_length;
    let sequence = channel + 16;
    let key_epoch = sequence + 8;
    let content_length = key_epoch + 8;
    let content_type = content_length + 4;
    let content_type_length = read_u32(record, content_length) as usize;
    let hash = content_type + content_type_length;
    let ciphertext_length = hash + 32;
    let ciphertext = ciphertext_length + 8;
    let authentication_tag = ciphertext + read_u64(record, ciphertext_length) as usize;
    Layout {
        originator: 33,
        channel_id: channel,
        sequence,
        key_epoch,
        content_type,
        plaintext_hash: hash,
        ciphertext,
        authentication_tag,
        signature: authentication_tag + 16,
    }
}

fn binary_negative(name: &str, phase: &str, expected: &str, record: &[u8]) -> JsonValue {
    JsonValue::Object(vec![
        ("name".into(), string(name)),
        ("phase".into(), string(phase)),
        ("d18m_b64".into(), string(encode_base64(record))),
        ("expected_error".into(), string(expected)),
    ])
}

fn recipe(field: &str, length: &str, expected: &str) -> JsonValue {
    JsonValue::Object(vec![
        ("field".into(), string(field)),
        ("declared_length".into(), string(length)),
        ("expected_error".into(), string(expected)),
    ])
}

fn epoch_key(epoch: u64, key: [u8; 32]) -> JsonValue {
    JsonValue::Object(vec![
        ("key_epoch".into(), string(epoch.to_string())),
        ("key_hex".into(), string(encode_hex(&key))),
    ])
}

fn string(value: impl Into<String>) -> JsonValue {
    JsonValue::String(value.into())
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn read_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_be_bytes(bytes[offset..offset + 8].try_into().unwrap())
}

fn put_u32_bytes(output: &mut Vec<u8>, bytes: &[u8]) {
    output.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    output.extend_from_slice(bytes);
}

fn frame(fields: &[&[u8]]) -> Vec<u8> {
    let mut output = Vec::new();
    for field in fields {
        output.extend_from_slice(&(field.len() as u64).to_be_bytes());
        output.extend_from_slice(field);
    }
    output
}

fn uppercase_first_hash_digit(source: &str) -> String {
    let marker = "\"plaintext_hash_hex\":\"";
    let start = source.find(marker).unwrap() + marker.len();
    let mut bytes = source.as_bytes().to_vec();
    let position = (start..start + 64)
        .find(|index| (b'a'..=b'f').contains(&bytes[*index]))
        .unwrap();
    bytes[position] = bytes[position].to_ascii_uppercase();
    String::from_utf8(bytes).unwrap()
}

fn remove_base64_padding(source: &str, marker: &str) -> String {
    let start = source.find(marker).unwrap() + marker.len();
    let end = source[start..].find('"').unwrap() + start;
    assert_eq!(source.as_bytes()[end - 1], b'=');
    let mut output = source.to_owned();
    output.remove(end - 1);
    output
}

fn encode_base64(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = u32::from(chunk[0]);
        let second = u32::from(*chunk.get(1).unwrap_or(&0));
        let third = u32::from(*chunk.get(2).unwrap_or(&0));
        let word = (first << 16) | (second << 8) | third;
        output.push(char::from(BASE64[((word >> 18) & 63) as usize]));
        output.push(char::from(BASE64[((word >> 12) & 63) as usize]));
        output.push(if chunk.len() > 1 {
            char::from(BASE64[((word >> 6) & 63) as usize])
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            char::from(BASE64[(word & 63) as usize])
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
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 15)]));
    }
    output
}
