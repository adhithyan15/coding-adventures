use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "adj_formula_inventory_{tag}_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

fn inventory(path: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_adj-formula-inventory"))
        .arg(path)
        .output()
        .unwrap()
}

#[test]
fn emits_parser_order_and_exact_byte_hashes() {
    let dir = scratch("bytes");
    let path = dir.join("library.adj");
    let source = concat!(
        "formulabook demo {\n",
        "  formula total(a, b) = a + b source \"sum\" locator \"cas://sum\" trust authoritative\n",
        "  formula scaled(x) {\n",
        "    let doubled = x * 2\n",
        "    doubled / 10\n",
        "  } source \"scale\" locator \"cas://scale\" trust authoritative\n",
        "}\n",
    );
    fs::write(&path, source).unwrap();

    let output = inventory(&path);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["kind"], "formula_parser_inventory");
    assert_eq!(value["parser_contract"], "adj-lang/formula_source_map/v1");
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["scope"], "source_file");
    assert_eq!(value["source_size"], source.len());
    assert_eq!(
        value["source_sha256"],
        coding_adventures_sha256::sha256_hex(source.as_bytes())
    );

    let formulas = value["formulas"].as_array().unwrap();
    assert_eq!(formulas.len(), 2);
    assert_eq!(formulas[0]["formula"], "total");
    assert_eq!(formulas[0]["parameters"], serde_json::json!(["a", "b"]));
    assert_eq!(formulas[0]["step_count"], 0);
    assert_eq!(formulas[1]["formula"], "scaled");
    assert_eq!(formulas[1]["step_count"], 1);

    for formula in formulas {
        for field in ["body", "declaration"] {
            let span = &formula[field];
            let start = span["start"].as_u64().unwrap() as usize;
            let end = span["end"].as_u64().unwrap() as usize;
            assert!(start < end && end <= source.len());
            assert_eq!(
                span["sha256"],
                coding_adventures_sha256::sha256_hex(&source.as_bytes()[start..end])
            );
        }
    }
    assert_eq!(
        &source[formulas[1]["body"]["start"].as_u64().unwrap() as usize
            ..formulas[1]["body"]["end"].as_u64().unwrap() as usize],
        "doubled / 10"
    );
}

#[test]
fn rejects_non_utf8_and_malformed_input() {
    let dir = scratch("errors");
    let non_utf8 = dir.join("non-utf8.adj");
    fs::write(&non_utf8, [0xff, 0xfe]).unwrap();
    let output = inventory(&non_utf8);
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("not UTF-8"));

    let malformed = dir.join("malformed.adj");
    fs::write(&malformed, "formulabook broken { formula nope(a) = }").unwrap();
    let output = inventory(&malformed);
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("source map failed"));
}

#[test]
fn preserves_unicode_crlf_bytes_and_does_not_resolve_imports() {
    let dir = scratch("unicode");
    let path = dir.join("library.adj");
    let source = concat!(
        "% pi: π\r\n",
        "import \"absent.adj\"\r\n",
        "formulabook local {\r\n",
        "  formula decimal(x) = x + 0.1 source \"fixture\" locator \"cas://decimal\" trust authoritative\r\n",
        "}\r\n",
    );
    fs::write(&path, source).unwrap();

    let output = inventory(&path);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["scope"], "source_file");
    assert_eq!(value["formulas"].as_array().unwrap().len(), 1);
    let body = &value["formulas"][0]["body"];
    let start = body["start"].as_u64().unwrap() as usize;
    let end = body["end"].as_u64().unwrap() as usize;
    assert_eq!(&source.as_bytes()[start..end], b"x + 0.1");
    assert!(value["formulas"][0].get("ast").is_none());
    let canonical = format!("{}\n", serde_json::to_string_pretty(&value).unwrap());
    assert_eq!(output.stdout, canonical.as_bytes());
}

#[test]
fn rejects_a_source_larger_than_the_cas_object_limit_before_reading() {
    let dir = scratch("oversized");
    let path = dir.join("oversized.adj");
    fs::File::create(&path)
        .unwrap()
        .set_len(64 * 1024 * 1024 + 1)
        .unwrap();

    let output = inventory(&path);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("byte limit"));
}
