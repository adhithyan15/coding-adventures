use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use forme_shell_desktop_native::{
    commands::{NativePreviewCommands, PreviewBuildRequest},
    worker::{validate_worker_response, verify_worker_identity, NativeProductWorker, WorkerError},
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    net::TcpStream,
};
use tempfile::TempDir;

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn response(revision: &str, path: &str, bytes: &[u8]) -> Vec<u8> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct FileEntry<'a> {
        output_path: &'a str,
        content_type: &'static str,
        size_bytes: usize,
        sha256: String,
        source: &'static str,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Manifest<'a> {
        version: u8,
        file_count: usize,
        total_size_bytes: usize,
        files: BTreeMap<&'a str, FileEntry<'a>>,
    }

    #[derive(Serialize)]
    struct WorkerFile<'a> {
        path: &'a str,
        size: usize,
        sha256: String,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Success<'a> {
        schema_version: u8,
        revision: &'a str,
        build_id: &'static str,
        manifest_sha256: String,
        manifest: Manifest<'a>,
        files: Vec<WorkerFile<'a>>,
    }

    let digest = Sha256::digest(bytes);
    let mut manifest_files = BTreeMap::new();
    manifest_files.insert(
        path,
        FileEntry {
            output_path: path,
            content_type: "text/html; charset=utf-8",
            size_bytes: bytes.len(),
            sha256: BASE64.encode(digest),
            source: "extra",
        },
    );
    let manifest = Manifest {
        version: 1,
        file_count: 1,
        total_size_bytes: bytes.len(),
        files: manifest_files,
    };
    let canonical_manifest = format!("{}\n", serde_json::to_string_pretty(&manifest).unwrap());
    let manifest_sha256 = BASE64.encode(Sha256::digest(canonical_manifest.as_bytes()));
    serde_json::to_vec(&Success {
        schema_version: 1,
        revision,
        build_id: "build-1",
        manifest_sha256,
        manifest,
        files: vec![WorkerFile {
            path,
            size: bytes.len(),
            sha256: sha256_hex(bytes),
        }],
    })
    .unwrap()
}

#[test]
fn validates_exact_worker_metadata_against_the_contained_output_tree() {
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join("nested")).unwrap();
    let bytes = b"worker output";
    fs::write(temp.path().join("nested/index.html"), bytes).unwrap();

    let output = validate_worker_response(
        "revision-1",
        temp.path(),
        &response("revision-1", "nested/index.html", bytes),
    )
    .unwrap();
    assert_eq!(output.revision, "revision-1");
    assert_eq!(output.build_id, "build-1");
    assert_eq!(output.manifest_sha256.len(), 44);
    assert_eq!(output.files.len(), 1);
}

#[test]
fn rejects_mismatches_expansion_and_noncanonical_protocol_data() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("index.html"), b"actual").unwrap();

    assert_eq!(
        validate_worker_response(
            "revision-1",
            temp.path(),
            &response("other-revision", "index.html", b"actual"),
        ),
        Err(WorkerError::Protocol)
    );
    assert_eq!(
        validate_worker_response(
            "revision-1",
            temp.path(),
            &response("revision-1", "../escape", b"actual"),
        ),
        Err(WorkerError::Protocol)
    );
    for unsafe_path in ["C:escape", "CON", "name.", "nested\\escape"] {
        assert_eq!(
            validate_worker_response(
                "revision-1",
                temp.path(),
                &response("revision-1", unsafe_path, b"actual"),
            ),
            Err(WorkerError::Protocol),
            "unsafe portable path {unsafe_path:?} must be rejected"
        );
    }
    assert_eq!(
        validate_worker_response(
            "revision-1",
            temp.path(),
            &response("revision-1", "index.html", b"different"),
        ),
        Err(WorkerError::Output)
    );
    let mut expanded = response("revision-1", "index.html", b"actual");
    expanded.splice(
        expanded.len() - 1..expanded.len() - 1,
        b",\"extra\":true".iter().copied(),
    );
    assert_eq!(
        validate_worker_response("revision-1", temp.path(), &expanded),
        Err(WorkerError::Protocol)
    );
    let spaced = String::from_utf8(response("revision-1", "index.html", b"actual"))
        .unwrap()
        .replace(":1", ": 1");
    assert_eq!(
        validate_worker_response("revision-1", temp.path(), spaced.as_bytes()),
        Err(WorkerError::Protocol)
    );
}

#[test]
fn verifies_the_exact_bundled_worker_identity() {
    let temp = TempDir::new().unwrap();
    let worker = temp.path().join("worker");
    fs::write(&worker, b"reviewed worker").unwrap();
    assert_eq!(
        verify_worker_identity(&worker, &sha256_hex(b"reviewed worker")),
        Ok(())
    );
    assert_eq!(
        verify_worker_identity(&worker, &sha256_hex(b"other worker")),
        Err(WorkerError::Identity)
    );
}

#[test]
fn generated_worker_runs_without_path_or_environment_selection() {
    let suffix = if cfg!(windows) { ".exe" } else { "" };
    let package = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let executable = package
        .join("dist/worker")
        .join(format!("forme-product-worker{suffix}"));
    if !executable.exists() {
        // `sh BUILD` creates and smoke-tests this generated resource before
        // native tests. Plain `cargo test` remains useful without Node SEA.
        return;
    }
    let expected = fs::read_to_string(executable.with_extension(if cfg!(windows) {
        "exe.sha256"
    } else {
        "sha256"
    }))
    .unwrap();
    let launcher = package.join("dist/worker/forme-sandbox-macos");
    if !launcher.exists() {
        return;
    }
    let launcher_expected = fs::read_to_string(launcher.with_extension("sha256")).unwrap();
    let workspaces = TempDir::new().unwrap();
    let workspace_root = workspaces.path().join("workspaces");
    let worker = NativeProductWorker::open(
        executable,
        expected.trim().to_owned(),
        launcher,
        launcher_expected.trim().to_owned(),
        workspace_root.clone(),
    )
    .unwrap();
    let project = serde_json::json!({
        "schemaVersion": 1,
        "projectId": "01952c0d-7e63-7000-8000-000000000068",
        "title": "Native worker fixture",
        "site": { "baseUrl": null, "themeId": "forme-classless" },
        "workflow": { "lastPublication": null },
        "activeDocumentId": "01952c0d-7e63-7000-8000-000000000069",
        "documents": [{
            "id": "01952c0d-7e63-7000-8000-000000000069",
            "slug": "welcome",
            "title": "Welcome",
            "status": "draft",
            "body": {
                "type": "document",
                "children": [{
                    "type": "paragraph",
                    "children": [{ "type": "text", "value": "Native preview" }]
                }]
            }
        }]
    });
    assert!(worker
        .build(&serde_json::json!({ "invalid": true }), "failed-revision")
        .is_err());
    assert_eq!(fs::read_dir(&workspace_root).unwrap().count(), 0);
    let commands = NativePreviewCommands::open(worker).unwrap();
    let revision = "sha256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
    let result = commands
        .preview_build(
            project,
            PreviewBuildRequest {
                revision: revision.to_owned(),
            },
        )
        .unwrap();
    assert_eq!(result.outcome, "ready");
    assert_eq!(result.revision, revision);
    assert!(result.diagnostics.is_empty());

    let preview_url = commands.preview_url();
    let remainder = preview_url.strip_prefix("http://").unwrap();
    let (address, path) = remainder.split_once('/').unwrap();
    let mut stream = TcpStream::connect(address).unwrap();
    write!(
        stream,
        "GET /{path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    assert!(response.starts_with("HTTP/1.1 200 OK"));
    assert!(response.contains("Native preview"));
    commands.workspace_dispose().unwrap();
}
