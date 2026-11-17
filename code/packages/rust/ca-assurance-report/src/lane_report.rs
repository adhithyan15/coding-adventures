//! # Lane rungs of a package report (WD-004b2, VER04i section 8.3)
//!
//! A Rust package's assurance report answers one more question than it used to:
//! *for each language lane the consumer registry knows, is there evidence, and
//! how much?*  The answer is one [`LaneRung`] per lane:
//!
//! ```text
//!   absent   the registry has no row for <lane>/<name>: nothing was run
//!   planned  the row exists and says `planned`: a reservation, not evidence
//!   live     the row is `active` AND `load_lane` passed every named check
//! ```
//!
//! `live` has exactly one source: [`crate::lane_loader::load_lane`] returning
//! `Ok`.  There is no partial result, so a rung can never be stronger than the
//! checks that ran.  Every rung also says `release_ready: false` and
//! `sandbox: absent`, because no lane arms a runtime jail yet (VER04i 12,
//! CAP-RT-1xx), and `live` carries the loader's own list of what it did not
//! establish.
//!
//! ## Isolation
//!
//! Nothing in this module reads the Rust proof manifest, the effect
//! certificate, the weakness matrix, the claims or the verdict, and nothing
//! outside it feeds a lane: a Rust proof says nothing about a Python
//! implementation, and a live Python lane says nothing about the Rust one.
//! The functions here take the repository root and a package name, and return
//! data.
//!
//! ## Active means loadable
//!
//! An `active` registry row whose lane fails to load is **not** a rung: it is an
//! error ([`derive_lane_rungs`] returns it), so a report cannot be generated
//! or checked against a lane that has stopped loading.

#![deny(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::json::JsonValue;
use crate::lane_loader::{load_lane, load_registry, LaneLoadError, LoadedLane, UNCLAIMED};

/// Rung names, weakest first.  The position is the strength.
pub const LANE_RUNGS: [&str; 3] = ["absent", "planned", "live"];

/// The only sandbox state any lane can report today (VER04i 12).
pub const SANDBOX_ABSENT: &str = "absent";

/// The coverage labels a lane may quote (VER04i 8, the sidecar).
const KNOWN_LABELS: [&str; 3] = ["full", "bounded-set", "sample"];

/// One lane's rung in a report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaneRung {
    pub lane: String,
    /// `<lane>/<name>`: the package that carries (or would carry) the lane.
    pub package: String,
    /// One of [`LANE_RUNGS`].
    pub rung: String,
    /// Names of the passed readiness checks, in order; empty unless `live`.
    pub checks: Vec<String>,
    /// The sidecar's per-label case counts; empty unless `live`.
    pub cases_by_label: BTreeMap<String, u64>,
    /// What the loader did not establish; empty unless `live`.
    pub unclaimed: Vec<String>,
    /// Always `false` (the sandbox is absent in every lane).
    pub release_ready: bool,
    /// Always [`SANDBOX_ABSENT`].
    pub sandbox: String,
}

/// What the registry and the loader say about one `<lane>/<name>`.
#[derive(Clone, Debug)]
pub enum LaneStatus {
    /// Active and every named check passed.
    Live(Box<LoadedLane>),
    /// A registry row exists and is `planned`.
    Planned,
    /// No row (or no registry at all); the string says which.
    Absent(String),
}

/// Ask the registry and, for an active row, the loader.  An active row that
/// fails to load is the loader's error.
pub fn lane_status(repo_root: &Path, package: &str) -> Result<LaneStatus, LaneLoadError> {
    let Some(registry) = load_registry(repo_root)? else {
        return Ok(LaneStatus::Absent("there is no consumer registry".into()));
    };
    match registry.consumers.iter().find(|row| row.package == package) {
        None => Ok(LaneStatus::Absent(format!(
            "the consumer registry has no row for {package}"
        ))),
        Some(row) if row.status == "active" => {
            Ok(LaneStatus::Live(Box::new(load_lane(repo_root, package)?)))
        }
        Some(_) => Ok(LaneStatus::Planned),
    }
}

fn rung_of(lane: &str, package: &str, status: LaneStatus) -> LaneRung {
    let mut rung = LaneRung {
        lane: lane.to_owned(),
        package: package.to_owned(),
        rung: "absent".into(),
        checks: Vec::new(),
        cases_by_label: BTreeMap::new(),
        unclaimed: Vec::new(),
        // Not copied from the loader: the sandbox is absent and nothing here
        // can say otherwise until a lane arms one.
        release_ready: false,
        sandbox: SANDBOX_ABSENT.into(),
    };
    match status {
        LaneStatus::Absent(_) => {}
        LaneStatus::Planned => rung.rung = "planned".into(),
        LaneStatus::Live(loaded) => {
            rung.rung = "live".into();
            rung.checks = loaded
                .readiness
                .passed
                .iter()
                .map(|c| c.to_string())
                .collect();
            rung.cases_by_label = loaded.labels.cases_by_label.clone();
            rung.unclaimed = UNCLAIMED.iter().map(|u| u.to_string()).collect();
        }
    }
    rung
}

/// The lane rungs of the Rust package `rust_package` (for example `rust/gf256`):
/// one per lane the consumer registry lists, sorted by lane.  An unknown lane
/// universe (no registry file) is an empty list; a broken registry or an
/// `active` row that does not load is an error.
pub fn derive_lane_rungs(
    repo_root: &Path,
    rust_package: &str,
) -> Result<Vec<LaneRung>, LaneLoadError> {
    let Some(name) = rust_package.strip_prefix("rust/") else {
        return Err(LaneLoadError {
            code: "path",
            message: format!("{rust_package} is not a Rust package"),
        });
    };
    let Some(registry) = load_registry(repo_root)? else {
        return Ok(Vec::new());
    };
    let lanes: BTreeSet<&str> = registry
        .consumers
        .iter()
        .map(|row| row.lane.as_str())
        .collect();
    let mut rungs = Vec::with_capacity(lanes.len());
    for lane in lanes {
        let package = format!("{lane}/{name}");
        let status = lane_status(repo_root, &package)?;
        rungs.push(rung_of(lane, &package, status));
    }
    Ok(rungs)
}

/// The human line for a non-live lane, as `--check-lane` prints it.
pub fn render_not_live(package: &str, status: &LaneStatus) -> String {
    match status {
        LaneStatus::Absent(why) => format!(
            "lane: {package}\nrung: ABSENT ({why})\nrelease_ready: false\nsandbox: absent\n"
        ),
        LaneStatus::Planned => format!(
            "lane: {package}\nrung: PLANNED (the registry row is planned; a reservation is not evidence)\n\
             release_ready: false\nsandbox: absent\n"
        ),
        LaneStatus::Live(loaded) => loaded.render_readiness(),
    }
}

// ---------------------------------------------------------------------------
// JSON
// ---------------------------------------------------------------------------

fn strings(items: &[String]) -> JsonValue {
    JsonValue::Array(items.iter().map(|s| JsonValue::String(s.clone())).collect())
}

/// The `lanes` value of a report.
pub fn lanes_to_json(lanes: &[LaneRung]) -> JsonValue {
    JsonValue::Array(
        lanes
            .iter()
            .map(|lane| {
                let mut map = BTreeMap::new();
                map.insert("lane".into(), JsonValue::String(lane.lane.clone()));
                map.insert("package".into(), JsonValue::String(lane.package.clone()));
                map.insert("rung".into(), JsonValue::String(lane.rung.clone()));
                map.insert("checks".into(), strings(&lane.checks));
                map.insert(
                    "cases_by_label".into(),
                    JsonValue::Object(
                        lane.cases_by_label
                            .iter()
                            .map(|(k, v)| (k.clone(), JsonValue::Number(*v as f64)))
                            .collect(),
                    ),
                );
                map.insert("unclaimed".into(), strings(&lane.unclaimed));
                map.insert("release_ready".into(), JsonValue::Bool(lane.release_ready));
                map.insert("sandbox".into(), JsonValue::String(lane.sandbox.clone()));
                JsonValue::Object(map)
            })
            .collect(),
    )
}

fn text_field(map: &BTreeMap<String, JsonValue>, key: &str) -> Result<String, String> {
    map.get(key)
        .and_then(JsonValue::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("lane field `{key}` must be a string"))
}

fn text_list(map: &BTreeMap<String, JsonValue>, key: &str) -> Result<Vec<String>, String> {
    let items = map
        .get(key)
        .and_then(JsonValue::as_array)
        .ok_or_else(|| format!("lane field `{key}` must be an array"))?;
    items
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("lane field `{key}` must hold strings"))
        })
        .collect()
}

/// Parse the `lanes` value of a report.  The schema is closed: exactly the
/// eight fields, so an extra claim has nowhere to hide.
pub fn lanes_from_json(value: &JsonValue) -> Result<Vec<LaneRung>, String> {
    let items = value.as_array().ok_or("`lanes` must be an array")?;
    let mut lanes = Vec::with_capacity(items.len());
    for item in items {
        let map = item.as_object().ok_or("a lane entry must be an object")?;
        const FIELDS: [&str; 8] = [
            "lane",
            "package",
            "rung",
            "checks",
            "cases_by_label",
            "unclaimed",
            "release_ready",
            "sandbox",
        ];
        if map.len() != FIELDS.len() || FIELDS.iter().any(|f| !map.contains_key(*f)) {
            return Err("a lane entry's fields do not match the closed schema".into());
        }
        let by_label = map
            .get("cases_by_label")
            .and_then(JsonValue::as_object)
            .ok_or("lane field `cases_by_label` must be an object")?;
        let mut cases_by_label = BTreeMap::new();
        for (label, count) in by_label {
            // The same bound the sidecar enforces: a non-negative integer within 2^53.
            match count.as_i64() {
                Some(n) if n >= 0 && (n as u64) <= (1 << 53) => {
                    cases_by_label.insert(label.clone(), n as u64);
                }
                _ => {
                    return Err(format!(
                        "cases_by_label.{label} must be a bounded non-negative integer"
                    ))
                }
            }
        }
        lanes.push(LaneRung {
            lane: text_field(map, "lane")?,
            package: text_field(map, "package")?,
            rung: text_field(map, "rung")?,
            checks: text_list(map, "checks")?,
            cases_by_label,
            unclaimed: text_list(map, "unclaimed")?,
            release_ready: map
                .get("release_ready")
                .and_then(JsonValue::as_bool)
                .ok_or("lane field `release_ready` must be a boolean")?,
            sandbox: text_field(map, "sandbox")?,
        });
    }
    Ok(lanes)
}

// ---------------------------------------------------------------------------
// The offline shape check
// ---------------------------------------------------------------------------

fn rank(rung: &str) -> Option<usize> {
    LANE_RUNGS.iter().position(|r| *r == rung)
}

/// What a report's `lanes` must look like before anything is re-derived (VER04i
/// 8.3): sorted, unique, packages that follow from the report's package, known
/// rungs, the loader's own checks and unclaimed list for `live`, nothing for
/// any other rung, and never a release or a sandbox claim.
pub fn check_lane_shape(report_package: &str, lanes: &[LaneRung]) -> Result<(), String> {
    let name = report_package
        .strip_prefix("rust/")
        .ok_or_else(|| format!("{report_package} is not a Rust package"))?;
    if !lanes.windows(2).all(|pair| pair[0].lane < pair[1].lane) {
        return Err("lanes must be unique and sorted by lane".into());
    }
    let checks: Vec<String> = crate::lane_loader::READINESS_CHECKS
        .iter()
        .map(|c| c.to_string())
        .collect();
    let unclaimed: Vec<String> = UNCLAIMED.iter().map(|u| u.to_string()).collect();
    for lane in lanes {
        let who = &lane.lane;
        if lane.package != format!("{who}/{name}") {
            return Err(format!(
                "lane {who}: package {:?} is not {who}/{name}",
                lane.package
            ));
        }
        if rank(&lane.rung).is_none() {
            return Err(format!(
                "lane {who}: rung {:?} is not absent, planned or live",
                lane.rung
            ));
        }
        if lane.release_ready {
            return Err(format!(
                "lane {who}: release_ready is true, but no lane arms a sandbox"
            ));
        }
        if lane.sandbox != SANDBOX_ABSENT {
            return Err(format!(
                "lane {who}: sandbox is {:?}, not absent",
                lane.sandbox
            ));
        }
        if let Some(label) = lane
            .cases_by_label
            .keys()
            .find(|l| !KNOWN_LABELS.contains(&l.as_str()))
        {
            return Err(format!("lane {who}: unknown coverage label {label:?}"));
        }
        if lane.rung == "live" {
            if lane.checks != checks {
                return Err(format!(
                    "lane {who}: a live lane carries exactly the named readiness checks, in order"
                ));
            }
            if lane.unclaimed != unclaimed {
                return Err(format!(
                    "lane {who}: a live lane carries the loader's unclaimed list"
                ));
            }
        } else if !lane.checks.is_empty()
            || !lane.cases_by_label.is_empty()
            || !lane.unclaimed.is_empty()
        {
            return Err(format!(
                "lane {who}: a {} lane has no checks, cases or unclaimed list",
                lane.rung
            ));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// The re-derivation
// ---------------------------------------------------------------------------

/// How a report's lanes differ from the ones re-derived from the repository.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LaneDiff {
    /// A different lane set or package id (VER04i 9 check 17).
    Mismatch(String),
    /// The report claims more than the loader supports (check 24).
    Overclaim(String),
    /// Anything else that differs: the report is stale, regenerate it.
    Stale(String),
}

/// Hold `report` to `derived`.  Order matters: identity first, then the claims
/// that are stronger than the evidence, then plain staleness.
pub fn compare_lane_rungs(report: &[LaneRung], derived: &[LaneRung]) -> Result<(), LaneDiff> {
    let ids = |lanes: &[LaneRung]| -> Vec<(String, String)> {
        lanes
            .iter()
            .map(|l| (l.lane.clone(), l.package.clone()))
            .collect()
    };
    if ids(report) != ids(derived) {
        return Err(LaneDiff::Mismatch(format!(
            "the report lists lanes {:?} but the consumer registry gives {:?}",
            ids(report),
            ids(derived)
        )));
    }
    for (claimed, actual) in report.iter().zip(derived) {
        let who = &claimed.lane;
        if claimed.release_ready {
            return Err(LaneDiff::Overclaim(format!(
                "lane {who}: release_ready is true"
            )));
        }
        if claimed.sandbox != SANDBOX_ABSENT {
            return Err(LaneDiff::Overclaim(format!(
                "lane {who}: sandbox is {:?}, not absent",
                claimed.sandbox
            )));
        }
        match (rank(&claimed.rung), rank(&actual.rung)) {
            (Some(c), Some(a)) if c > a => {
                return Err(LaneDiff::Overclaim(format!(
                    "lane {who}: the report says {} but the registry and loader support {}",
                    claimed.rung, actual.rung
                )));
            }
            (None, _) | (_, None) => {
                return Err(LaneDiff::Mismatch(format!("lane {who}: unknown rung")));
            }
            _ => {}
        }
        for (label, cases) in &claimed.cases_by_label {
            if *cases > actual.cases_by_label.get(label).copied().unwrap_or(0) {
                return Err(LaneDiff::Overclaim(format!(
                    "lane {who}: the report quotes more {label} cases than the sidecar records"
                )));
            }
        }
        if claimed != actual {
            return Err(LaneDiff::Stale(format!(
                "lane {who} differs from the re-derivation from the registry and the lane artifacts: regenerate the report"
            )));
        }
    }
    Ok(())
}
