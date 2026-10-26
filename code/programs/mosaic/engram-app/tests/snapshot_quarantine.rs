//! An Engram host must never save the demo deck over a snapshot it could not
//! load.
//!
//! Each hand-written Engram host loads a persisted snapshot at startup and,
//! if that fails, resets to the demo deck and saves:
//!
//! ```text
//!   host                        snapshot lives in
//!   --------------------------  ----------------------------------------------
//!   host/electron/host.js       ENGRAM_SNAPSHOT_PATH, else ~/.engram/mosaic-snapshot.v1.json
//!   host/web/engram-host.mjs    localStorage["engram.snapshot.v1"]
//!   host/xaml/MosaicHost.cs     the same file as Electron (pinned in package_compiles)
//! ```
//!
//! Before these tests existed that save simply replaced the unreadable
//! snapshot, so one written by a newer build, or damaged by a crash
//! mid-write, was destroyed along with someone's study history. The contract
//! they pin, for every host:
//!
//! ```text
//!   snapshot loads?   copied aside?                 what is stored afterwards
//!   ---------------   ---------------------------   ---------------------------
//!   yes               (not attempted)               the snapshot, untouched
//!   no                to the first free .corrupt    copy = old, snapshot = demo
//!   no                fails, or 10 names all taken  snapshot = old; nothing saved
//! ```
//!
//! "First free" is `.corrupt`, then `.corrupt.1` … `.corrupt.9`. A copy never
//! replaces an earlier quarantine, because the earlier one may be the real
//! history: startup rejects it, and then a reload after an import rejects the
//! merged result too.
//!
//! The tests run the real host files under Node with stand-ins only at the
//! edges: a fake wasm engine whose `loadSnapshot` accepts exactly the text
//! `GOOD` and whose `snapshot` returns a fixed demo state, plus (for
//! Electron) an `electron` package exporting an unused `dialog`, or (for the
//! web host) a `window` with an in-memory `localStorage` and a `fetch` that
//! returns an empty wasm body. Everything between them — the load, the
//! quarantine, the save — is the shipped code. When `node` is not on `PATH`
//! the tests say so and pass, because the package's own build does not
//! otherwise need Node.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const FAKE_ENGINE: &str = r#"
export function createEngramEngine() {
  return {
    loadSnapshot(text) {
      return text === "GOOD" ? { ok: true } : { ok: false, error: "rejected" };
    },
    resetDemo() {},
    snapshot() {
      return { ok: true, state: { demo: true } };
    },
    createMosaicHost() {
      return { async handleEvent() { return {}; } };
    },
  };
}
"#;

const DEMO_STATE: &str = r#"{"demo":true}"#;
const STUDY_HISTORY: &str = "a newer build's study history";

fn host_file(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

/// The engine stand-in and an ES-module `package.json`, shared by both hosts.
fn stage_engine(root: &Path) {
    fs::write(root.join("engram-mosaic-host-wasm.mjs"), FAKE_ENGINE).expect("fake engine");
    fs::write(root.join("engram_engine.wasm"), b"").expect("wasm placeholder");
    fs::write(root.join("package.json"), r#"{"type":"module"}"#).expect("package.json");
}

/// Run an ES-module `script` with Node from `root`. Returns `None` when Node
/// is absent, and fails the test when the script itself fails.
fn run_node(root: &Path, envs: &[(&str, &Path)], script: &str) -> Option<Output> {
    let mut command = Command::new("node");
    command
        .current_dir(root)
        .args(["--input-type=module", "-e", script]);
    for (key, value) in envs {
        command.env(key, value);
    }
    let output = match command.output() {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("node is not on PATH; skipping the Engram snapshot quarantine test");
            return None;
        }
        Err(error) => panic!("could not run node: {error}"),
    };
    assert!(
        output.status.success(),
        "the host failed to start\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Some(output)
}

// --------------------------------------------------------------------------
// Electron: the snapshot is a file
// --------------------------------------------------------------------------

/// Lay out a directory where `import("./host.js")` resolves: the host, the
/// engine stand-in beside it, and a stub `electron` package.
fn stage_electron(root: &Path) {
    fs::copy(host_file("host/electron/host.js"), root.join("host.js")).expect("copy host.js");
    stage_engine(root);
    let electron = root.join("node_modules/electron");
    fs::create_dir_all(&electron).expect("electron stub dir");
    fs::write(
        electron.join("package.json"),
        r#"{"name":"electron","type":"module","main":"index.js"}"#,
    )
    .expect("electron package.json");
    fs::write(electron.join("index.js"), "export const dialog = {};\n").expect("electron stub");
}

/// Start the Electron host once against `snapshot`.
fn start_electron(root: &Path, snapshot: &Path) -> Option<Output> {
    run_node(
        root,
        &[("ENGRAM_SNAPSHOT_PATH", snapshot)],
        "const m = await import('./host.js'); await m.createMosaicHost();",
    )
}

struct ElectronCase {
    _tmp: tempfile::TempDir,
    root: PathBuf,
    snapshot: PathBuf,
    corrupt: PathBuf,
}

fn electron_case(contents: &str) -> ElectronCase {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().to_path_buf();
    stage_electron(&root);
    let snapshot = root.join("state/mosaic-snapshot.v1.json");
    let corrupt = root.join("state/mosaic-snapshot.v1.json.corrupt");
    fs::create_dir_all(snapshot.parent().unwrap()).unwrap();
    fs::write(&snapshot, contents).unwrap();
    ElectronCase {
        _tmp: tmp,
        root,
        snapshot,
        corrupt,
    }
}

#[test]
fn electron_leaves_a_loadable_snapshot_alone() {
    let case = electron_case("GOOD");
    if start_electron(&case.root, &case.snapshot).is_none() {
        return;
    }
    assert_eq!(fs::read_to_string(&case.snapshot).unwrap(), "GOOD");
    assert!(!case.corrupt.exists());
}

/// `<snapshot>.corrupt` for 0, `<snapshot>.corrupt.<n>` after that.
fn electron_quarantine(case: &ElectronCase, index: usize) -> PathBuf {
    if index == 0 {
        case.corrupt.clone()
    } else {
        PathBuf::from(format!("{}.{index}", case.corrupt.display()))
    }
}

#[test]
fn electron_quarantines_an_unloadable_snapshot_before_saving_the_demo() {
    let case = electron_case(STUDY_HISTORY);
    if start_electron(&case.root, &case.snapshot).is_none() {
        return;
    }
    assert_eq!(fs::read_to_string(&case.corrupt).unwrap(), STUDY_HISTORY);
    assert_eq!(fs::read_to_string(&case.snapshot).unwrap(), DEMO_STATE);
}

#[test]
fn electron_never_replaces_an_earlier_quarantine() {
    let case = electron_case(STUDY_HISTORY);
    fs::write(&case.corrupt, "the real history, quarantined at startup").unwrap();
    // A directory in the next slot is skipped like a file.
    fs::create_dir_all(electron_quarantine(&case, 1)).unwrap();
    if start_electron(&case.root, &case.snapshot).is_none() {
        return;
    }
    assert_eq!(
        fs::read_to_string(&case.corrupt).unwrap(),
        "the real history, quarantined at startup"
    );
    assert_eq!(
        fs::read_to_string(electron_quarantine(&case, 2)).unwrap(),
        STUDY_HISTORY
    );
    assert_eq!(fs::read_to_string(&case.snapshot).unwrap(), DEMO_STATE);
}

#[test]
fn electron_saves_nothing_over_the_snapshot_when_every_quarantine_name_is_taken() {
    let case = electron_case(STUDY_HISTORY);
    for index in 0..10 {
        fs::write(
            electron_quarantine(&case, index),
            format!("quarantine {index}"),
        )
        .unwrap();
    }
    if start_electron(&case.root, &case.snapshot).is_none() {
        return;
    }
    assert_eq!(fs::read_to_string(&case.snapshot).unwrap(), STUDY_HISTORY);
    assert!(!electron_quarantine(&case, 10).exists());
    assert_eq!(fs::read_to_string(&case.corrupt).unwrap(), "quarantine 0");
}

// --------------------------------------------------------------------------
// Web: the snapshot is a localStorage entry
// --------------------------------------------------------------------------

/// A Node driver for `engram-host.mjs`. It installs a `window` whose
/// `localStorage` starts from `$ENGRAM_TEST_STORAGE` (a JSON object) and can
/// be told to refuse writes to one key (`$ENGRAM_TEST_REFUSE`), imports the
/// host, waits until it announces itself ready, and prints the storage as
/// JSON.
const WEB_DRIVER: &str = r#"
const fs = await import("node:fs");
const initial = JSON.parse(fs.readFileSync(process.env.ENGRAM_TEST_STORAGE, "utf8"));
const refuse = initial.__refuse__;
delete initial.__refuse__;
const store = new Map(Object.entries(initial));
const ready = new Promise(resolve => {
  globalThis.window = {
    localStorage: {
      getItem: key => (store.has(key) ? store.get(key) : null),
      setItem: (key, value) => {
        if (key === refuse) throw new Error("QuotaExceededError");
        store.set(key, String(value));
      },
    },
    dispatchEvent: event => {
      if (event.type === "mosaic-host-ready") resolve();
      return true;
    },
  };
});
globalThis.fetch = async () => ({ ok: true, arrayBuffer: async () => new ArrayBuffer(0) });
await import("./engram-host.mjs");
await ready;
process.stdout.write(JSON.stringify(Object.fromEntries(store)));
"#;

/// Start the web host once with `storage` as its localStorage, refusing
/// writes to `refuse` if given. Returns the storage afterwards.
fn start_web(storage: &[(&str, &str)], refuse: Option<&str>) -> Option<serde_json::Value> {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    fs::copy(
        host_file("host/web/engram-host.mjs"),
        root.join("engram-host.mjs"),
    )
    .expect("copy engram-host.mjs");
    stage_engine(root);
    let mut initial = serde_json::Map::new();
    for (key, value) in storage {
        initial.insert((*key).into(), (*value).into());
    }
    if let Some(refuse) = refuse {
        initial.insert("__refuse__".into(), refuse.into());
    }
    let storage_path = root.join("storage.json");
    fs::write(
        &storage_path,
        serde_json::Value::Object(initial).to_string(),
    )
    .unwrap();
    let output = run_node(root, &[("ENGRAM_TEST_STORAGE", &storage_path)], WEB_DRIVER)?;
    Some(serde_json::from_slice(&output.stdout).expect("storage JSON"))
}

const WEB_KEY: &str = "engram.snapshot.v1";
const WEB_CORRUPT_KEY: &str = "engram.snapshot.v1.corrupt";

#[test]
fn web_leaves_a_loadable_snapshot_alone() {
    let Some(storage) = start_web(&[(WEB_KEY, "GOOD")], None) else {
        return;
    };
    assert_eq!(storage[WEB_KEY], "GOOD");
    assert!(storage.get(WEB_CORRUPT_KEY).is_none());
}

#[test]
fn web_sets_an_unloadable_snapshot_aside_before_saving_the_demo() {
    let Some(storage) = start_web(&[(WEB_KEY, STUDY_HISTORY)], None) else {
        return;
    };
    assert_eq!(storage[WEB_CORRUPT_KEY], STUDY_HISTORY);
    assert_eq!(storage[WEB_KEY], DEMO_STATE);
}

#[test]
fn web_never_replaces_an_earlier_quarantine() {
    let Some(storage) = start_web(
        &[
            (WEB_KEY, STUDY_HISTORY),
            (WEB_CORRUPT_KEY, "the real history, quarantined at startup"),
        ],
        None,
    ) else {
        return;
    };
    assert_eq!(
        storage[WEB_CORRUPT_KEY],
        "the real history, quarantined at startup"
    );
    assert_eq!(storage["engram.snapshot.v1.corrupt.1"], STUDY_HISTORY);
    assert_eq!(storage[WEB_KEY], DEMO_STATE);
}

#[test]
fn web_does_not_copy_a_snapshot_it_already_holds() {
    let Some(storage) = start_web(
        &[(WEB_KEY, STUDY_HISTORY), (WEB_CORRUPT_KEY, STUDY_HISTORY)],
        None,
    ) else {
        return;
    };
    assert!(storage.get("engram.snapshot.v1.corrupt.1").is_none());
    assert_eq!(storage[WEB_KEY], DEMO_STATE);
}

#[test]
fn web_saves_nothing_over_the_snapshot_when_setting_it_aside_fails() {
    let Some(storage) = start_web(&[(WEB_KEY, STUDY_HISTORY)], Some(WEB_CORRUPT_KEY)) else {
        return;
    };
    assert_eq!(storage[WEB_KEY], STUDY_HISTORY);
    assert!(storage.get(WEB_CORRUPT_KEY).is_none());
}

#[test]
fn web_saves_nothing_over_the_snapshot_when_every_quarantine_key_is_taken() {
    let mut keys = vec![WEB_CORRUPT_KEY.to_string()];
    keys.extend((1..10).map(|index| format!("{WEB_CORRUPT_KEY}.{index}")));
    let mut storage: Vec<(&str, &str)> = keys.iter().map(|key| (key.as_str(), "older")).collect();
    storage.push((WEB_KEY, STUDY_HISTORY));
    let Some(storage) = start_web(&storage, None) else {
        return;
    };
    assert_eq!(storage[WEB_KEY], STUDY_HISTORY);
    assert!(storage.get("engram.snapshot.v1.corrupt.10").is_none());
}
