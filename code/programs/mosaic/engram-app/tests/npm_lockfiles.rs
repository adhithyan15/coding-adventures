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

/// Packages allowed to declare an install script. The build installs with
/// `npm ci --ignore-scripts`, so none of these run; the list exists so that a
/// regenerated lock which brings in a NEW install-script package is reviewed
/// rather than slipping in -- the shape most npm supply-chain attacks take.
const INSTALL_SCRIPT_PACKAGES: &[&str] = &["@swc/core", "esbuild", "fsevents"];

fn lock_packages(lock: &str) -> serde_json::Map<String, Value> {
    let path = package_root()
        .join("npm")
        .join(lock)
        .join("package-lock.json");
    read_json(&path)["packages"]
        .as_object()
        .expect("packages map")
        .clone()
}

/// The package an entry installs: `node_modules/a/node_modules/@s/b` is
/// `@s/b`, unless the entry is an npm alias, which records the real name.
fn package_name<'a>(key: &'a str, entry: &'a Value) -> &'a str {
    entry["name"].as_str().unwrap_or_else(|| {
        key.rsplit_once("node_modules/")
            .map(|(_, name)| name)
            .unwrap_or(key)
    })
}

/// The directory whose `node_modules/` an entry was installed into:
/// `node_modules/a/node_modules/b` -> `node_modules/a`, `node_modules/a` -> ``.
fn parent_of(key: &str) -> &str {
    key.rfind("/node_modules/")
        .map(|at| &key[..at])
        .unwrap_or("")
}

/// Where Node would find dependency `name` from the package at `from`: its own
/// `node_modules/` first, then each enclosing one, up to the root's.
fn resolve<'a>(
    packages: &'a serde_json::Map<String, Value>,
    from: &str,
    name: &str,
) -> Option<&'a String> {
    let mut dir = from.to_string();
    loop {
        let candidate = if dir.is_empty() {
            format!("node_modules/{name}")
        } else {
            format!("{dir}/node_modules/{name}")
        };
        if let Some((key, _)) = packages.get_key_value(&candidate) {
            return Some(key);
        }
        if dir.is_empty() {
            return None;
        }
        dir = parent_of(&dir).to_string();
    }
}

#[test]
fn every_locked_package_is_the_registry_tarball_it_names() {
    for lock in ["web", "electron"] {
        for (key, entry) in lock_packages(lock) {
            if key.is_empty() {
                continue;
            }
            assert!(
                entry.get("link").is_none(),
                "npm/{lock}: {key} is a link, not a registry package"
            );
            // The tarball must be the one for this entry's own name and
            // version, so `node_modules/react` cannot quietly install some
            // other package's tarball.
            let name = package_name(&key, &entry);
            let version = entry["version"].as_str().unwrap_or_default();
            let basename = name.rsplit('/').next().unwrap_or(name);
            let expected = format!("https://registry.npmjs.org/{name}/-/{basename}-{version}.tgz");
            assert_eq!(
                entry["resolved"].as_str(),
                Some(expected.as_str()),
                "npm/{lock}: {key} resolves somewhere other than its registry tarball"
            );
            let integrity = entry["integrity"].as_str().unwrap_or_default();
            assert!(
                integrity.starts_with("sha512-"),
                "npm/{lock}: {key} has no sha512 integrity hash"
            );
        }
    }
}

#[test]
fn every_locked_package_is_reachable_from_the_project() {
    // An entry nothing depends on is still installed by `npm ci`. Walk the
    // dependency graph from the root the way Node resolves modules and fail on
    // any entry the walk never reaches.
    //
    // Two edges are treated strictly, because each is a way to smuggle a
    // package in under a name the walk would otherwise accept:
    //
    //   optional peer   `debug` lists `supports-color` as an optional peer, so
    //                   following it would bless an injected `supports-color`
    //                   -- which `debug` then loads. Optional peers are not
    //                   followed; a non-optional peer is installed by npm and is.
    //   alias           An entry's own `"name"` says which tarball it really
    //                   is. It may differ from the directory name only when the
    //                   dependent asked for exactly that, `npm:<name>@<range>`;
    //                   otherwise `node_modules/picocolors` could be `once`.
    for lock in ["web", "electron"] {
        let packages = lock_packages(lock);
        let mut reached = std::collections::BTreeSet::from([String::new()]);
        let mut queue = vec![String::new()];
        while let Some(key) = queue.pop() {
            let entry = &packages[&key];
            let mut sections = vec!["dependencies", "optionalDependencies", "peerDependencies"];
            if key.is_empty() {
                sections.push("devDependencies");
            }
            for section in sections {
                let Some(deps) = entry[section].as_object() else {
                    continue;
                };
                for (name, spec) in deps {
                    if section == "peerDependencies"
                        && entry["peerDependenciesMeta"][name]["optional"].as_bool() == Some(true)
                    {
                        continue;
                    }
                    // A missing optional or peer dependency is npm's to judge.
                    let Some(found) = resolve(&packages, &key, name) else {
                        continue;
                    };
                    if let Some(real) = packages[found]["name"].as_str() {
                        let spec = spec.as_str().unwrap_or_default();
                        assert!(
                            real == name || spec.starts_with(&format!("npm:{real}@")),
                            "npm/{lock}: {found} installs {real:?}, but {key:?} asked \
                             for {name:?} as {spec:?}, not as an alias of it"
                        );
                    }
                    if reached.insert(found.clone()) {
                        queue.push(found.clone());
                    }
                }
            }
        }
        let unreachable: Vec<_> = packages
            .keys()
            .filter(|key| !reached.contains(*key))
            .collect();
        assert!(
            unreachable.is_empty(),
            "npm/{lock}: entries no dependency reaches: {unreachable:?}"
        );
    }
}

#[test]
fn only_known_packages_declare_install_scripts() {
    for lock in ["web", "electron"] {
        for (key, entry) in lock_packages(lock) {
            if entry["hasInstallScript"].as_bool() != Some(true) {
                continue;
            }
            let name = package_name(&key, &entry);
            assert!(
                INSTALL_SCRIPT_PACKAGES.contains(&name),
                "npm/{lock}: {key} has an install script; review it and add it to \
                 INSTALL_SCRIPT_PACKAGES only if it is expected"
            );
        }
    }
}
