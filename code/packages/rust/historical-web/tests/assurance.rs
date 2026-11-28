//! Independent conformance checks for the bounded historical Web profile.
//!
//! The HTTP oracle below is intentionally a tiny framing model: it does not
//! call the production encoder/parser to decide whether a wire value is valid.
//! HTML fixtures are compared structurally, while tree integrity is checked
//! with an iterative graph walk so the verifier itself tolerates deep trees.

use coding_adventures_historical_web::{
    HtmlDocument, HtmlNodeKind, Http09Address, Http09RequestError, Http09ResponseError,
    decode_http09_response, encode_http09_request, parse_html_tags_1992, parse_http09_request,
};
use coding_adventures_test_bootstrap::fuzz::{
    CoverageBitmap, FuzzOptions, FuzzOutcome, fuzz_target,
};
use serde_json::{Value, json};
use std::collections::VecDeque;

const FIXTURE: &str = "../../../specs/fixtures/historical-web-v1/vectors.json";

fn fixture() -> Value {
    let raw = std::fs::read_to_string(FIXTURE).expect("shared fixture must be readable");
    serde_json::from_str(&raw).expect("shared fixture must be valid JSON")
}

fn hex_bytes(raw: &str) -> Vec<u8> {
    let (pairs, remainder) = raw.as_bytes().as_chunks::<2>();
    assert!(remainder.is_empty(), "odd-length fixture hex: {raw}");
    pairs
        .iter()
        .map(|pair| {
            let digit = |b: u8| match b {
                b'0'..=b'9' => b - b'0',
                b'a'..=b'f' => b - b'a' + 10,
                b'A'..=b'F' => b - b'A' + 10,
                _ => panic!("invalid hex byte in {raw}"),
            };
            digit(pair[0]) * 16 + digit(pair[1])
        })
        .collect()
}

fn children(kind: &HtmlNodeKind) -> &[usize] {
    match kind {
        HtmlNodeKind::Text(_) => &[],
        HtmlNodeKind::Element { children, .. } => children,
    }
}

/// A JSON-shaped independent view of the public arena, preserving valueless
/// attributes as JSON null and retaining the child order.
fn node_json(doc: &HtmlDocument, id: usize) -> Value {
    match &doc.nodes[id].kind {
        HtmlNodeKind::Text(text) => json!(text),
        HtmlNodeKind::Element {
            tag,
            attributes,
            children,
        } => {
            let attrs: serde_json::Map<String, Value> = attributes
                .iter()
                .map(|a| {
                    (
                        a.name.clone(),
                        a.value.clone().map(Value::String).unwrap_or(Value::Null),
                    )
                })
                .collect();
            json!({
                "tag": tag,
                "attributes": attrs,
                "children": children.iter().map(|&child| node_json(doc, child)).collect::<Vec<_>>(),
            })
        }
    }
}

fn doc_json(doc: &HtmlDocument) -> Value {
    Value::Array(doc.roots.iter().map(|&id| node_json(doc, id)).collect())
}

fn assert_arena_well_formed(doc: &HtmlDocument) {
    let mut owners = vec![0usize; doc.nodes.len()];
    let mut seen = vec![false; doc.nodes.len()];
    let mut queue = VecDeque::new();
    for &root in &doc.roots {
        assert!(root < doc.nodes.len(), "root index out of bounds");
        owners[root] += 1;
        queue.push_back(root);
    }
    while let Some(id) = queue.pop_front() {
        assert!(!seen[id], "cycle or aliased child at node {id}");
        seen[id] = true;
        if let HtmlNodeKind::Element {
            tag, attributes, ..
        } = &doc.nodes[id].kind
        {
            assert!(!tag.is_empty());
            assert!(tag.bytes().all(|b| !b.is_ascii_uppercase()));
            let mut names = std::collections::HashSet::new();
            for attr in attributes {
                assert!(names.insert(&attr.name), "duplicate recognized attribute");
            }
        }
        for &child in children(&doc.nodes[id].kind) {
            assert!(child < doc.nodes.len(), "child index out of bounds");
            owners[child] += 1;
            queue.push_back(child);
        }
    }
    assert!(seen.iter().all(|x| *x), "unreachable arena node");
    assert!(
        owners.iter().all(|n| *n == 1),
        "each node has exactly one owner"
    );
}

#[test]
fn replay_every_shared_vector_and_fail_closed_on_missing_categories() {
    let v = fixture();
    let http = &v["http-0.9"];
    assert!(http["requests"].as_array().is_some_and(|x| !x.is_empty()));
    assert!(http["responses"].as_array().is_some_and(|x| !x.is_empty()));
    let docs = v["html-tags-1992"]
        .as_array()
        .expect("HTML vectors category");
    let exchanges = v["exchanges"]
        .as_array()
        .expect("exchange vectors category");
    assert!(!docs.is_empty() && !exchanges.is_empty());

    for tc in http["requests"].as_array().unwrap() {
        let address = tc["address"].as_str().expect("address string");
        let want = hex_bytes(tc["wire_hex"].as_str().expect("wire hex"));
        let address_value = Http09Address::new(address.to_owned()).unwrap();
        let lf_wire = [b"GET ".as_slice(), address.as_bytes(), b"\n"].concat();
        let crlf_wire = [b"GET ".as_slice(), address.as_bytes(), b"\r\n"].concat();
        assert!(
            want == lf_wire || want == crlf_wire,
            "fixture wire must match the shared LF/CRLF vector"
        );
        assert_eq!(
            encode_http09_request(&address_value),
            [b"GET ".as_slice(), address.as_bytes(), b"\r\n"].concat(),
            "encoder emits canonical CRLF even when fixture accepts LF: {tc}"
        );
        assert_eq!(
            parse_http09_request(&want).unwrap().as_str(),
            address,
            "{tc}"
        );
    }
    for address in http["invalid_addresses"].as_array().unwrap() {
        assert!(
            Http09Address::new(address.as_str().unwrap().to_owned()).is_err(),
            "{address}"
        );
    }
    for wire in http["invalid_request_hex"].as_array().unwrap() {
        assert!(
            parse_http09_request(&hex_bytes(wire.as_str().unwrap())).is_err(),
            "{wire}"
        );
    }
    for tc in http["responses"].as_array().unwrap() {
        assert_eq!(
            decode_http09_response(&hex_bytes(tc["wire_hex"].as_str().unwrap())).unwrap(),
            tc["body"].as_str().unwrap()
        );
    }
    for wire in http["invalid_response_hex"].as_array().unwrap() {
        assert!(
            decode_http09_response(&hex_bytes(wire.as_str().unwrap())).is_err(),
            "{wire}"
        );
    }

    for tc in docs {
        let doc = parse_html_tags_1992(tc["source"].as_str().unwrap())
            .unwrap_or_else(|e| panic!("{}: {e}", tc["id"]));
        assert_arena_well_formed(&doc);
        assert_eq!(doc_json(&doc), tc["tree"], "{}", tc["id"]);
    }
    for tc in exchanges {
        let request = hex_bytes(tc["request_hex"].as_str().unwrap());
        assert_eq!(
            parse_http09_request(&request).unwrap().as_str(),
            tc["address"].as_str().unwrap()
        );
        let body =
            decode_http09_response(&hex_bytes(tc["response_hex"].as_str().unwrap())).unwrap();
        let doc = parse_html_tags_1992(&body).unwrap();
        assert_arena_well_formed(&doc);
        assert_eq!(doc_json(&doc), tc["tree"]);
    }
}

fn independent_wire_oracle(wire: &[u8]) -> Option<&[u8]> {
    if wire.len() < 6 || !wire.starts_with(b"GET ") || wire.last() != Some(&b'\n') {
        return None;
    }
    let mut end = wire.len() - 1;
    if end > 0 && wire[end - 1] == b'\r' {
        end -= 1;
    }
    let address = &wire[4..end];
    if address.is_empty() || address.iter().any(|b| !(0x21..=0x7e).contains(b)) {
        None
    } else {
        Some(address)
    }
}

#[test]
fn exhaust_all_short_http_payloads_with_both_line_endings() {
    // 2 * (1 + 256 + 65,536) = 131,586 framing/payload pairs.
    let mut cases = 0usize;
    for len in 0..=2usize {
        let count = 256usize.pow(len as u32);
        for value in 0..count {
            let payload: Vec<u8> = (0..len).map(|i| (value >> (8 * i)) as u8).collect();
            for ending in [&b"\n"[..], &b"\r\n"[..]] {
                let mut wire = b"GET ".to_vec();
                wire.extend_from_slice(&payload);
                wire.extend_from_slice(ending);
                match independent_wire_oracle(&wire) {
                    Some(want) => assert_eq!(
                        parse_http09_request(&wire).unwrap().as_str().as_bytes(),
                        want,
                        "{wire:?}"
                    ),
                    None => assert!(parse_http09_request(&wire).is_err(), "accepted {wire:?}"),
                }
                cases += 1;
            }
        }
    }
    assert_eq!(cases, 131_586);
}

#[test]
fn deterministic_properties_cover_request_and_response_algebras() {
    // Small explicit PRNG makes this test reproducible without a dependency.
    let mut state = 0x1992_1103_d00d_f00du64;
    for _ in 0..1_000 {
        let len = (state as usize % 64) + 1;
        let mut address = String::with_capacity(len);
        for _ in 0..len {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            address.push(char::from(0x21 + (state as u8 % 0x5e)));
        }
        let value = Http09Address::new(address.clone()).unwrap();
        let wire = encode_http09_request(&value);
        assert_eq!(wire, format!("GET {address}\r\n").into_bytes());
        assert_eq!(parse_http09_request(&wire).unwrap().as_str(), address);
    }
    for len in 0..=256usize {
        let body: Vec<u8> = (0..len)
            .map(|i| (i.wrapping_mul(37) & 0x7f) as u8)
            .collect();
        assert_eq!(decode_http09_response(&body).unwrap().as_bytes(), body);
    }
}

#[test]
fn public_errors_and_arena_node_accessor_return_typed_results() {
    assert_eq!(
        Http09Address::new("/bad address"),
        Err(Http09RequestError::InvalidAddress)
    );
    assert_eq!(
        Http09RequestError::InvalidAddress.to_string(),
        "invalid HTTP/0.9 document address"
    );
    assert_eq!(
        parse_http09_request(b"POST /\n"),
        Err(Http09RequestError::InvalidRequest)
    );
    assert_eq!(
        Http09RequestError::InvalidRequest.to_string(),
        "invalid HTTP/0.9 request"
    );
    assert_eq!(
        decode_http09_response(&[0x80]),
        Err(Http09ResponseError::NonAscii)
    );
    assert_eq!(
        Http09ResponseError::NonAscii.to_string(),
        "HTTP/0.9 response is not ASCII"
    );

    let doc = parse_html_tags_1992("<A>linked</A>").unwrap();
    let root = doc.roots[0];
    assert!(matches!(
        doc.node(root).map(|node| &node.kind),
        Some(HtmlNodeKind::Element { tag, .. }) if tag == "a"
    ));
    assert!(doc.node(doc.nodes.len()).is_none());
}

#[test]
fn recognized_anchor_attributes_keep_first_valueless_and_quoted_values() {
    let doc = parse_html_tags_1992("<A HREF HREF=/later NAME='a &amp; b' TYPE=\"UP>DOWN\">x</A>")
        .unwrap();
    let HtmlNodeKind::Element {
        tag, attributes, ..
    } = &doc.nodes[doc.roots[0]].kind
    else {
        panic!("expected anchor")
    };
    assert_eq!(tag, "a");
    assert_eq!(
        attributes.iter().find(|a| a.name == "href").unwrap().value,
        None
    );
    assert_eq!(
        attributes
            .iter()
            .find(|a| a.name == "name")
            .unwrap()
            .value
            .as_deref(),
        Some("a & b")
    );
    assert_eq!(
        attributes
            .iter()
            .find(|a| a.name == "type")
            .unwrap()
            .value
            .as_deref(),
        Some("UP>DOWN")
    );
}

#[test]
fn archived_sgml_list_omission_utf8_and_single_pass_entities_are_preserved() {
    let doc =
        parse_html_tags_1992("<UL><LI>one<LI>two</UL><P>café &amp; &lt; &unknown;</P>").unwrap();
    assert_arena_well_formed(&doc);
    let json = doc_json(&doc);
    assert_eq!(json[0]["children"][1]["children"][0], "two");
    assert_eq!(json[1], json!({"tag":"p", "attributes":{}, "children":[]}));
    assert_eq!(json[2], "café & < &unknown;");
    // HTML 1992 permits this local list-item omission; XML's explicit-end-tag
    // grammar would reject the same source, so do not route through XML here.
    assert_eq!(json[0]["tag"], "ul");
}

#[test]
fn raw_text_end_tag_search_handles_multibyte_unaligned_window_endpoints() {
    // Each multibyte `é` lands beside a candidate search-window boundary. A
    // byte-oriented scan must never slice the UTF-8 source at that boundary.
    // Both raw-text elements must retain the complete body and resume normal
    // tokenization immediately after their matching end tags.
    let source = "<XMP>aaaaaé</XMP><P>after-xmp</P><LISTING>aaaaaaaé</LISTING><P>after-listing</P>";
    let doc = parse_html_tags_1992(source).expect("raw-text scan must preserve UTF-8 boundaries");
    assert_arena_well_formed(&doc);
    assert_eq!(
        doc_json(&doc),
        json!([
            {"tag":"xmp", "attributes":{}, "children":["aaaaaé"]},
            {"tag":"p", "attributes":{}, "children":[]},
            "after-xmp",
            {"tag":"listing", "attributes":{}, "children":["aaaaaaaé"]},
            {"tag":"p", "attributes":{}, "children":[]},
            "after-listing"
        ])
    );
}

#[test]
fn deep_malformed_html_parses_and_arena_drop_is_non_recursive() {
    let mut source = String::new();
    for _ in 0..20_000 {
        source.push_str("<A>");
    }
    for _ in 0..20_000 {
        source.push_str("</TITLE>");
    }
    source.push_str("tail");
    let doc = parse_html_tags_1992(&source).expect("malformed closes should recover");
    assert_arena_well_formed(&doc);
    drop(doc);
}

#[test]
fn repository_fuzz_engine_exercises_wire_and_markup_with_independent_checks() {
    let seeds = [
        b"GET /\r\n".to_vec(),
        b"GET /bad path\n".to_vec(),
        b"POST /\n".to_vec(),
        b"<A HREF='x'>ok</A>".to_vec(),
        b"<!--x--><UL><LI>one<LI>two</UL>".to_vec(),
        vec![0xff, b'<', b'A', b'>'],
    ];
    let options = FuzzOptions {
        max_iterations: 2_048,
        seed: 0x4857_4542_3033,
        max_input_len: 256,
        base_energy: 10,
    };
    let report = fuzz_target(&seeds, &options, |input, coverage: &mut CoverageBitmap| {
        coverage.record_edge(input.len().min(255));
        // Check the wire oracle for every mutation, including inputs that do
        // not begin with the method prefix. A prefix gate would leave most of
        // the generated byte space outside the parser's oracle.
        coverage.record_edge(256);
        let wire_matches = match (independent_wire_oracle(input), parse_http09_request(input)) {
            (Some(want), Ok(got)) => got.as_str().as_bytes() == want,
            (None, Err(_)) => true,
            _ => false,
        };
        if !wire_matches {
            return FuzzOutcome::Crash {
                kind: "wire-oracle-mismatch".into(),
                frames: vec!["assurance".into()],
            };
        }

        // HTML has a UTF-8 string API. Fuzz every valid UTF-8 mutation and
        // require parsing to succeed: malformed profile markup is recovered,
        // while parser errors are reserved for impossible internal states.
        // Invalid byte sequences remain covered by HTTP but are not silently
        // converted with replacement characters into different HTML inputs.
        if let Ok(source) = std::str::from_utf8(input) {
            coverage.record_edge(257);
            match parse_html_tags_1992(source) {
                Ok(doc) => {
                    assert_arena_well_formed(&doc);
                    FuzzOutcome::Pass
                }
                Err(_) => FuzzOutcome::Crash {
                    kind: "valid-utf8-html-error".into(),
                    frames: vec!["assurance".into()],
                },
            }
        } else {
            FuzzOutcome::Pass
        }
    });
    assert_eq!(report.iterations, options.max_iterations);
    assert!(
        report.crashes.is_empty(),
        "fuzz crashes: {:?}",
        report.crashes
    );
    assert_eq!(report.timeouts, 0);
    assert_eq!(report.ooms, 0);

    let response_seeds = [
        vec![],
        b"HTTP/1.0 500\r\n".to_vec(),
        vec![0x80],
        vec![0, 0x7f, b'\n'],
    ];
    let response_report = fuzz_target(
        &response_seeds,
        &options,
        |body, coverage: &mut CoverageBitmap| {
            coverage.record_edge(body.len().min(255));
            let ascii = body.iter().all(|b| *b < 0x80);
            match decode_http09_response(body) {
                Ok(text) if ascii && text.as_bytes() == body => FuzzOutcome::Pass,
                Err(_) if !ascii => FuzzOutcome::Pass,
                _ => FuzzOutcome::Crash {
                    kind: "response-oracle-mismatch".into(),
                    frames: vec!["assurance".into()],
                },
            }
        },
    );
    assert_eq!(response_report.iterations, options.max_iterations);
    assert!(
        response_report.crashes.is_empty(),
        "response fuzz crashes: {:?}",
        response_report.crashes
    );
    assert_eq!(response_report.timeouts, 0);
    assert_eq!(response_report.ooms, 0);
}
