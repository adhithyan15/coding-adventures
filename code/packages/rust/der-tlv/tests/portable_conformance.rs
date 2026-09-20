use der_tlv::{
    decode_exact, decode_one, DerCursor, DerElement, DerError, DerErrorKind, DerLimits, TagClass,
};
use serde_json::{json, Value};
use std::path::PathBuf;

fn fixture() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/fixtures/der-tlv-v1/cases.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("read DER TLV fixture"))
        .expect("parse DER TLV fixture")
}

fn hex_bytes(text: &str) -> Vec<u8> {
    assert_eq!(text.len() % 2, 0, "fixture hex must contain whole bytes");
    text.as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair).expect("ASCII fixture hex");
            u8::from_str_radix(pair, 16).expect("valid fixture hex")
        })
        .collect()
}

fn materialize(segments: &Value) -> Vec<u8> {
    let mut result = Vec::new();
    for segment in segments.as_array().expect("input segments") {
        if let Some(text) = segment.get("hex").and_then(Value::as_str) {
            result.extend(hex_bytes(text));
        } else {
            let byte = hex_bytes(
                segment["repeat_hex"]
                    .as_str()
                    .expect("repeat_hex string"),
            );
            assert_eq!(byte.len(), 1);
            let count = segment["count"].as_u64().expect("repeat count") as usize;
            result.extend(std::iter::repeat_n(byte[0], count));
        }
    }
    result
}

fn limits(document: &Value, case: &Value) -> DerLimits {
    let defaults = &document["defaults"];
    let overrides = case.get("limits");
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

fn class_name(class: TagClass) -> &'static str {
    match class {
        TagClass::Universal => "universal",
        TagClass::Application => "application",
        TagClass::ContextSpecific => "context-specific",
        TagClass::Private => "private",
    }
}

fn error_id(kind: DerErrorKind) -> &'static str {
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

fn element_projection(element: DerElement<'_>, element_offset: usize) -> Value {
    let tag = element.tag();
    json!({
        "outcome": "element",
        "element_offset": element_offset,
        "tag": {
            "class": class_name(tag.class),
            "constructed": tag.constructed,
            "number": tag.number,
        },
        "header_len": element.header().len(),
        "encoded_len": element.encoded().len(),
        "remainder_offset": element_offset + element.encoded().len(),
    })
}

fn error_projection(error: DerError) -> Value {
    json!({
        "outcome": "error",
        "error_id": error_id(error.kind()),
        "offset": error.offset(),
    })
}

fn run_decode(case: &Value, input: &[u8], limits: DerLimits) -> Value {
    match case["operation"].as_str().expect("operation") {
        "decode-exact" => match decode_exact(input, limits) {
            Ok(element) => element_projection(element, 0),
            Err(error) => error_projection(error),
        },
        "decode-one" => match decode_one(input, limits) {
            Ok((element, remainder)) => {
                let projection = element_projection(element, 0);
                assert_eq!(
                    projection["remainder_offset"].as_u64().unwrap() as usize,
                    input.len() - remainder.len()
                );
                projection
            }
            Err(error) => error_projection(error),
        },
        operation => panic!("unsupported decode operation {operation}"),
    }
}

fn run_cursor(case: &Value, input: &[u8], limits: DerLimits) -> Value {
    let mut cursor = Some(DerCursor::new(input, limits).expect("fixture cursor construction"));
    let mut events = Vec::new();
    let mut final_elements = 0;
    let mut final_offset = 0;
    for action in case["actions"].as_array().expect("cursor actions") {
        let action = action.as_str().expect("cursor action string");
        let current = cursor.as_mut().expect("finish must be the final action");
        final_elements = current.elements_read();
        final_offset = input.len() - current.remaining().len();
        match action {
            "read" => match current.read() {
                Ok(Some(element)) => {
                    let offset = input.len() - current.remaining().len() - element.encoded().len();
                    events.push(element_projection(element, offset));
                }
                Ok(None) => events.push(json!({"outcome": "end"})),
                Err(error) => events.push(error_projection(error)),
            },
            "finish" => {
                let owned = cursor.take().expect("cursor available for finish");
                match owned.finish() {
                    Ok(()) => events.push(json!({"outcome": "finished"})),
                    Err(error) => events.push(error_projection(error)),
                }
            }
            other => panic!("unsupported cursor action {other}"),
        }
        if let Some(current) = cursor.as_ref() {
            final_elements = current.elements_read();
            final_offset = input.len() - current.remaining().len();
        }
    }
    json!({
        "events": events,
        "elements_read": final_elements,
        "remaining_offset": final_offset,
    })
}

#[test]
fn consumes_every_closed_der_tlv_case() {
    let document = fixture();
    let cases = document["cases"].as_array().expect("fixture cases");
    assert_eq!(cases.len(), 54, "closed DER TLV profile changed");
    assert_eq!(document["error_ids"].as_array().unwrap().len(), 17);

    for case in cases {
        let id = case["id"].as_str().expect("case id");
        let input = materialize(&case["input"]);
        let configured = limits(&document, case);
        let actual = if case["operation"] == "cursor" {
            run_cursor(case, &input, configured)
        } else {
            run_decode(case, &input, configured)
        };
        assert_eq!(actual, case["expected"], "{id}");

        if let Some(redacted) = case.get("redacted_input_hex").and_then(Value::as_str) {
            let error = decode_exact(&input, configured).expect_err("redaction case fails");
            assert!(!error.to_string().contains(redacted), "{id}: payload leaked");
        }
    }
}
