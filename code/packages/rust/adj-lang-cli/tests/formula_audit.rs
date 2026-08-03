use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("adj-lang-cli must remain below code/packages/rust")
        .to_path_buf()
}

#[test]
fn formula_audit_is_canonical_and_preserves_nested_formula_identity() {
    let root = repo_root();
    let program = "code/specs/data/adj-formula-stdlib/arithmetic/ratio.query.adj";
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_adj-formula-audit"))
            .current_dir(&root)
            .arg(program)
            .output()
            .expect("run formula audit")
    };

    let first = run();
    let second = run();
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert_eq!(
        first.stdout, second.stdout,
        "canonical output must be byte-stable"
    );
    assert!(first.stdout.ends_with(b"\n"));

    let audit: serde_json::Value = serde_json::from_slice(&first.stdout).expect("audit JSON");
    let derivation = &audit["derivations"][0];
    assert_eq!(derivation["export"]["name"], "ratio");
    assert_eq!(derivation["formula_sequence"][0]["name"], "ratio");
    assert_eq!(derivation["formula_sequence"][1]["name"], "quotient");
    assert_eq!(derivation["result"]["f64_bits"], "3fe8000000000000");
    assert_eq!(derivation["result"]["exact_rational"]["numerator"], "3");
    assert_eq!(derivation["result"]["exact_rational"]["denominator"], "4");
    assert_eq!(
        derivation["verification"]["computation"]["status"],
        "rechecked"
    );
    assert_eq!(audit["imports"].as_array().expect("imports").len(), 2);
}

#[test]
fn formula_audit_rejects_duplicate_export_names_before_using_runtime_mapping() {
    let directory = std::env::temp_dir().join(format!(
        "adj_formula_audit_duplicate_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("create temporary source directory");
    let program = directory.join("duplicate.adj");
    fs::write(
        &program,
        "formulabook first {\n\
             formula answer(x) = x source \"first definition\"\n\
         }\n\
         formulabook second {\n\
             formula answer(x) = x source \"second definition\"\n\
         }\n\
         observe x(2)\n\
         ? answer(x)\n",
    )
    .expect("write duplicate program");

    let output = Command::new(env!("CARGO_BIN_EXE_adj-formula-audit"))
        .arg(&program)
        .output()
        .expect("run formula audit");
    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("duplicate formula export name: answer")
    );

    fs::remove_dir_all(directory).expect("remove temporary source directory");
}

#[test]
fn formula_audit_rejects_provenance_that_maps_to_multiple_exports() {
    let directory = std::env::temp_dir().join(format!(
        "adj_formula_audit_ambiguous_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("create temporary source directory");
    let program = directory.join("ambiguous.adj");
    fs::write(
        &program,
        "formulabook formulas {\n\
             formula first(x) = x source \"shared definition\"\n\
             formula second(x) = x source \"shared definition\"\n\
         }\n\
         observe x(2)\n\
         ? first(x)\n",
    )
    .expect("write ambiguous program");

    let output = Command::new(env!("CARGO_BIN_EXE_adj-formula-audit"))
        .arg(&program)
        .output()
        .expect("run formula audit");
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("formula provenance maps ambiguously to 2 exports: shared definition"));

    fs::remove_dir_all(directory).expect("remove temporary source directory");
}

#[test]
fn formula_audit_consumes_a_quoted_fact_from_an_imported_source() {
    let directory = std::env::temp_dir().join(format!(
        "adj_formula_audit_imported_fact_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&directory);
    let snapshots = directory.join("snapshots");
    fs::create_dir_all(&snapshots).expect("create temporary snapshot directory");
    let fact_bytes = b"imported value is 3.";
    let fact_snapshot = coding_adventures_sha256::sha256_hex(fact_bytes);
    fs::write(snapshots.join(&fact_snapshot), fact_bytes).expect("write fact snapshot");
    let formula_bytes = b"Double a value by multiplying it by 2.";
    let formula_snapshot = coding_adventures_sha256::sha256_hex(formula_bytes);
    fs::write(snapshots.join(&formula_snapshot), formula_bytes).expect("write formula snapshot");
    fs::write(
        directory.join("dependency.adj"),
        format!(
            "dictionary imported_vocab {{\n\
                 define imported : finding\n\
             }}\n\
             observe imported(3)\n\
               quote \"imported value is 3.\" at 0 snapshot \"{fact_snapshot}\"\n\
               source \"imported value fixture\"\n\
               locator \"https://example.test/imported-value\"\n\
               trust authoritative\n\
             formulabook imported_math {{\n\
               formula double(value) = value * 2\n\
                 quote \"Double a value by multiplying it by 2.\" at 0 snapshot \"{formula_snapshot}\"\n\
                 source \"Double a value by multiplying it by 2.\"\n\
                 locator \"https://example.test/double\"\n\
                 trust authoritative\n\
             }}\n"
        ),
    )
    .expect("write imported fact dependency");
    let query = directory.join("query.adj");
    fs::write(&query, "import \"dependency.adj\"\n? double(imported)\n")
        .expect("write imported fact query");

    let output = Command::new(env!("CARGO_BIN_EXE_adj-formula-audit"))
        .arg("--snapshots")
        .arg(&snapshots)
        .arg(&query)
        .output()
        .expect("run imported fact audit");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let audit: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("imported fact audit JSON");
    assert_eq!(audit["imports"].as_array().expect("imports").len(), 1);
    let derivation = &audit["derivations"][0];
    assert_eq!(derivation["inputs"][0]["term"], "imported(3)");
    assert_eq!(
        derivation["inputs"][0]["provenance"]["quote"]["snapshot_sha256"],
        fact_snapshot
    );
    assert_eq!(derivation["result"]["f64_bits"], "4018000000000000");
    assert_eq!(derivation["verification"]["fully_verified"], true);

    fs::remove_dir_all(directory).expect("remove temporary source directory");
}

#[test]
fn formula_audit_accepts_a_zero_input_constant_formula() {
    let directory =
        std::env::temp_dir().join(format!("adj_formula_audit_constant_{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    let snapshots = directory.join("snapshots");
    fs::create_dir_all(&snapshots).expect("create temporary snapshot directory");
    let source_bytes = b"The answer is 42.";
    let snapshot = coding_adventures_sha256::sha256_hex(source_bytes);
    fs::write(snapshots.join(&snapshot), source_bytes).expect("write source snapshot");
    let program = directory.join("constant.adj");
    fs::write(
        &program,
        format!(
            "formulabook constants {{\n\
                 formula answer(seed) = 42\n\
                   quote \"The answer is 42.\" at 0 snapshot \"{snapshot}\"\n\
                   source \"The answer is 42.\"\n\
                   locator \"https://example.test/constant\"\n\
                   trust authoritative\n\
             }}\n\
             ? answer(0)\n"
        ),
    )
    .expect("write constant program");

    let output = Command::new(env!("CARGO_BIN_EXE_adj-formula-audit"))
        .arg("--snapshots")
        .arg(&snapshots)
        .arg(&program)
        .output()
        .expect("run formula audit");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let audit: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("constant audit JSON");
    let derivation = &audit["derivations"][0];
    assert_eq!(derivation["export"]["name"], "answer");
    assert_eq!(derivation["inputs"], serde_json::json!([]));
    assert_eq!(
        derivation["verification"]["input_quotes"],
        serde_json::json!([])
    );
    assert_eq!(derivation["verification"]["fully_verified"], true);
    assert!(audit["imports"].as_array().expect("imports").is_empty());

    fs::remove_dir_all(directory).expect("remove temporary source directory");
}
