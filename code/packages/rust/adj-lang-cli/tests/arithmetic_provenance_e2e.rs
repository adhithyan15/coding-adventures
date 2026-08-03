//! End-to-end CAS projection and verification for arithmetic stdlib roots.

use std::collections::BTreeSet;
use std::fs::{File, OpenOptions};
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

fn lock_cas(root: &Path) -> File {
    let lock_path = root.join("code/specs/data/adj-stdlib-provenance/cas/lock");
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&lock_path)
        .unwrap_or_else(|error| panic!("open CAS lock {}: {error}", lock_path.display()));
    file.lock()
        .unwrap_or_else(|error| panic!("acquire CAS lock {}: {error}", lock_path.display()));
    file
}

fn read_bundle(root: &Path, digest: &str) -> serde_json::Value {
    serde_json::from_slice(&read_cas_object(root, digest)).expect("parse provenance bundle")
}

fn collect_bundle_snapshots(
    root: &Path,
    bundle_hash: &str,
    visited: &mut BTreeSet<String>,
    snapshot_hashes: &mut BTreeSet<String>,
) {
    if !visited.insert(bundle_hash.to_owned()) {
        return;
    }
    let bundle = read_bundle(root, bundle_hash);
    for dependency in bundle["dependencies"]
        .as_array()
        .expect("bundle dependencies")
    {
        collect_bundle_snapshots(
            root,
            dependency.as_str().expect("dependency hash string"),
            visited,
            snapshot_hashes,
        );
    }
    for clause in bundle["clauses"].as_array().expect("bundle clauses") {
        snapshot_hashes.insert(
            clause["snapshot_sha256"]
                .as_str()
                .expect("clause snapshot hash")
                .to_owned(),
        );
    }
}

fn project_bundle_snapshots(root: &Path, snapshots: &Path, bundle_id: &str) -> usize {
    let _cas_lock = lock_cas(root);
    let manifest_path = root.join("code/specs/data/adj-stdlib-provenance/manifest.json");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest_path).expect("read provenance manifest"))
            .expect("parse provenance manifest");
    let selected = manifest["bundle_hashes"]
        .as_array()
        .expect("bundle hash array")
        .iter()
        .map(|value| value.as_str().expect("bundle hash string"))
        .find(|digest| read_bundle(root, digest)["bundle_id"] == bundle_id)
        .unwrap_or_else(|| panic!("manifest does not register bundle {bundle_id}"));
    let mut snapshot_hashes = BTreeSet::new();
    collect_bundle_snapshots(root, selected, &mut BTreeSet::new(), &mut snapshot_hashes);
    std::fs::create_dir_all(snapshots).expect("create snapshot directory");
    for digest in &snapshot_hashes {
        std::fs::write(snapshots.join(digest), read_cas_object(root, digest))
            .expect("project CAS snapshot");
    }
    snapshot_hashes.len()
}

#[test]
fn rust_reader_uses_the_repository_cas_lock() {
    let root = repo_root();
    let _guard = lock_cas(&root);
    let lock_path = root.join("code/specs/data/adj-stdlib-provenance/cas/lock");
    let contender = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&lock_path)
        .expect("open competing CAS lock handle");
    let error = contender
        .try_lock()
        .expect_err("a second reader must not bypass the CAS lock");
    assert!(matches!(error, std::fs::TryLockError::WouldBlock));
}

#[test]
fn arithmetic_bundle_projects_and_fully_verifies_all_four_queries() {
    let root = repo_root();
    let snapshots =
        std::env::temp_dir().join(format!("adj_arithmetic_provenance_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&snapshots);
    assert_eq!(
        project_bundle_snapshots(&root, &snapshots, "adj.math.arithmetic.primitives.query.v1"),
        5
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

#[test]
fn ratio_bundle_reuses_quotient_and_fully_verifies_its_query() {
    let root = repo_root();
    let snapshots =
        std::env::temp_dir().join(format!("adj_ratio_provenance_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&snapshots);
    assert_eq!(
        project_bundle_snapshots(&root, &snapshots, "adj.math.arithmetic.ratio.query.v1"),
        6
    );

    let program = "code/specs/data/adj-formula-stdlib/arithmetic/ratio.query.adj";
    let execution = Command::new(env!("CARGO_BIN_EXE_adj-lang-cli"))
        .current_dir(&root)
        .arg(program)
        .output()
        .expect("run ratio query");
    let answers = String::from_utf8(execution.stdout).expect("UTF-8 query output");
    assert!(execution.status.success(), "execution failed: {answers}");
    let answer_json: serde_json::Value = serde_json::from_str(&answers).expect("JSON query output");
    assert_eq!(answer_json["derived"][0]["name"], "ratio", "{answers}");
    assert_eq!(answer_json["derived"][0]["value"], 0.75, "{answers}");
    assert!(answers.contains("mathworld.wolfram.com/Ratio.html"));
    assert!(answers.contains("mathworld.wolfram.com/Quotient.html"));

    let verification = Command::new(env!("CARGO_BIN_EXE_adj-verify"))
        .current_dir(&root)
        .arg("--snapshots")
        .arg(&snapshots)
        .arg(program)
        .output()
        .expect("run adj-verify for ratio");
    let output = String::from_utf8(verification.stdout).expect("UTF-8 verifier output");
    assert!(
        verification.status.success(),
        "verification failed: {output}"
    );
    let verified: serde_json::Value =
        serde_json::from_str(&output).expect("JSON verification output");
    assert_eq!(verified["fully_verified"], true, "{output}");
    assert_eq!(verified["totals"]["quotes_verified"], 4, "{output}");
    assert_eq!(
        verified["totals"]["query_computations_fully_verified"], 1,
        "{output}"
    );

    let audit = Command::new(env!("CARGO_BIN_EXE_adj-formula-audit"))
        .current_dir(&root)
        .arg("--snapshots")
        .arg(&snapshots)
        .arg(program)
        .output()
        .expect("run formula audit for ratio");
    assert!(
        audit.status.success(),
        "formula audit failed: {}",
        String::from_utf8_lossy(&audit.stderr)
    );
    let audit_json: serde_json::Value =
        serde_json::from_slice(&audit.stdout).expect("formula audit JSON");
    let derivation = &audit_json["derivations"][0];
    assert_eq!(derivation["verification"]["fully_verified"], true);
    assert_eq!(derivation["formula_sequence"][0]["name"], "ratio");
    assert_eq!(derivation["formula_sequence"][1]["name"], "quotient");
    assert_eq!(
        derivation["verification"]["formula_quotes"][0]["quote"]["status"],
        "verified"
    );
    assert_eq!(
        derivation["verification"]["input_quotes"][0]["quote"]["status"],
        "verified"
    );
    assert_eq!(
        derivation["verification"]["formula_quotes"][0]["provenance"]["quote"]["snapshot_sha256"]
            .as_str()
            .expect("formula snapshot identity")
            .len(),
        64
    );

    std::fs::remove_dir_all(snapshots).expect("remove projected snapshots");
}

#[test]
fn percent_of_bundle_composes_product_and_quotient_with_full_provenance() {
    let root = repo_root();
    let snapshots =
        std::env::temp_dir().join(format!("adj_percent_of_provenance_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&snapshots);
    assert_eq!(
        project_bundle_snapshots(&root, &snapshots, "adj.math.arithmetic.percent_of.query.v1"),
        6
    );

    let program = "code/specs/data/adj-formula-stdlib/arithmetic/percent-of.query.adj";
    let execution = Command::new(env!("CARGO_BIN_EXE_adj-lang-cli"))
        .current_dir(&root)
        .arg(program)
        .output()
        .expect("run percent-of query");
    let answers = String::from_utf8(execution.stdout).expect("UTF-8 query output");
    assert!(execution.status.success(), "execution failed: {answers}");
    let answer_json: serde_json::Value = serde_json::from_str(&answers).expect("JSON query output");
    assert_eq!(answer_json["derived"][0]["name"], "percent_of", "{answers}");
    assert_eq!(answer_json["derived"][0]["value"], 10, "{answers}");
    assert!(answers.contains("openstax.org/books/contemporary-mathematics"));
    assert!(answers.contains("mathworld.wolfram.com/Product.html"));
    assert!(answers.contains("mathworld.wolfram.com/Quotient.html"));

    let verification = Command::new(env!("CARGO_BIN_EXE_adj-verify"))
        .current_dir(&root)
        .arg("--snapshots")
        .arg(&snapshots)
        .arg(program)
        .output()
        .expect("run adj-verify for percent-of");
    let output = String::from_utf8(verification.stdout).expect("UTF-8 verifier output");
    assert!(
        verification.status.success(),
        "verification failed: {output}"
    );
    let verified: serde_json::Value =
        serde_json::from_str(&output).expect("JSON verification output");
    assert_eq!(verified["fully_verified"], true, "{output}");
    assert_eq!(verified["totals"]["quotes_verified"], 5, "{output}");
    assert_eq!(
        verified["totals"]["query_computations_fully_verified"], 1,
        "{output}"
    );

    std::fs::remove_dir_all(snapshots).expect("remove projected snapshots");
}
