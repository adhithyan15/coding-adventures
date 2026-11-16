//! WD-004b1: falsification tests for the safe lane artifact loader.
//!
//! Every test that corrupts something does so in a throw-away copy of the
//! artifact layout (`Repo`), never in a tracked file.  The shape of each test
//! is the one VER04h uses: start from a copy that loads, change exactly one
//! thing, and expect the named failure code.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use coding_adventures_ca_assurance_report::json::JsonValue;
use coding_adventures_ca_assurance_report::lane_evidence::parse_consumer_registry;
use coding_adventures_ca_assurance_report::lane_loader::{
    canonical_json, labels_manifest_digest, lexical_containment, load_active_lanes, load_lane,
    parse_domain_labels, read_repo_file, LaneLoadError, PathFlavor, READINESS_CHECKS, UNCLAIMED,
};

const PACKAGES: [&str; 3] = [
    "go/ct-compare",
    "python/ct-compare",
    "typescript/ct-compare",
];

fn real_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../..")
}

// ---------------------------------------------------------------------------
// A throw-away repository holding exactly the files a load opens.
// ---------------------------------------------------------------------------

static COUNTER: AtomicU32 = AtomicU32::new(0);

struct Repo {
    root: PathBuf,
}

impl Repo {
    fn new() -> Repo {
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!("wd004b1-{}-{id}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let repo = Repo { root };
        let real = real_root();
        let fixture = "code/specs/fixtures/ct-compare-warrant-v1";
        for rel in [
            "code/specs/ct-compare.warrant".to_string(),
            format!("{fixture}/corpus.json"),
            format!("{fixture}/cases.tfv"),
            format!("{fixture}/consumers.json"),
        ] {
            repo.copy_from(&real, &rel);
        }
        let registry = fs::read(real.join(fixture).join("consumers.json")).unwrap();
        let registry = parse_consumer_registry(&registry).expect("checked-in registry parses");
        for row in &registry.consumers {
            let labels = row
                .evidence_manifest
                .replace("test-manifest.json", "domain-labels.json");
            for rel in [
                &row.binding,
                &row.runner_entry,
                &row.evidence_manifest,
                &labels,
            ]
            .into_iter()
            .chain(row.build_files.iter())
            {
                repo.copy_from(&real, rel);
            }
        }
        repo
    }

    fn copy_from(&self, real: &Path, rel: &str) {
        let to = self.root.join(rel);
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::copy(real.join(rel), to).unwrap_or_else(|e| panic!("copy {rel}: {e}"));
    }

    fn read(&self, rel: &str) -> Vec<u8> {
        fs::read(self.root.join(rel)).unwrap()
    }

    fn write(&self, rel: &str, bytes: &[u8]) {
        fs::write(self.root.join(rel), bytes).unwrap();
    }

    fn edit_text(&self, rel: &str, from: &str, to: &str) {
        let text = String::from_utf8(self.read(rel)).unwrap();
        assert!(text.contains(from), "{rel} does not contain {from:?}");
        self.write(rel, text.replacen(from, to, 1).as_bytes());
    }

    fn load(&self, package: &str) -> Result<(), LaneLoadError> {
        load_lane(&self.root, package).map(|_| ())
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn manifest_rel(package: &str) -> String {
    format!("code/packages/{package}/evidence/test-manifest.json")
}

fn labels_rel(package: &str) -> String {
    format!("code/packages/{package}/evidence/domain-labels.json")
}

fn expect_code<T>(result: Result<T, LaneLoadError>, code: &str, needle: &str) {
    match result {
        Ok(_) => panic!("expected failure `{code}` ({needle}) but the load succeeded"),
        Err(error) => {
            assert_eq!(error.code, code, "wrong failure code: {error}");
            assert!(
                error.to_string().contains(needle),
                "`{error}` does not mention {needle:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// The checked-in artifacts load.
// ---------------------------------------------------------------------------

#[test]
fn every_checked_in_lane_loads_with_its_named_checks_and_no_release_claim() {
    for package in PACKAGES {
        let lane =
            load_lane(&real_root(), package).unwrap_or_else(|e| panic!("{package} must load: {e}"));
        assert_eq!(lane.consumer.package, package);
        assert_eq!(lane.manifest.package, package);
        assert_eq!(lane.manifest.tests.len(), 29);
        // The honest headline is the label split, never the manifest's total.
        assert_eq!(lane.labels.cases_by_label.get("full"), Some(&65_537));
        assert_eq!(
            lane.labels.cases_by_label.get("bounded-set"),
            Some(&506_572)
        );
        assert_eq!(lane.readiness.passed, READINESS_CHECKS.to_vec());
        assert!(!lane.readiness.release_ready);
        assert_eq!(lane.readiness.sandbox, "not-armed");
        let text = lane.render_readiness();
        assert!(text.contains("release_ready: false"), "{text}");
        assert!(text.contains("sandbox: not-armed"), "{text}");
        assert!(text.contains("makes no claim"), "{text}");
        for unclaimed in UNCLAIMED {
            assert!(text.contains(unclaimed), "{text} lacks {unclaimed}");
        }
    }
}

#[test]
fn the_checked_in_registry_is_active_for_exactly_the_three_lanes() {
    let bytes =
        fs::read(real_root().join("code/specs/fixtures/ct-compare-warrant-v1/consumers.json"))
            .unwrap();
    let registry = parse_consumer_registry(&bytes).unwrap();
    let rows: Vec<(&str, &str)> = registry
        .consumers
        .iter()
        .map(|row| (row.package.as_str(), row.status.as_str()))
        .collect();
    assert_eq!(
        rows,
        vec![
            ("go/ct-compare", "active"),
            ("python/ct-compare", "active"),
            ("typescript/ct-compare", "active"),
        ]
    );
    assert_eq!(registry.active_consumer_count, 3);
    let lanes = load_active_lanes(&real_root()).expect("all active lanes load");
    let loaded: Vec<&str> = lanes
        .iter()
        .map(|lane| lane.consumer.package.as_str())
        .collect();
    assert_eq!(loaded, PACKAGES.to_vec());
}

#[test]
fn a_rust_package_is_not_a_lane_package() {
    // The legacy loader owns `rust/*`; a Rust manifest is never read as a lane.
    let repo = Repo::new();
    expect_code(
        repo.load("rust/ct-compare"),
        "not-registered",
        "rust/ct-compare",
    );
}

// ---------------------------------------------------------------------------
// Registry join.
// ---------------------------------------------------------------------------

fn set_python_status(repo: &Repo, status: &str) {
    let rel = "code/specs/fixtures/ct-compare-warrant-v1/consumers.json";
    let text = String::from_utf8(repo.read(rel)).unwrap();
    let mut registry = JsonValue::parse(&text).unwrap();
    let object = match &mut registry {
        JsonValue::Object(object) => object,
        _ => panic!("registry object"),
    };
    let rows = match object.get_mut("consumers") {
        Some(JsonValue::Array(rows)) => rows,
        _ => panic!("rows"),
    };
    let mut active = 0.0;
    for row in rows.iter_mut() {
        if let JsonValue::Object(row) = row {
            if row.get("package").and_then(JsonValue::as_str) == Some("python/ct-compare") {
                row.insert("status".into(), JsonValue::String(status.into()));
            }
            if row.get("status").and_then(JsonValue::as_str) == Some("active") {
                active += 1.0;
            }
        }
    }
    object.insert("active_consumer_count".into(), JsonValue::Number(active));
    repo.write(rel, registry.to_pretty_string(2).as_bytes());
}

#[test]
fn a_planned_row_cannot_authorize_a_load_but_its_siblings_still_load() {
    let repo = Repo::new();
    set_python_status(&repo, "planned");
    expect_code(repo.load("python/ct-compare"), "not-active", "planned");
    repo.load("go/ct-compare")
        .expect("an active sibling is unaffected");
    let lanes = load_active_lanes(&repo.root).expect("two active lanes load");
    assert_eq!(lanes.len(), 2);
}

#[test]
fn a_manifest_for_another_package_is_refused() {
    // The Go manifest placed where the Python registry row expects its own.
    let repo = Repo::new();
    let go = repo.read(&manifest_rel("go/ct-compare"));
    repo.write(&manifest_rel("python/ct-compare"), &go);
    expect_code(repo.load("python/ct-compare"), "manifest", "package");
}

#[test]
fn a_registry_that_does_not_parse_is_refused() {
    let repo = Repo::new();
    repo.write(
        "code/specs/fixtures/ct-compare-warrant-v1/consumers.json",
        b"{}",
    );
    expect_code(
        repo.load("python/ct-compare"),
        "registry",
        "consumer registry",
    );
}

// ---------------------------------------------------------------------------
// Raw digest checks.
// ---------------------------------------------------------------------------

#[test]
fn a_changed_corpus_spec_or_tfv_byte_is_a_digest_failure() {
    for (rel, needle) in [
        (
            "code/specs/fixtures/ct-compare-warrant-v1/corpus.json",
            "corpus_sha256",
        ),
        (
            "code/specs/fixtures/ct-compare-warrant-v1/cases.tfv",
            "cases_sha256",
        ),
        ("code/specs/ct-compare.warrant", "spec_sha256"),
    ] {
        let repo = Repo::new();
        let mut bytes = repo.read(rel);
        bytes.push(b'\n');
        repo.write(rel, &bytes);
        for package in PACKAGES {
            expect_code(repo.load(package), "digest", needle);
        }
    }
}

#[test]
fn a_changed_binding_byte_is_a_digest_failure_for_that_lane_only() {
    for package in PACKAGES {
        let repo = Repo::new();
        let rel = format!("code/packages/{package}/evidence.bindings");
        let mut bytes = repo.read(&rel);
        bytes.extend_from_slice(b"# edited\n");
        repo.write(&rel, &bytes);
        expect_code(repo.load(package), "digest", "binding_sha256");
        for other in PACKAGES.iter().filter(|p| **p != package) {
            repo.load(other).expect("the other lanes are unaffected");
        }
    }
}

#[test]
fn a_manifest_row_edit_that_keeps_the_schema_is_caught_by_the_join() {
    let repo = Repo::new();
    let rel = manifest_rel("python/ct-compare");
    let text = String::from_utf8(repo.read(&rel)).unwrap();
    let marker = "\"cases\": 120409";
    assert!(text.contains(marker));
    repo.write(
        &rel,
        text.replacen(marker, "\"cases\": 120408", 1).as_bytes(),
    );
    expect_code(repo.load("python/ct-compare"), "join", "");
}

#[test]
fn a_manifest_that_fails_the_strict_parser_is_refused() {
    let repo = Repo::new();
    let rel = manifest_rel("typescript/ct-compare");
    // A duplicate key is rejected by the existing duplicate-key-rejecting parser.
    repo.edit_text(
        &rel,
        "\"faulted\": false,",
        "\"faulted\": false,\n  \"faulted\": false,",
    );
    expect_code(repo.load("typescript/ct-compare"), "manifest", "");
}

// ---------------------------------------------------------------------------
// The labels sidecar and its digest.
// ---------------------------------------------------------------------------

fn sidecar_digest(package: &str) -> String {
    let bytes = fs::read(real_root().join(labels_rel(package))).unwrap();
    parse_domain_labels(&bytes)
        .expect("checked-in sidecar parses")
        .manifest_sha256
}

#[test]
fn the_labels_digest_reproduces_each_runner_written_value() {
    // The three runners wrote `manifest_sha256` with their own canonical
    // writers; agreeing with all three is the cross-language check.
    for package in PACKAGES {
        let manifest = fs::read(real_root().join(manifest_rel(package))).unwrap();
        assert_eq!(
            labels_manifest_digest(&manifest).unwrap(),
            sidecar_digest(package),
            "{package}"
        );
    }
}

#[test]
fn the_labels_digest_ignores_the_toolchain_version_but_not_its_name() {
    let manifest = fs::read(real_root().join(manifest_rel("python/ct-compare"))).unwrap();
    let text = String::from_utf8(manifest).unwrap();
    let base = labels_manifest_digest(text.as_bytes()).unwrap();
    let version = text.replacen("\"version\": \"3.13.18\"", "\"version\": \"0.0.1\"", 1);
    assert_ne!(version, text);
    assert_eq!(labels_manifest_digest(version.as_bytes()).unwrap(), base);
    let name = text.replacen("\"name\": \"python\"", "\"name\": \"cpython\"", 1);
    assert_ne!(name, text);
    assert_ne!(labels_manifest_digest(name.as_bytes()).unwrap(), base);
}

#[test]
fn a_different_toolchain_version_still_loads_but_a_different_name_does_not() {
    let repo = Repo::new();
    let rel = manifest_rel("python/ct-compare");
    repo.edit_text(&rel, "\"version\": \"3.13.18\"", "\"version\": \"9.9.9\"");
    repo.load("python/ct-compare")
        .expect("toolchain.version is not evidence");
    repo.edit_text(&rel, "\"name\": \"python\"", "\"name\": \"pypy\"");
    expect_code(repo.load("python/ct-compare"), "labels", "manifest_sha256");
}

#[test]
fn the_canonical_writer_follows_the_conformance_profile() {
    let value = JsonValue::parse(
        "{\"b\":[],\"a\":{},\"\u{e9}\":\"\u{1f600}\\u007f\\u0001\\\"\\\\\\/\\n\",\"n\":[1,20]}",
    )
    .unwrap();
    assert_eq!(
        canonical_json(&value),
        "{\n  \"a\": {},\n  \"b\": [],\n  \"n\": [\n    1,\n    20\n  ],\n  \
         \"\\u00e9\": \"\\ud83d\\ude00\\u007f\\u0001\\\"\\\\/\\n\"\n}\n"
    );
}

#[test]
fn sidecar_tampering_is_refused() {
    let cases: [(&str, &str, &str); 6] = [
        // A label stronger than the descriptors allow.
        ("\"label\": \"bounded-set\"", "\"label\": \"full\"", "label"),
        // A stale binding digest.
        (
            "\"manifest_sha256\": \"",
            "\"manifest_sha256\": \"0",
            "manifest_sha256",
        ),
        // A case count that no longer matches the manifest row.
        ("\"cases\": 120409", "\"cases\": 120408", "cases"),
        // A different package identity.
        (
            "\"package\": \"python/ct-compare\"",
            "\"package\": \"python/other\"",
            "package",
        ),
        // A different schema.
        (
            "warrant-lane-domain-labels-v1",
            "warrant-lane-domain-labels-v2",
            "schema",
        ),
        // An unknown descriptor cannot be labelled.
        (
            "\"domain\": \"u64_edge\"",
            "\"domain\": \"u64_any\"",
            "descriptor",
        ),
    ];
    for (from, to, needle) in cases {
        let repo = Repo::new();
        repo.edit_text(&labels_rel("python/ct-compare"), from, to);
        expect_code(repo.load("python/ct-compare"), "labels", needle);
    }
}

#[test]
fn a_sidecar_with_an_extra_key_or_a_missing_file_is_refused() {
    let repo = Repo::new();
    repo.edit_text(
        &labels_rel("go/ct-compare"),
        "\"schema\": \"warrant-lane-domain-labels-v1\"",
        "\"schema\": \"warrant-lane-domain-labels-v1\",\n  \"extra\": 1",
    );
    expect_code(repo.load("go/ct-compare"), "labels", "closed schema");
    let repo = Repo::new();
    fs::remove_file(repo.root.join(labels_rel("go/ct-compare"))).unwrap();
    expect_code(repo.load("go/ct-compare"), "io", "domain-labels.json");
}

#[test]
fn the_cases_by_label_total_must_be_the_sum_of_the_suites() {
    let repo = Repo::new();
    repo.edit_text(
        &labels_rel("python/ct-compare"),
        "\"full\": 65537",
        "\"full\": 65536",
    );
    expect_code(repo.load("python/ct-compare"), "labels", "cases_by_label");
}

// ---------------------------------------------------------------------------
// Static readiness.
// ---------------------------------------------------------------------------

#[test]
fn the_runner_identity_is_checked() {
    let repo = Repo::new();
    repo.edit_text(
        &manifest_rel("go/ct-compare"),
        "\"name\": \"warrant-corpus\"",
        "\"name\": \"home-made\"",
    );
    expect_code(
        repo.load("go/ct-compare"),
        "runner-identity",
        "warrant-corpus",
    );
}

#[test]
fn each_build_file_must_route_the_manifest_and_the_sabotage_step() {
    for (rel, from, needle) in [
        (
            "code/packages/python/ct-compare/BUILD",
            "evidence/test-manifest.json",
            "test-manifest",
        ),
        (
            "code/packages/go/ct-compare/BUILD_windows",
            "sabotage",
            "sabotage",
        ),
    ] {
        let repo = Repo::new();
        let text = String::from_utf8(repo.read(rel)).unwrap();
        repo.write(rel, text.replace(from, "removed").as_bytes());
        let package = if rel.contains("/python/") {
            "python/ct-compare"
        } else {
            "go/ct-compare"
        };
        expect_code(repo.load(package), "build-routing", needle);
    }
}

#[test]
fn a_missing_registered_file_is_refused() {
    for rel in [
        "code/packages/typescript/ct-compare/tests/warrant-evidence.ts",
        "code/packages/typescript/ct-compare/evidence.bindings",
        "code/packages/typescript/ct-compare/BUILD_windows",
        "code/packages/typescript/ct-compare/evidence/test-manifest.json",
    ] {
        let repo = Repo::new();
        fs::remove_file(repo.root.join(rel)).unwrap();
        expect_code(repo.load("typescript/ct-compare"), "io", rel);
    }
}

// ---------------------------------------------------------------------------
// Bounded, contained reads.
// ---------------------------------------------------------------------------

#[test]
fn reads_are_bounded_before_the_bytes_are_used() {
    let repo = Repo::new();
    let rel = "code/packages/python/ct-compare/BUILD";
    let size = repo.read(rel).len() as u64;
    assert_eq!(
        read_repo_file(&repo.root, rel, size).unwrap().len() as u64,
        size
    );
    expect_code(
        read_repo_file(&repo.root, rel, size - 1),
        "too-large",
        "limit",
    );
    // The real manifest cap: one byte over 16 MiB is refused on size alone.
    let big = vec![b' '; 16 * 1024 * 1024 + 1];
    repo.write(&manifest_rel("python/ct-compare"), &big);
    expect_code(
        repo.load("python/ct-compare"),
        "too-large",
        "test-manifest.json",
    );
}

#[test]
fn paths_that_could_leave_the_repository_are_refused_before_any_io() {
    let repo = Repo::new();
    for bad in [
        "../outside.txt",
        "code/../../outside.txt",
        "/etc/passwd",
        "code\\packages",
        "C:/Windows/win.ini",
        "//server/share/x",
        "code//packages",
        "code/./packages",
        "",
        "code/packages/caf\u{e9}",
    ] {
        expect_code(read_repo_file(&repo.root, bad, 1024), "path", "");
    }
}

#[test]
fn a_directory_is_not_a_regular_file() {
    let repo = Repo::new();
    expect_code(
        read_repo_file(&repo.root, "code/packages", 1024),
        "not-regular",
        "regular file",
    );
}

#[cfg(unix)]
#[test]
fn a_symlink_file_or_directory_component_is_refused() {
    use std::os::unix::fs::symlink;

    let repo = Repo::new();
    let outside = repo.root.join("outside.json");
    fs::write(&outside, b"{}").unwrap();

    // The manifest itself is a link to a file outside the package.
    let rel = manifest_rel("go/ct-compare");
    fs::remove_file(repo.root.join(&rel)).unwrap();
    symlink(&outside, repo.root.join(&rel)).unwrap();
    expect_code(repo.load("go/ct-compare"), "symlink", "test-manifest.json");

    // A directory component is a link: `evidence/` points elsewhere.
    let repo = Repo::new();
    let evidence = repo.root.join("code/packages/python/ct-compare/evidence");
    let moved = repo.root.join("moved-evidence");
    fs::rename(&evidence, &moved).unwrap();
    symlink(&moved, &evidence).unwrap();
    expect_code(repo.load("python/ct-compare"), "symlink", "evidence");

    // A link in the middle of a registered path, read directly.
    expect_code(
        read_repo_file(
            &repo.root,
            "code/packages/python/ct-compare/evidence/test-manifest.json",
            1 << 20,
        ),
        "symlink",
        "evidence",
    );

    // The repository root itself may be reached through a link: it is trusted.
    let alias = std::env::temp_dir().join(format!("wd004b1-alias-{}", std::process::id()));
    let _ = fs::remove_file(&alias);
    let clean = Repo::new();
    symlink(&clean.root, &alias).unwrap();
    let result = load_lane(&alias, "go/ct-compare");
    let _ = fs::remove_file(&alias);
    result.expect("a symlinked repository root is trusted");
}

// ---------------------------------------------------------------------------
// Lexical containment (the lane runners' shared rule).
// ---------------------------------------------------------------------------

fn inside(flavor: PathFlavor, root: &str, child: &str) -> (bool, bool) {
    let c = lexical_containment(flavor, root, child);
    (c.inside, c.same)
}

#[test]
fn posix_containment_compares_whole_components() {
    use PathFlavor::Posix;
    assert_eq!(inside(Posix, "/r", "/r/a/b"), (true, false));
    assert_eq!(inside(Posix, "/r", "/r"), (true, true));
    assert_eq!(inside(Posix, "/r", "/r/"), (true, true));
    assert_eq!(inside(Posix, "/r", "/rr"), (false, false));
    assert_eq!(inside(Posix, "/r", "/r/.."), (false, false));
    assert_eq!(inside(Posix, "/r", "/r/..foo"), (true, false));
    assert_eq!(inside(Posix, "/r", "/r/a/../b"), (true, false));
    assert_eq!(inside(Posix, "/r", "/r/a/../../x"), (false, false));
    assert_eq!(inside(Posix, "/", "/anything"), (true, false));
    // A `..` at the root stays at the root.
    assert_eq!(inside(Posix, "/", "/../.."), (true, true));
    // Case-sensitive; a backslash is an ordinary character.
    assert_eq!(inside(Posix, "/r", "/R/a"), (false, false));
    assert_eq!(inside(Posix, "/r", "/r\\a"), (false, false));
    // Not absolute: never inside.
    assert_eq!(inside(Posix, "/r", "r/a"), (false, false));
    assert_eq!(inside(Posix, "r", "/r/a"), (false, false));
}

#[test]
fn posix_double_leading_slash_is_implementation_defined_so_never_inside() {
    use PathFlavor::Posix;
    assert_eq!(inside(Posix, "/r", "//r/a"), (false, false));
    assert_eq!(inside(Posix, "//r", "//r/a"), (false, false));
    assert_eq!(inside(Posix, "/r", "///r/a"), (true, false));
    assert_eq!(inside(Posix, "///r", "/r/a"), (true, false));
    // `//` inside a path is an ordinary separator.
    assert_eq!(inside(Posix, "/r", "/r//a"), (true, false));
}

#[test]
fn win32_containment_has_no_cross_drive_or_device_escape() {
    use PathFlavor::Win32;
    assert_eq!(inside(Win32, "C:\\r", "C:\\r\\a"), (true, false));
    assert_eq!(inside(Win32, "C:\\r", "c:/R/A"), (true, false));
    assert_eq!(inside(Win32, "C:\\r", "C:\\r"), (true, true));
    // `path.relative` returns an absolute path across drives: never inside.
    assert_eq!(inside(Win32, "C:\\r", "D:\\r\\a"), (false, false));
    assert_eq!(inside(Win32, "C:\\r", "C:\\rr"), (false, false));
    assert_eq!(inside(Win32, "C:\\r", "C:\\r\\.."), (false, false));
    assert_eq!(inside(Win32, "C:\\r", "C:\\r\\..foo"), (true, false));
    // Drive-relative and rootless paths are not absolute.
    assert_eq!(inside(Win32, "C:\\r", "C:r\\a"), (false, false));
    assert_eq!(inside(Win32, "C:\\r", "r\\a"), (false, false));
    // UNC shares compare server and share case-insensitively.
    assert_eq!(
        inside(Win32, "\\\\S\\share\\r", "\\\\s\\SHARE\\r\\a"),
        (true, false)
    );
    assert_eq!(
        inside(Win32, "\\\\s\\share\\r", "\\\\s\\other\\r\\a"),
        (false, false)
    );
    assert_eq!(inside(Win32, "\\\\s\\share\\r", "C:\\r\\a"), (false, false));
}

#[test]
fn win32_device_paths_are_never_inside_whatever_the_separators() {
    use PathFlavor::Win32;
    for device in [
        "\\\\?\\C:\\r\\a",
        "\\\\.\\C:\\r\\a",
        "\\/?\\C:\\r\\a",
        "/\\?/C:/r/a",
        "\\\\?/C:\\r\\a",
        "\\\\?\\UNC\\server\\share\\r\\a",
    ] {
        assert_eq!(inside(Win32, "C:\\r", device), (false, false), "{device}");
        assert_eq!(
            inside(Win32, device, "C:\\r\\a"),
            (false, false),
            "{device}"
        );
        assert_eq!(inside(Win32, device, device), (false, false), "{device}");
    }
}

#[test]
fn the_host_flavour_is_what_production_uses() {
    let expected = if cfg!(windows) {
        PathFlavor::Win32
    } else {
        PathFlavor::Posix
    };
    assert_eq!(PathFlavor::host(), expected);
}

// ---------------------------------------------------------------------------
// Security-review follow-ups: overflow, strict integer tokens, reserved
// names, the spec-pinned readiness list, more negative cases, and the shared
// containment fixture.
// ---------------------------------------------------------------------------

/// A sidecar with `suites` suites of `cases` cases each, all `full` over `bool`.
fn synthetic_sidecar(suites: usize, cases: u64, label: &str, binders: &[(&str, &str)]) -> Vec<u8> {
    let binders_json: Vec<String> = binders
        .iter()
        .map(|(domain, l)| format!("{{\"name\":\"x\",\"domain\":\"{domain}\",\"label\":\"{l}\"}}"))
        .collect();
    // Declared total: the true sum, capped at the loader's 2^53 count bound.
    let declared = (suites as u128 * cases as u128).min(1 << 53);
    let rows: Vec<String> = (0..suites)
        .map(|i| {
            format!(
                "{{\"id\":\"s{i:06}\",\"label\":\"{label}\",\"domain\":\"d\",\"cases\":{cases},\"binders\":[{}]}}",
                binders_json.join(",")
            )
        })
        .collect();
    format!(
        "{{\"schema\":\"warrant-lane-domain-labels-v1\",\"package\":\"p/q\",\"module\":\"m\",\
         \"manifest_sha256\":\"{}\",\"note\":\"n\",\"cases_by_label\":{{\"{label}\":{declared}}},\"suites\":[{}]}}",
        "0".repeat(64),
        rows.join(",")
    )
    .into_bytes()
}

#[test]
fn a_hostile_sidecar_cannot_overflow_the_case_sum() {
    // 2,100 suites at 2^53 cases sum past u64::MAX; this must be a clean
    // refusal, not a debug panic or a release wrap.
    let bytes = synthetic_sidecar(2_100, 1 << 53, "full", &[("bool", "full")]);
    expect_code(parse_domain_labels(&bytes), "labels", "overflow");
}

#[test]
fn the_sidecar_is_parsed_with_the_strict_integer_rules_of_the_manifest() {
    for bad in ["65537.0", "6.5537e4", "065537", "-0"] {
        let repo = Repo::new();
        repo.edit_text(
            &labels_rel("python/ct-compare"),
            "\"full\": 65537",
            &format!("\"full\": {bad}"),
        );
        expect_code(repo.load("python/ct-compare"), "labels", "integer");
    }
}

#[test]
fn windows_reserved_device_names_and_trailing_dots_are_refused() {
    let repo = Repo::new();
    for bad in [
        "code/CON",
        "code/nul.txt",
        "code/Aux.tar.gz",
        "code/prn",
        "code/com1",
        "code/LPT9.log",
        "code/packages/COM5.json",
        "code/trailing.",
    ] {
        expect_code(read_repo_file(&repo.root, bad, 1024), "path", "");
    }
    // Names that merely contain or extend a reserved word stay legal.
    for ok in [
        "code/console",
        "code/com10",
        "code/nullable.txt",
        "code/auxiliary",
    ] {
        let error = read_repo_file(&repo.root, ok, 1024).unwrap_err();
        assert_eq!(error.code, "io", "{ok}: {error}");
    }
}

#[test]
fn the_readiness_list_is_pinned_to_the_spec_table_in_order() {
    let spec =
        fs::read_to_string(real_root().join("code/specs/VER04i-polyglot-warrant-evidence.md"))
            .unwrap();
    let start = spec
        .find("**Static readiness checks (the whole list).**")
        .expect("spec section");
    let names: Vec<String> = spec[start..]
        .lines()
        .skip_while(|l| !l.starts_with("| Name"))
        .skip(2)
        .take_while(|l| l.starts_with('|'))
        .map(|l| l.split('`').nth(1).expect("backticked name").to_owned())
        .collect();
    let code: Vec<String> = READINESS_CHECKS.iter().map(|s| s.to_string()).collect();
    assert_eq!(names, code);
}

#[test]
fn more_manifest_and_sidecar_identity_failures_are_refused() {
    // runner.lane differing from the manifest lane.
    let repo = Repo::new();
    repo.edit_text(
        &manifest_rel("go/ct-compare"),
        "\"lane\": \"go\",\n    \"name\"",
        "\"lane\": \"python\",\n    \"name\"",
    );
    expect_code(repo.load("go/ct-compare"), "manifest", "runner lane");
    // The sidecar's module.
    let repo = Repo::new();
    repo.edit_text(
        &labels_rel("go/ct-compare"),
        "\"module\": \"ct_compare\"",
        "\"module\": \"other\"",
    );
    expect_code(repo.load("go/ct-compare"), "labels", "module");
    // A suite id that the manifest does not have (still sorted and unique).
    let repo = Repo::new();
    repo.edit_text(
        &labels_rel("go/ct-compare"),
        "warrant/ct_compare/exhaustive/select_rejects_mismatch",
        "warrant/ct_compare/exhaustive/zz_select_rejects_mismatch",
    );
    expect_code(repo.load("go/ct-compare"), "labels", "suites differ");
    // A non-UTF-8 BUILD file.
    let repo = Repo::new();
    repo.write("code/packages/go/ct-compare/BUILD", &[0xff, 0xfe, b'\n']);
    expect_code(repo.load("go/ct-compare"), "build-routing", "UTF-8");
}

#[test]
fn a_suite_is_as_weak_as_its_weakest_binder() {
    let mixed = [("bool", "full"), ("bytes_d1", "bounded-set")];
    let bytes = synthetic_sidecar(1, 5, "full", &mixed);
    expect_code(parse_domain_labels(&bytes), "labels", "weakest");
    let ok = synthetic_sidecar(1, 5, "bounded-set", &mixed);
    parse_domain_labels(&ok).expect("the weakest binder's label is accepted");
}

#[test]
fn the_shared_containment_fixture_passes_in_both_flavours_on_any_host() {
    let text = fs::read_to_string(
        real_root().join("code/specs/fixtures/warrant-corpus-conformance-v1/containment.json"),
    )
    .unwrap();
    let doc = JsonValue::parse(&text).unwrap();
    let cases = doc.get("cases").and_then(JsonValue::as_array).unwrap();
    assert_eq!(cases.len(), 77, "a case added to the fixture must be run");
    for case in cases {
        let id = case.get("id").and_then(JsonValue::as_str).unwrap();
        let flavor = match case.get("flavor").and_then(JsonValue::as_str).unwrap() {
            "posix" => PathFlavor::Posix,
            "win32" => PathFlavor::Win32,
            other => panic!("{id}: unknown flavor {other}"),
        };
        let root = case.get("root").and_then(JsonValue::as_str).unwrap();
        let child = case.get("child").and_then(JsonValue::as_str).unwrap();
        let expect = case.get("expect").unwrap();
        let want = (
            expect.get("inside").and_then(JsonValue::as_bool).unwrap(),
            expect.get("same").and_then(JsonValue::as_bool).unwrap(),
        );
        assert_eq!(
            inside(flavor, root, child),
            want,
            "{id}: {root:?} vs {child:?}"
        );
    }
}
