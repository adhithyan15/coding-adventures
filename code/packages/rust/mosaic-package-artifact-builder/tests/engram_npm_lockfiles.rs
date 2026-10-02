//! A change to the npm versions this crate writes must come with Engram's
//! regenerated lockfiles.
//!
//! Engram's release lanes install the React and Electron projects this crate
//! emits with `npm ci`, from lockfiles committed in
//! `code/programs/mosaic/engram-app/npm/`. `npm ci` refuses to install when the
//! emitted package.json names a version the lock does not have. That is the
//! safety property, but those lanes run only when Engram itself changes, so a
//! version bump made here would otherwise first fail at release time.
//!
//! The engram-app package checks the same lockfiles in its own tests; this
//! copy exists because the build tool reruns a crate's tests when its
//! `[dependencies]` change, and engram-app only dev-depends on this crate.
//!
//! What is compared is each lock's root entry, `packages[""]` -- npm's record
//! of the package.json it was resolved from, and what `npm ci` checks.

use std::fs;
use std::path::PathBuf;

use mosaic_package_artifact_builder::{build_package, Backend, BuildOptions};
use serde_json::Value;

fn engram_app() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../programs/mosaic/engram-app")
}

fn read_json(path: &PathBuf) -> Value {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{} is not JSON: {e}", path.display()))
}

fn assert_emitted_versions_are_locked(backend: Backend, dir_name: &str, lock: &str) {
    let out = tempfile::tempdir().expect("temp dist root");
    build_package(&BuildOptions {
        package_root: engram_app(),
        output_root: out.path().to_path_buf(),
        backend,
        emit_project: true,
        theme: None,
    })
    .unwrap_or_else(|e| panic!("{backend:?} should emit the Engram project: {e:?}"));
    let emitted = read_json(&out.path().join(dir_name).join("package.json"));
    let root = read_json(
        &engram_app()
            .join("npm")
            .join(lock)
            .join("package-lock.json"),
    )["packages"][""]
        .clone();

    for section in ["dependencies", "devDependencies"] {
        let Some(deps) = emitted[section].as_object() else {
            continue;
        };
        for (name, version) in deps {
            assert_eq!(
                &root[section][name], version,
                "{backend:?} emits {section}.{name} = {version}, but \
                 engram-app/npm/{lock}/package-lock.json locks {}; regenerate it with \
                 `code/programs/mosaic/engram-app/scripts/build-{lock}.sh --update-lock` \
                 and commit the result",
                root[section][name]
            );
        }
    }
}

#[test]
fn engram_web_lock_covers_the_emitted_react_versions() {
    assert_emitted_versions_are_locked(Backend::React, "react", "web");
}

#[test]
fn engram_electron_lock_covers_the_emitted_electron_versions() {
    assert_emitted_versions_are_locked(Backend::Electron, "electron", "electron");
}
