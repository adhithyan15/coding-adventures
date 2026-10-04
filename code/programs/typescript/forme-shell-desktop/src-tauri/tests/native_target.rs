use forme_shell_desktop_native::{
    target::{NativeTargets, TargetError},
    worker::NativeProductWorker,
};
use std::fs;
use tempfile::TempDir;

fn private_temp() -> TempDir {
    TempDir::new_in(env!("CARGO_MANIFEST_DIR")).unwrap()
}

fn generated_worker(workspaces: &std::path::Path) -> Option<NativeProductWorker> {
    let suffix = if cfg!(windows) { ".exe" } else { "" };
    let package = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let executable = package
        .join("dist/worker")
        .join(format!("forme-product-worker{suffix}"));
    let launcher = package.join("dist/worker/forme-sandbox-macos");
    if !executable.exists() || !launcher.exists() {
        return None;
    }
    let digest = fs::read_to_string(executable.with_extension(if cfg!(windows) {
        "exe.sha256"
    } else {
        "sha256"
    }))
    .unwrap();
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

fn project(text: &str) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": 1,
        "projectId": "01952c0d-7e63-7000-8000-000000000068",
        "title": "Native publication fixture",
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
                    "children": [{ "type": "text", "value": text }]
                }]
            }
        }]
    })
}

#[test]
fn configures_only_safe_directories_and_never_discloses_the_native_path() {
    let temp = private_temp();
    let forbidden = temp.path().join("profile");
    let target = temp.path().join("chosen-folder");
    fs::create_dir(&forbidden).unwrap();
    fs::create_dir(&target).unwrap();
    let targets = NativeTargets::new(vec![forbidden.clone()]).unwrap();
    assert_eq!(targets.configure_path(forbidden), Err(TargetError::Unsafe));
    let review = targets.configure_path(target.clone()).unwrap();
    assert!(review.target_id.starts_with("local-"));
    assert_eq!(review.label, "Local folder");
    assert!(review.destination.contains("chosen-folder"));
    assert!(!review.destination.contains(temp.path().to_str().unwrap()));
    assert_eq!(targets.list().unwrap(), vec![review]);
}

#[test]
fn atomically_replaces_only_an_empty_or_owned_complete_tree() {
    let temp = private_temp();
    let Some(worker) = generated_worker(&temp.path().join("workspaces")) else {
        return;
    };
    let destination = temp.path().join("site");
    fs::create_dir(&destination).unwrap();
    let targets = NativeTargets::new(vec![temp.path().join("workspaces")]).unwrap();
    let review = targets.configure_path(destination.clone()).unwrap();

    let first = worker
        .build(&project("First publication"), "revision-1")
        .unwrap();
    let receipt = targets.publish(&review.target_id, &first).unwrap();
    assert_eq!(receipt.target_id, review.target_id);
    assert_eq!(receipt.manifest_sha256.len(), 44);
    assert!(fs::read_to_string(destination.join("welcome.html"))
        .unwrap()
        .contains("First publication"));
    assert_eq!(
        fs::read_to_string(destination.join(".forme-owner")).unwrap(),
        review.target_id
    );

    let second = worker
        .build(&project("Second publication"), "revision-2")
        .unwrap();
    targets.publish(&review.target_id, &second).unwrap();
    assert!(fs::read_to_string(destination.join("welcome.html"))
        .unwrap()
        .contains("Second publication"));

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let outside = temp.path().join("outside");
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("keep.txt"), "outside data").unwrap();
        symlink(&outside, destination.join("nested-link")).unwrap();
        let third = worker
            .build(&project("Unsafe publication"), "revision-3")
            .unwrap();
        assert_eq!(
            targets.publish(&review.target_id, &third),
            Err(TargetError::Unsafe)
        );
        third.retire().unwrap();
        assert_eq!(
            fs::read_to_string(outside.join("keep.txt")).unwrap(),
            "outside data"
        );
    }
}

#[test]
fn refuses_unowned_content_and_changed_directory_identity() {
    let temp = private_temp();
    let unowned = temp.path().join("unowned");
    fs::create_dir(&unowned).unwrap();
    fs::write(unowned.join("keep.txt"), "user data").unwrap();
    let targets = NativeTargets::new(Vec::new()).unwrap();
    let review = targets.configure_path(unowned.clone()).unwrap();

    let Some(worker) = generated_worker(&temp.path().join("workspaces")) else {
        return;
    };
    let run = worker.build(&project("Refused"), "revision-1").unwrap();
    assert_eq!(
        targets.publish(&review.target_id, &run),
        Err(TargetError::Unsafe)
    );
    assert_eq!(
        fs::read_to_string(unowned.join("keep.txt")).unwrap(),
        "user data"
    );

    let empty = temp.path().join("empty");
    let moved = temp.path().join("moved");
    fs::create_dir(&empty).unwrap();
    let stale = targets.configure_path(empty.clone()).unwrap();
    fs::rename(&empty, &moved).unwrap();
    fs::create_dir(&empty).unwrap();
    assert_eq!(
        targets.publish(&stale.target_id, &run),
        Err(TargetError::Unsafe)
    );
}

#[cfg(unix)]
#[test]
fn rejects_shared_writable_targets_and_non_sticky_ancestors() {
    use std::os::unix::fs::PermissionsExt;

    let temp = private_temp();
    let target = temp.path().join("target");
    fs::create_dir(&target).unwrap();
    fs::set_permissions(&target, fs::Permissions::from_mode(0o777)).unwrap();
    let targets = NativeTargets::new(Vec::new()).unwrap();
    assert_eq!(targets.configure_path(target), Err(TargetError::Unsafe));

    let ancestor = temp.path().join("shared");
    let child = ancestor.join("child");
    fs::create_dir_all(&child).unwrap();
    fs::set_permissions(&ancestor, fs::Permissions::from_mode(0o777)).unwrap();
    assert_eq!(targets.configure_path(child), Err(TargetError::Unsafe));
}

#[cfg(target_os = "macos")]
#[test]
fn rejects_mutating_extended_acl_grants_on_targets_and_ancestors() {
    use std::process::Command;

    let temp = private_temp();
    let targets = NativeTargets::new(Vec::new()).unwrap();
    let target = temp.path().join("acl-target");
    fs::create_dir(&target).unwrap();
    assert!(Command::new("chmod")
        .args(["+a", "everyone allow write,delete,add_file,delete_child"])
        .arg(&target)
        .status()
        .unwrap()
        .success());
    assert_eq!(targets.configure_path(target), Err(TargetError::Unsafe));

    let ancestor = temp.path().join("acl-ancestor");
    let child = ancestor.join("child");
    fs::create_dir_all(&child).unwrap();
    assert!(Command::new("chmod")
        .args(["+a", "everyone allow write,delete,add_file,delete_child"])
        .arg(&ancestor)
        .status()
        .unwrap()
        .success());
    assert_eq!(targets.configure_path(child), Err(TargetError::Unsafe));
}
