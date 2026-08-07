use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Deserialize)]
struct DiscoveryFixture {
    workspace: FixtureWorkspace,
    expected: FixtureExpected,
}

#[derive(Debug, Deserialize)]
struct FixtureWorkspace {
    files: Vec<FixtureFile>,
}

#[derive(Debug, Deserialize)]
struct FixtureFile {
    path: String,
    content_utf8: String,
}

#[derive(Debug, Deserialize)]
struct FixtureExpected {
    diagnostics: Vec<FixtureDiagnostic>,
}

#[derive(Debug, Deserialize)]
struct FixtureDiagnostic {
    code: String,
    package: String,
    details: FixtureDiagnosticDetails,
}

#[derive(Debug, Deserialize)]
struct FixtureDiagnosticDetails {
    paths: Vec<String>,
}

fn load_fixture() -> DiscoveryFixture {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/fixtures/build-tool-v1/cases/discovery-duplicate-identity.json");
    let data = fs::read(&path)
        .unwrap_or_else(|error| panic!("read shared fixture {}: {error}", path.display()));
    serde_json::from_slice(&data)
        .unwrap_or_else(|error| panic!("decode shared fixture {}: {error}", path.display()))
}

fn materialize_fixture(fixture: &DiscoveryFixture) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "build_tool_rust_duplicate_identity_cli_{}_{}",
        std::process::id(),
        nonce
    ));

    for file in &fixture.workspace.files {
        let path = root.join(Path::new(&file.path));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, file.content_utf8.as_bytes()).unwrap();
    }

    root
}

#[test]
fn cli_fails_closed_on_shared_duplicate_identity_fixture() {
    let fixture = load_fixture();
    let root = materialize_fixture(&fixture);
    let output = Command::new(env!("CARGO_BIN_EXE_build-tool"))
        .args(["--root", root.to_str().unwrap(), "--force", "--dry-run"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let diagnostic = &fixture.expected.diagnostics[0];
    let expected = format!(
        "{}: package={} paths={}\n",
        diagnostic.code,
        diagnostic.package,
        diagnostic.details.paths.join(",")
    );
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(stderr, expected);
    assert!(!stderr.contains(&root.to_string_lossy().to_string()));

    let _ = fs::remove_dir_all(root);
}
