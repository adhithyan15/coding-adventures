//! Cargo owns artifact locations. Exercise a nondefault directory in a child
//! process so other integration tests never observe a changed environment.

fn artifact(target: &str, kinds: &[&str], files: &[&str]) -> String {
    serde_json::json!({
        "reason": "compiler-artifact",
        "target": {"name": target, "crate_types": kinds},
        "filenames": files,
    })
    .to_string()
}

#[test]
fn selects_static_archive_among_other_cargo_outputs() {
    for name in ["libgc_core_capi.a", "gc_core_capi.lib"] {
        let root = tempfile::tempdir().unwrap();
        let archive = root.path().join(name);
        let record = artifact(
            "gc_core_capi",
            &["lib", "cdylib", "staticlib"],
            &[
                "libgc_core_capi.rlib",
                "gc_core_capi.dll",
                "gc_core_capi.dll.lib",
                archive.to_str().unwrap(),
            ],
        );
        let unrelated = artifact("other", &["staticlib"], &["libgc_core_capi.a"]);
        let stream =
            format!("{unrelated}\n{record}\n{{\"reason\":\"build-finished\",\"success\":true}}\n");
        assert_eq!(
            common::gc_staticlib_from_messages(&stream).unwrap(),
            archive
        );
    }
}

#[test]
fn rejects_missing_ambiguous_and_malformed_artifacts() {
    for stream in [
        String::new(),
        artifact("gc_core_capi", &["cdylib"], &["gc_core_capi.lib"]),
        artifact("gc_core_capi", &["staticlib"], &["gc_core_capi.dll.lib"]),
        artifact(
            "gc_core_capi",
            &["staticlib"],
            &["gc_core_capi.lib", "libgc_core_capi.a"],
        ),
        String::from("not JSON"),
    ] {
        assert!(
            common::gc_staticlib_from_messages(&stream).is_err(),
            "accepted {stream}"
        );
    }
}

// This target exercises archive lookup, not common's platform linker arguments.
#[allow(dead_code)]
mod common;

#[test]
fn archive_lookup_honors_cargo_target_dir() {
    let root = tempfile::tempdir().expect("temporary build root");
    let target = root.path().join("target output with spaces");
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "archive_lookup_child",
            "--nocapture",
        ])
        .env("CARGO_TARGET_DIR", &target)
        .env("LANG_ARCHIVE_PROBE_DIR", &target)
        .output()
        .expect("launch archive lookup child");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "archive child failed:\n{stdout}\n{stderr}"
    );
    assert!(
        stdout.contains("GC_ARCHIVE_PROBE_PASSED"),
        "child must execute: {stdout}"
    );
}

#[test]
#[ignore = "invoked only by the parent with an isolated Cargo environment"]
fn archive_lookup_child() {
    let expected = std::path::PathBuf::from(
        std::env::var_os("LANG_ARCHIVE_PROBE_DIR").expect("parent supplies target directory"),
    );
    let archive = common::gc_core_capi_archive();
    assert!(
        archive.is_file(),
        "archive does not exist: {}",
        archive.display()
    );
    assert!(
        archive
            .canonicalize()
            .unwrap()
            .starts_with(expected.canonicalize().unwrap()),
        "archive {} must be under {}",
        archive.display(),
        expected.display(),
    );
    println!("GC_ARCHIVE_PROBE_PASSED {}", archive.display());
}
