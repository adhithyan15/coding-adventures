//! The committed npm lockfiles still match what the emitter writes.
//!
//! `scripts/build-web.sh` and `scripts/build-electron.sh` install the emitted
//! React and Electron projects with `npm ci` from lockfiles committed beside
//! this test (`npm/web/` and `npm/electron/`). `npm ci` refuses to install when
//! package.json and the lock disagree, which is the safety property -- but the
//! scripts only run in the release lanes, so on its own that refusal would
//! surface at release time, far from the change that caused it.
//!
//! This test moves the failure to the PR that causes it. The emitter lives in
//! `mosaic-package-artifact-builder`, a dependency of this crate, so a change
//! there that bumps an npm version reruns this test and fails it here:
//!
//! ```text
//!   emitter writes "vite": "7.3.7"     lock root says "vite": "7.3.6"
//!                  |                                  |
//!                  +------------ mismatch ------------+
//!                                    |
//!       "regenerate with scripts/build-web.sh --update-lock"
//! ```
//!
//! The comparison is against the lock's root entry (`packages[""]`), which is
//! npm's own record of the package.json it was resolved from -- the same thing
//! `npm ci` compares.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use mosaic_package_artifact_builder::{build_package, Backend, BuildOptions};
use serde_json::Value;

fn package_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_json(path: &PathBuf) -> Value {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{} is not JSON: {e}", path.display()))
}

/// `{name: version}` for one dependency section; empty when absent.
fn section(manifest: &Value, key: &str) -> BTreeMap<String, String> {
    manifest
        .get(key)
        .and_then(Value::as_object)
        .map(|deps| {
            deps.iter()
                .map(|(name, version)| {
                    let version = version
                        .as_str()
                        .unwrap_or_else(|| panic!("{key}.{name} is not a string"));
                    (name.clone(), version.to_string())
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The package.json `backend` emits for Engram.
fn emitted_package_json(backend: Backend, dir_name: &str) -> Value {
    let tmp = tempfile::tempdir().expect("temp dist root");
    build_package(&BuildOptions {
        package_root: package_root(),
        output_root: tmp.path().to_path_buf(),
        backend,
        emit_project: true,
        theme: None,
    })
    .unwrap_or_else(|e| panic!("{backend:?} should emit the Engram project: {e}"));
    read_json(&tmp.path().join(dir_name).join("package.json"))
}

/// The lock's record of the package.json it was resolved from.
fn lock_root(lock: &str) -> Value {
    let path = package_root()
        .join("npm")
        .join(lock)
        .join("package-lock.json");
    let lockfile = read_json(&path);
    assert_eq!(
        lockfile["lockfileVersion"],
        3,
        "{} should be a lockfileVersion 3 lock",
        path.display()
    );
    lockfile["packages"][""].clone()
}

/// The devDependencies build-electron.sh writes into the emitted package.json
/// after emission, read from the script itself: lines of the form
/// `dev["name"] = "version"`.
fn electron_script_dev_dependencies() -> BTreeMap<String, String> {
    let path = package_root().join("scripts").join("build-electron.sh");
    let script = fs::read_to_string(&path).expect("read build-electron.sh");
    script
        .lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("dev[\"")?;
            let (name, rest) = rest.split_once("\"] = \"")?;
            let version = rest.strip_suffix('"')?;
            Some((name.to_string(), version.to_string()))
        })
        .collect()
}

fn assert_section_matches(
    lock: &str,
    key: &str,
    expected: &BTreeMap<String, String>,
    root: &Value,
) {
    assert_eq!(
        &section(root, key),
        expected,
        "npm/{lock}/package-lock.json `{key}` no longer matches the emitted \
         package.json; regenerate it with `scripts/build-{lock}.sh --update-lock`, \
         review the diff, and commit it"
    );
}

#[test]
fn web_lock_matches_the_emitted_react_project() {
    let emitted = emitted_package_json(Backend::React, "react");
    let root = lock_root("web");
    for key in ["dependencies", "devDependencies"] {
        assert_section_matches("web", key, &section(&emitted, key), &root);
    }
}

#[test]
fn electron_lock_matches_the_emitted_project_plus_packaging_tools() {
    let emitted = emitted_package_json(Backend::Electron, "electron");
    let root = lock_root("electron");

    let injected = electron_script_dev_dependencies();
    // The packaging tools: electron-builder makes the installer and
    // @electron/asar opens it again to check the engine is inside. Both must
    // come from the lock, which means both must be injected here.
    for tool in ["electron-builder", "@electron/asar"] {
        assert!(
            injected.contains_key(tool),
            "build-electron.sh should pin {tool} as a devDependency"
        );
    }

    let mut dev = section(&emitted, "devDependencies");
    dev.extend(injected);
    assert_section_matches(
        "electron",
        "dependencies",
        &section(&emitted, "dependencies"),
        &root,
    );
    assert_section_matches("electron", "devDependencies", &dev, &root);
}

#[test]
fn locks_resolve_only_from_the_public_registry_with_integrity() {
    for lock in ["web", "electron"] {
        let path = package_root()
            .join("npm")
            .join(lock)
            .join("package-lock.json");
        let lockfile = read_json(&path);
        let packages = lockfile["packages"].as_object().expect("packages map");
        for (key, entry) in packages {
            if key.is_empty() || entry.get("link").is_some() {
                continue;
            }
            let resolved = entry["resolved"].as_str().unwrap_or_default();
            assert!(
                resolved.starts_with("https://registry.npmjs.org/"),
                "npm/{lock}: {key} resolves from {resolved:?}, not the npm registry"
            );
            let integrity = entry["integrity"].as_str().unwrap_or_default();
            assert!(
                integrity.starts_with("sha512-"),
                "npm/{lock}: {key} has no sha512 integrity hash"
            );
        }
    }
}
