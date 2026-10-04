use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use forme_shell_desktop_native::commands::{
    CommandError, NativeCommands, PreviewBuildRequest, ProjectCompareAndSwapRequest,
    TargetPublishRequest,
};
use serde_json::json;
use std::{collections::BTreeSet, fs, path::Path};
use tempfile::TempDir;
use uuid::{Uuid, Version};

const PROJECT: &[u8] = br#"{"cursor":0,"history":[{"activeDocumentId":null,"documents":[],"projectId":"01952c0d-7e63-7000-8000-000000000068","schemaVersion":1,"site":{"baseUrl":null,"themeId":"forme-classless"},"title":"Command fixture","workflow":{"lastPublication":null}}],"historyLimit":1,"schemaVersion":1}"#;

#[test]
fn ipc_storage_round_trip_exposes_only_bytes_and_revision() {
    let temp = TempDir::new().unwrap();
    let commands = NativeCommands::open(temp.path().join("profile")).unwrap();
    assert_eq!(commands.project_load().unwrap(), None);

    let committed = commands
        .project_compare_and_swap(ProjectCompareAndSwapRequest {
            expected_revision: None,
            bytes_base64: BASE64.encode(PROJECT),
        })
        .unwrap();
    let loaded = commands.project_load().unwrap().unwrap();
    assert_eq!(loaded.revision, committed.revision);
    assert_eq!(BASE64.decode(&loaded.bytes_base64).unwrap(), PROJECT);
    assert_eq!(
        serde_json::to_value(loaded).unwrap(),
        json!({
            "bytesBase64": BASE64.encode(PROJECT),
            "revision": committed.revision,
        })
    );
}

#[test]
fn ipc_rejects_invalid_base64_and_maps_storage_failures_to_closed_codes() {
    let temp = TempDir::new().unwrap();
    let commands = NativeCommands::open(temp.path().join("profile")).unwrap();
    assert_eq!(
        commands.project_compare_and_swap(ProjectCompareAndSwapRequest {
            expected_revision: None,
            bytes_base64: "%%%".into(),
        }),
        Err(CommandError::new("INVALID_REQUEST"))
    );

    commands
        .project_compare_and_swap(ProjectCompareAndSwapRequest {
            expected_revision: None,
            bytes_base64: BASE64.encode(PROJECT),
        })
        .unwrap();
    assert_eq!(
        commands.project_compare_and_swap(ProjectCompareAndSwapRequest {
            expected_revision: None,
            bytes_base64: BASE64.encode(PROJECT),
        }),
        Err(CommandError::new("STORAGE_CONFLICT"))
    );
}

#[test]
fn ipc_request_deserialization_rejects_unknown_fields() {
    let request = json!({
        "expectedRevision": null,
        "bytesBase64": BASE64.encode(PROJECT),
        "path": "/tmp/escape"
    });
    assert!(serde_json::from_value::<ProjectCompareAndSwapRequest>(request).is_err());

    assert!(serde_json::from_value::<PreviewBuildRequest>(json!({
        "revision": "sha256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
        "project": { "injected": true }
    }))
    .is_err());
    assert!(serde_json::from_value::<TargetPublishRequest>(json!({
        "targetId": "local-01952c0d-7e63-7000-8000-000000000068",
        "revision": "sha256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
        "project": { "injected": true }
    }))
    .is_err());
}

#[test]
fn identity_command_accepts_no_renderer_input_and_returns_uuid_v7() {
    let temp = TempDir::new().unwrap();
    let commands = NativeCommands::open(temp.path().join("profile")).unwrap();
    let identity = commands.identity_create().unwrap();
    let parsed = Uuid::parse_str(&identity).unwrap();
    assert_eq!(parsed.get_version(), Some(Version::SortRand));
    assert_eq!(parsed.hyphenated().to_string(), identity);
}

#[test]
fn main_window_grants_only_the_closed_application_commands() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let capability: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("capabilities/main.json")).unwrap()).unwrap();
    let actual = capability["permissions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        actual,
        BTreeSet::from([
            "allow-identity-create",
            "allow-preview-build",
            "allow-project-compare-and-swap",
            "allow-project-load",
            "allow-target-configure",
            "allow-target-list",
            "allow-target-publish",
            "allow-workspace-dispose",
        ])
    );

    let config: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("tauri.conf.json")).unwrap()).unwrap();
    let csp = config["app"]["security"]["csp"].as_str().unwrap();
    assert!(csp.contains("default-src 'self'"));
    assert!(csp.contains("form-action 'none'"));
    assert!(csp.contains("object-src 'none'"));
    assert!(!csp.contains("https:"));
}

#[test]
fn native_leaf_declares_structured_reviewable_capabilities() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("required_capabilities.json")).unwrap())
            .unwrap();
    assert_eq!(manifest["version"], 1);
    assert_eq!(manifest["package"], "typescript/forme-shell-desktop");
    let capabilities = manifest["capabilities"].as_array().unwrap();
    assert!(!capabilities.is_empty());
    for capability in capabilities {
        let object = capability.as_object().unwrap();
        assert_eq!(object.len(), 4);
        for field in ["category", "action", "target", "justification"] {
            assert!(object[field]
                .as_str()
                .is_some_and(|value| !value.is_empty()));
        }
    }
}
