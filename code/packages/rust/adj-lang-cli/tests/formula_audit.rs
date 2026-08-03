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
