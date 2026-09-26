use der_asn1::{
    decode_bit_string, decode_boolean, decode_ia5_string, decode_implicit_ia5_string,
    decode_implicit_object_identifier, decode_implicit_octet_string, decode_integer, decode_null,
    decode_object_identifier, decode_octet_string, Asn1Decoder, Asn1Element, Asn1Error,
    Asn1ErrorKind, Asn1Limits,
};
use der_tlv::{DerErrorKind, DerLimits, TagClass};
use serde_json::{json, Map, Value};
use std::path::PathBuf;

fn fixture(name: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/fixtures")
        .join(name)
        .join("cases.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("read fixture"))
        .expect("parse fixture")
}

fn hex_bytes(text: &str) -> Vec<u8> {
    assert_eq!(text.len() % 2, 0, "fixture hex must contain whole bytes");
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let pair = std::str::from_utf8(pair).expect("ASCII fixture hex");
            u8::from_str_radix(pair, 16).expect("valid fixture hex")
        })
        .collect()
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn materialize(segments: &Value) -> Vec<u8> {
    let mut result = Vec::new();
    for segment in segments.as_array().expect("input segments") {
        if let Some(text) = segment.get("hex").and_then(Value::as_str) {
            result.extend(hex_bytes(text));
        } else {
            let byte = hex_bytes(segment["repeat_hex"].as_str().expect("repeat_hex"));
            assert_eq!(byte.len(), 1);
            let count = segment["count"].as_u64().expect("repeat count") as usize;
            result.extend(std::iter::repeat_n(byte[0], count));
        }
    }
    result
}

fn der_limits(defaults: &Value, overrides: Option<&Value>) -> DerLimits {
    let value = |name: &str| {
        overrides
            .and_then(|item| item.get(name))
            .unwrap_or(&defaults[name])
    };
    let max_value = value("max_value_len");
    DerLimits {
        max_input_len: value("max_input_len").as_u64().expect("max_input_len") as usize,
        max_value_len: if max_value.as_str() == Some("host-max") {
            usize::MAX
        } else {
            max_value.as_u64().expect("max_value_len") as usize
        },
        max_elements: value("max_elements").as_u64().expect("max_elements") as usize,
        max_tag_number: value("max_tag_number").as_u64().expect("max_tag_number") as u32,
    }
}

fn limits(document: &Value, case: &Value) -> Asn1Limits {
    let defaults = &document["defaults"];
    let overrides = case.get("limits");
    let value = |name: &str| {
        overrides
            .and_then(|item| item.get(name))
            .unwrap_or(&defaults[name])
    };
    Asn1Limits {
        der: der_limits(&defaults["der"], overrides.and_then(|item| item.get("der"))),
        max_depth: value("max_depth").as_u64().expect("max_depth") as usize,
        max_total_elements: value("max_total_elements")
            .as_u64()
            .expect("max_total_elements") as usize,
        max_oid_arcs: value("max_oid_arcs").as_u64().expect("max_oid_arcs") as usize,
    }
}

fn class_name(class: TagClass) -> &'static str {
    match class {
        TagClass::Universal => "universal",
        TagClass::Application => "application",
        TagClass::ContextSpecific => "context-specific",
        TagClass::Private => "private",
    }
}

fn der_error_id(kind: DerErrorKind) -> &'static str {
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

fn error_id(kind: Asn1ErrorKind) -> (&'static str, Option<&'static str>) {
    match kind {
        Asn1ErrorKind::Framing(kind) => ("framing", Some(der_error_id(kind))),
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

fn tag_projection(element: Asn1Element<'_>) -> Value {
    let tag = element.tag();
    json!({
        "class": class_name(tag.class),
        "constructed": tag.constructed,
        "number": tag.number,
    })
}

fn failure(error: Asn1Error, scope: &str) -> Value {
    let (id, framing) = error_id(error.kind());
    let mut result = Map::from_iter([
        ("outcome".into(), json!("error")),
        ("error_id".into(), json!(id)),
        ("offset".into(), json!(error.offset())),
        ("offset_scope".into(), json!(scope)),
    ]);
    if let Some(framing) = framing {
        result.insert("framing_error_id".into(), json!(framing));
    }
    Value::Object(result)
}

fn verify_upstream(upstream: &Value, case: &Value) -> Value {
    let id = case["der_tlv_case_id"].as_str().expect("DER TLV case id");
    let referenced = upstream["cases"]
        .as_array()
        .expect("upstream cases")
        .iter()
        .find(|candidate| candidate["id"] == id)
        .expect("referenced DER TLV case");
    let input = materialize(&referenced["input"]);
    let configured = Asn1Limits {
        der: der_limits(&upstream["defaults"], referenced.get("limits")),
        ..Asn1Limits::default()
    };
    let mut decoder = Asn1Decoder::new(configured);
    match decoder.decode_exact(&input) {
        Ok(element) => {
            let expected = &referenced["expected"];
            assert_eq!(expected["outcome"], "element", "{id}");
            assert_eq!(tag_projection(element), expected["tag"], "{id}");
            assert_eq!(element.header().len(), expected["header_len"], "{id}");
            assert_eq!(element.encoded().len(), expected["encoded_len"], "{id}");
            assert_eq!(input.len(), expected["remainder_offset"], "{id}");
        }
        Err(error) => {
            let expected = &referenced["expected"];
            assert_eq!(expected["outcome"], "error", "{id}");
            let (_, framing) = error_id(error.kind());
            assert_eq!(framing, expected["error_id"].as_str(), "{id}");
            assert_eq!(error.offset(), expected["offset"], "{id}");
        }
    }
    json!({"outcome": "upstream"})
}

fn primitive(
    operation: &str,
    element: Asn1Element<'_>,
    configured: Asn1Limits,
    tag_number: Option<u32>,
) -> Result<Value, Asn1Error> {
    Ok(match operation {
        "decode-boolean" => {
            json!({"outcome":"value", "boolean":decode_boolean(element)?, "elements_read":1})
        }
        "decode-integer" | "integer-to-u64" => {
            let integer = decode_integer(element)?;
            let mut value = Map::from_iter([
                ("outcome".into(), json!("value")),
                ("signed_hex".into(), json!(to_hex(integer.signed_bytes()))),
                ("negative".into(), json!(integer.is_negative())),
            ]);
            if operation == "integer-to-u64" {
                value.insert("u64_decimal".into(), json!(integer.to_u64()?.to_string()));
            }
            Value::Object(value)
        }
        "decode-bit-string" => {
            let bits = decode_bit_string(element)?;
            json!({"outcome":"value", "bytes_hex":to_hex(bits.bytes()), "unused_bits":bits.unused_bits(), "bit_length":bits.bit_len()})
        }
        "decode-octet-string" => {
            json!({"outcome":"value", "bytes_hex":to_hex(decode_octet_string(element)?)})
        }
        "decode-implicit-octet-string" => json!({
            "outcome":"value",
            "bytes_hex":to_hex(decode_implicit_octet_string(element, tag_number.expect("tag number"))?)
        }),
        "decode-ia5-string" => {
            json!({"outcome":"value", "text":decode_ia5_string(element)?})
        }
        "decode-implicit-ia5-string" => json!({
            "outcome":"value",
            "text":decode_implicit_ia5_string(element, tag_number.expect("tag number"))?
        }),
        "decode-null" => {
            decode_null(element)?;
            json!({"outcome":"value"})
        }
        "decode-object-identifier" | "decode-implicit-object-identifier" => {
            let oid = if operation == "decode-object-identifier" {
                decode_object_identifier(element, configured)?
            } else {
                decode_implicit_object_identifier(
                    element,
                    tag_number.expect("tag number"),
                    configured,
                )?
            };
            let arcs: Vec<String> = oid.arcs().map(|arc| arc.to_string()).collect();
            json!({"outcome":"value", "bytes_hex":to_hex(oid.encoded()), "arcs_decimal":arcs, "arc_count":oid.arc_count()})
        }
        other => panic!("unsupported primitive operation {other}"),
    })
}

fn cursor_case(
    case: &Value,
    decoder: &mut Asn1Decoder,
    root: Asn1Element<'_>,
) -> Result<Value, Asn1Error> {
    let mut cursor = Some(decoder.sequence(root)?);
    let total = cursor.as_ref().expect("cursor").remaining().len();
    let mut events = Vec::new();
    for action in case["actions"].as_array().expect("cursor actions") {
        match action.as_str().expect("action") {
            "finish" => {
                let owned = cursor.take().expect("finish is final");
                events.push(match owned.finish() {
                    Ok(()) => json!({"outcome":"finished"}),
                    Err(error) => failure(error, "container-value"),
                });
            }
            "read-with-different-limits" => {
                let mut mismatch = decoder.limits();
                mismatch.max_total_elements += 1;
                let mut other = Asn1Decoder::new(mismatch);
                let current = cursor.as_mut().expect("cursor");
                events.push(match current.read(&mut other) {
                    Ok(Some(element)) => {
                        json!({"outcome":"value", "tag":tag_projection(element), "depth":element.depth()})
                    }
                    Ok(None) => json!({"outcome":"end"}),
                    Err(error) => failure(error, "container-value"),
                });
            }
            "read" => {
                let current = cursor.as_mut().expect("cursor");
                events.push(match current.read(decoder) {
                    Ok(Some(element)) => {
                        json!({"outcome":"value", "tag":tag_projection(element), "depth":element.depth()})
                    }
                    Ok(None) => json!({"outcome":"end"}),
                    Err(error) => failure(error, "container-value"),
                });
            }
            "read-nested-sequence" => {
                let current = cursor.as_mut().expect("cursor");
                events.push(match current.read(decoder) {
                    Ok(Some(element)) => match decoder.sequence(element) {
                        Ok(mut nested) => match nested.read(decoder) {
                            Ok(Some(grandchild)) => match nested.finish() {
                                Ok(()) => json!({"outcome":"value", "tag":tag_projection(grandchild), "depth":grandchild.depth()}),
                                Err(error) => failure(error, "container-value"),
                            },
                            Ok(None) => panic!("nested sequence must contain one child"),
                            Err(error) => failure(error, "container-value"),
                        },
                        Err(error) => failure(error, "container-value"),
                    },
                    Ok(None) => panic!("outer sequence must contain a nested child"),
                    Err(error) => failure(error, "container-value"),
                });
            }
            other => panic!("unsupported cursor action {other}"),
        }
    }
    let remaining = cursor.as_ref().map_or(0, |value| value.remaining().len());
    Ok(json!({
        "outcome":"value",
        "elements_read":decoder.elements_read(),
        "remaining_offset":total - remaining,
        "events":events,
    }))
}

fn run_case(document: &Value, upstream: &Value, case: &Value) -> Value {
    if case.get("der_tlv_case_id").is_some() {
        return verify_upstream(upstream, case);
    }
    let configured = limits(document, case);
    let input = materialize(&case["input"]);
    let mut decoder = Asn1Decoder::new(configured);
    let operation = case["operation"].as_str().expect("operation");
    let result = (|| {
        let root = decoder.decode_exact(&input)?;
        Ok(match operation {
            "decode-exact" => json!({
                "outcome":"value", "tag":tag_projection(root),
                "header_hex":to_hex(root.header()), "value_hex":to_hex(root.value()),
                "encoded_hex":to_hex(root.encoded()), "depth":root.depth(),
                "elements_read":decoder.elements_read(),
            }),
            "cursor-script" => return cursor_case(case, &mut decoder, root),
            "sequence" | "set" => {
                let cursor = if operation == "sequence" {
                    decoder.sequence(root)?
                } else {
                    decoder.set(root)?
                };
                json!({"outcome":"value", "elements_read":decoder.elements_read(), "remaining_offset":root.value().len() - cursor.remaining().len()})
            }
            "explicit" => {
                let child = decoder.explicit(
                    root,
                    case["tag_number"].as_u64().expect("tag number") as u32,
                )?;
                json!({"outcome":"value", "tag":tag_projection(child), "value_hex":to_hex(child.value()), "depth":child.depth(), "elements_read":decoder.elements_read()})
            }
            _ => primitive(
                operation,
                root,
                configured,
                case.get("tag_number")
                    .and_then(Value::as_u64)
                    .map(|n| n as u32),
            )?,
        })
    })();
    match result {
        Ok(value) => value,
        Err(error) => {
            let scope =
                if operation == "explicit" && matches!(error.kind(), Asn1ErrorKind::Framing(_)) {
                    "container-value"
                } else {
                    "operation-input"
                };
            failure(error, scope)
        }
    }
}

#[test]
fn consumes_every_closed_der_asn1_case() {
    let document = fixture("der-asn1-v1");
    let upstream = fixture("der-tlv-v1");
    let cases = document["cases"].as_array().expect("fixture cases");
    assert_eq!(cases.len(), 122, "closed DER ASN.1 profile changed");
    assert_eq!(document["error_ids"].as_array().unwrap().len(), 22);

    for case in cases {
        let id = case["id"].as_str().expect("case id");
        let actual = run_case(&document, &upstream, case);
        assert_eq!(actual, case["expected"], "{id}");
        if let Some(redacted) = case.get("redacted_input_hex").and_then(Value::as_str) {
            assert!(
                !actual.to_string().contains(redacted),
                "{id}: payload leaked"
            );
        }
    }
}
