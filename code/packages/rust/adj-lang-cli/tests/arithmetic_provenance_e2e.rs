//! End-to-end CAS projection and verification for the arithmetic stdlib root.

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
fn arithmetic_bundle_projects_and_fully_verifies_all_four_queries() {
    let root = repo_root();
    let snapshots =
        std::env::temp_dir().join(format!("adj_arithmetic_provenance_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&snapshots);

    let python = if cfg!(windows) { "python" } else { "python3" };
    let projection = Command::new(python)
        .current_dir(&root)
        .arg("code/scripts/adj_stdlib_provenance.py")
        .arg("project")
        .arg("--output")
        .arg(&snapshots)
        .output()
        .expect("run offline provenance projection");
    assert!(
        projection.status.success(),
        "projection failed: {}",
        String::from_utf8_lossy(&projection.stdout)
    );

    let program = "code/specs/data/adj-formula-stdlib/arithmetic/arithmetic.query.adj";
    let execution = Command::new(env!("CARGO_BIN_EXE_adj-lang-cli"))
        .current_dir(&root)
        .arg(program)
        .output()
        .expect("run arithmetic queries");
    let answers = String::from_utf8(execution.stdout).expect("UTF-8 query output");
    assert!(execution.status.success(), "execution failed: {answers}");
    let answer_json: serde_json::Value = serde_json::from_str(&answers).expect("JSON query output");
    let derived = answer_json["derived"]
        .as_array()
        .expect("derived results array");
    let actual: Vec<_> = derived
        .iter()
        .map(|item| {
            (
                item["name"].as_str().expect("derived result name"),
                item["value"].as_i64().expect("derived integer value"),
            )
        })
        .collect();
    assert_eq!(
        actual,
        [
            ("sum", 12),
            ("difference", 2),
            ("product", 42),
            ("quotient", 4),
        ]
    );

    let verification = Command::new(env!("CARGO_BIN_EXE_adj-verify"))
        .current_dir(&root)
        .arg("--snapshots")
        .arg(&snapshots)
        .arg(program)
        .output()
        .expect("run adj-verify");
    let output = String::from_utf8(verification.stdout).expect("UTF-8 verifier output");
    assert!(
        verification.status.success(),
        "verification failed: {output}"
    );
    let verified: serde_json::Value =
        serde_json::from_str(&output).expect("JSON verification output");
    assert_eq!(verified["fully_verified"], true, "{output}");
    assert_eq!(verified["totals"]["quotes_verified"], 12, "{output}");
    assert_eq!(
        verified["totals"]["query_computations_fully_verified"], 4,
        "{output}"
    );

    std::fs::remove_dir_all(snapshots).expect("remove projected snapshots");
}
