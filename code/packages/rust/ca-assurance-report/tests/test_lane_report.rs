//! WD-004b2: the lane rungs of a package report, `--check-lane` and
//! `--require-active` (VER04i section 8.3).
//!
//! Registry-only fixtures are enough for the rung states that do not load a
//! lane (`absent`, `planned`, a broken registry); the `live` rung and the
//! report round trip use the real checked-in lanes, because "live" is exactly
//! the claim that has to come from the real loader.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use coding_adventures_ca_assurance_report::checker::{verify_assurance_report, CheckError};
use coding_adventures_ca_assurance_report::evidence::EvidenceCheckError;
use coding_adventures_ca_assurance_report::lane_loader::{READINESS_CHECKS, UNCLAIMED};
use coding_adventures_ca_assurance_report::lane_report::{
    check_lane_shape, compare_lane_rungs, derive_lane_rungs, lane_status, lanes_from_json,
    lanes_to_json, LaneDiff, LaneRung, LaneStatus,
};
use coding_adventures_ca_assurance_report::registry::WeaknessRegistry;
use coding_adventures_ca_assurance_report::report::AssuranceReport;
use coding_adventures_ca_assurance_report::{generate_package_report, verify_evidence, Evidence};

const REGISTRY: &str = "code/specs/fixtures/ct-compare-warrant-v1/consumers.json";

fn real_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../..")
}

fn real_evidence() -> Evidence {
    Evidence::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../ct-compare"))
        .expect("ct-compare evidence loads")
}

// ---------------------------------------------------------------------------
// A throw-away repository that holds only the consumer registry.
// ---------------------------------------------------------------------------

static COUNTER: AtomicU32 = AtomicU32::new(0);

struct Repo {
    root: PathBuf,
}

impl Repo {
    /// A repository with a copy of the real registry, optionally edited.
    fn with_registry(edit: impl Fn(String) -> String) -> Repo {
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!("wd004b2-lane-{}-{id}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let repo = Repo { root };
        let text = fs::read_to_string(real_root().join(REGISTRY)).unwrap();
        repo.write(REGISTRY, &edit(text));
        repo
    }

    fn write(&self, rel: &str, text: &str) {
        let path = self.root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// Flip one lane's row to `planned` (the registry counts must follow).
fn plan(text: String, lane: &str) -> String {
    let marker = format!("\"lane\": \"{lane}\"");
    let at = text.find(&marker).expect("lane row");
    let status = at + text[at..].find("\"status\": \"active\"").expect("status");
    let mut out = text.clone();
    out.replace_range(
        status..status + "\"status\": \"active\"".len(),
        "\"status\": \"planned\"",
    );
    out.replace(
        "\"active_consumer_count\": 3",
        "\"active_consumer_count\": 2",
    )
}

// ---------------------------------------------------------------------------
// Rungs: live only from the loader, absent and planned honest.
// ---------------------------------------------------------------------------

#[test]
fn ct_compare_has_a_live_rung_for_each_registered_lane() {
    let lanes = derive_lane_rungs(&real_root(), "rust/ct-compare").unwrap();
    let names: Vec<&str> = lanes.iter().map(|l| l.lane.as_str()).collect();
    assert_eq!(
        names,
        ["go", "python", "typescript"],
        "sorted, one per registry lane"
    );
    let checks: Vec<String> = READINESS_CHECKS.iter().map(|c| c.to_string()).collect();
    for lane in &lanes {
        assert_eq!(lane.package, format!("{}/ct-compare", lane.lane));
        assert_eq!(lane.rung, "live");
        assert_eq!(lane.checks, checks, "live means every named check passed");
        assert_eq!(lane.cases_by_label.get("full"), Some(&65_537));
        assert_eq!(lane.cases_by_label.get("bounded-set"), Some(&506_572));
        assert_eq!(lane.unclaimed, UNCLAIMED.map(String::from).to_vec());
        assert!(
            !lane.release_ready,
            "never release-ready beyond what is checked"
        );
        assert_eq!(lane.sandbox, "absent", "no lane arms a jail");
    }
}

#[test]
fn a_package_no_lane_covers_gets_an_absent_rung_per_registered_lane() {
    let lanes = derive_lane_rungs(&real_root(), "rust/gf256").unwrap();
    assert_eq!(lanes.len(), 3);
    for lane in &lanes {
        assert_eq!(lane.rung, "absent");
        assert_eq!(lane.package, format!("{}/gf256", lane.lane));
        assert!(
            lane.checks.is_empty() && lane.cases_by_label.is_empty() && lane.unclaimed.is_empty()
        );
        assert!(!lane.release_ready);
        assert_eq!(lane.sandbox, "absent");
    }
}

#[test]
fn a_planned_row_is_a_planned_rung_never_live() {
    let repo = Repo::with_registry(|t| plan(t, "python"));
    let lanes = derive_lane_rungs(&repo.root, "rust/ct-compare").unwrap_err();
    // The other two rows are active, and this repository holds none of their
    // artifacts: an active row that does not load is an error, not a rung.
    assert_eq!(lanes.code, "io", "{lanes}");

    // Every row planned: nothing is loaded, every rung is planned.
    let repo = Repo::with_registry(|t| {
        let t = plan(t, "go");
        let t = plan(t, "python");
        plan(t, "typescript").replace(
            "\"active_consumer_count\": 2",
            "\"active_consumer_count\": 0",
        )
    });
    let lanes = derive_lane_rungs(&repo.root, "rust/ct-compare").unwrap();
    assert_eq!(lanes.len(), 3);
    for lane in &lanes {
        assert_eq!(lane.rung, "planned");
        assert!(lane.checks.is_empty() && lane.cases_by_label.is_empty());
        assert!(!lane.release_ready);
    }
    match lane_status(&repo.root, "python/ct-compare").unwrap() {
        LaneStatus::Planned => {}
        other => panic!("expected planned, got {other:?}"),
    }
}

#[test]
fn an_active_row_that_does_not_load_fails_instead_of_becoming_a_rung() {
    // The registry says active, the artifacts are not there.
    let repo = Repo::with_registry(|t| t);
    let error = derive_lane_rungs(&repo.root, "rust/ct-compare").unwrap_err();
    assert_eq!(error.code, "io");
    // ... and the same for a package with no row, which never touches a lane.
    assert_eq!(
        derive_lane_rungs(&repo.root, "rust/other").unwrap().len(),
        3
    );
}

#[test]
fn no_registry_file_means_an_unknown_lane_universe_not_a_live_lane() {
    let root = std::env::temp_dir().join(format!("wd004b2-empty-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    assert_eq!(derive_lane_rungs(&root, "rust/ct-compare").unwrap(), vec![]);
    match lane_status(&root, "python/ct-compare").unwrap() {
        LaneStatus::Absent(why) => assert!(why.contains("no consumer registry"), "{why}"),
        other => panic!("expected absent, got {other:?}"),
    }
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_registry_that_exists_but_is_broken_is_an_error_never_an_empty_universe() {
    let repo = Repo::with_registry(|_| "{ not json".into());
    let error = derive_lane_rungs(&repo.root, "rust/ct-compare").unwrap_err();
    assert_eq!(error.code, "registry");
    assert_eq!(
        lane_status(&repo.root, "python/ct-compare")
            .unwrap_err()
            .code,
        "registry"
    );

    #[cfg(unix)]
    {
        // A registry that is a link is refused, not skipped.
        let repo = Repo::with_registry(|t| t);
        let path = repo.root.join(REGISTRY);
        let moved = repo.root.join("real-registry.json");
        fs::rename(&path, &moved).unwrap();
        std::os::unix::fs::symlink(&moved, &path).unwrap();
        assert_eq!(
            derive_lane_rungs(&repo.root, "rust/ct-compare")
                .unwrap_err()
                .code,
            "symlink"
        );
    }
}

#[test]
fn only_rust_packages_have_lane_rungs() {
    let error = derive_lane_rungs(&real_root(), "python/ct-compare").unwrap_err();
    assert!(error.message.contains("not a Rust package"), "{error}");
}

// ---------------------------------------------------------------------------
// JSON and the offline shape check.
// ---------------------------------------------------------------------------

fn live_lanes() -> Vec<LaneRung> {
    derive_lane_rungs(&real_root(), "rust/ct-compare").unwrap()
}

#[test]
fn lanes_round_trip_through_json_with_a_closed_schema() {
    let lanes = live_lanes();
    let json = lanes_to_json(&lanes);
    assert_eq!(lanes_from_json(&json).unwrap(), lanes);

    // An extra field, a missing field, a wrong type, a non-integer count.
    let mut extra = json.clone();
    if let coding_adventures_ca_assurance_report::json::JsonValue::Array(items) = &mut extra {
        if let coding_adventures_ca_assurance_report::json::JsonValue::Object(map) = &mut items[0] {
            map.insert(
                "note".into(),
                coding_adventures_ca_assurance_report::json::JsonValue::Null,
            );
        }
    }
    assert!(lanes_from_json(&extra)
        .unwrap_err()
        .contains("closed schema"));
    let text = json.to_pretty_string(2);
    let parse = |t: &str| coding_adventures_ca_assurance_report::json::JsonValue::parse(t).unwrap();
    for (from, to, needle) in [
        (
            "\"release_ready\": false",
            "\"release_ready\": \"no\"",
            "release_ready",
        ),
        ("\"full\": 65537", "\"full\": 1.5", "bounded"),
        ("\"full\": 65537", "\"full\": -1", "bounded"),
        ("\"sandbox\": \"absent\"", "\"sandbox\": 0", "sandbox"),
    ] {
        let bad = text.replacen(from, to, 1);
        assert_ne!(bad, text, "{from} not found");
        let error = lanes_from_json(&parse(&bad)).unwrap_err();
        assert!(error.contains(needle), "{error}");
    }
    assert!(lanes_from_json(&parse("{}")).unwrap_err().contains("array"));
    assert!(lanes_from_json(&parse("[1]"))
        .unwrap_err()
        .contains("object"));
}

#[test]
fn the_shape_check_accepts_what_the_generator_writes() {
    check_lane_shape("rust/ct-compare", &live_lanes()).unwrap();
    check_lane_shape(
        "rust/gf256",
        &derive_lane_rungs(&real_root(), "rust/gf256").unwrap(),
    )
    .unwrap();
    check_lane_shape("rust/ct-compare", &[]).unwrap();
}

/// One falsification: what it does, how it edits the lanes, what the error names.
type Case<'a> = (&'a str, Box<dyn Fn(&mut Vec<LaneRung>)>, &'a str);

#[test]
fn the_shape_check_refuses_every_overclaim_and_malformation() {
    let live = live_lanes();
    let cases: Vec<Case> = vec![
        (
            "release",
            Box::new(|l| l[0].release_ready = true),
            "release_ready",
        ),
        (
            "sandbox",
            Box::new(|l| l[0].sandbox = "pass".into()),
            "sandbox",
        ),
        (
            "rung name",
            Box::new(|l| l[0].rung = "armed".into()),
            "rung",
        ),
        (
            "package",
            Box::new(|l| l[0].package = "go/other".into()),
            "package",
        ),
        ("order", Box::new(|l| l.reverse()), "sorted"),
        ("duplicate", Box::new(|l| l[1] = l[0].clone()), "sorted"),
        (
            "checks short",
            Box::new(|l| {
                l[0].checks.pop();
            }),
            "checks",
        ),
        (
            "checks order",
            Box::new(|l| l[0].checks.reverse()),
            "checks",
        ),
        (
            "unclaimed",
            Box::new(|l| {
                l[0].unclaimed.pop();
            }),
            "unclaimed",
        ),
        (
            "label",
            Box::new(|l| {
                l[0].cases_by_label.insert("exhaustive".into(), 1);
            }),
            "label",
        ),
        (
            "absent with checks",
            Box::new(|l| {
                l[0].rung = "absent".into();
            }),
            "no checks",
        ),
        (
            "planned with cases",
            Box::new(|l| {
                l[0].rung = "planned".into();
                l[0].checks.clear();
                l[0].unclaimed.clear();
            }),
            "no checks",
        ),
    ];
    for (what, mutate, needle) in cases {
        let mut lanes = live.clone();
        mutate(&mut lanes);
        let error = check_lane_shape("rust/ct-compare", &lanes).expect_err(what);
        assert!(error.contains(needle), "{what}: {error}");
    }
    assert!(check_lane_shape("python/x", &[]).is_err());
}

// ---------------------------------------------------------------------------
// The re-derivation names its failures.
// ---------------------------------------------------------------------------

#[test]
fn the_comparison_names_lane_mismatch_label_overclaim_and_staleness() {
    let derived = live_lanes();
    assert_eq!(compare_lane_rungs(&derived, &derived), Ok(()));

    let mismatch = |lanes: &[LaneRung]| match compare_lane_rungs(lanes, &derived) {
        Err(LaneDiff::Mismatch(m)) => m,
        other => panic!("expected a mismatch, got {other:?}"),
    };
    mismatch(&derived[..2]);
    mismatch(&[]);
    let mut renamed = derived.clone();
    renamed[0].package = "go/other".into();
    mismatch(&renamed);
    let mut unknown = derived.clone();
    unknown[0].rung = "armed".into();
    mismatch(&unknown);

    let overclaim =
        |lanes: &[LaneRung], against: &[LaneRung]| match compare_lane_rungs(lanes, against) {
            Err(LaneDiff::Overclaim(m)) => m,
            other => panic!("expected an overclaim, got {other:?}"),
        };
    let mut release = derived.clone();
    release[1].release_ready = true;
    overclaim(&release, &derived);
    let mut sandbox = derived.clone();
    sandbox[1].sandbox = "armed".into();
    overclaim(&sandbox, &derived);
    // live where the loader supports absent or planned
    let absent = derive_lane_rungs(&real_root(), "rust/gf256").unwrap();
    let mut claims_live = absent.clone();
    claims_live[0] = LaneRung {
        package: absent[0].package.clone(),
        ..derived[0].clone()
    };
    overclaim(&claims_live, &absent);
    let mut planned = absent.clone();
    planned[0].rung = "planned".into();
    let mut claims_live = planned.clone();
    claims_live[0].rung = "live".into();
    overclaim(&claims_live, &planned);
    // more cases under a label, or a label the loader never recorded
    let mut more = derived.clone();
    *more[2].cases_by_label.get_mut("full").unwrap() += 1;
    overclaim(&more, &derived);
    let mut invented = derived.clone();
    invented[2].cases_by_label.insert("sample".into(), 1);
    overclaim(&invented, &derived);

    // Weaker or merely different is stale, not an overclaim.
    let mut weaker = derived.clone();
    *weaker[0].cases_by_label.get_mut("full").unwrap() -= 1;
    assert!(matches!(
        compare_lane_rungs(&weaker, &derived),
        Err(LaneDiff::Stale(_))
    ));
    let mut downgraded = derived.clone();
    downgraded[0] = LaneRung {
        rung: "absent".into(),
        checks: vec![],
        cases_by_label: BTreeMap::new(),
        unclaimed: vec![],
        ..derived[0].clone()
    };
    assert!(matches!(
        compare_lane_rungs(&downgraded, &derived),
        Err(LaneDiff::Stale(_))
    ));
}

// ---------------------------------------------------------------------------
// The report: generation, JSON, `--check`, isolation.
// ---------------------------------------------------------------------------

fn generated() -> (AssuranceReport, Evidence) {
    let ev = real_evidence();
    let report = generate_package_report(
        "rust/ct-compare",
        "0.1.0",
        "c0ffee",
        &WeaknessRegistry::embedded(),
        &ev,
    )
    .unwrap();
    (report, ev)
}

#[test]
fn a_generated_report_carries_the_lane_rungs_and_checks_clean() {
    let (report, ev) = generated();
    assert_eq!(report.lanes, live_lanes());
    let json = report.to_json_string();
    assert!(json.contains("\"lanes\""), "{json}");
    assert!(json.contains("\"rung\": \"live\""), "{json}");
    assert!(json.contains("\"release_ready\": false"), "{json}");
    assert!(json.contains("\"sandbox\": \"absent\""), "{json}");
    assert_eq!(AssuranceReport::from_json_str(&json).unwrap(), report);
    verify_assurance_report(&report, &WeaknessRegistry::embedded()).unwrap();
    // The Rust evidence still verifies: the rungs of the lanes are an addition.
    let unverified = verify_evidence(&report, &ev).map_err(|e| e.to_string());
    // (`unverified::` static attestations are accepted or refused by the CLI,
    // not here; what matters is that no lane error appears.)
    if let Err(e) = unverified {
        assert!(!e.contains("lane"), "{e}");
    }
}

#[test]
fn a_report_without_lanes_is_refused_not_read_as_no_lanes() {
    let (report, _) = generated();
    let json = report.to_json_string();
    let value = coding_adventures_ca_assurance_report::json::JsonValue::parse(&json).unwrap();
    let coding_adventures_ca_assurance_report::json::JsonValue::Object(mut map) = value else {
        panic!("object")
    };
    map.remove("lanes");
    let stripped =
        coding_adventures_ca_assurance_report::json::JsonValue::Object(map).to_pretty_string(2);
    let error = AssuranceReport::from_json_str(&stripped)
        .unwrap_err()
        .to_string();
    assert!(error.contains("lanes"), "{error}");
}

fn lane_error(result: Result<Vec<String>, EvidenceCheckError>) -> EvidenceCheckError {
    result.expect_err("the lanes must be held to the registry and the loader")
}

#[test]
fn check_holds_the_report_to_the_registry_and_the_loader() {
    let (report, ev) = generated();

    // A lane dropped from the report (hiding it) is a mismatch.
    let mut hidden = report.clone();
    hidden.lanes.pop();
    assert!(matches!(
        lane_error(verify_evidence(&hidden, &ev)),
        EvidenceCheckError::LaneMismatch(_)
    ));
    let mut none = report.clone();
    none.lanes.clear();
    assert!(matches!(
        lane_error(verify_evidence(&none, &ev)),
        EvidenceCheckError::LaneMismatch(_)
    ));

    // A lane that says more than the loader supports is an overclaim.
    let mut release = report.clone();
    release.lanes[0].release_ready = true;
    assert!(matches!(
        lane_error(verify_evidence(&release, &ev)),
        EvidenceCheckError::LabelOverclaim(_)
    ));
    let mut sandbox = report.clone();
    sandbox.lanes[0].sandbox = "pass".into();
    assert!(matches!(
        lane_error(verify_evidence(&sandbox, &ev)),
        EvidenceCheckError::LabelOverclaim(_)
    ));
    let mut cases = report.clone();
    *cases.lanes[0].cases_by_label.get_mut("full").unwrap() += 1;
    assert!(matches!(
        lane_error(verify_evidence(&cases, &ev)),
        EvidenceCheckError::LabelOverclaim(_)
    ));

    // A weaker or different one is stale.
    let mut stale = report.clone();
    stale.lanes[0].rung = "absent".into();
    stale.lanes[0].checks.clear();
    stale.lanes[0].cases_by_label.clear();
    stale.lanes[0].unclaimed.clear();
    assert!(matches!(
        lane_error(verify_evidence(&stale, &ev)),
        EvidenceCheckError::StaleEvidence(_)
    ));

    // The offline check refuses the malformed ones before any re-derivation.
    let mut forged = report.clone();
    forged.lanes[0].release_ready = true;
    match verify_assurance_report(&forged, &WeaknessRegistry::embedded()) {
        Err(CheckError::InvalidLaneRung(m)) => assert!(m.contains("release_ready"), "{m}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn lane_rungs_are_isolated_from_the_rust_evidence_and_the_verdict() {
    let (report, ev) = generated();
    // The Rust rungs come from the Rust manifests and nothing else.
    let mut without = ev.clone();
    without.lanes.clear();
    assert_eq!(ev.rungs(), without.rungs());
    let bare = generate_package_report(
        "rust/ct-compare",
        "0.1.0",
        "c0ffee",
        &WeaknessRegistry::embedded(),
        &without,
    )
    .unwrap();
    assert!(bare.lanes.is_empty());
    // Every Rust-only field is identical with or without the lanes, so the lanes
    // neither feed nor are fed by the proof, effect and verdict evidence.
    let mut stripped = report.clone();
    stripped.lanes.clear();
    assert_eq!(stripped, bare);
    assert_eq!(report.verdict, bare.verdict);
    assert_eq!(report.rungs, bare.rungs);
}

// ---------------------------------------------------------------------------
// The CLI.
// ---------------------------------------------------------------------------

fn cli(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_ca-assurance-report-rs"))
        .args(args)
        .output()
        .unwrap();
    (
        out.status.code().unwrap(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn check_lane_prints_the_readiness_of_a_live_lane_and_exits_zero() {
    let root = real_root();
    let root = root.to_str().unwrap();
    for flags in [&[][..], &["--require-active"][..]] {
        let mut args = vec!["--check-lane", "python/ct-compare", "--repo-root", root];
        args.extend_from_slice(flags);
        let (code, stdout, stderr) = cli(&args);
        assert_eq!(code, 0, "{stderr}");
        assert!(stdout.contains("rung: LIVE"), "{stdout}");
        // The root that was checked is printed, absolute, so a wrong --repo-root shows.
        let line = stdout
            .lines()
            .find(|l| l.starts_with("repo root: "))
            .expect(&stdout);
        assert!(
            Path::new(&line["repo root: ".len()..]).is_absolute(),
            "{line}"
        );
        assert!(
            stdout.contains("source-fresh, runner-source-fresh"),
            "{stdout}"
        );
        assert!(stdout.contains("release_ready: false"), "{stdout}");
        assert!(stdout.contains("sandbox: absent"), "{stdout}");
        assert!(
            stdout.contains("not checked: toolchain authenticity, build execution, sandbox"),
            "{stdout}"
        );
    }
}

#[test]
fn check_lane_reports_absent_honestly_and_require_active_turns_it_into_a_failure() {
    let root = real_root();
    let root = root.to_str().unwrap();
    let (code, stdout, _) = cli(&["--check-lane", "python/gf256", "--repo-root", root]);
    assert_eq!(code, 0);
    assert!(stdout.contains("rung: ABSENT"), "{stdout}");
    assert!(stdout.contains("repo root: "), "{stdout}");
    assert!(
        stdout.contains("release_ready: false") && stdout.contains("sandbox: absent"),
        "{stdout}"
    );
    assert!(!stdout.contains("LIVE"), "{stdout}");

    let (code, stdout, stderr) = cli(&[
        "--check-lane",
        "python/gf256",
        "--repo-root",
        root,
        "--require-active",
    ]);
    assert_eq!(code, 1);
    assert!(stdout.contains("rung: ABSENT"), "{stdout}");
    assert!(stderr.contains("--require-active"), "{stderr}");
}

#[test]
fn require_active_also_refuses_a_planned_row() {
    let repo = Repo::with_registry(|t| {
        let t = plan(t, "go");
        let t = plan(t, "python");
        plan(t, "typescript").replace(
            "\"active_consumer_count\": 2",
            "\"active_consumer_count\": 0",
        )
    });
    let root = repo.root.to_str().unwrap();
    let (code, stdout, _) = cli(&["--check-lane", "go/ct-compare", "--repo-root", root]);
    assert_eq!(code, 0);
    assert!(stdout.contains("rung: PLANNED"), "{stdout}");
    let (code, _, stderr) = cli(&[
        "--check-lane",
        "go/ct-compare",
        "--repo-root",
        root,
        "--require-active",
    ]);
    assert_eq!(code, 1);
    assert!(stderr.contains("not an active lane"), "{stderr}");
}

#[test]
fn an_active_lane_that_fails_to_load_exits_one_with_or_without_the_flag() {
    let repo = Repo::with_registry(|t| t);
    let root = repo.root.to_str().unwrap();
    for flags in [&[][..], &["--require-active"][..]] {
        let mut args = vec!["--check-lane", "go/ct-compare", "--repo-root", root];
        args.extend_from_slice(flags);
        let (code, stdout, stderr) = cli(&args);
        assert_eq!(code, 1, "{stdout}");
        assert!(
            stderr.contains("LANE CHECK FAILED") && stderr.contains("[io]"),
            "{stderr}"
        );
    }
}

#[test]
fn a_stale_lane_source_fails_the_lane_check() {
    // The lane BUILD line: a lane edit re-runs the loader, and a stale manifest
    // fails it.  Use a throw-away copy of the python lane.
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let root = std::env::temp_dir().join(format!("wd004b2-stale-{}-{id}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    copy_lane(&real_root(), &root, "python");
    let root_text = root.to_str().unwrap();
    let args = [
        "--check-lane",
        "python/ct-compare",
        "--repo-root",
        root_text,
        "--require-active",
    ];
    let (code, _, stderr) = cli(&args);
    assert_eq!(code, 0, "{stderr}");
    fs::write(
        root.join("code/packages/python/ct-compare/src/coding_adventures_ct_compare/zz_added.py"),
        b"x = 1\n",
    )
    .unwrap();
    let (code, _, stderr) = cli(&args);
    assert_eq!(code, 1);
    assert!(stderr.contains("[stale-source]"), "{stderr}");
    let _ = fs::remove_dir_all(&root);
}

/// Copy everything a load of `<lane>/ct-compare` opens into `to`.
fn copy_lane(real: &Path, to: &Path, lane: &str) {
    fn copy(real: &Path, to: &Path, rel: &str) {
        let from = real.join(rel);
        if from.is_dir() {
            for entry in fs::read_dir(&from).unwrap() {
                let entry = entry.unwrap();
                let name = entry.file_name().to_string_lossy().into_owned();
                if ["__pycache__", "node_modules", ".venv", "dist", "target"]
                    .contains(&name.as_str())
                {
                    continue;
                }
                copy(real, to, &format!("{rel}/{name}"));
            }
        } else if from.is_file() {
            let target = to.join(rel);
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::copy(from, target).unwrap();
        }
    }
    for rel in [
        "code/specs/ct-compare.warrant",
        "code/specs/fixtures/ct-compare-warrant-v1",
        &format!("code/packages/{lane}/ct-compare"),
        &format!("code/packages/{lane}/warrant-corpus"),
    ] {
        copy(real, to, rel);
    }
}

#[test]
fn the_lane_check_modes_are_not_mixed_up() {
    let (code, _, stderr) = cli(&["--require-active"]);
    assert_eq!(code, 2);
    assert!(stderr.contains("--check-lane"), "{stderr}");
    let (code, _, _) = cli(&["--repo-root", "."]);
    assert_eq!(code, 2);
    let (code, _, stderr) = cli(&["--check-lane", "python/ct-compare", "--generate"]);
    assert_eq!(code, 2);
    assert!(stderr.contains("cannot be combined"), "{stderr}");
    let (code, _, _) = cli(&["--check-lane"]);
    assert_eq!(code, 2);
    let (code, _, _) = cli(&["--repo-root"]);
    assert_eq!(code, 2);
}
