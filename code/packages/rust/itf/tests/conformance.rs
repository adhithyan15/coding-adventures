use coding_adventures_sha256::sha256_hex;
use itf::{encode_itf, itf_error_id, normalize_itf};
use serde_json::Value;
use std::fs;
use std::path::Path;

fn materialize_input(test_case: &Value) -> String {
    let input = &test_case["input"];
    if let Some(text) = input["text"].as_str() {
        return text.to_string();
    }
    let repeat = &input["repeat"];
    repeat["text"]
        .as_str()
        .unwrap()
        .repeat(repeat["count"].as_u64().unwrap() as usize)
}

fn modules(data: &str) -> String {
    format!(
        "1010{}11101",
        encode_itf(data)
            .unwrap()
            .into_iter()
            .map(|pair| pair.binary_pattern)
            .collect::<String>()
    )
}

fn run_lengths(bits: &str) -> Vec<usize> {
    let mut runs = Vec::new();
    let mut previous = None;
    for bit in bits.bytes() {
        if previous == Some(bit) {
            *runs.last_mut().unwrap() += 1;
        } else {
            runs.push(1);
            previous = Some(bit);
        }
    }
    runs
}

#[test]
fn executes_all_itf_v1_cases() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .unwrap();
    let raw =
        fs::read_to_string(repo_root.join("code/specs/fixtures/barcode-symbologies-v1/cases.json"))
            .unwrap();
    let corpus: Value = serde_json::from_str(&raw).unwrap();
    let cases: Vec<&Value> = corpus["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|test_case| test_case["symbology"] == "itf")
        .collect();
    assert_eq!(cases.len(), 10);

    for test_case in cases {
        let data = materialize_input(test_case);
        let expected = &test_case["expected"];
        if let Some(expected_error) = expected["error"].as_str() {
            let error = normalize_itf(&data).unwrap_err();
            assert_eq!(itf_error_id(&error), Some(expected_error));
            continue;
        }

        let normalized = normalize_itf(&data).unwrap();
        let encoded_modules = modules(&data);
        let runs = run_lengths(&encoded_modules);
        if let Some(expected_normalized) = expected["normalized"].as_str() {
            assert_eq!(normalized, expected_normalized);
            assert_eq!(encoded_modules, expected["modules"].as_str().unwrap());
            assert_eq!(
                serde_json::to_value(&runs).unwrap(),
                expected["run_lengths"]
            );
        } else {
            assert_eq!(
                sha256_hex(normalized.as_bytes()),
                expected["normalized_sha256"]
            );
            assert_eq!(encoded_modules.len() as u64, expected["module_count"]);
            assert_eq!(
                sha256_hex(encoded_modules.as_bytes()),
                expected["module_sha256"]
            );
            assert_eq!(runs.len() as u64, expected["run_count"]);
            let compact_runs = serde_json::to_string(&runs).unwrap();
            assert_eq!(
                sha256_hex(compact_runs.as_bytes()),
                expected["run_lengths_sha256"]
            );
        }
    }
}
