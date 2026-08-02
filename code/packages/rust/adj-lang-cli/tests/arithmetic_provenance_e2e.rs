//! End-to-end CAS projection and verification for the arithmetic stdlib root.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("adj-lang-cli must remain below code/packages/rust")
        .to_path_buf()
}

fn read_cas_object(root: &Path, digest: &str) -> Vec<u8> {
    assert_eq!(digest.len(), 64, "CAS digest must be SHA-256 hex");
    let path = root
        .join("code/specs/data/adj-stdlib-provenance/cas/objects")
        .join(&digest[..2])
        .join(&digest[2..]);
    let bytes = std::fs::read(&path).unwrap_or_else(|error| {
        panic!("read CAS object {}: {error}", path.display());
    });
    assert_eq!(
        coding_adventures_sha256::sha256_hex(&bytes),
        digest,
        "CAS object bytes must match their fanout path"
    );
    bytes
}

fn project_manifest_snapshots(root: &Path, snapshots: &Path) -> usize {
    std::fs::create_dir_all(snapshots).expect("create snapshot directory");
    let manifest_path = root.join("code/specs/data/adj-stdlib-provenance/manifest.json");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest_path).expect("read provenance manifest"))
            .expect("parse provenance manifest");
    let bundle_hashes = manifest["bundle_hashes"]
        .as_array()
        .expect("bundle hash array");
    let mut snapshot_hashes = BTreeSet::new();
    for bundle_hash in bundle_hashes {
        let bundle_hash = bundle_hash.as_str().expect("bundle hash string");
        let bundle: serde_json::Value = serde_json::from_slice(&read_cas_object(root, bundle_hash))
            .expect("parse provenance bundle");
        for clause in bundle["clauses"].as_array().expect("bundle clauses") {
            snapshot_hashes.insert(
                clause["snapshot_sha256"]
                    .as_str()
                    .expect("clause snapshot hash")
                    .to_owned(),
            );
        }
    }
    for digest in &snapshot_hashes {
        std::fs::write(snapshots.join(digest), read_cas_object(root, digest))
            .expect("project CAS snapshot");
    }
    snapshot_hashes.len()
}

#[test]
fn arithmetic_bundle_projects_and_fully_verifies_all_four_queries() {
    let root = repo_root();
    let snapshots =
        std::env::temp_dir().join(format!("adj_arithmetic_provenance_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&snapshots);
    assert_eq!(project_manifest_snapshots(&root, &snapshots), 5);

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
