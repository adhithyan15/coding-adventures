use smart_home_home_assistant_migration::HomeAssistantMigrationArtifact;
use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn cli_writes_dry_run_and_applied_artifacts() {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock should be after unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("ha-migration-cli-{nanos}"));
    fs::create_dir_all(&root).unwrap();
    let input = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/basic-export.json"
    );

    let dry_run_path = root.join("dry-run.json");
    let dry_run = Command::new(env!("CARGO_BIN_EXE_smart-home-import-home-assistant"))
        .args([input, dry_run_path.to_str().unwrap(), "--dry-run"])
        .output()
        .unwrap();
    assert!(
        dry_run.status.success(),
        "{}",
        String::from_utf8_lossy(&dry_run.stderr)
    );
    let dry_run: HomeAssistantMigrationArtifact =
        serde_json::from_slice(&fs::read(&dry_run_path).unwrap()).unwrap();
    assert!(dry_run.dry_run);
    assert!(dry_run.receipt.is_none());
    assert_eq!(dry_run.plan.summary.entities, 1);

    let applied_path = root.join("applied.json");
    let applied = Command::new(env!("CARGO_BIN_EXE_smart-home-import-home-assistant"))
        .args([input, applied_path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        applied.status.success(),
        "{}",
        String::from_utf8_lossy(&applied.stderr)
    );
    let applied: HomeAssistantMigrationArtifact =
        serde_json::from_slice(&fs::read(&applied_path).unwrap()).unwrap();
    assert!(!applied.dry_run);
    assert!(applied.receipt.is_some());
    assert_eq!(applied.runtime_snapshot.unwrap().entities.len(), 1);
    assert_eq!(applied.automation_snapshot.unwrap().definitions.len(), 1);

    fs::remove_dir_all(root).unwrap();
}
