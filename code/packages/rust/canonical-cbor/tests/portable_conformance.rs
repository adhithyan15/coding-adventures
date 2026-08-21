//! Execute every language-neutral CBR01 vector against the Rust reference.

use coding_adventures_canonical_cbor::{decode, try_encode, try_encode_into, CborError, CborValue};

const PROJECTION: &str =
    include_str!("../../../../specs/fixtures/canonical-cbor-v1/canonical_cbor_vectors.h");

#[derive(Debug)]
struct Vector<'a> {
    id: &'a str,
    operation: &'a str,
    input: &'a str,
    expected: &'a str,
}

fn vectors() -> Vec<Vector<'static>> {
    PROJECTION
        .lines()
        .filter_map(|line| {
            let row = line.trim().strip_prefix("{\"")?.strip_suffix("\"},")?;
            let fields: Vec<_> = row.split("\", \"").collect();
            assert_eq!(fields.len(), 4, "malformed generated fixture row");
            Some(Vector {
                id: fields[0],
                operation: fields[1],
                input: fields[2],
                expected: fields[3],
            })
        })
        .collect()
}

fn hex_decode(text: &str) -> Vec<u8> {
    fn nibble(byte: u8) -> u8 {
        match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            _ => panic!("fixture hex must be lowercase"),
        }
    }
    text.as_bytes()
        .as_chunks::<2>().0.iter()
        .map(|pair| (nibble(pair[0]) << 4) | nibble(pair[1]))
        .collect()
}

fn error_id(error: &CborError) -> &'static str {
    match error {
        CborError::UnexpectedEof => "unexpected-eof",
        CborError::TrailingBytes => "trailing-bytes",
        CborError::Reserved => "reserved",
        CborError::Indefinite => "indefinite",
        CborError::NonMinimalInteger => "non-minimal-integer",
        CborError::InvalidUtf8 => "invalid-utf8",
        CborError::NonCanonicalMapOrder => "non-canonical-map-order",
        CborError::UnsupportedSimple => "unsupported-simple",
        CborError::FloatNotSupported => "float-not-supported",
        CborError::TooDeep => "too-deep",
        CborError::LengthTooLarge => "length-too-large",
        CborError::DuplicateMapKey => "duplicate-map-key",
        CborError::EncodeTooDeep => "encode-too-deep",
        CborError::EncodeTooLarge => "encode-too-large",
    }
}

fn nested_array(depth: usize) -> CborValue {
    let mut value = CborValue::Null;
    for _ in 0..depth {
        value = CborValue::Array(vec![value]);
    }
    value
}

fn build_generated(spec: &str) -> CborValue {
    if let Some(depth) = spec.strip_prefix("nested-array:") {
        return nested_array(depth.parse().expect("fixture depth"));
    }
    let rest = spec
        .strip_prefix("bytes-repeat:")
        .expect("closed generated value");
    let (length, byte) = rest.split_once(':').expect("repeat fields");
    CborValue::Bytes(vec![
        hex_decode(byte)[0];
        length.parse().expect("fixture length")
    ])
}

fn build_wire(spec: &str) -> Vec<u8> {
    if let Some(depth) = spec.strip_prefix("wire:nested-array:") {
        let mut wire = vec![0x81; depth.parse().expect("fixture depth")];
        wire.push(0xF6);
        return wire;
    }
    let rest = spec
        .strip_prefix("wire:bytes-repeat:")
        .expect("closed generated wire");
    let (length, byte) = rest.split_once(':').expect("repeat fields");
    let length: usize = length.parse().expect("fixture length");
    let mut wire = match length {
        0..=23 => vec![0x40 | length as u8],
        24..=0xFF => vec![0x58, length as u8],
        0x100..=0xFFFF => vec![0x59, (length >> 8) as u8, length as u8],
        _ => vec![
            0x5A,
            (length >> 24) as u8,
            (length >> 16) as u8,
            (length >> 8) as u8,
            length as u8,
        ],
    };
    wire.extend(std::iter::repeat_n(hex_decode(byte)[0], length));
    wire
}

fn build_map(spec: &str) -> CborValue {
    CborValue::Map(
        spec.split(';')
            .map(|fragment| {
                let (key, value) = fragment.split_once('=').expect("map fragment");
                (
                    decode(&hex_decode(key)).expect("fixture key"),
                    decode(&hex_decode(value)).expect("fixture value"),
                )
            })
            .collect(),
    )
}

#[test]
fn portable_conformance_vectors() {
    let vectors = vectors();
    assert_eq!(vectors.len(), 55);
    for vector in vectors {
        match vector.operation {
            "round-trip" => {
                let value = decode(&hex_decode(vector.input)).expect(vector.id);
                assert_eq!(
                    try_encode(&value).expect(vector.id),
                    hex_decode(vector.expected)
                );
            }
            "decode-error" => {
                let wire = if let Some(depth) = vector.input.strip_prefix("nested-array-wire:") {
                    build_wire(&format!("wire:nested-array:{depth}"))
                } else {
                    hex_decode(vector.input)
                };
                let error = decode(&wire).expect_err(vector.id);
                assert_eq!(error_id(&error), vector.expected, "{}", vector.id);
                assert!(error.to_string().starts_with("canonical-cbor:"));
            }
            operation => {
                let value = if operation == "encode-map" {
                    build_map(vector.input)
                } else if vector.input == "duplicate-map-key" {
                    build_map("6161=00;6161=01")
                } else {
                    build_generated(vector.input)
                };
                let result = try_encode(&value);
                if operation == "encode-error" {
                    let error = result.expect_err(vector.id);
                    assert_eq!(error_id(&error), vector.expected, "{}", vector.id);
                    assert!(error.to_string().starts_with("canonical-cbor:"));
                    let mut destination = vec![0xAA];
                    assert!(try_encode_into(&value, &mut destination).is_err());
                    assert_eq!(destination, vec![0xAA], "{}", vector.id);
                } else {
                    let expected = if operation == "encode-map" {
                        hex_decode(vector.expected)
                    } else {
                        build_wire(vector.expected)
                    };
                    assert_eq!(result.expect(vector.id), expected, "{}", vector.id);
                }
            }
        }
    }
}
