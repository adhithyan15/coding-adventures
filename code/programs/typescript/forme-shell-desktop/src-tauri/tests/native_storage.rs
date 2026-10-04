use forme_shell_desktop_native::{
    identity::create_document_identity,
    storage::{NativeProjectStore, StoreError, MAX_PROJECT_BYTES},
};
use std::fs;
use tempfile::TempDir;
use uuid::{Uuid, Variant, Version};

const FIRST: &[u8] = br#"{"cursor":0,"history":[{"activeDocumentId":null,"documents":[],"projectId":"01952c0d-7e63-7000-8000-000000000068","schemaVersion":1,"site":{"baseUrl":null,"themeId":"forme-classless"},"title":"First","workflow":{"lastPublication":null}}],"historyLimit":1,"schemaVersion":1}"#;
const SECOND: &[u8] = br#"{"cursor":0,"history":[{"activeDocumentId":null,"documents":[],"projectId":"01952c0d-7e63-7000-8000-000000000068","schemaVersion":1,"site":{"baseUrl":null,"themeId":"forme-classless"},"title":"Second","workflow":{"lastPublication":null}}],"historyLimit":1,"schemaVersion":1}"#;
const CURRENT_PROJECT: &[u8] = br#"{"cursor":0,"history":[{"activeDocumentId":null,"documents":[],"projectId":"01952c0d-7e63-7000-8000-000000000068","schemaVersion":1,"site":{"baseUrl":null,"themeId":"forme-classless"},"title":"Current","workflow":{"lastPublication":null}}],"historyLimit":1,"schemaVersion":1}"#;

fn open_store(temp: &TempDir) -> NativeProjectStore {
    NativeProjectStore::open(temp.path().join("profile")).expect("owned profile")
}

#[cfg(target_os = "macos")]
#[test]
fn rejects_a_profile_root_with_a_mutating_extended_acl_grant() {
    use std::process::Command;

    let temp = TempDir::new().unwrap();
    let profile = temp.path().join("profile");
    fs::create_dir(&profile).unwrap();
    assert!(Command::new("chmod")
        .args(["+a", "everyone allow write,delete,add_file,delete_child"])
        .arg(&profile)
        .status()
        .unwrap()
        .success());
    assert!(matches!(
        NativeProjectStore::open(profile),
        Err(StoreError::UnsafeProfile)
    ));
}

#[test]
fn missing_profile_loads_as_absent_and_initial_compare_and_swap_is_durable() {
    let temp = TempDir::new().unwrap();
    let store = open_store(&temp);
    assert_eq!(store.load().unwrap(), None);

    let revision = store.compare_and_swap(None, FIRST).unwrap();
    let loaded = store.load().unwrap().expect("committed profile");
    assert_eq!(loaded.bytes, FIRST);
    assert_eq!(loaded.revision, revision);

    let reopened = open_store(&temp).load().unwrap().expect("restart profile");
    assert_eq!(reopened, loaded);
}

#[test]
fn compare_and_swap_rejects_stale_or_missing_expectations_without_mutation() {
    let temp = TempDir::new().unwrap();
    let store = open_store(&temp);
    let first_revision = store.compare_and_swap(None, FIRST).unwrap();

    assert_eq!(
        store.compare_and_swap(None, SECOND),
        Err(StoreError::Conflict)
    );
    assert_eq!(
        store.compare_and_swap(Some("sha256:stale"), SECOND),
        Err(StoreError::Conflict)
    );
    assert_eq!(store.load().unwrap().unwrap().bytes, FIRST);

    let second_revision = store
        .compare_and_swap(Some(&first_revision), SECOND)
        .unwrap();
    assert_ne!(second_revision, first_revision);
    assert_eq!(store.load().unwrap().unwrap().bytes, SECOND);
}

#[test]
fn exact_revision_operation_reads_the_persisted_current_project_under_the_transaction() {
    let temp = TempDir::new().unwrap();
    let store = open_store(&temp);
    let revision = store.compare_and_swap(None, CURRENT_PROJECT).unwrap();

    let title = store
        .with_project_at_revision(&revision, |project| {
            project["title"].as_str().unwrap().to_owned()
        })
        .unwrap();
    assert_eq!(title, "Current");
    assert_eq!(
        store.with_project_at_revision("sha256:stale", |_| ()),
        Err(StoreError::Conflict)
    );
}

#[test]
fn rejects_noncanonical_or_oversized_input_before_touching_the_store() {
    let temp = TempDir::new().unwrap();
    let store = open_store(&temp);

    assert_eq!(
        store.compare_and_swap(None, br#"{ "schemaVersion": 1 }"#),
        Err(StoreError::InvalidBytes)
    );
    assert_eq!(
        store.compare_and_swap(None, &vec![b'x'; MAX_PROJECT_BYTES + 1]),
        Err(StoreError::InvalidBytes)
    );
    assert_eq!(store.load().unwrap(), None);
}

#[test]
fn rejects_canonical_but_invalid_session_and_project_shapes() {
    let temp = TempDir::new().unwrap();
    let store = open_store(&temp);
    for invalid in [
        br#"{"cursor":0,"history":[],"historyLimit":1,"schemaVersion":1}"#.as_slice(),
        br#"{"cursor":1,"history":[{"activeDocumentId":null,"documents":[],"projectId":"01952c0d-7e63-7000-8000-000000000068","schemaVersion":1,"site":{"baseUrl":null,"themeId":"forme-classless"},"title":"First","workflow":{"lastPublication":null}}],"historyLimit":1,"schemaVersion":1}"#.as_slice(),
        br#"{"cursor":0,"history":[{"activeDocumentId":null,"documents":[],"projectId":"01952c0d-7e63-7000-8000-000000000068","schemaVersion":1,"site":{"baseUrl":"file:///etc/passwd","themeId":"forme-classless"},"title":"First","workflow":{"lastPublication":null}}],"historyLimit":1,"schemaVersion":1}"#.as_slice(),
    ] {
        assert_eq!(
            store.compare_and_swap(None, invalid),
            Err(StoreError::InvalidBytes)
        );
    }
    assert_eq!(store.load().unwrap(), None);
}

#[test]
fn matches_the_shared_typescript_storage_parity_corpus() {
    let cases: serde_json::Value = serde_json::from_slice(
        &fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join("tests/fixtures/storage-parity.json"),
        )
        .unwrap(),
    )
    .unwrap();
    for parity in cases.as_array().unwrap() {
        let temp = TempDir::new().unwrap();
        let store = open_store(&temp);
        let bytes = serde_json::to_vec(&parity["value"]).unwrap();
        let accepted = store.compare_and_swap(None, &bytes).is_ok();
        assert_eq!(
            accepted,
            parity["valid"].as_bool().unwrap(),
            "{}",
            parity["name"]
        );
    }
}

#[test]
fn rejects_any_json_array_above_the_authoring_entry_ceiling() {
    let temp = TempDir::new().unwrap();
    let store = open_store(&temp);
    let mut envelope: serde_json::Value = serde_json::from_slice(FIRST).unwrap();
    let project = &mut envelope["history"][0];
    project["activeDocumentId"] = serde_json::json!("01952c0d-7e63-7000-8000-000000000069");
    project["documents"] = serde_json::json!([{
        "body": {
            "children": [{
                "align": vec![serde_json::Value::Null; 100_001],
                "children": [],
                "type": "table"
            }],
            "type": "document"
        },
        "id": "01952c0d-7e63-7000-8000-000000000069",
        "slug": "welcome",
        "status": "draft",
        "title": "Welcome"
    }]);
    let bytes = serde_json::to_vec(&envelope).unwrap();
    assert_eq!(
        store.compare_and_swap(None, &bytes),
        Err(StoreError::InvalidBytes)
    );
}

#[test]
fn malformed_existing_store_fails_closed_without_replacement() {
    let temp = TempDir::new().unwrap();
    let store = open_store(&temp);
    let path = temp.path().join("profile").join("authoring-session.json");
    fs::write(&path, b"not json").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    }

    assert_eq!(store.load(), Err(StoreError::MalformedStore));
    assert_eq!(
        store.compare_and_swap(None, FIRST),
        Err(StoreError::MalformedStore)
    );
    assert_eq!(fs::read(path).unwrap(), b"not json");
}

#[cfg(unix)]
#[test]
fn rejects_symlinked_profile_roots_and_store_files() {
    use std::os::unix::fs::symlink;

    let temp = TempDir::new().unwrap();
    let outside = temp.path().join("outside");
    fs::create_dir(&outside).unwrap();
    let linked_root = temp.path().join("linked-profile");
    symlink(&outside, &linked_root).unwrap();
    assert!(matches!(
        NativeProjectStore::open(linked_root),
        Err(StoreError::UnsafeProfile)
    ));

    let store = open_store(&temp);
    let outside_file = outside.join("outside.json");
    fs::write(&outside_file, FIRST).unwrap();
    symlink(
        &outside_file,
        temp.path().join("profile").join("authoring-session.json"),
    )
    .unwrap();
    assert_eq!(store.load(), Err(StoreError::UnsafeProfile));
    assert_eq!(
        store.compare_and_swap(None, SECOND),
        Err(StoreError::UnsafeProfile)
    );
    assert_eq!(fs::read(outside_file).unwrap(), FIRST);
}

#[cfg(unix)]
#[test]
fn committed_store_is_owner_only() {
    use std::os::unix::fs::PermissionsExt;

    let temp = TempDir::new().unwrap();
    let store = open_store(&temp);
    store.compare_and_swap(None, FIRST).unwrap();
    let profile_mode = fs::metadata(temp.path().join("profile"))
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    let file_mode = fs::metadata(temp.path().join("profile/authoring-session.json"))
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(profile_mode, 0o700);
    assert_eq!(file_mode, 0o600);
}

#[test]
fn document_identity_is_canonical_uuid_v7() {
    let text = create_document_identity();
    let parsed = Uuid::parse_str(&text).unwrap();
    assert_eq!(parsed.get_version(), Some(Version::SortRand));
    assert_eq!(parsed.get_variant(), Variant::RFC4122);
    assert_eq!(parsed.hyphenated().to_string(), text);
}
