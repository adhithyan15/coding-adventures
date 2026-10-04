use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use forme_shell_desktop_native::{preview::PreviewServer, worker::validate_worker_response};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    net::TcpStream,
};
use tempfile::TempDir;

fn response(bytes: &[u8]) -> Vec<u8> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct ManifestFile {
        output_path: &'static str,
        content_type: &'static str,
        size_bytes: usize,
        sha256: String,
        source: &'static str,
    }
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Manifest {
        version: u8,
        file_count: usize,
        total_size_bytes: usize,
        files: BTreeMap<&'static str, ManifestFile>,
    }
    #[derive(Serialize)]
    struct WorkerFile {
        path: &'static str,
        size: usize,
        sha256: String,
    }
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Success {
        schema_version: u8,
        revision: &'static str,
        build_id: &'static str,
        manifest_sha256: String,
        manifest: Manifest,
        files: Vec<WorkerFile>,
    }

    let mut files = BTreeMap::new();
    files.insert(
        "welcome.html",
        ManifestFile {
            output_path: "welcome.html",
            content_type: "text/html; charset=utf-8",
            size_bytes: bytes.len(),
            sha256: BASE64.encode(Sha256::digest(bytes)),
            source: "extra",
        },
    );
    let manifest = Manifest {
        version: 1,
        file_count: 1,
        total_size_bytes: bytes.len(),
        files,
    };
    let canonical = format!("{}\n", serde_json::to_string_pretty(&manifest).unwrap());
    serde_json::to_vec(&Success {
        schema_version: 1,
        revision: "revision-1",
        build_id: "build-1",
        manifest_sha256: BASE64.encode(Sha256::digest(canonical.as_bytes())),
        manifest,
        files: vec![WorkerFile {
            path: "welcome.html",
            size: bytes.len(),
            sha256: format!("{:x}", Sha256::digest(bytes)),
        }],
    })
    .unwrap()
}

fn request(server: &PreviewServer, target: &str, method: &str) -> Vec<u8> {
    let address = server.address();
    let server_url = server.url();
    let token_path = server_url
        .strip_prefix(&format!("http://{address}"))
        .unwrap()
        .trim_end_matches('/');
    let target = if target == "/" {
        format!("{token_path}/")
    } else {
        format!("{token_path}{target}")
    };
    let mut stream = TcpStream::connect(address).unwrap();
    write!(
        stream,
        "{method} {target} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        address
    )
    .unwrap();
    let mut response = Vec::new();
    stream.read_to_end(&mut response).unwrap();
    response
}

fn request_with_host(server: &PreviewServer, host: &str) -> Vec<u8> {
    let mut stream = TcpStream::connect(server.address()).unwrap();
    write!(
        stream,
        "GET {} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n",
        server
            .url()
            .strip_prefix(&format!("http://{}", server.address()))
            .unwrap()
    )
    .unwrap();
    let mut response = Vec::new();
    stream.read_to_end(&mut response).unwrap();
    response
}

#[test]
fn serves_only_the_last_validated_snapshot_on_random_loopback() {
    let server = PreviewServer::bind().unwrap();
    assert!(server.address().ip().is_loopback());
    assert_ne!(server.address().port(), 0);
    assert!(String::from_utf8(request(&server, "/", "GET"))
        .unwrap()
        .starts_with("HTTP/1.1 503 Service Unavailable"));

    let temp = TempDir::new().unwrap();
    let html = b"<!doctype html><h1>Validated preview</h1>";
    fs::write(temp.path().join("welcome.html"), html).unwrap();
    let output = validate_worker_response("revision-1", temp.path(), &response(html)).unwrap();
    server.publish_validated(&output, temp.path()).unwrap();

    let root = request(&server, "/", "GET");
    let text = String::from_utf8(root.clone()).unwrap();
    assert!(text.starts_with("HTTP/1.1 200 OK"));
    assert!(text.contains("Content-Security-Policy: default-src 'none'"));
    assert!(text.contains("X-Content-Type-Options: nosniff"));
    assert!(root.ends_with(html));
    assert!(request(&server, "/welcome.html", "HEAD").ends_with(b"\r\n\r\n"));
    server.clear().unwrap();
    assert!(String::from_utf8(request(&server, "/", "GET"))
        .unwrap()
        .starts_with("HTTP/1.1 503 Service Unavailable"));
}

#[test]
fn rejects_non_get_methods_and_unpublished_paths() {
    let server = PreviewServer::bind().unwrap();
    assert!(String::from_utf8(request(&server, "/", "POST"))
        .unwrap()
        .starts_with("HTTP/1.1 405 Method Not Allowed"));
    assert!(String::from_utf8(request(&server, "/escape", "GET"))
        .unwrap()
        .starts_with("HTTP/1.1 503 Service Unavailable"));
    assert!(
        String::from_utf8(request_with_host(&server, "attacker.example"))
            .unwrap()
            .starts_with("HTTP/1.1 400 Bad Request")
    );
}
