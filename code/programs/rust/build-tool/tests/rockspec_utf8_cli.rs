use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Deserialize)]
struct ResolutionFixture {
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
    #[serde(default)]
    content_utf8: String,
    #[serde(default)]
    content_base64: String,
}

#[derive(Debug, Deserialize)]
struct FixtureExpected {
    diagnostics: Vec<FixtureDiagnostic>,
}

#[derive(Debug, Deserialize)]
struct FixtureDiagnostic {
    code: String,
    path: String,
    package: String,
    details: FixtureDiagnosticDetails,
}

#[derive(Debug, Deserialize)]
struct FixtureDiagnosticDetails {
    encoding: String,
}

fn decode_base64(input: &str) -> Vec<u8> {
    let mut output = Vec::new();
    let mut accumulator = 0u32;
    let mut bits = 0u8;

    for byte in input.bytes() {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => break,
            b'\r' | b'\n' | b'\t' | b' ' => continue,
            _ => panic!("invalid base64 fixture byte: {byte}"),
        };
        accumulator = (accumulator << 6) | u32::from(value);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            output.push((accumulator >> bits) as u8);
            accumulator &= (1 << bits) - 1;
        }
    }

    output
}

fn load_fixture(name: &str) -> ResolutionFixture {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/fixtures/build-tool-v1/cases")
        .join(name);
    let data = fs::read(&path)
        .unwrap_or_else(|error| panic!("read shared fixture {}: {error}", path.display()));
    serde_json::from_slice(&data)
        .unwrap_or_else(|error| panic!("decode shared fixture {}: {error}", path.display()))
}

fn materialize_fixture(fixture: &ResolutionFixture) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "build_tool_rust_cli_{}_{}",
        std::process::id(),
        nonce
    ));

    for file in &fixture.workspace.files {
        let path = root.join(Path::new(&file.path));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let data = if file.content_base64.is_empty() {
            file.content_utf8.as_bytes().to_vec()
        } else {
            decode_base64(&file.content_base64)
        };
        fs::write(path, data).unwrap();
    }

    root
}

#[test]
fn cli_fails_closed_on_shared_invalid_utf8_fixture() {
    let fixture = load_fixture("resolution-lua-invalid-utf8.json");
    let root = materialize_fixture(&fixture);
    let output = Command::new(env!("CARGO_BIN_EXE_build-tool"))
        .args([
            "--root",
            root.to_str().unwrap(),
            "--force",
            "--dry-run",
            "--language",
            "lua",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let diagnostic = &fixture.expected.diagnostics[0];
    let expected = format!(
        "{}: package={} manifest={} encoding={}\n",
        diagnostic.code, diagnostic.package, diagnostic.path, diagnostic.details.encoding
    );
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(stderr, expected);
    assert!(!stderr.contains(&root.to_string_lossy().to_string()));

    let _ = fs::remove_dir_all(root);
}
