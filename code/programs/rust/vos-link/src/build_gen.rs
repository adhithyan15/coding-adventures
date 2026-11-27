//! # `gen-build`: the canonical region of BUILD, generated (VOS-039a, VOS04 section 5.6)
//!
//! The first draft of `check-component` *recognised* the required BUILD lines in the author's
//! text.  A security review showed that cannot hold: the build tool runs **every non-comment
//! line as its own `sh -c`**, so `echo cargo test ...`, `true # cargo test ...`,
//! `false && ...`, `... || true`, a `"Linux"` in a comment, a second `--fail-under 0` or an
//! `--exclude-files` all contain the required words and run something else.  A lint over
//! author-written text never converges (the same lesson as VOS-012b0).  So the lines are not
//! recognised, they are **reproduced**: this module writes them from the crate's declared
//! inputs, and `check-component` requires the file to begin with exactly what it writes, the
//! way `check-doc` requires the document to equal what `gen-doc` writes.
//!
//! ```text
//!   vos.toml ─────────────── kind, evidence-features, coverage-floor, excused-controls
//!   evidence.bindings ────── the bound operations and their faults   (the generator's reader)
//!   code/specs/<crate>.warrant ─ claim and example names              (parsed, not scanned)
//!   warrant/sabotage.plan ── which claim attacks which fault          (names only, no commands)
//!        │
//!        ▼  canonical_build()
//!   BUILD, BUILD_windows ─── header, lines 1-9, the Linux coverage gate, check-component,
//!                            then MARKER.  Below the marker: the author's, run, never counted.
//!   warrant/check_evidence.sh ─ only when excused-controls is declared
//! ```
//!
//! ## The sabotage plan
//!
//! The claim-to-operation map lives inside the Warrant expressions, so which claim should
//! attack which fault is a choice the author records, one entry per line:
//! `<tier>/<name> <fault> [attack=<op>]`.  [`validate_plan`] holds the entries to the
//! document (the claim or example exists) and to the bindings (the fault is bound, every
//! bound fault and every `field-flip` operation is attacked).  Whether the claim really
//! *catches* the fault is the run's: the shared checker `code/scripts/warrant_negctl.py`
//! fails a control that does not.  A plan holds names, never command text.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use coding_adventures_warrant_grammar::binding::parse_evidence_bindings;
use vos_manifest::warrant::{claim_calls, declared_names};
use vos_shape_check::manifest::{is_backlog_ref, Excuse, SatLane};
use vos_shape_check::DEFAULT_EVIDENCE_FEATURES;

use crate::check::WARRANT_STACK_BYTES;
use crate::component::{read_component_vos_toml, read_text};
use crate::error::{Finding, NoVerdict};
use crate::fsio::{ensure_plain_dirs, esc, write_replacing};
use crate::inputs::resolve_layout;
use crate::ops::Outcome;

/// The line that ends the generated region.  Everything after it is the author's.
pub const MARKER: &str = "# --- crate-specific (run, but never counted as evidence) ---";
/// The sabotage plan, relative to the crate directory.
pub const PLAN_FILE: &str = "warrant/sabotage.plan";
/// The generated tolerance script, relative to the crate directory.
pub const TOLERANCE_SCRIPT: &str = "warrant/check_evidence.sh";
/// The record fault that cancels itself when two operations list it (VER04j 4.8).
pub const FIELD_FLIP: &str = "field-flip";

const PLAN_MAX_BYTES: usize = 64 * 1024;
const BINDINGS_MAX_BYTES: usize = 256 * 1024;
const DOC_MAX_BYTES: usize = vos_manifest::caps::DOCUMENT_MAX_BYTES;
const PLAN_MAX_ENTRIES: usize = 512;
const BACKLOG_MAX_BYTES: usize = 4 * 1024 * 1024;

/// One line of the sabotage plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanEntry {
    /// `exhaustive` or `example`.
    pub tier: String,
    /// The claim or example name.
    pub name: String,
    /// The fault injected.
    pub fault: String,
    /// `attack=<op>`, for `field-flip`.
    pub attack: Option<String>,
    /// 1-based line of the plan, for findings.
    pub line: usize,
}

/// What generation needs, all of it read from files a reviewer sees.
#[derive(Clone, Debug)]
pub struct GenInputs {
    /// The crate name.
    pub krate: String,
    /// `vos.toml` `evidence-features`.
    pub features: Vec<String>,
    /// `vos.toml` `coverage-floor`.
    pub floor: u8,
    /// `vos.toml` `excused-controls`.
    pub excused: Vec<Excuse>,
    /// The sabotage plan, in file order.
    pub plan: Vec<PlanEntry>,
    /// The claims of the document.
    pub claims: Vec<String>,
    /// The examples of the document.
    pub examples: Vec<String>,
    /// The faults of each bound operation, from `evidence.bindings`.
    pub faults: BTreeMap<String, Vec<String>>,
    /// For each claim, the bound operations its text calls (lexical, VOS-D108).
    pub calls: Vec<(String, Vec<String>)>,
    /// `evidence.bindings` `property_cases`.
    pub property_cases: u64,
    /// `evidence.bindings` `fuzz_iterations`.
    pub fuzz_iterations: u64,
}

fn plain_name(s: &str) -> bool {
    !s.is_empty() && s.len() <= 64 && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

fn plain_fault(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 32
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// Parse the plan text.  Pure.  Every problem is one message naming the line.
pub fn parse_plan(text: &str) -> (Vec<PlanEntry>, Vec<String>) {
    let mut entries: Vec<PlanEntry> = Vec::new();
    let mut problems = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let line = i + 1;
        let t = raw.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let at = |m: String| format!("{PLAN_FILE}:{line}: {m}");
        let words: Vec<&str> = t.split_whitespace().collect();
        if words.len() < 2 || words.len() > 3 {
            problems.push(at(format!(
                "an entry is `<tier>/<name> <fault> [attack=<op>]`, found {}",
                esc(t)
            )));
            continue;
        }
        let Some((tier, name)) = words[0].split_once('/') else {
            problems.push(at(format!("{} is not `<tier>/<name>`", esc(words[0]))));
            continue;
        };
        if tier != "exhaustive" && tier != "example" {
            problems.push(at(format!(
                "tier {} is not exhaustive or example",
                esc(tier)
            )));
            continue;
        }
        if !plain_name(name) {
            problems.push(at(format!("{} is not a plain name", esc(name))));
            continue;
        }
        let fault = words[1];
        if !plain_fault(fault) {
            problems.push(at(format!("{} is not a plain fault name", esc(fault))));
            continue;
        }
        let attack = match words.get(2) {
            None => None,
            Some(w) => match w.strip_prefix("attack=") {
                Some(op) if plain_name(op) => Some(op.to_string()),
                _ => {
                    problems.push(at(format!("{} is not `attack=<op>`", esc(w))));
                    continue;
                }
            },
        };
        if (fault == FIELD_FLIP) != attack.is_some() {
            problems.push(at(format!(
                "`{FIELD_FLIP}` takes `attack=<op>` and no other fault does"
            )));
            continue;
        }
        let e = PlanEntry {
            tier: tier.to_string(),
            name: name.to_string(),
            fault: fault.to_string(),
            attack,
            line,
        };
        if let Some(prev) = entries.iter().find(|p| {
            (&p.tier, &p.name, &p.fault, &p.attack) == (&e.tier, &e.name, &e.fault, &e.attack)
        }) {
            problems.push(at(format!("repeats the entry of line {}", prev.line)));
            continue;
        }
        if entries.len() >= PLAN_MAX_ENTRIES {
            problems.push(at(format!("more than {PLAN_MAX_ENTRIES} entries")));
            break;
        }
        entries.push(e);
    }
    (entries, problems)
}

/// Hold a plan to the document and the bindings.  `faults` maps each bound operation to its
/// faults.  Pure.
pub fn validate_plan(
    plan: &[PlanEntry],
    faults: &BTreeMap<String, Vec<String>>,
    claims: &[String],
    examples: &[String],
    calls: &[(String, Vec<String>)],
    excused: &[Excuse],
) -> Vec<String> {
    let mut p = Vec::new();
    let bound: BTreeSet<&str> = faults.values().flatten().map(String::as_str).collect();
    for e in plan {
        let at = |m: String| format!("{PLAN_FILE}:{}: {m}", e.line);
        let (pool, what) = if e.tier == "exhaustive" {
            (claims, "claim")
        } else {
            (examples, "example")
        };
        if !pool.contains(&e.name) {
            p.push(at(format!(
                "{} is not a {what} of the document",
                esc(&e.name)
            )));
        }
        if !bound.contains(e.fault.as_str()) {
            p.push(at(format!(
                "fault {} is bound to no operation in evidence.bindings",
                esc(&e.fault)
            )));
        }
        if let Some(op) = &e.attack {
            if !faults
                .get(op)
                .is_some_and(|f| f.iter().any(|x| x == FIELD_FLIP))
            {
                p.push(at(format!(
                    "attack={} names no operation that lists `{FIELD_FLIP}`",
                    esc(op)
                )));
            }
        }
    }
    for f in &bound {
        if *f != FIELD_FLIP && !plan.iter().any(|e| e.fault == *f) {
            p.push(format!(
                "{PLAN_FILE}: no entry attacks the bound fault `{f}`: nothing proves the evidence can fail on it"
            ));
        }
    }
    for (op, fs) in faults {
        if fs.iter().any(|f| f == FIELD_FLIP)
            && !plan
                .iter()
                .any(|e| e.fault == FIELD_FLIP && e.attack.as_deref() == Some(op))
        {
            p.push(format!(
                "{PLAN_FILE}: no entry attacks `{FIELD_FLIP}` on operation `{op}` (`attack={op}`)"
            ));
        }
    }
    // Every claim that calls a bound operation is attacked at least once (review M3): an
    // author cannot attack one easy claim per fault and leave the others untried.  A claim is
    // exempt only when every fault of every operation it calls is excused by a declared
    // control (rr's `init`/`field-flip`).  Counting controls per operation and tier is VOS-D105.
    for (claim, called) in calls {
        let all_excused = called.iter().all(|op| {
            faults.get(op).is_some_and(|fs| {
                fs.iter().all(|f| {
                    excused
                        .iter()
                        .any(|e| e.id.ends_with(&format!("/{op}/{f}")))
                })
            })
        });
        if !called.is_empty()
            && !all_excused
            && !plan
                .iter()
                .any(|e| e.tier == "exhaustive" && e.name == *claim)
        {
            p.push(format!(
                "{PLAN_FILE}: claim `{claim}` calls the bound operation(s) {} and no entry attacks it (every claim that calls a bound operation needs an entry)",
                called.join(", ")
            ));
        }
    }
    p
}

/// Every value `gen-build` pastes into a BUILD line or the tolerance script, checked in ONE
/// place (review M1).  An unquoted value is shell text: `ghost #` truncates a line, `ghost&&x`
/// runs `x` on cmd, `$(..)` runs in line 2.  So each is held to a plain alphabet here, and a
/// feature must also be on the central evidence-feature list `vos-shape-check` enforces.
pub fn interpolation_problems(g: &GenInputs) -> Vec<String> {
    let mut p = Vec::new();
    let plain = |s: &str, first_lower: bool, extra: &[u8]| {
        !s.is_empty()
            && s.len() <= 128
            && (!first_lower || s.as_bytes()[0].is_ascii_lowercase())
            && s.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || extra.contains(&b))
    };
    if !plain(&g.krate, true, b"-") {
        p.push(format!(
            "crate name {} is not [a-z][a-z0-9-]*",
            esc(&g.krate)
        ));
    }
    for f in &g.features {
        if !plain(f, true, b"_-") {
            p.push(format!(
                "vos.toml: evidence feature {} is not [a-z][a-z0-9_-]*",
                esc(f)
            ));
        } else if !DEFAULT_EVIDENCE_FEATURES.contains(&f.as_str()) {
            p.push(format!(
                "vos.toml: evidence feature {} is not in the central list ({})",
                esc(f),
                DEFAULT_EVIDENCE_FEATURES.join(", ")
            ));
        }
    }
    for e in &g.plan {
        if !plain_name(&e.name) || !(e.tier == "exhaustive" || e.tier == "example") {
            p.push(format!("{PLAN_FILE}:{}: tier or name is not plain", e.line));
        }
        if !plain_fault(&e.fault) || e.attack.as_deref().is_some_and(|a| !plain_name(a)) {
            p.push(format!(
                "{PLAN_FILE}:{}: fault or attack is not plain",
                e.line
            ));
        }
    }
    for x in &g.excused {
        let ok = !x.id.is_empty()
            && x.id.len() <= 128
            && x.id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_/.-".contains(&b));
        if !ok || !is_backlog_ref(&x.reason) {
            p.push(format!(
                "vos.toml: excused control {} / {} is not plain",
                esc(&x.id),
                esc(&x.reason)
            ));
        }
    }
    p
}

// ---------------------------------------------------------------------------------------
// The text
// ---------------------------------------------------------------------------------------

fn features_arg(g: &GenInputs) -> String {
    if g.features.is_empty() {
        String::new()
    } else {
        format!(" --features {}", g.features.join(","))
    }
}

/// The canonical region of `BUILD` (`windows == false`) or `BUILD_windows`, ending with
/// [`MARKER`] and a newline.
pub fn canonical_build(g: &GenInputs, windows: bool) -> String {
    let c = &g.krate;
    let feat = features_arg(g);
    let vl = "../../../programs/rust/vos-link/Cargo.toml";
    let mut s = String::new();
    let mut l = |line: String| {
        s.push_str(&line);
        s.push('\n');
    };
    l(format!("# GENERATED by `vos-link gen-build` (VOS04 5.6) from vos.toml, evidence.bindings, {PLAN_FILE} and code/specs/{c}.warrant."));
    l("# Edit those and run gen-build: `vos-link check-component` requires this region byte for byte.".to_string());
    l(
        "# Your own lines go after the marker at the end; they run but satisfy no requirement."
            .to_string(),
    );
    l("# needs-toolchain: python".to_string());
    l("# Lines 1 to 5 of VOS04 5.2: unit (with and without the evidence feature, then doc), shape, capability, document, harness drift.".to_string());
    l(format!(
        "cargo test -p {c} --lib --test behaviour -- --nocapture"
    ));
    if !g.features.is_empty() {
        l(format!("cargo test -p {c}{feat} --lib --test behaviour"));
    }
    l(format!("cargo test -p {c} --doc"));
    l("cargo run -q -p vos-shape-check -- .".to_string());
    l("cargo run -q -p coding_adventures_ca_capability_analyzer -- --dir .".to_string());
    l(format!("cargo run -q --manifest-path {vl} -- check-doc ."));
    l(format!(
        "cargo run -q -p coding_adventures_warrant_grammar --example gen_evidence -- --check {c}"
    ));
    l("# Lines 6 to 8: the generated suite (exhaustive, property, fuzz) rewrites the evidence manifest.  The old one is removed first, so a".to_string());
    l("# run that exits 0 without rewriting it leaves no file and fails check-component (CC02), instead of passing on a stale one.".to_string());
    if windows {
        l(
            "if exist evidence\\test-manifest.json del /f /q evidence\\test-manifest.json"
                .to_string(),
        );
    } else {
        l("rm -f evidence/test-manifest.json".to_string());
    }
    if !g.excused.is_empty() {
        let sh = if windows { "bash" } else { "sh" };
        l(format!("{sh} {TOLERANCE_SCRIPT}"));
    } else if windows {
        l(format!("set WARRANT_EVIDENCE_OUT=evidence/test-manifest.json&& cargo test -q -p {c}{feat} --test warrant_evidence"));
    } else {
        l(format!("WARRANT_EVIDENCE_OUT=evidence/test-manifest.json cargo test -q -p {c}{feat} --test warrant_evidence"));
    }
    l("# Line 9: sabotage.  Each fault must make the suite of the claim it names report `not ok` (the shared checker, VOS-D099).".to_string());
    for e in &g.plan {
        let filter = format!("/{}/{}", e.tier, e.name);
        let tail = format!("../../../scripts/warrant_negctl.py -- cargo test -q -p {c}{feat} --test warrant_evidence");
        if windows {
            let mut x = format!("set WARRANT_EVIDENCE_FAULT={}&& ", e.fault);
            if let Some(a) = &e.attack {
                x.push_str(&format!("set WARRANT_EVIDENCE_ATTACK={a}&& "));
            }
            x.push_str(&format!(
                "set WARRANT_EVIDENCE_FILTER={filter}&& python {tail}"
            ));
            l(x);
        } else {
            let mut x = format!("WARRANT_EVIDENCE_FAULT={} ", e.fault);
            if let Some(a) = &e.attack {
                x.push_str(&format!("WARRANT_EVIDENCE_ATTACK={a} "));
            }
            x.push_str(&format!("WARRANT_EVIDENCE_FILTER={filter} python3 {tail}"));
            l(x);
        }
    }
    if !windows {
        l(format!("# VOS-D070: line coverage of src/ must stay at or above `coverage-floor` ({}) of vos.toml; Linux only (tarpaulin).", g.floor));
        l(format!(
            "if [ \"$(uname)\" = \"Linux\" ]; then cargo tarpaulin -p {c}{feat} --lib --test behaviour --ignore-tests --engine llvm --include-files '{c}/src/*' --fail-under {} --skip-clean; fi",
            g.floor
        ));
    }
    l(
        "# The component kit's check, last: it reads the manifest the run above rewrote."
            .to_string(),
    );
    l(format!(
        "cargo run -q --manifest-path {vl} -- check-component ."
    ));
    l(MARKER.to_string());
    s
}

/// The canonical region of `BUILD` and `BUILD_windows` of a **contract crate** (VOS-039b-1,
/// VOS04 5.6), ending with [`MARKER`] and a newline.  The two files are identical: a contract has
/// no evidence harness, so there is no environment assignment, no python line and nothing that
/// cmd and sh would spell differently.  Every line is exactly one command.
///
/// ```text
///   unit tests (lib and every integration test), doc tests      the trait is declarations only
///   vos-shape-check, the capability analyzer                    the K-class rules
///   vos-link check-contract .      the lemmas, the mutants, the config mutants, the table
///   vos-model-sat check .          ONLY when vos.toml says `sat = "model"` (VOS31 10.1)
///   vos-link check-component .     last: this file byte for byte, the file set, the row
/// ```
///
/// `sat` is the parsed `vos.toml` key of VOS31 10.3.  `None` (the key is absent) generates
/// exactly the region this function wrote before VOS-040c, so every contract crate that does
/// not opt in keeps its BUILD files byte for byte.
pub fn canonical_contract_build(krate: &str, sat: Option<SatLane>) -> String {
    let vl = "../../../programs/rust/vos-link/Cargo.toml";
    let mut s = String::new();
    let mut l = |line: String| {
        s.push_str(&line);
        s.push('\n');
    };
    l(
        "# GENERATED by `vos-link gen-build` (VOS04 5.6) for a contract crate, from vos.toml."
            .to_string(),
    );
    l("# Edit vos.toml and run gen-build: `vos-link check-component` requires this region byte for byte.".to_string());
    l(
        "# Your own lines go after the marker at the end; they run but satisfy no requirement."
            .to_string(),
    );
    l(format!(
        "cargo test -p {krate} --lib --tests -- --nocapture"
    ));
    l(format!("cargo test -p {krate} --doc"));
    l("cargo run -q -p vos-shape-check -- .".to_string());
    l("cargo run -q -p coding_adventures_ca_capability_analyzer -- --dir .".to_string());
    l(format!(
        "cargo run -q --manifest-path {vl} -- check-contract ."
    ));
    if sat == Some(SatLane::Model) {
        l(SAT_COMMENT.to_string());
        l(SAT_LINE.to_string());
    }
    l(format!(
        "cargo run -q --manifest-path {vl} -- check-component ."
    ));
    l(MARKER.to_string());
    s
}

/// The comment above [`SAT_LINE`]: it says why the line exists and what it is NOT.
pub const SAT_COMMENT: &str = "# VOS31 10.1: the host model-SAT lane (sat = \"model\" in vos.toml). It certifies the LEMMAS of this contract's model, not any implementation.";

/// The model-SAT lane's BUILD line (VOS31 10.1), the same text in `BUILD` and `BUILD_windows`.
/// `--release`: SAT in a debug build is slow.  The lane lives in a standalone cargo root, so the
/// line names its manifest the way the `vos-link` lines do, and it runs from the crate directory
/// (the loader refuses a `..` in the crate path).  `vos-link` cannot CALL the lane (the Cargo
/// edge runs the other way: `vos-model-sat` depends on `vos-link`), so `check-contract` rule
/// CK11 checks that this line is present and that the committed table is well formed, and the
/// line itself does the solving.
pub const SAT_LINE: &str = "cargo run -q --release --manifest-path ../../../programs/rust/vos-model-sat/Cargo.toml -- check .";

/// `vos-link gen-build` for a contract crate: write both BUILD files, keeping the author's
/// lines below the marker.
fn gen_build_contract(
    layout: &crate::inputs::Layout,
    vt: &vos_shape_check::manifest::VosToml,
    mut out: Outcome,
) -> Result<Outcome, NoVerdict> {
    let mut wrote = Vec::new();
    for name in ["BUILD", "BUILD_windows"] {
        let existing = read_text(&layout.impl_dir.join(name), 256 * 1024)?.unwrap_or_default();
        let text = format!(
            "{}{}",
            canonical_contract_build(&layout.impl_pkg, vt.sat),
            trailer_of(&existing)
        );
        if text != existing {
            write_replacing(&layout.impl_dir, name, text.as_bytes())?;
            wrote.push(name.to_string());
        }
    }
    out.action = Some(if wrote.is_empty() {
        "unchanged BUILD, BUILD_windows".to_string()
    } else {
        format!("wrote {}", wrote.join(", "))
    });
    Ok(out)
}

/// The generated tolerance script, if the crate declares `excused-controls`.
pub fn tolerance_script(g: &GenInputs) -> Option<String> {
    if g.excused.is_empty() {
        return None;
    }
    let c = &g.krate;
    let feat = features_arg(g);
    let ids = g
        .excused
        .iter()
        .map(|e| e.id.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let why = g
        .excused
        .iter()
        .map(|e| format!("#   {} ({})\n", e.id, e.reason))
        .collect::<String>();
    Some(format!(
        "#!/bin/sh\n\
# GENERATED by `vos-link gen-build` (VOS04 5.6) from `excused-controls` in vos.toml.  Do not edit:\n\
# `vos-link check-component` requires this file byte for byte.\n\
#\n\
# Runs the generated suite and rewrites the manifest.  Exactly the controls listed below may fail\n\
# (VOS-D065, VOS-D098: a claim that cannot see the fault); anything else failing, or one of them\n\
# passing, or no test passing at all, is an error.  Each excuse names its backlog row, and\n\
# `vos-link` refuses it once that row is not open or queued:\n\
{why}\
set -eu\n\
\n\
known_ids='{ids}'\n\
\n\
status=0\n\
out=$(WARRANT_EVIDENCE_OUT=evidence/test-manifest.json cargo test -q -p {c}{feat} --test warrant_evidence 2>/dev/null) || status=$?\n\
\n\
ok=$(printf '%s\\n' \"$out\" | grep -c '^ok ' || true)\n\
failed=$(printf '%s\\n' \"$out\" | sed -n 's/^not ok [0-9][0-9]* - //p' | sort)\n\
expected=$(printf '%s\\n' \"$known_ids\" | sort)\n\
\n\
echo \"evidence: cargo status $status, $ok ok\"\n\
if [ \"$status\" -ne 1 ] || [ \"$ok\" -eq 0 ] || [ \"$failed\" != \"$expected\" ]; then\n\
    echo \"evidence: expected status 1, some ok and exactly these failures:\" >&2\n\
    printf '%s\\n' \"$expected\" >&2\n\
    echo \"evidence: got these failures:\" >&2\n\
    printf '%s\\n' \"$failed\" >&2\n\
    exit 1\n\
fi\n"
    ))
}

// ---------------------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------------------

/// Read everything generation needs.  `Ok(Err(findings))` is "the inputs are wrong".
pub fn load_inputs(
    layout: &crate::inputs::Layout,
    vt: &vos_shape_check::manifest::VosToml,
) -> Result<Result<GenInputs, Vec<Finding>>, NoVerdict> {
    let mut f: Vec<Finding> = Vec::new();
    let krate = layout.impl_pkg.clone();
    let Some(floor) = vt.coverage_floor else {
        f.push(Finding::new(
            "CC07",
            "coverage: vos.toml declares no `coverage-floor` (VOS04 5.6 item G, VOS-D070)",
        ));
        return Ok(Err(f));
    };
    if floor < crate::component::COVERAGE_FLOOR_MIN {
        f.push(Finding::new(
            "CC07",
            format!(
                "coverage: coverage-floor = {floor} is below the repository minimum {}",
                crate::component::COVERAGE_FLOOR_MIN
            ),
        ));
        return Ok(Err(f));
    }

    // evidence.bindings, by the evidence generator's own reader.
    let mut faults: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let (mut property_cases, mut fuzz_iterations) = (0, 0);
    match read_text(
        &layout.impl_dir.join("evidence.bindings"),
        BINDINGS_MAX_BYTES,
    )? {
        None => f.push(Finding::new("CC04", "evidence.bindings: not found")),
        Some(t) => match parse_evidence_bindings(&t) {
            Ok(b) => {
                for (op, ob) in &b.manifest.bindings {
                    faults.insert(op.clone(), ob.negative_controls.clone());
                }
                property_cases = b.property_cases;
                fuzz_iterations = b.fuzz_iterations;
            }
            Err(e) => f.push(Finding::new(
                "CC04",
                format!("evidence.bindings: {}", esc(&e)),
            )),
        },
    }

    // The document's claims and examples, and which bound operations each claim calls.
    let doc_path = layout.specs_dir().join(layout.doc_name());
    let (mut claims, mut examples, mut calls) = (Vec::new(), Vec::new(), Vec::new());
    match read_text(&doc_path, DOC_MAX_BYTES)? {
        None => f.push(Finding::new(
            "CC04",
            format!(
                "{}: not found (run `vos-link gen-doc`)",
                layout.doc_display()
            ),
        )),
        Some(t) => {
            let ops: Vec<String> = faults.keys().cloned().collect();
            let h = std::thread::Builder::new()
                .stack_size(WARRANT_STACK_BYTES)
                .spawn(move || (declared_names(&t), claim_calls(&t, &ops)))
                .map_err(|e| NoVerdict::new(format!("cannot start the Warrant thread: {e}")))?;
            match h
                .join()
                .map_err(|_| NoVerdict::new("the Warrant thread panicked"))?
            {
                (Ok((c, e)), Ok(k)) => {
                    claims = c;
                    examples = e;
                    calls = k;
                }
                (Err(e), _) | (_, Err(e)) => f.push(Finding::new(
                    "CC04",
                    format!("{}: {}", layout.doc_display(), esc(&e.to_string())),
                )),
            }
        }
    }

    // Each excuse names a backlog row that exists and is not done (review M4).
    if !vt.excused_controls.is_empty() {
        let backlog = layout.specs_dir().join("VOS-BACKLOG.md");
        let text = read_text(&backlog, BACKLOG_MAX_BYTES)?;
        for x in &vt.excused_controls {
            let problem = match backlog_row_status(text.as_deref().unwrap_or(""), &x.reason) {
                RowState::Open => continue,
                RowState::Missing => format!(
                    "{} is not a row of code/specs/VOS-BACKLOG.md",
                    x.reason
                ),
                RowState::Duplicate => format!(
                    "{} is the first cell of more than one backlog row, so the reference is ambiguous",
                    x.reason
                ),
                RowState::Closed(st) => format!(
                    "{} has status {}, which is not open or queued: remove the excuse (the control must now pass)",
                    x.reason,
                    esc(&st)
                ),
            };
            f.push(Finding::new(
                "CC04",
                format!("vos.toml: excuse {}: {problem}", esc(&x.id)),
            ));
        }
    }

    // The plan.
    let mut plan = Vec::new();
    match read_text(&layout.impl_dir.join(PLAN_FILE), PLAN_MAX_BYTES)? {
        None => f.push(Finding::new("CC04", format!("{PLAN_FILE}: not found"))),
        Some(t) => {
            let (entries, problems) = parse_plan(&t);
            f.extend(problems.into_iter().map(|m| Finding::new("CC04", m)));
            if f.is_empty() {
                for m in validate_plan(
                    &entries,
                    &faults,
                    &claims,
                    &examples,
                    &calls,
                    &vt.excused_controls,
                ) {
                    f.push(Finding::new("CC04", m));
                }
            }
            plan = entries;
        }
    }
    if !f.is_empty() {
        return Ok(Err(f));
    }
    let g = GenInputs {
        krate,
        features: vt.evidence_features.clone(),
        floor,
        excused: vt.excused_controls.clone(),
        plan,
        claims,
        examples,
        faults,
        calls,
        property_cases,
        fuzz_iterations,
    };
    let bad = interpolation_problems(&g);
    if !bad.is_empty() {
        return Ok(Err(bad
            .into_iter()
            .map(|m| Finding::new("CC04", m))
            .collect()));
    }
    Ok(Ok(g))
}

/// What the backlog says about a row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RowState {
    /// No line starts `| <id> |`.
    Missing,
    /// More than one line does: the reference is ambiguous.
    Duplicate,
    /// The status cell begins `open` or `queued` (after `*` markup): an excuse may cite it.
    Open,
    /// Anything else (`done in ..`, `resolved ..`, `withdrawn`, `partly done`, ...), with the
    /// cell as written.  An ALLOWLIST: a status that is not known to be open is not open.
    Closed(String),
}

/// Read the status of backlog row `id` (`VOS-Dnnn`).  A row is a line starting `| <id> |`; its
/// status is the last cell.  The vocabulary of open D rows in VOS-BACKLOG.md is `queued` and
/// `open` (no D row uses `in progress`).
pub fn backlog_row_status(backlog: &str, id: &str) -> RowState {
    let head = format!("| {id} |");
    let mut rows = backlog.lines().filter(|l| l.starts_with(&head));
    let Some(row) = rows.next() else {
        return RowState::Missing;
    };
    if rows.next().is_some() {
        return RowState::Duplicate;
    }
    let cell = row
        .trim_end()
        .trim_end_matches('|')
        .rsplit('|')
        .next()
        .unwrap_or("");
    let status = cell.trim().trim_matches('*').trim();
    let lower = status.to_ascii_lowercase();
    if lower.starts_with("open") || lower.starts_with("queued") {
        RowState::Open
    } else {
        RowState::Closed(status.to_string())
    }
}

/// What follows the marker line of an existing BUILD file: the author's own lines.
fn trailer_of(existing: &str) -> &str {
    let needle = format!("{MARKER}\n");
    match existing.find(&needle) {
        Some(i) if i == 0 || existing.as_bytes()[i - 1] == b'\n' => &existing[i + needle.len()..],
        _ => "",
    }
}

/// `vos-link gen-build <crate-dir>`.
pub fn gen_build(crate_arg: &Path, root_arg: Option<&Path>) -> Result<Outcome, NoVerdict> {
    let layout = resolve_layout(crate_arg, root_arg)?;
    ensure_plain_dirs(&layout.root, &["code", "packages", "rust"])?;
    let mut out = Outcome::default();
    let vt = match read_component_vos_toml(&layout)? {
        Ok(v) => v,
        Err(finding) => {
            out.findings.push(finding);
            return Ok(out);
        }
    };
    out.header = format!("crate {}", layout.impl_pkg);
    if vt.kind == Some(vos_shape_check::manifest::Kind::Contract) {
        return gen_build_contract(&layout, &vt, out);
    }
    let g = match load_inputs(&layout, &vt)? {
        Ok(g) => g,
        Err(findings) => {
            out.findings = findings;
            return Ok(out);
        }
    };
    let mut wrote = Vec::new();
    for (name, windows) in [("BUILD", false), ("BUILD_windows", true)] {
        let existing = read_text(&layout.impl_dir.join(name), 256 * 1024)?.unwrap_or_default();
        let text = format!("{}{}", canonical_build(&g, windows), trailer_of(&existing));
        if text != existing {
            write_replacing(&layout.impl_dir, name, text.as_bytes())?;
            wrote.push(name.to_string());
        }
    }
    if let Some(script) = tolerance_script(&g) {
        let existing = read_text(&layout.impl_dir.join(TOLERANCE_SCRIPT), 64 * 1024)?;
        if existing.as_deref() != Some(script.as_str()) {
            write_replacing(
                &layout.impl_dir.join("warrant"),
                "check_evidence.sh",
                script.as_bytes(),
            )?;
            wrote.push(TOLERANCE_SCRIPT.to_string());
        }
    }
    out.action = Some(if wrote.is_empty() {
        "unchanged BUILD, BUILD_windows".to_string()
    } else {
        format!("wrote {}", wrote.join(", "))
    });
    Ok(out)
}
