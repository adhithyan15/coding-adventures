use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use forme_shell_desktop_native::{
    commands::{
        NativeCommands, NativePreviewCommands, PreviewBuildRequest, ProjectCompareAndSwapRequest,
        TargetPublishRequest,
    },
    preview::PreviewServer,
    target::NativeTargets,
    worker::NativeProductWorker,
};
use std::fs;
use tempfile::TempDir;

fn private_temp() -> TempDir {
    TempDir::new_in(env!("CARGO_MANIFEST_DIR")).unwrap()
}

fn generated_worker(workspaces: &std::path::Path) -> Option<NativeProductWorker> {
    let package = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let executable = package.join("dist/worker/forme-product-worker");
    let launcher = package.join("dist/worker/forme-sandbox-macos");
    if !executable.exists() || !launcher.exists() {
        return None;
    }
    let digest = fs::read_to_string(executable.with_extension("sha256")).unwrap();
    let launcher_digest = fs::read_to_string(launcher.with_extension("sha256")).unwrap();
    Some(
        NativeProductWorker::open(
            executable,
            digest.trim().to_owned(),
            launcher,
            launcher_digest.trim().to_owned(),
            workspaces.to_path_buf(),
        )
        .unwrap(),
    )
}

#[test]
fn empty_profile_to_exact_preview_and_atomic_local_publication() {
    let temp = private_temp();
    let workspaces = temp.path().join("workspaces");
    let Some(worker) = generated_worker(&workspaces) else {
        return;
    };
    let storage = NativeCommands::open(temp.path().join("profile")).unwrap();
    let project = serde_json::json!({
        "schemaVersion": 1,
        "projectId": "01952c0d-7e63-7000-8000-000000000068",
        "title": "Native product fixture",
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
                    "children": [{ "type": "text", "value": "Published from an empty profile" }]
                }]
            }
        }]
    });
    let envelope = serde_json::to_vec(&serde_json::json!({
        "cursor": 0,
        "history": [project],
        "historyLimit": 1,
        "schemaVersion": 1
    }))
    .unwrap();
    let revision = storage
        .project_compare_and_swap(ProjectCompareAndSwapRequest {
            expected_revision: None,
            bytes_base64: BASE64.encode(envelope),
        })
        .unwrap()
        .revision;

    let destination = temp.path().join("site");
    fs::create_dir(&destination).unwrap();
    let targets = NativeTargets::new(vec![temp.path().join("profile"), workspaces]).unwrap();
    let target = targets.configure_path(destination.clone()).unwrap();
    let product = NativePreviewCommands::open_with_server_targets_and_lifecycle(
        worker,
        PreviewServer::bind().unwrap(),
        targets,
        storage.lifecycle(),
    );

    let preview = storage
        .with_current_project(&revision, |persisted| {
            product.preview_build(
                persisted,
                PreviewBuildRequest {
                    revision: revision.clone(),
                },
            )
        })
        .unwrap();
    assert_eq!(preview.outcome, "ready");
    assert_eq!(preview.revision, revision);

    let publication = storage
        .with_current_project(&revision, |persisted| {
            product.target_publish(
                persisted,
                TargetPublishRequest {
                    target_id: target.target_id.clone(),
                    revision: revision.clone(),
                },
            )
        })
        .unwrap();
    assert_eq!(publication.outcome, "published");
    assert_eq!(publication.revision, revision);
    assert!(fs::read_to_string(destination.join("welcome.html"))
        .unwrap()
        .contains("Published from an empty profile"));
    product.workspace_dispose().unwrap();
    assert_eq!(
        storage.project_load(),
        Err(forme_shell_desktop_native::commands::CommandError::new(
            "WORKSPACE_POISONED"
        ))
    );
}
