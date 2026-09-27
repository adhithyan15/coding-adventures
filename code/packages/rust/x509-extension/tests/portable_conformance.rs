use der_asn1::{Asn1Decoder, Asn1ErrorKind, Asn1Limits};
use der_tlv::{DerErrorKind, DerLimits};
use serde_json::{json, Map, Value};
use std::path::PathBuf;
use x509_extension::{decode_x509_extension, X509ExtensionErrorKind};

fn fixture(name: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/fixtures")
        .join(name)
        .join("cases.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("read fixture"))
        .expect("parse fixture")
}

fn hex_bytes(text: &str) -> Vec<u8> {
    assert_eq!(text.len() % 2, 0);
    text.as_bytes()
        .chunks(2)
        .map(|pair| {
            u8::from_str_radix(std::str::from_utf8(pair).expect("ASCII hex"), 16)
                .expect("valid hex")
        })
        .collect()
}

fn materialize(segments: &Value) -> Vec<u8> {
    let mut bytes = Vec::new();
    for segment in segments.as_array().expect("input segments") {
        if let Some(text) = segment.get("hex").and_then(Value::as_str) {
            bytes.extend(hex_bytes(text));
        } else {
            let byte = hex_bytes(segment["repeat_hex"].as_str().expect("repeat hex"));
            assert_eq!(byte.len(), 1);
            let count = segment["count"].as_u64().expect("repeat count") as usize;
            bytes.extend(std::iter::repeat_n(byte[0], count));
        }
    }
    bytes
}

fn der_limits(defaults: &Value, overrides: Option<&Value>) -> DerLimits {
    let value = |name: &str| {
        overrides
            .and_then(|item| item.get(name))
            .unwrap_or(&defaults[name])
    };
    DerLimits {
        max_input_len: value("max_input_len").as_u64().unwrap() as usize,
        max_value_len: value("max_value_len").as_u64().unwrap() as usize,
        max_elements: value("max_elements").as_u64().unwrap() as usize,
        max_tag_number: value("max_tag_number").as_u64().unwrap() as u32,
    }
}

fn limits(upstream: &Value, case: &Value) -> Asn1Limits {
    let defaults = &upstream["defaults"];
    let overrides = case.get("limits");
    let value = |name: &str| {
        overrides
            .and_then(|item| item.get(name))
            .unwrap_or(&defaults[name])
    };
    Asn1Limits {
        der: der_limits(&defaults["der"], overrides.and_then(|item| item.get("der"))),
        max_depth: value("max_depth").as_u64().unwrap() as usize,
        max_total_elements: value("max_total_elements").as_u64().unwrap() as usize,
        max_oid_arcs: value("max_oid_arcs").as_u64().unwrap() as usize,
    }
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn framing_id(kind: DerErrorKind) -> &'static str {
    match kind {
        DerErrorKind::EmptyInput => "empty-input",
        DerErrorKind::TruncatedHighTag => "truncated-high-tag",
        DerErrorKind::TruncatedLength => "truncated-length",
        DerErrorKind::TruncatedValue => "truncated-value",
        DerErrorKind::EndOfContents => "end-of-contents",
        DerErrorKind::NonMinimalTag => "non-minimal-tag",
        DerErrorKind::TagOverflow => "tag-overflow",
        DerErrorKind::IndefiniteLength => "indefinite-length",
        DerErrorKind::ReservedLength => "reserved-length",
        DerErrorKind::NonMinimalLength => "non-minimal-length",
        DerErrorKind::LengthTooWide => "length-too-wide",
        DerErrorKind::LengthHostOverflow => "length-host-overflow",
        DerErrorKind::InputLimitExceeded => "input-limit-exceeded",
        DerErrorKind::ValueLimitExceeded => "value-limit-exceeded",
        DerErrorKind::ElementLimitExceeded => "element-limit-exceeded",
        DerErrorKind::TagLimitExceeded => "tag-limit-exceeded",
        DerErrorKind::TrailingData => "trailing-data",
    }
}

fn asn1_ids(kind: Asn1ErrorKind) -> (&'static str, Option<&'static str>) {
    match kind {
        Asn1ErrorKind::Framing(kind) => ("framing", Some(framing_id(kind))),
        Asn1ErrorKind::UnexpectedTag => ("unexpected-tag", None),
        Asn1ErrorKind::DecoderLimitMismatch => ("decoder-limit-mismatch", None),
        Asn1ErrorKind::DepthLimitExceeded => ("depth-limit-exceeded", None),
        Asn1ErrorKind::ElementLimitExceeded => ("element-limit-exceeded", None),
        Asn1ErrorKind::InvalidBooleanLength => ("invalid-boolean-length", None),
        Asn1ErrorKind::InvalidBooleanValue => ("invalid-boolean-value", None),
        Asn1ErrorKind::EmptyInteger => ("empty-integer", None),
        Asn1ErrorKind::NonMinimalInteger => ("non-minimal-integer", None),
        Asn1ErrorKind::NegativeInteger => ("negative-integer", None),
        Asn1ErrorKind::IntegerOverflow => ("integer-overflow", None),
        Asn1ErrorKind::MissingUnusedBitCount => ("missing-unused-bit-count", None),
        Asn1ErrorKind::InvalidUnusedBitCount => ("invalid-unused-bit-count", None),
        Asn1ErrorKind::NonZeroBitPadding => ("non-zero-bit-padding", None),
        Asn1ErrorKind::BitLengthOverflow => ("bit-length-overflow", None),
        Asn1ErrorKind::NonEmptyNull => ("non-empty-null", None),
        Asn1ErrorKind::NonAsciiIa5String => ("non-ascii-ia5-string", None),
        Asn1ErrorKind::EmptyObjectIdentifier => ("empty-object-identifier", None),
        Asn1ErrorKind::UnterminatedObjectIdentifier => ("unterminated-object-identifier", None),
        Asn1ErrorKind::NonMinimalObjectIdentifier => ("non-minimal-object-identifier", None),
        Asn1ErrorKind::ObjectIdentifierOverflow => ("object-identifier-overflow", None),
        Asn1ErrorKind::OidArcLimitExceeded => ("oid-arc-limit-exceeded", None),
    }
}

fn error_projection(error: x509_extension::X509ExtensionError, elements_read: usize) -> Value {
    let (error_id, nested) = match error.kind() {
        X509ExtensionErrorKind::Structure(kind) => ("structure", Some(kind)),
        X509ExtensionErrorKind::MissingExtensionId => ("missing-extension-id", None),
        X509ExtensionErrorKind::InvalidExtensionId(kind) => ("invalid-extension-id", Some(kind)),
        X509ExtensionErrorKind::InvalidCritical(kind) => ("invalid-critical", Some(kind)),
        X509ExtensionErrorKind::EncodedDefaultCritical => ("encoded-default-critical", None),
        X509ExtensionErrorKind::MissingExtensionValue => ("missing-extension-value", None),
        X509ExtensionErrorKind::InvalidExtensionValue(kind) => {
            ("invalid-extension-value", Some(kind))
        }
        X509ExtensionErrorKind::TrailingElement => ("trailing-element", None),
    };
    let mut result = Map::from_iter([
        ("outcome".into(), json!("error")),
        ("error_id".into(), json!(error_id)),
        ("offset".into(), json!(error.offset())),
        ("offset_scope".into(), json!("extension-element")),
        ("elements_read".into(), json!(elements_read)),
    ]);
    if let Some(kind) = nested {
        let (asn1_id, framing) = asn1_ids(kind);
        result.insert("asn1_error_id".into(), json!(asn1_id));
        if let Some(id) = framing {
            result.insert("framing_error_id".into(), json!(id));
        }
    }
    Value::Object(result)
}

fn attempt(decoder: &mut Asn1Decoder, root: der_asn1::Asn1Element<'_>) -> Value {
    match decode_x509_extension(decoder, root) {
        Ok(extension) => json!({
            "outcome":"value",
            "extension_id_arcs_decimal":extension.extension_id().arcs().map(|arc| arc.to_string()).collect::<Vec<_>>(),
            "critical":extension.critical(),
            "extension_value_hex":to_hex(extension.extension_value()),
            "elements_read":decoder.elements_read(),
        }),
        Err(error) => error_projection(error, decoder.elements_read()),
    }
}

#[test]
fn consumes_every_closed_x509_extension_case() {
    let document = fixture("x509-extension-v1");
    let upstream = fixture("der-asn1-v1");
    let cases = document["cases"].as_array().expect("fixture cases");
    assert_eq!(cases.len(), 48, "closed Extension profile changed");
    assert_eq!(document["error_ids"].as_array().unwrap().len(), 8);

    for case in cases {
        let id = case["id"].as_str().expect("case id");
        let input = materialize(&case["input"]);
        let mut decoder = Asn1Decoder::new(limits(&upstream, case));
        let root = decoder
            .decode_exact(&input)
            .expect("fixture root is canonical DER");
        let actual = if case["operation"] == "extension-script" {
            let events = case["actions"]
                .as_array()
                .expect("script actions")
                .iter()
                .map(|action| {
                    assert_eq!(action, "decode");
                    attempt(&mut decoder, root)
                })
                .collect::<Vec<_>>();
            json!({"outcome":"script", "events":events})
        } else {
            attempt(&mut decoder, root)
        };
        assert_eq!(actual, case["expected"], "{id}");
        if let Some(hostile) = case.get("redacted_input_hex").and_then(Value::as_str) {
            assert!(
                !actual.to_string().contains(hostile),
                "{id}: payload leaked"
            );
        }
    }
}
