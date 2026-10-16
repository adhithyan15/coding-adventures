//! D3a baseline controls: generic checked import preserves arbitrary metadata,
//! while its existing raw decoder already rejects nested decoded duplicates.
//! These are passing characterization controls, not a new-gate semantic red.
use coding_adventures_correlation_vector::{CVLog, GraphLimits, Origin};
use serde_json::{json, Value};
use std::collections::HashMap;

const NS: &str = "closurec_source_v1";
fn snapshot() -> (String, String) {
    let mut log = CVLog::new_compact(true);
    let hash = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    let root=log.try_create(Some(Origin{source:"input_file".into(),location:"a.js".into(),timestamp:None,
        meta:HashMap::from([("byte_len".into(),json!(0)),("content_sha256".into(),json!(hash)),
            (NS.into(),json!({"schema":"1","role":"input","buffer_domain":"read_to_string_utf8","encoding":"utf8","grammar":"javascript_standard","edition":"es2025","sha256":hash,"byte_length":"0"}))])})).unwrap();
    let point = json!({"byte_offset":"0","scalar_offset":"0","utf16_offset":"0","line":"0","scalar_column":"0","utf16_column":"0"});
    let eof=log.try_derive(&root,Some(Origin{source:"lexer_token".into(),location:"a.js:1:1".into(),timestamp:None,
        meta:HashMap::from([("kind".into(),json!("eof")),("lexeme_byte_len".into(),json!(0)),("token_index".into(),json!(0)),
            (NS.into(),json!({"schema":"1","role":"piece","input_id":root,"piece_index":"0","kind":"eof","start":point,"end":point,"token_index":"0","token_type":"EOF"}))])})).unwrap();
    log.contribute(&root,"source.inventory","finalized",HashMap::from([(NS.into(),json!({"schema":"1","role":"inventory","status":"available","piece_count":"1","token_count":"1","piece_ids":[eof]}))])).unwrap();
    (log.to_json_string().unwrap(), root)
}
#[test]
fn generic_checked_graph_acceptance_does_not_admit_the_source_namespace() {
    let (raw, root) = snapshot();
    CVLog::from_checked_json(&raw, GraphLimits::default()).unwrap();
    for case in [
        "unknown-field",
        "leading-zero",
        "wrong-domain",
        "missing-inventory",
        "unavailable-with-child",
    ] {
        let mut wire: Value = serde_json::from_str(&raw).unwrap();
        let entry = &mut wire["entries"][&root];
        match case {
            "unknown-field" => entry["origin"]["meta"][NS]["extra"] = json!(true),
            "leading-zero" => entry["origin"]["meta"][NS]["byte_length"] = json!("00"),
            "wrong-domain" => entry["origin"]["meta"][NS]["buffer_domain"] = json!("caller_guess"),
            "missing-inventory" => entry["contributions"] = json!([]),
            "unavailable-with-child" => {
                entry["contributions"][0]["meta"][NS] = json!({"schema":"1","role":"inventory","status":"unavailable","piece_count":"0","token_count":"0","piece_ids":[],"reason":"lexical_error"})
            }
            _ => unreachable!(),
        }
        CVLog::from_checked_json(
            &serde_json::to_string(&wire).unwrap(),
            GraphLimits::default(),
        )
        .unwrap();
        println!("D3A_GENERIC_ONLY {case}");
    }
}
#[test]
fn original_raw_nested_duplicate_namespace_keys_are_already_rejected() {
    let (raw, _) = snapshot();
    assert!(raw.contains("\"byte_length\":\"0\""));
    for key in ["byte_length", "\\u0062yte_length"] {
        let replacement = format!("\"byte_length\":\"0\",\"{key}\":\"0\"");
        let malformed = raw.replacen("\"byte_length\":\"0\"", &replacement, 1);
        let error = CVLog::from_checked_json(&malformed, GraphLimits::default())
            .err()
            .expect("duplicate must fail on original raw bytes");
        assert!(error.contains("duplicate checked CV object key"), "{error}");
        println!("D3A_EXISTING_DUPLICATE_REFUSAL {key}");
    }
}
